## Purpose selected text and sidebar interactions — 2026-10-29

- 103 desktop tests passed, including Unicode formatting ranges, mixed selection, multi-line
  indent rebasing, stable image anchors, rich content/fold-state persistence and layer-named
  PNG exports with duplicate names preserved.
- 824 UI tests passed (1 ignored). Dependency layers, asset attribution, WASM and plugin gates
  passed. PowerShell syntax and the Windows GUI probe's C# compilation passed locally.
- Inspected native GUI captures: word/paragraph selection, bold/italic/underline and heading 2,
  image drag between paragraphs, channel folding, hovered video menu and rename, compact library
  cards and thumbnail drag preview, and a page-named PNG exported below the normal export button.
- Clicked the embedded FilmCraft fullscreen button: the editor occupies 1920×1080, host chrome
  is hidden, and Esc restores 1918×1059 and the shell. Verified the complete viewport with Mesa
  lavapipe Vulkan under Xvfb. The cloud Mesa OpenGL path showed a clipped bottom strip after a
  resize even with correct window/egui geometry; that backend was not used as visual proof of
  complete fullscreen rendering. Windows is verified separately by the installer workflow.
- Full CI attempted: the existing board-table row-growth test still fails (122 → 122;
  1391 passed, 1 failed, 2 ignored). A clean full CI result is not claimed.

## Purpose conversation and interaction fixes — 2026-10-29

- 97 desktop tests passed: selected CLI authentication/chat, user-message persistence, public JSON
  activity/usage, cancellation, Unicode indentation, formatting persistence, cross-video copies,
  native design PNG export and timeline integration, plus the prior desktop regressions.
- 824 UI tests passed (1 ignored). Workspace Clippy with warnings denied, layers, asset attribution,
  WASM and plugin gates passed.
- Inspected actual GUI captures: user/assistant bubbles and progress with a local mock CLI,
  formatting controls/Tab, Ctrl+wheel affecting the entire shell, drop to another video's script,
  image drop to FigCraft, PNG library export and drop to native FilmCraft. Fullscreen works via
  F11/Esc, the host button and the embedded editor button (1920×1080) with a real window manager.
- No live ChatGPT inference is claimed. CLI protocol, auth handling and process cancellation use
  deterministic local fixtures. The app still invokes the official CLI with ChatGPT auth.
- Full CI was attempted; the existing board-table test still fails at 122 → 122 (1391 passed,
  1 failed, 2 ignored in the engine suite). It is not a clean full CI run.
- Windows packaging now probes WM_GETICON/ICON_BIG and fullscreen/restore on the installed GUI;
  those platform-specific results belong to the release workflow.

## Purpose video maker FilmCraft integration — 2026-10-29

- Desktop test suite: 87 passed, including migration of native timeline trims/audio gain/captions,
  native project save/reopen, revision restoration, missing-current-file recovery, and clean
  external project import persistence.
- Native H.264/AAC export: passed. ffprobe verifies streams/dimensions and FFmpeg decodes the
  output; a decoded off-centre pixel checks that library sources fit the sequence.
- Actual Linux GUI: inspected history table/detail, FilmCraft timeline and media bins, moving
  program monitor, live audio levels, Korean captions, export settings, and transition to the
  FigCraft design editor with the host shell's size preserved. GUI export produced a decodable
  1280×720 H.264/AAC MP4 (2.005 s). This cloud instance has no audio output device, so audible
  hardware output is not claimed.
- Logo original PNG SHA-256 matches the supplied download; Windows ICO has seven size variants.
- UI crate: 824 passed, 1 ignored. WASM and plugin gates passed.
- Workspace Clippy with warnings denied, dependency layers and asset attribution: passed.
- Full `cargo xtask ci` attempted: retains the existing engine row-growth failure
  `board_tables_tests::a_double_click_types_in_the_cell_and_its_row_grows_with_the_text`
  (122 → 122; 1391 passed, 1 failed, 2 ignored). A clean full CI pass is not claimed.
- Installer workflow publishes only setup.exe. It also verifies the installed GUI opens and
  closes, the packaged media tools, Codex executable and FilmCraft licence notices on Windows.

# FigCraft Jiho build validation

## Papierwerk outward border, rebuilt from .5 — 2026-10-28

- Re-established the v0.11.2.5 (`273bdf51`) file tree, then added Papierwerk. Published Git
  history stays intact. The .6 inward erosion and wholesale parameter localization are removed;
  existing Paper shaders, original material parameters, BiRefNet and sliders remain as in .5.
