# Attribution

Every asset bundled in this repository (images, icons, fonts, data files) is listed here with its
author, source and licence. `cargo xtask assets` fails if a file under `assets/`, `docs/brand/` or
`docs/images/` is missing from this list.

FigCraft's user-interface icons are drawn in code (`crates/ui-egui/src/icons.rs`): original work,
MIT OR Apache-2.0. No third-party icon sets are used.

## Assets in this repository

| Path | Author | Source | Licence | Notes |
|---|---|---|---|---|
| `assets/purpose/purpose.png`, `assets/purpose/purpose.ico` | Project owner | User-supplied Google Drive original, see `assets/purpose/README.md` | Owner-provided Purpose branding, authorized in the task on 2026-10-29 | Original PNG unchanged; ICO size/format variants only |
| `assets/app-icon/` (all files: `figcraft.svg`, `figcraft-small.svg`, `figcraft-*.png`, `figcraft.ico`, `hicolor/`) | The project owner (engraving-style drawing, vectorised) | Original work, see [`assets/app-icon/README.md`](assets/app-icon/README.md); derived files rendered by `cargo xtask icon` | MIT OR Apache-2.0, [`assets/app-icon/LICENSE.txt`](assets/app-icon/LICENSE.txt) | FigCraft app icon (chameleon) |
| `docs/brand/artcraft-logo.svg`, `docs/brand/artcraft-logo.png`, `docs/brand/artcraft-logo-white.svg`, `docs/brand/artcraft-logo-white.png`, `docs/brand/artcraft-mark.svg`, `docs/brand/artcraft-mark.png`, `docs/brand/artcraft-mark-black.svg`, `docs/brand/artcraft-mark-black.png` | ArtCraft Team | storytold/craftrules `assets/brand/` | ArtCraft brand terms (`docs/brand/LICENSE-brand.txt`), not open source | Trademarks; unmodified use within FigCraft only |
| `docs/images/` (all screenshots and exports: `shot-*.png`, `art-*.png`) | FigCraft contributors | Screenshots of FigCraft showing synthetic demo documents made in FigCraft, and PNG exports of those documents by FigCraft's renderer | MIT OR Apache-2.0 | No third-party artwork |
| `examples/*.figcraft` | FigCraft contributors | Synthetic demo documents built through FigCraft's command API; invented copy, shapes and vectors drawn from scratch, no images or embedded fonts | MIT OR Apache-2.0 | Text uses Inter (OFL, from craft-fonts or the system); no third-party artwork |

## Fonts (build input, not in this repository)

