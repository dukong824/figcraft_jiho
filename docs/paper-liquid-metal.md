# Paper Liquid Metal (local native extension)

This build adds Paper's Apache-2.0 Liquid Metal material as a native effect. Select an image,
vector or frame and choose **Liquid Metal** from **+** in **Design → Custom** (**디자인 → 커스텀**). This adds
**Liquid Metal** (**리퀴드 메탈**) and opens its settings. Custom is a peer of Fill, Stroke and
Effects in the right panel. Open existing settings with the effect icon.
Speed 0 pauses; a negative speed reverses motion.
Numeric settings have sliders with live updates, alongside direct number entry. A drag is one
undo step; the track respects each parameter's range. Frame defaults to a ten-second track,
which expands to accommodate longer or negative times typed into the field.
The canvas and prototype preview animate, and `.figcraft` saves the effect parameters.

Existing documents automatically show their Paper material in Custom. The saved effect stack,
parameters, rendering order and command API are unchanged; no file migration is needed.

The original GLSL rendering formulas are stored in `crates/render/src/paper-shaders/`, with
LICENSE, NOTICE, commit and hashes. The WebGPU host changes resource declarations and the
combined sampler signature. The alpha-mask and red/black SOR preprocessing is translated to Rust
with the same working resolution, iteration count, constants and update order. Native image
resampling can differ slightly from a browser's Canvas implementation; this is not a pixel-identical
browser emulation. The source layer is capped at 1024 pixels on its longest side (512 in the live canvas), and processing
is capped at 4,194,304 pixels for extreme aspect ratios.

Paper uses **alpha**, not the photograph's RGB luminance, to construct the surface. A photograph
with an opaque background becomes a rectangular metal surface. Transparent PNGs, vector shapes
and text expose their own silhouette. This effect does not remove image backgrounds.

Parameters: tint and background colours, speed, starting frame in milliseconds, repetition,
softness, distortion, contour, angle, red/blue channel shifts, scale and rotation. Disable
**Use layer alpha** to use Paper's built-in None/Circle/Daisy/Diamond/Metaballs shapes.
Edits use `effect.add` / `effect.set`, including undo and command/control access:

```json
{"command":"effect.add","params":{"type":"paperLiquidMetal","ids":[2]}}
```

Native Windows rendering uses Direct3D 12 (OpenGL fallback), following the application's existing
Windows backend policy; `WGPU_BACKEND` may override it. Failed GPU initialization logs an error and
leaves the source visible. This delivered build targets Windows x64. The web target retains the
parameters and source artwork but does not render this native GPU effect.

PNG/JPEG/WebP/SVG/PDF exports are still frames (starting frame). SVG and PDF embed the material
as a raster image. `.fig` and Android Vector Drawable have no matching material and warn that
the effect is omitted. Open native documents in this custom build: the official unmodified
FigCraft build does not understand this new effect type.

## Provenance

- FigCraft base: `storytold/figcraft` at `43af145a12eacd1e87f5f5e6984c49b7da040288`.
- Paper: `paper-design/shaders` at `1923e9f536ea079e2a95d23291cbfacfd69d37f6`.
- Paper Copyright Lost Coast Labs, Inc.; Apache-2.0. See the included LICENSE and NOTICE.
