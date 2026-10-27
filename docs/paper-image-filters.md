# Paper Image Filters

Select an image, vector, text or frame, then open **Design → Custom → + → Image Filters**
(**디자인 → 커스텀 → + → 이미지 필터**) and choose a filter. The seven filters from Paper's
Image Filters category are included:

| Filter | Korean label | Behavior |
|---|---|---|
| Paper Texture | 종이 질감 | Grain, fibers, folds, wrinkles, crumples and image blending |
| Fluted Glass | 플루티드 글라스 | Ribbed glass grids, distortion shapes, blur, highlights and margins |
| Water | 물결 | Animated caustics, waves, highlights and distortion |
| Image Dithering | 이미지 디더링 | Random / 2×2 / 4×4 / 8×8 dithering, original colors and palettes |
| Halftone Dots | 하프톤 도트 | Square or hex grids; classic, gooey, holes and soft dots |
| Halftone CMYK | 하프톤 CMYK | Four ink colors, ink gain, flooding, grain and dot styles |
| Lens Distortion | 렌즈 왜곡 | Chromatic dispersion, lens bulge, swirl, focus and grain |

New filters open their controls immediately. Existing Liquid Metal remains in Custom. No other
Logo Animations or Effects shaders have been added. Filter rows can be switched, disabled,
removed, copied, pasted and reordered through the existing effect commands with undo.

Water animates on the native canvas and prototype preview. Speed 0 pauses; a negative speed
reverses motion. Frame is in milliseconds and determines the still export frame. The other six
filters are static, as in Paper. All exposed uniforms have their original defaults and ranges,
including colors and sizing controls. Color values can be changed with a picker or hex input.

`.figcraft` saves filter types and settings. PNG/JPEG/WebP export still frames; SVG and PDF
embed the filtered artwork as a raster. `.fig` and Android Vector Drawable warn when these
unsupported filters are omitted. The native canvas uses a maximum input side of 512 pixels;
still exports use 1024. The web target preserves the settings but shows the source artwork.

The original Apache-2.0 GLSL, expanded shared template literals and noise texture are pinned to
Paper commit `1923e9f536ea079e2a95d23291cbfacfd69d37f6`. See
`crates/render/src/paper-shaders/UPSTREAM.md`, LICENSE and NOTICE for source details and hashes.
Only WebGPU bindings, GLSL declarations, sampler helper arguments and framebuffer coordinates
are adapted. Native rasterization and texture sampling may differ slightly from the browser.

The command API uses a serialized effect such as:

```json
{"command":"effect.add","params":{"ids":[2],"effect":{"type":"paperImageFilter","filter":"water","speed":1,"params":{"waves":0.5,"caustic":0.3}}}}
```

Type aliases include `paperTexture`, `paperFlutedGlass`, `paperWater`, `paperImageDithering`,
`paperHalftoneDots`, `paperHalftoneCmyk` and `paperLensDistortion` for default insertion.
Unknown parameter and color names are rejected by the command API.
