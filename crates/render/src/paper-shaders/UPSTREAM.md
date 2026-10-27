# Paper Shaders

Repository: https://github.com/paper-design/shaders
Commit: 1923e9f536ea079e2a95d23291cbfacfd69d37f6

These GLSL files contain the original vertex and liquid-metal fragment shader. The fragment's four shared template strings are expanded verbatim from shader-utils.ts. Rendering formulas are unchanged. Runtime adaptation to WebGPU bindings is in paper_metal/gpu.rs. Image preprocessing in paper_metal/mod.rs ports toProcessedLiquidMetal and its original red/black SOR solver.

Upstream fragment SHA-256 (expanded): d8db52fe5c9fa361ebb6364360a674ce4ee31df2deb0222bf12dd82e5384db7e
Upstream vertex SHA-256: d5452769f5dc08d3a378e77611f42343d78c7962fc7983d9508126dc1b8c65d2

## Image Filters

The seven Image Filters listed at shaders.paper.design are extracted from the same pinned commit.
Shared shader-utils template literals and numeric metadata constants are expanded verbatim.
No additional Logo Animations or Effects shaders are included. The Apache-2.0 noise texture is
decoded unchanged from get-shader-noise-texture.ts. Defaults and parameter ranges are taken from
the shader comments and React default presets. WebGPU resource adaptation is in paper_filters/gpu.rs.

SHA-256 of expanded sources and the decoded texture:

```
paper-texture.frag.glsl: 2f6d919f6d4a6daee657a2d59a5cef8980b25bc75a44d17f3cdb307f33ab5803
fluted-glass.frag.glsl: 768eb2b42bc1e27f8757c16dbe5472c455b8e81622c0ec5ab02a3d5ca4e38126
water.frag.glsl: faea240e90aa18ac0d38be392391d5c9b1117802279e00bc17b5f4135059a957
image-dithering.frag.glsl: b1758de74abd3e6bcadded8a2bfc18aa02d48731d2e907506958d562e1a97102
halftone-dots.frag.glsl: a58006c62c9b06f873b4aac70090820032f85b5cfc834c35e4237eb6cf7fd8c5
halftone-cmyk.frag.glsl: 48f8af4ce66f572b05340a38bb05b0ecf570a782f9ad39f8baa8951bcf69df18
lens-distortion.frag.glsl: a2efb4e8b1b61ae97cf8198c218a0f53c41606cf791fa040764fa35e8a8a8cb1
noise.png: 5116a06c428a75e2db9bd55062c560bb02600383ee54da007f1628e845b2b73a
```
