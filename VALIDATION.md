# Paper Liquid Metal port validation

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
