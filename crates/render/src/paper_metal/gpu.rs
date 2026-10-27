//! WebGPU host for Paper's original GLSL. Only resource bindings and GLSL version change.
use figcraft_doc::PaperLiquidMetal;
use std::{
    borrow::Cow,
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use tiny_skia::Pixmap;
use wgpu::util::DeviceExt;

static GPU: OnceLock<Result<Mutex<Gpu>, String>> = OnceLock::new();

pub(super) fn render(source: &Pixmap, params: &PaperLiquidMetal, time: f64) -> Result<Pixmap, String> {
    if source.width() > 2048 || source.height() > 2048 {
        return Err("Paper source exceeds the 2048-pixel GPU limit".into());
    }
    let gpu = GPU.get_or_init(|| Gpu::new().map(Mutex::new));
    let mut guard = gpu.as_ref().map_err(Clone::clone)?.lock().map_err(|_| "GPU lock is unavailable".to_owned())?;
    guard.render(source, &params.sanitized(), time)
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    quad: wgpu::Buffer,
    sampler: wgpu::Sampler,
    sources: HashMap<u64, wgpu::Texture>,
}

fn bytes(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// The upstream sources are kept verbatim in paper-shaders. This adapts the WebGL API's
/// implicit uniform locations and combined sampler to WebGPU's explicit bindings.
fn adapt(source: &str, fragment: bool) -> String {
    let uniforms = r#"layout(set=0,binding=0,std140) uniform PaperUniforms {
vec4 viewport; vec4 motion; vec4 material; vec4 sizing; vec4 position; vec4 flags; vec4 back; vec4 tint;
} paper;
#define u_resolution paper.viewport.xy
#define u_pixelRatio paper.viewport.z
#define u_imageAspectRatio paper.viewport.w
#define u_time paper.motion.x
#define u_repetition paper.motion.y
#define u_softness paper.motion.z
#define u_distortion paper.motion.w
#define u_contour paper.material.x
#define u_angle paper.material.y
#define u_shiftRed paper.material.z
#define u_shiftBlue paper.material.w
#define u_scale paper.sizing.x
#define u_rotation paper.sizing.y
#define u_originX paper.sizing.z
#define u_originY paper.sizing.w
#define u_offsetX paper.position.x
#define u_offsetY paper.position.y
#define u_worldWidth paper.position.z
#define u_worldHeight paper.position.w
#define u_fit paper.flags.x
#define u_isImage (paper.flags.y > 0.5)
#define u_shape paper.flags.z
#define u_colorBack paper.back
#define u_colorTint paper.tint
"#;
    let mut result = String::from("#version 450\n");
    result.push_str(uniforms);
    if fragment {
        result.push_str("layout(set=0,binding=1) uniform texture2D paperImage;\nlayout(set=0,binding=2) uniform sampler paperSampler;\n#define u_image sampler2D(paperImage,paperSampler)\n");
    }
    let varyings = ["v_objectUV", "v_objectBoxSize", "v_responsiveUV", "v_responsiveBoxGivenSize", "v_patternUV", "v_patternBoxSize", "v_imageUV"];
    for line in source.lines() {
        let trim = line.trim();
        if trim.starts_with("#version") || trim.starts_with("precision ") || trim.starts_with("uniform ") {
            continue;
        }
        if trim == "out vec4 fragColor;" {
            result.push_str("layout(location=0) out vec4 fragColor;\n");
            continue;
        }
        if let Some((location, _)) = varyings.iter().enumerate().find(|(_, v)| trim == format!("{} vec2 {v};", if fragment { "in" } else { "out" })) {
            result.push_str(&format!("layout(location={location}) {line}\n"));
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    // Naga uses separate image/sampler handles. Removing this opaque function parameter
    // leaves the nine samples and all their positions and weights exactly as upstream.
    if fragment {
        result = result
            .replace("blurEdge3x3(sampler2D tex, ", "blurEdge3x3(")
            .replace("blurEdge3x3(u_image, ", "blurEdge3x3(")
            .replace("textureSize(tex, ", "textureSize(paperImage, ")
            .replace("textureGrad(tex, ", "textureGrad(u_image, ");
    }
    result
}

impl Gpu {
    fn new() -> Result<Self, String> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        // Keep shader optimization enabled in the distributable dev-profile build too.
        // Direct3D's DEBUG flag otherwise disables optimization for every animated pixel.
        descriptor.flags.remove(wgpu::InstanceFlags::DEBUG);
        #[cfg(target_os = "windows")]
        {
            descriptor.backends = wgpu::Backends::from_env().unwrap_or(wgpu::Backends::DX12.union(wgpu::Backends::GL));
        }
        let instance = wgpu::Instance::new(descriptor);
        let adapter = pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default() }),
        )
        .map_err(|e| e.to_string())?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("Paper Liquid Metal"), ..Default::default() }))
                .map_err(|e| e.to_string())?;
        device.on_uncaptured_error(Arc::new(|e| log::error!("Paper GPU: {e}")));
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original Paper vertex"),
            source: wgpu::ShaderSource::Glsl {
                shader: Cow::Owned(adapt(include_str!("../paper-shaders/paper.vert.glsl"), false)),
                stage: wgpu::naga::ShaderStage::Vertex,
                defines: &[],
            },
        });
        let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original Paper Liquid Metal"),
            source: wgpu::ShaderSource::Glsl {
                shader: Cow::Owned(adapt(include_str!("../paper-shaders/liquid-metal.frag.glsl"), true)),
                stage: wgpu::naga::ShaderStage::Fragment,
                defines: &[],
            },
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Paper Liquid Metal"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 0, shader_location: 0 }],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(e.to_string());
        }
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &bytes(&[-1., -1., 0., 1., 1., -1., 0., 1., -1., 1., 0., 1., -1., 1., 0., 1., 1., -1., 0., 1., 1., 1., 0., 1.]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        log::info!("Paper Liquid Metal GPU: {}", adapter.get_info().name);
        Ok(Self { device, queue, pipeline, uniforms, quad, sampler, sources: HashMap::new() })
    }

    fn texture(&self, w: u32, h: u32, usage: wgpu::TextureUsages) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        })
    }

    fn render(&mut self, source: &Pixmap, p: &PaperLiquidMetal, time: f64) -> Result<Pixmap, String> {
        let (w, h) = (source.width(), source.height());
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        (w, h).hash(&mut hash);
        for alpha in source.data().chunks_exact(4).filter_map(|px| px.get(3)) {
            alpha.hash(&mut hash);
        }
        let key = hash.finish();
        if !self.sources.contains_key(&key) {
            let processed = super::preprocess(source).ok_or("Could not preprocess layer alpha")?;
            let tex = self.texture(w, h, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                processed.as_raw(),
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            if self.sources.len() >= 8 {
                self.sources.clear();
            }
            self.sources.insert(key, tex);
        }
        let tex = self.sources.get(&key).ok_or("Prepared image missing")?;
        let clock = p.frame * 0.001 + if time.is_finite() { time * p.speed } else { 0.0 };
        let values = [
            w as f32,
            h as f32,
            1.0,
            w as f32 / h as f32,
            clock as f32,
            p.repetition as f32,
            p.softness as f32,
            p.distortion as f32,
            p.contour as f32,
            p.angle as f32,
            p.shift_red as f32,
            p.shift_blue as f32,
            p.scale as f32,
            p.rotation as f32,
            0.5,
            0.5,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            if p.use_image { 1.0 } else { 0.0 },
            p.shape as f32,
            0.0,
            p.color_back.r,
            p.color_back.g,
            p.color_back.b,
            p.color_back.a,
            p.color_tint.r,
            p.color_tint.g,
            p.color_tint.b,
            p.color_tint.a,
        ];
        self.queue.write_buffer(&self.uniforms, 0, &bytes(&values));
        let view = tex.create_view(&Default::default());
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        let target = self.texture(w, h, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
        let target_view = target.create_view(&Default::default());
        let row = (w * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row) * u64::from(h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.set_vertex_buffer(0, self.quad.slice(..));
            pass.draw(0..6, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let submission = self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: Some(Duration::from_secs(10)) })
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(10)).map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(e.to_string());
        }
        let mapped = buffer.slice(..).get_mapped_range().map_err(|e| e.to_string())?;
        let mut out = Pixmap::new(w, h).ok_or("Output allocation failed")?;
        for (dst, src) in out.data_mut().chunks_exact_mut(w as usize * 4).zip(mapped.chunks_exact(row as usize)) {
            dst.copy_from_slice(src.get(..w as usize * 4).ok_or("Invalid readback row")?);
        }
        drop(mapped);
        buffer.unmap();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_glsl_compiles_and_time_changes_native_pixels() {
        let mut source = Pixmap::new(64, 64).unwrap();
        source.fill(tiny_skia::Color::WHITE);
        let p = PaperLiquidMetal::default();
        let first = render(&source, &p, 0.0).unwrap();
        let later = render(&source, &p, 2.0).unwrap();
        assert_ne!(first.data(), later.data());
        let stopped = PaperLiquidMetal { speed: 0.0, ..p };
        assert_eq!(render(&source, &stopped, 0.0).unwrap().data(), render(&source, &stopped, 4.0).unwrap().data());
    }
}