- Ported the pinned MIT source `gentrax/papierwerk-editor@4ec40a640dec268d2cdf34a77b1d8d2983377bbd`:
  mulberry32, mean-centered multi-octave noise, exact separable EDT and addTornBorder compositing.
  Original license, excerpt and host adaptations are in `third-party/papierwerk` and the package.
- New render regressions: **6 passed**. Includes **24 pixel samples** generated by executing the
  actual pinned upstream JS, outward bounds, every opaque original pixel preserved, deterministic
  seed/zero-width/visibility behavior, premultiplied alpha, translucent paint, shadows and clipped
  views under rotation/scale. The native effect round-trip/scale/undo engine regression passes.
- Actual native Windows PNG export: original **480 × 320**, output **572 × 412**, 46-pixel bounds
  margin each side. Every RGBA byte inside the original rectangle equals the decoded source.
  No resampling or inward cutting. Native save, SVG and PDF exports also succeeded; the latter
  preserve the effect as an embedded raster. Unsupported .fig/Android effects emit omission warnings.
- Full renderer: **178 passed, 3 ignored**. Full UI: **823 passed, 1 ignored**. An initial catalog
  regression caught a dynamic Torn Paper display key; it was registered explicitly and the full
  UI suite passed. The final Korean row caption was shortened to fit the existing dropdown;
  the final **21** catalog tests pass.
- `cargo xtask ci` was attempted. Formatting and workspace Clippy with warnings denied passed;
  engine **1,392 passed, 1 failed, 2 ignored**. The unchanged optional-font board-table row-growth
  failure remains (`122 → 122`); a fully green CI run is not claimed. The separate unchanged GNU
  pathops floating-point baseline from .5 remains documented below.
- Layer/attribution checks, WASM and all four plugin checks/builds, workspace doctests and native
  GUI/CLI/GIF-helper builds passed. Screenshots checked with the original procedural landscape:
  source bounds remain intact, paper extends outside them, and the three sliders/color are visible.
- `preview-v0.11.2.7` uses the optimized-dev Windows x64 GNU profile. BUILD-INFO.txt records the
  exact source commit. See `docs/papierwerk-torn-paper.md` for usage, export and texture-cache limits.

## Local BiRefNet background removal — 2026-10-28

- Added Windows Custom → Remove background (BiRefNet), with progress/cancel, original restoration,
  captured document/paint guards and one undo step. Source bytes, full-size soft mask and lossless
  cutout are retained in `.figcraft`; crop, filters and existing Paper materials stay attached.
- Engine regressions: **2 passed** (round trip/restore/undo, image references, unchanged source,
  stale-source rejection, dimensions and corrupt input rollback). Async UI regressions: **2 passed**
  (selection changes keep the captured image; switching documents cancels late completion).
  Existing panel tests **47 passed** and custom slider tests **3 passed**.
- Real inference used NASA's public-domain Eileen Collins portrait via scikit-image, kept only
  in local test work. Actual FP32 CPU and FP16 DirectML masks were generated. Original size and
  every decoded RGB byte were preserved, including transparent pixels; masks have soft alpha.
- RTX 5050 Laptop GPU (8 GB), same native pipeline: **19.9 s cold / 2.6 s warm** on a 512 × 512
  photo, including preprocessing and PNG encoding. Timings are examples, not a performance promise.
  A 20-second idle cache returns model memory. GPU fusion was disabled after real testing found
  fused execution exceeded memory/time limits; the same 1024-pixel model now runs successfully.
- The actual native Rust HTTP path downloaded and SHA-256-verified the 17.9 MB ONNX runtime package.
  Both model variants and runtime archives use pinned sizes and SHA-256; licence texts are included.
- `cargo xtask ci` was attempted: engine **1,391 passed, 1 failed, 2 ignored**. The only failure
  remains the pre-existing optional-font board-table row-growth case (`122 → 122`); it is unrelated
  to this feature. A clean full CI result is not claimed.
- A broader workspace run excluding that row-growth case reached a separate unchanged pathops
  exact-equality failure: `measures_paths_by_length`, `0.0` versus `-1.8501900455444168e-15` at a
  closed-circle endpoint. It also fails alone in this GNU environment; geometry code is unchanged.
  This second observed failure is reported, not hidden by a code or tolerance change.
- Final workspace Clippy with warnings denied, layer and asset checks, WASM and all four plugin
  checks/builds passed. Native renderer: **172 passed, 3 ignored**. Whole-UI testing also caught
  missing English catalog keys for Custom/Paper and BiRefNet; those keys were added.
  Final full UI suite: **823 passed, 1 ignored**; the 21 catalog tests also passed separately.
- Actual Korean Windows UI was inspected before processing, during progress and after completion.
  The portrait renders with a transparent background and the original-background restoration
  control. The saved native test document contains the original, cutout and mask references.
