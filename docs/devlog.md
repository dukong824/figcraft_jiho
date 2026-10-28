# Local extension devlog

## 2026-10-28 — M-JIHO-BIREFNET: local background removal

Add official MIT BiRefNet general ONNX inference to the Windows Custom section through native
platform services. Download pinned, SHA-256-verified weights and ONNX/DirectML runtimes on first
use; no photo upload or Python dependency. Run on a worker with progress, cancellation and stale
document/paint guards. Preserve original RGB, alpha multiplication, ICC profile and image dimensions;
retain the encoded original and soft mask alongside the cutout in `.figcraft`, with one-step undo
and original restoration. Rendering and exchange exports consume the ordinary resulting PNG, so
existing Paper materials keep working. Include model/runtime licences and document supported bit
depth, memory bounds, first-use download and mask-resolution limitations. Verification is recorded
in `VALIDATION.md`.
Complete the Custom/Paper/BiRefNet English keys and enumerate Paper filter labels in the dynamic
catalog checks; remove four obsolete official-updater English entries for the fork's release channel.

## 2026-10-27 — M-JIHO-SLIDERS: custom effect parameter sliders

Pair the existing number fields with accessible horizontal sliders for Liquid Metal and all
seven Image Filters. Preserve direct entry, original ranges, integer steps for counts, dropdowns
and checkboxes. Frame uses a useful ten-second track that expands for typed long/negative values.
Route slider edits through effect.set and the existing interaction mechanism so values update
live and the release frame commits one undo step even when it changes no value. Merely opening
the controls never normalizes the saved data. Add panel regressions for live/clamped dragging,
undo and release, typed-value synchronization, integer counts and all eight custom materials.

## 2026-10-27 — M-JIHO-FILTERS: seven Paper Image Filters

Port Paper Texture, Fluted Glass, Water, Image Dithering, Halftone Dots, Halftone CMYK and Lens
Distortion from the same pinned Apache-2.0 Paper revision. Retain original GLSL formulas, shared
constants and the noise texture; adapt explicit GPU bindings and framebuffer coordinates.
Add serialized, bounded parameter catalogs with original defaults, command validation, color
profile visiting, animation clocks, raster export and unsupported-format warnings. Native source
uploads recover straight RGB from premultiplied pixels and cache the complete image content.
Custom's plus now opens a chooser for the existing Liquid Metal and these seven filters. No
additional Logo Animations or Effects shaders are added. Test GPU compilation, RGB cache
invalidation, animation/still frames, alpha/orientation, round trips, undo and UI insertion.

## 2026-10-27 — M-JIHO-CUSTOM: a Custom section for personal materials

Move Liquid Metal into a Custom section alongside Fill, Stroke and Effects. Its plus button
adds the material and opens its controls; ordinary effect menus no longer offer it. Keep the
serialized stack and command API unchanged so existing files retain their parameters and order.
Map each filtered row back to its persisted index for edit, removal, reordering and picker state.
Refresh the document snapshot between sections after an ordinary effect changes its stack.
Add UI regressions for insertion with undo, interleaved removal with picker state, and reordering.
Update the bundled demo and usage instructions. Verification is recorded in `VALIDATION.md`.

## 2026-10-27 — M-JIHO: personal fork and release maintenance

Follow-up: explicitly dispatch branch CI for bot-created update PRs, which GitHub may otherwise
leave awaiting approval, and reuse an existing update branch on repeat runs.

Replay the Paper feature on the real official Git history. Archive the inherited workflows,
add Windows CI/release and manually triggered upstream-review workflows, and document the merge
and release process. The title and version output identify figcraft_jiho. Update checks now open
the personal Releases page; official binary replacement is disabled until this fork has its own
signed updater. The initial public package records its actual build profile.

## 2026-10-27 — M-PAPER: native Paper Liquid Metal

Port the original Apache-2.0 GLSL and alpha/Poisson preprocessing into the native renderer,
add serialized effect parameters and Design-panel controls, and propagate animation time through
the canvas worker, retained renderer and prototype image clock. SVG/PDF rasterize this material;
unsupported exchange formats report the omission. See `paper-liquid-metal.md` for scope,
provenance, resampling differences and the native-only GPU limitation.

Verification results are recorded in the delivered `VALIDATION.md`.
