# Paper Liquid Metal port validation

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