Release builds embed fonts from the optional craft-fonts build input
([storytold/craft-fonts](https://github.com/storytold/craft-fonts), `CRAFT_FONTS_DIR`). Its own
`ATTRIBUTION.md` and licence files are authoritative; the packages ship each font's licence.

| Family | Licence | Upstream |
|---|---|---|
| Inter | SIL Open Font License 1.1 | https://github.com/rsms/inter |
| Archivo | SIL Open Font License 1.1 | [Google Fonts / Archivo](https://github.com/google/fonts/tree/6c70c829f09ea345d3590406693220ea35c6553f/ofl/archivo) |
| Roboto, Montserrat, Lato, Open Sans, Poppins, Nunito Sans, Merriweather, Playfair Display, Carlito, Caladea | SIL Open Font License 1.1 | Google Fonts / upstream repositories (see craft-fonts) |
| Liberation Sans, Serif, Mono | SIL Open Font License 1.1 | https://github.com/liberationfonts |
| BIZ UDPGothic, BIZ UDMincho, Shippori Mincho, Noto Sans Arabic, Noto Sans Georgian | SIL Open Font License 1.1 | Google Fonts (see craft-fonts) |

The central catalog also contains Adobe-authored Source Sans 3 and Source Serif 4.
FigCraft filters both families from every build in accordance with its asset policy.

egui's default fonts (compiled into the `egui`/`epaint` crates) keep their own licences (OFL / Apache-2.0 / Ubuntu Font Licence); see those crates.

## egui dependency font icons

| Asset | Author | Source | Licence | Use |
|---|---|---|---|---|
| `epaint_default_fonts::EGUI_ICONS`, `epaint_default_fonts::EMOJI_ICON` (dependency font inputs) | John Slegers | [egui 6d518fc7](https://github.com/storytold/egui/tree/6d518fc799681571963842f95b55a6b68431b15a/crates/epaint_default_fonts/fonts), [emoji-icon-font](https://github.com/jslegers/emoji-icon-font) | MIT (Copyright (c) 2014 John Slegers; full upstream text in `NOTICE`) | egui-icons.ttf is the five-glyph subset, included by default_fonts independently of monochrome_emoji_fonts; emoji-icon-font.ttf is the full monochrome fallback. Dependency assets unmodified here; no font file copied into this repository. |


## figcraft_jiho demo

| Path | Author / source | License | Notes |
|---|---|---|---|
| `examples/jiho/silhouette.png` | Original geometry made for this FigCraft fork | MIT OR Apache-2.0 | Synthetic six-lobed silhouette; no third-party artwork |
| `examples/jiho/Demo.figcraft` | Original demo made for this FigCraft fork | MIT OR Apache-2.0 | Uses the silhouette above and system-rendered text; no font files embedded |
| `examples/jiho/filter-source.png` | Original procedural landscape made for this FigCraft fork | MIT OR Apache-2.0 | Geometric gradient illustration, generated from original shapes; no third-party photo |
| `examples/jiho/Image Filters.figcraft` | Original demo made for this FigCraft fork | MIT OR Apache-2.0 | The landscape above with seven Paper filters and system-rendered text; no font files embedded |

## Paper Image Filters

| Path | Author / source | License | Notes |
|---|---|---|---|
| `crates/render/src/paper-shaders/noise.png` | Lost Coast Labs, Inc. / [Paper Shaders, commit 1923e9f5](https://github.com/paper-design/shaders/blob/1923e9f536ea079e2a95d23291cbfacfd69d37f6/packages/shaders/src/get-shader-noise-texture.ts) | Apache-2.0 | Original 128 × 128 noise texture decoded unchanged from the source data URL; LICENSE, NOTICE and SHA-256 are beside it |

## BiRefNet (downloaded on demand)

| Asset / path | Author / source | License | Notes |
|---|---|---|---|
| `BiRefNet-general-epoch_244.onnx` (application cache, not in Git) | ZhengPeng / [official BiRefNet v1](https://github.com/ZhengPeng7/BiRefNet/releases/tag/v1) | MIT | Official general-purpose model; fixed SHA-256 in `docs/birefnet-background-removal.md`; notice in `third-party/birefnet/LICENSE` |
| `BiRefNet-general-fp16.onnx` (application cache) | ZhengPeng, ONNX Community / [fixed model revision](https://huggingface.co/onnx-community/BiRefNet-ONNX/tree/534d3c82d3bb8b2f0867db6dfbc3a525b8e42f67) | MIT | FP16 conversion of the same model; fixed SHA-256; no reduction of inference dimensions |
| ONNX Runtime 1.22.0 native DLLs (application cache) | Microsoft / official NuGet | MIT + third-party notices | Complete notices in `third-party/birefnet` |
| DirectML 1.15.4 native DLL (application cache) | Microsoft / official NuGet | Microsoft DirectML binary licence | Windows only; complete terms in `third-party/birefnet/DirectML-LICENSE.txt` |

## Papierwerk example

| Path | Author / source | License | Notes |
|---|---|---|---|
| `examples/jiho/Papierwerk.figcraft` | FigCraft Jiho contributors, original procedural `filter-source.png` landscape and plain backdrop | MIT OR Apache-2.0 | Editable outward-paper example; no third-party images. Papierwerk algorithm attribution and MIT license: `third-party/papierwerk` |
