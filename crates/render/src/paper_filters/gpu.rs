//! WebGPU host for Paper's original GLSL. Only resource bindings and GLSL version change.
use figcraft_doc::{PaperFilterKind, PaperImageFilter};
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

pub(super) fn render(source: &Pixmap, params: &PaperImageFilter, time: f64) -> Result<Pixmap, String> {
    if source.width() > 2048 || source.height() > 2048 {
        return Err("Paper source exceeds the 2048-pixel GPU limit".into());
    }
    let gpu = GPU.get_or_init(|| Gpu::new().map(Mutex::new));
    let mut guard = gpu.as_ref().map_err(Clone::clone)?.lock().map_err(|_| "GPU lock is unavailable".to_owned())?;
    guard.render(source, params, time)
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: HashMap<PaperFilterKind, wgpu::RenderPipeline>,
    bind_layout: wgpu::BindGroupLayout,
    noise: wgpu::Texture,
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
const COMMON: [(&str, &str); 14] = [
    ("resolution", "vec2"),
    ("pixelRatio", "float"),
    ("imageAspectRatio", "float"),
    ("time", "float"),
    ("isImage", "bool"),
    ("originX", "float"),
    ("originY", "float"),
    ("worldWidth", "float"),
    ("worldHeight", "float"),
    ("fit", "float"),
    ("scale", "float"),
    ("rotation", "float"),
    ("offsetX", "float"),
    ("offsetY", "float"),
];
fn uniform_names(kind: PaperFilterKind) -> Vec<(&'static str, &'static str)> {
    let mut names = COMMON.to_vec();
    for p in kind.params() {
        if !COMMON.iter().any(|(name, _)| *name == p.name) {
            names.push((p.name, if p.boolean { "bool" } else { "float" }));
        }
    }
    for c in kind.colors() {
        names.push((c.name, "vec4"));
    }
    names
}
fn fragment(kind: PaperFilterKind) -> &'static str {
    match kind {
        PaperFilterKind::PaperTexture => include_str!("../paper-shaders/paper-texture.frag.glsl"),
        PaperFilterKind::FlutedGlass => include_str!("../paper-shaders/fluted-glass.frag.glsl"),
        PaperFilterKind::Water => include_str!("../paper-shaders/water.frag.glsl"),
        PaperFilterKind::ImageDithering => include_str!("../paper-shaders/image-dithering.frag.glsl"),
        PaperFilterKind::HalftoneDots => include_str!("../paper-shaders/halftone-dots.frag.glsl"),
        PaperFilterKind::HalftoneCmyk => include_str!("../paper-shaders/halftone-cmyk.frag.glsl"),
        PaperFilterKind::LensDistortion => include_str!("../paper-shaders/lens-distortion.frag.glsl"),
    }
}
/// Expand WebGL implicit uniforms into explicit std140 slots. Formulas remain upstream GLSL.
fn adapt(source: &str, kind: PaperFilterKind, fragment: bool) -> String {
    let mut result = String::from("#version 450\nlayout(set=0,binding=0,std140) uniform PaperUniforms { vec4 data[64]; } figcraftUniforms;\n");
    for (i, (name, ty)) in uniform_names(kind).iter().enumerate() {
        let access = match *ty {
            "bool" => format!("(figcraftUniforms.data[{i}].x > 0.5)"),
            "vec2" => format!("figcraftUniforms.data[{i}].xy"),
            "vec4" => format!("figcraftUniforms.data[{i}]"),
            _ => format!("figcraftUniforms.data[{i}].x"),
        };
        result.push_str(&format!("#define u_{name} {access}\n"));
    }
    if fragment {
        result.push_str("layout(set=0,binding=1) uniform texture2D paperImage;\nlayout(set=0,binding=2) uniform sampler paperSampler;\nlayout(set=0,binding=3) uniform texture2D paperNoise;\nlayout(set=0,binding=4) uniform sampler noiseSampler;\n#define u_image sampler2D(paperImage,paperSampler)\n#define u_noiseTexture sampler2D(paperNoise,noiseSampler)\n");
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
    // Naga needs separate resource handles: helpers only ever receive the source image.
    result = result
        .replace("samplePremultiplied(sampler2D tex, ", "samplePremultiplied(")
        .replace("getBlur(sampler2D tex, ", "getBlur(")
        .replace("samplePremultiplied(tex, ", "samplePremultiplied(")
        .replace("getBlur(u_image, ", "getBlur(")
        .replace("texture(tex, ", "texture(u_image, ");
    if fragment && result.contains("gl_FragCoord") {
        result = result.replace("gl_FragCoord", "paperFragCoord").replace(
            "void main() {",
            "void main() {\nvec4 paperFragCoord = vec4(gl_FragCoord.x, u_resolution.y - gl_FragCoord.y, gl_FragCoord.zw);\n",
        );
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
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("Paper Image Filters"), ..Default::default() }))
                .map_err(|e| e.to_string())?;
        device.on_uncaptured_error(Arc::new(|e| log::error!("Paper GPU: {e}")));
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original Paper vertex"),
            source: wgpu::ShaderSource::Glsl {
                shader: Cow::Owned(adapt(include_str!("../paper-shaders/paper.vert.glsl"), PaperFilterKind::PaperTexture, false)),
                stage: wgpu::naga::ShaderStage::Vertex,
                defines: &[],
            },
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Paper filters"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Paper filters"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let mut pipelines = HashMap::new();
        for kind in PaperFilterKind::ALL {
            let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Original Paper Image Filters"),
                source: wgpu::ShaderSource::Glsl {
                    shader: Cow::Owned(adapt(fragment(kind), kind, true)),
                    stage: wgpu::naga::ShaderStage::Fragment,
                    defines: &[],
                },
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Paper Image Filters"),
                layout: Some(&pipeline_layout),
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
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
            pipelines.insert(kind, pipeline);
        }
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(e.to_string());
        }
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
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
        log::info!("Paper Image Filters GPU: {}", adapter.get_info().name);
        let decoded = image::load_from_memory(include_bytes!("../paper-shaders/noise.png")).map_err(|e| e.to_string())?.to_rgba8();
        let noise = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Original Paper noise"),
            size: wgpu::Extent3d { width: decoded.width(), height: decoded.height(), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &noise, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            decoded.as_raw(),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(decoded.width() * 4), rows_per_image: Some(decoded.height()) },
            wgpu::Extent3d { width: decoded.width(), height: decoded.height(), depth_or_array_layers: 1 },
        );
        Ok(Self { device, queue, pipelines, bind_layout, noise, uniforms, quad, sampler, sources: HashMap::new() })
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

    fn render(&mut self, source: &Pixmap, p: &PaperImageFilter, time: f64) -> Result<Pixmap, String> {
        let (w, h) = (source.width(), source.height());
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        (w, h).hash(&mut hash);
        source.data().hash(&mut hash);
        let key = hash.finish();
        if !self.sources.contains_key(&key) {
            // Browser image uploads are straight alpha; tiny-skia stores premultiplied RGB.
            let mut processed = source.data().to_vec();
            for px in processed.chunks_exact_mut(4) {
                let alpha = u32::from(px[3]);
                for channel in px.iter_mut().take(3) {
                    *channel = (u32::from(*channel) * 255 + alpha / 2).checked_div(alpha).unwrap_or(0).min(255) as u8;
                }
            }
            let tex = self.texture(w, h, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &processed,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            if self.sources.len() >= 8 {
                self.sources.clear();
            }
            self.sources.insert(key, tex);
        }
        let tex = self.sources.get(&key).ok_or("Prepared image missing")?;
        let names = uniform_names(p.filter);
        let mut values = vec![0.0_f32; 256];
        for (i, (name, _)) in names.iter().enumerate() {
            let value = match *name {
                "resolution" => [w as f32, h as f32, 0.0, 0.0],
                "pixelRatio" => [1.0, 0.0, 0.0, 0.0],
                "imageAspectRatio" => [w as f32 / h as f32, 0.0, 0.0, 0.0],
                "time" => [p.clock(time), 0.0, 0.0, 0.0],
                "isImage" => [1.0, 0.0, 0.0, 0.0],
                _ => {
                    if let Some(spec) = p.filter.params().iter().find(|s| s.name == *name) {
                        [p.value(spec) as f32, 0.0, 0.0, 0.0]
                    } else if let Some(spec) = p.filter.colors().iter().find(|s| s.name == *name) {
                        let color = p.color(spec);
                        [color.r, color.g, color.b, color.a]
                    } else {
                        [0.0; 4]
                    }
                }
            };
            if let Some(slot) = values.get_mut(i * 4..i * 4 + 4) {
                slot.copy_from_slice(&value);
            }
        }
        self.queue.write_buffer(&self.uniforms, 0, &bytes(&values));
        let view = tex.create_view(&Default::default());
        let noise_view = self.noise.create_view(&Default::default());
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&noise_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.sampler) },
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
            pass.set_pipeline(self.pipelines.get(&p.filter).ok_or("Image filter pipeline missing")?);
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