- Model input is 1024 × 1024, resized back to original dimensions. This preserves accepted 8-bit
  source RGB/resolution, not a guarantee of perfect segmentation at every hair or transparent edge.
  GIF/APNG/animated WebP and higher bit depths are explicitly rejected. See
  `docs/birefnet-background-removal.md` for supported input, licences and deployment requirements.

## Custom parameter sliders — 2026-10-27

- Continuous numeric settings in Liquid Metal and all seven Image Filters now pair their
  existing number field with a horizontal slider. Original parameter ranges and integer count
  steps are respected. Frame uses a ten-second track extended by typed long/negative values.
  Opening the panel leaves stored values untouched, including values finer than the drag step.
- New slider integration tests: **3 passed**. They cover live editing, end-stop clamping,
  interleaved effect indices, a release with no new value committing one undo, direct numeric
  entry synchronized to the slider, integer sample counts and all eight materials opening
  without changing their saved parameters or history.
- Existing panel regression suite: **47 passed**.
- Actual Korean native UI checked: controls and preview are visible. A pointer drag changed
  Water speed from `1` to `-1.55`; one undo restored `1`. The test used a separate profile and
  synthetic demo, and its temporary document changes were discarded.
- Native GUI/CLI build, formatting, workspace Clippy with warnings denied, layers, assets,
  WASM and plugin checks: passed. Rendering, serialized schema and shader sources are unchanged.
- Full `cargo xtask ci` was attempted: engine **1,389 passed, 1 failed, 2 ignored**. Only the
  unchanged board-table row-growth test failed without optional craft-fonts (`122 → 122`).
  The UI tests and later gates above were run separately; a clean full CI pass is not claimed.
- `preview-v0.11.2.4` is the optimized-dev Windows GNU Preview; its exact source commit is
  recorded in BUILD-INFO.txt.

## Seven Paper Image Filters — 2026-10-27

- Added only Paper's Image Filters category: Paper Texture, Fluted Glass, Water, Image Dithering,
  Halftone Dots, Halftone CMYK and Lens Distortion. Existing Liquid Metal remains available;
  no other Logo Animations or Effects shaders were added.
- Original GLSL, shared constants, palettes and noise texture are pinned to Paper commit
  `1923e9f536ea079e2a95d23291cbfacfd69d37f6`, with licenses and source hashes recorded.
- Native renderer suite: **172 passed, 3 ignored**. New GPU cases compile and render all seven
  shaders, check RGB cache invalidation, neutral interior orientation/alpha, water animation,
  paused frames and static-filter determinism. Existing Liquid Metal GPU/panning tests pass.
- Effect command suite: **8 passed**. The new regression covers all filter aliases, inspect →
  node.set round trips, native document serialization, undo, unknown parameter rejection and
  Water's omitted-speed default. Present null speed is rejected so a valid explicit speed 0
  is not falsely classified as an ignored property by the engine's key checker.
- Panel suite: **47 passed**, including the custom chooser and insertion with open controls.
- Native GUI/CLI and GIF-helper builds, Rust formatting, workspace Clippy with warnings denied,
  dependency layers, attribution, WASM and plugin checks: passed.
- Native screenshots checked: seven filter results, the Korean Custom add menu and Water
  controls. Speed 0 was accepted in the final native build. First GPU use initializes the shader
  pipelines and may delay the first canvas image.
- The original procedural landscape comparison demo saves, reopens and exports to PNG, SVG and
  PDF. `Open Image Filters.cmd` opens the packaged demo; licenses and usage notes are included.
- Full `cargo xtask ci` was attempted: **1,389 passed, 1 failed, 2 ignored** in the engine suite.
  Only the unchanged board-table row-growth test failed without optional craft-fonts (`122 → 122`).
  The later gates were run separately; a clean full CI pass is not claimed.
- `preview-v0.11.2.3` is an optimized-dev Windows GNU Preview; BUILD-INFO.txt records its source
  commit. The CI-gated release-profile tag workflow remains unchanged.

## Custom section — 2026-10-27

- Liquid Metal is listed in Design → Custom alongside Fill, Stroke and Effects. Ordinary
  Effects type menus no longer offer it. The Custom plus button adds it with settings open.
- Existing serialized effect stacks are retained without migration; mixed selections keep the
  original conservative edit behavior. Rendering, parameters and command names are unchanged.
- Panel regression suite: **46 passed**, including three new cases for adding with one undo
  step, deleting an interleaved custom effect while remapping the picker/settings indices, and
  reordering ordinary effects across persisted custom indices.
- Native GUI/CLI build, workspace Clippy with warnings denied, formatting, dependency layers,
  asset attribution, WASM compilation and plugin checks: passed.
- Native Korean UI screenshots checked: 채우기, 선, 효과, 커스텀 are peer sections, the material
  is labelled 리퀴드 메탈, and its settings open under Custom while the canvas animates.
  The bundled demo now points to Custom and was saved through the CLI command API.
- `cargo xtask ci` was attempted: the unchanged engine board-table row-growth test still fails
  without optional craft-fonts (1,388 passed, 1 failed, 2 ignored, measured `122 → 122`).
  The remaining gates above were run separately. A clean full CI pass is not claimed.
- `preview-v0.11.2.2` uses the optimized-dev Windows GNU profile, with the actual source commit
  recorded in BUILD-INFO.txt. It is an unsigned Preview, separate from CI-gated release tags.

## figcraft_jiho fork setup — 2026-10-27

The Paper commit was replayed on the actual official Git history, preserving ancestry from
`43af145a12eacd1e87f5f5e6984c49b7da040288`. This section covers the personal fork setup in
addition to the original material verification below.

- Native application/CLI build and workspace Clippy with warnings denied: passed.
- Actionlint 1.7.12 checked all three personal GitHub workflows: passed.
- Window-management/UI regression tests: 20 passed.
- PowerShell packaging syntax, dependency layering, asset attribution and WASM/plugin gates: passed.
- Actual native screenshot: the metal demo renders, and the update dialog links to
  `dukong824/figcraft_jiho/releases`.
- The official binary-replacement updater is removed from this fork. Downloading and replacing
  the portable build remains manual; a separately signed in-app updater is not implemented.
- Full CI was attempted again: engine tests reported 1,388 passed, 1 failed, 2 ignored. The same
  unchanged board-table row-growth test failed (`122 → 122`) without optional craft-fonts.
  A clean full CI pass is not claimed. The GitHub CI configuration loads the official pinned
  craft-fonts input; its remote result must be reviewed before a release-profile tag is shipped.
- The first `preview-v…` release is an experimental optimized-dev Windows GNU build; subsequent
  `jiho-v…` tag workflows require CI before packaging a Windows MSVC release-profile build.

## Original Paper material verification

Verified on Windows x64, 2026-10-27, using portable Rust 1.99.1 GNU.

| Check | Result |
|---|---|
| Native app and CLI build | Passed |
| Original GLSL compiles on GPU; time changes pixels; speed 0 is deterministic | Passed |
| Alpha preprocessing, empty input and panning coordinate stability | Passed |
| Full renderer tests | 169 passed, 3 ignored |
| Paper command edit / serialization / undo / invalid-key rejection | Passed |
| Effect-panel reorder / copy-paste / removal tests | 3 passed |
| Workspace Clippy, all targets, warnings denied | Passed |
| Rust formatting | Passed |
| Dependency layers and asset attribution gates | Passed |
| Web/WASM compilation, plugin SDK tests and four example plugin builds | Passed |
| Native document save, reopen, CLI inspection | Passed; demo has no import notes |
| PNG, SVG and PDF export | Passed; SVG/PDF rasterize the material |
| Actual app UI screenshot | Checked: material visible, settings readable |
| Canvas animation | Two captures differ within the metal artwork |
| GIF export | 512 × 512; 72 distinct frames; 3,000 ms; repeating, not guaranteed seamless |

`cargo xtask ci` was attempted. The engine suite reported **1,388 passed, 1 failed, 2 ignored**.
The failure was the separate existing board-table test
`board_tables_tests::a_double_click_types_in_the_cell_and_its_row_grows_with_the_text`
(expected the row to grow; measured `122 → 122`). This test's code was not changed by this port.
The Paper command test passed in that suite. Therefore a clean full-workspace CI pass is **not**
claimed. The remaining format, lint, renderer, UI-effect, layering, attribution and WASM gates
were run separately and passed. Optional craft-fonts were absent; installed/bundled fonts were used.

The application was packaged from the optimized dev profile (`opt-level=1`, dependencies optimized).
Debug symbols were stripped; the GUI PE subsystem was set to Windows. Paper's GPU shader
optimization is enabled. All imported runtime DLLs are Windows system libraries.

Known limits: Windows native effect; web preserves the source and parameters without rendering
the GPU effect. Canvas material input is bounded to 512 pixels, still/prototype renders to 1024.
Browser resampling and native resampling can differ at edges. The native renderer reads GPU pixels
back for CPU composition, so live frame rate depends on hardware and document size (the two-effect
demo measured roughly 100–135 ms per render here). The separate GIF exporter produces 24 fps
on average regardless of how long exporting takes. Paper uses alpha silhouettes and does not
remove a photograph's background. See `docs/paper-liquid-metal.md`.
