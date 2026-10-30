# Local extension devlog

## 2026-10-30 — M-PURPOSE-VIDEO: guided live editing and desktop interaction

Connect the official CLI to a per-request, authenticated loopback MCP adapter in the application.
Require the current common/channel guide before every tool action; bind tools to the originating
video and expose design/editor commands, previews and library operations. Keep script/guide writes
unavailable. Run the CLI filesystem in read-only mode; mutation is through the app command layer.
Support cancellation and expire capabilities when the task ends.

Add channel/common document-style guides and shared interface font/icon preferences. Restyle
settings, rename and standalone trash dialogs; refine search alignment, selection formatting
placement, title weight, sidebar hover actions and the upward-growing composer. Add library
context operations, range/marquee selection and copy/paste, preserving files used by other views.
Propagate friendly source names to the media bin and existing clips while keeping user clip names.

Probe and stream imported files on cancellable background workers, publishing complete files to
the originally selected video. Retain verified JSON backups and preserve corrupt originals during
recovery. The Windows package checks its installed MCP stdio entry point in addition to GUI tests.
Validation details and any unrelated full-CI failures are recorded in VALIDATION.md.

## 2026-10-29 — M-PURPOSE-VIDEO: one licensed Lucide icon pack

At the owner's request, download the unmodified Lucide 1.64.0 SVG sources from pinned revision
`be09619c1c1124ad09c19fc3e87ee3b939409bbd`, verify Git blob hashes and record SHA-256 checksums.
Retain the complete ISC/Feather MIT notice and distribute it in the Windows installer.

A small L6 `figcraft-ui-icons` crate rasterises the shared 24-unit glyphs at the current display
scale and applies the caller's tint. Clip to the requested icon box, avoid font glyph substitution
and bound the per-context texture cache. Map host navigation/window/menu/format/chat controls
and the embedded design/editor icon enums to the same sources. Standalone FigCraft keeps its
original renderer. Tree hover chooses one glyph before painting so the folding chevron replaces
the channel/video icon instead of drawing over it. Add render/scale/cache tests for the asset
set and real pointer events proving a hovered tree row contains one glyph and its toggle works.
Verification and any unrelated full-CI failures are recorded in VALIDATION.md.

## 2026-10-29 — M-PURPOSE-VIDEO: reconcile options on automatic model selection

When switching to the catalog default or entering a model ID, discard reasoning/speed options
that the new model does not advertise. Cover unsupported effort, retained supported effort and
fast-mode removal. Cancel the unpublished first 0.1.7 build to include this in the installer.

## 2026-10-29 — M-PURPOSE-VIDEO: pane height, request options and desktop details

Unify channel/video rename and trash actions, retain channel contents for restoration, and merge
restored channel shells without duplicate videos. Lift every right pane over the breadcrumb row
while retaining the browser bar and native window controls. Anchor the Codex composer at the bottom
and grow/scroll upward. Discover the signed-in CLI model catalog through public stdio JSON-RPC;
pass model, supported reasoning effort and advertised fast service tier to each CLI request.

Double-click script images to resize with a corner handle; persist width, preserve aspect ratio
and include it in document undo. Keep selected text black on blue. Leave Ctrl-wheel events over
the actual FigCraft canvas for native canvas zoom. Saved notices appear at the top and expire
after one second. Remove Windows undecorated shadow/border on fullscreen entry and restore on exit;
the installer probe now checks exact physical client coverage as well as window bounds.

Add Settings with a configurable incremental backup mirror, streamed media copies, a manifest
written after referenced files, self-recursion exclusion, progress/errors and a final close flush.
Only managed project files are covered; directly linked external editor media remains external.
After functional tests and native interaction checks, refine platform sans fonts, original drawn
menu/speed icons, neutral hover states, popup spacing, borders and shadows. Verification is recorded
in VALIDATION.md.

## 2026-10-29 — M-PURPOSE-VIDEO: physical-pixel Windows GUI probing

The 0.1.6 Windows build, installation and editor startup succeeded, but the pointer-based
fullscreen probe mixed DPI-virtualized client bounds with the target window's physical DPI.
Make the probe thread per-monitor DPI aware so button clicks and geometry checks use the same
physical pixels, fit the test window inside the CI monitor before clicking its rightmost control,
allow the initial font passes to finish, and label each transition in the log.
The failed probe did not publish the installer; application sources are unchanged by this fix.

## 2026-10-29 — M-PURPOSE-VIDEO: retain fullscreen in compact editor headers

Keep the embedded FilmCraft fullscreen control visible even below 1100pt, where the rest of the
quick-action cluster is collapsed. Reserve its space from the menus. Add a real pointer-click
regression at 550pt, 850pt and 1188pt so opening Codex cannot hide the only fullscreen button.
The initial 0.1.6 packaging run was cancelled before publication to include this correction.
Load thumbnails for visible media and retain only 32 recent textures so the larger script image
previews do not accumulate unbounded GPU memory in large libraries.

## 2026-10-29 — M-PURPOSE-VIDEO: selected text and media interactions

Add independent channel folding and hovered video menus for rename/trash, including visible
hover feedback on the channel add button. Compact the library header; remove per-asset action
buttons; clamp card names to two lines and list names to one, and draw thumbnail drag previews.

The script remains a freeform document. Persist Unicode character ranges for inline bold,
italic, underline, strike, size and line spacing, paragraph headings 1–4 and bullets. Double or
triple selection opens a floating formatting menu. Original system bold faces provide actual
bold text. Images sit at document line anchors and can be dragged between paragraphs; old
image attachments migrate into the flow. Text, formatting and images share undo/redo snapshots.

Keep one library-export action below the normal FigCraft export button in the same style. Name
PNGs after selected layers or the page, preserve the export suffix/scale, and number duplicate
names. The editor's existing fullscreen button now hides host chrome, targets the monitor's
physical size and restores the original window geometry and shell. Remove the duplicate host
fullscreen control. Package the next installer as 0.1.6.

## 2026-10-29 — M-PURPOSE-VIDEO: load native Windows icon resources

The Windows build and silent installation succeeded, but the installed GUI probe found no
explicit large icon. Load the owner's embedded multi-size ICO through Windows' native resource
loader (32px window icon, 48px taskbar icon), and propagate icon-loading errors rather than
silently clearing the taskbar icon. The probe now selects the visible Purpose window by its
process/title and records startup stderr. Keep the compile cache on failed platform checks.
No 0.1.5 installer has been published from either failed validation attempt.

## 2026-10-29 — M-PURPOSE-VIDEO: native Windows taskbar API correction

The first 0.1.5 Windows build exposed that this eframe fork's window_builder hook receives an
egui ViewportBuilder, rather than winit WindowAttributes. Set the separate large icon through
the CreationContext's actual winit window instead. Wait for icon setup (after graphics startup)
before the installed-GUI smoke test sends F11. The failed attempt did not publish a release.

## 2026-10-29 — M-PURPOSE-VIDEO: conversation UI and desktop interactions

Replace the last-answer-only Codex pane with persistent per-video conversations, immediate user
messages, Markdown replies, public CLI JSON activity, elapsed time, available usage metadata,
copy/append actions and cancellation. Keep ChatGPT login in the centre only when disconnected;
move account/CLI actions into the header menu and remove quota/explanatory boilerplate. Cancel
and join the CLI worker on close, retaining the conversation; preserve earlier turns in context.

Make library tiles and list rows draggable to sidebar library/script/design/edit destinations,
with a drag preview, target feedback, hover expansion and safe cross-video copies/reuse. Add
script font size/line spacing, Unicode-safe multi-line Tab/Shift+Tab indentation and inherited
indentation on Enter. Put real PNG-to-library export in the design header and inspector.
Route editor fullscreen through the native host window and support F11/Esc. Consume Ctrl+wheel
before it reaches either embedded editor, persist global zoom and support Ctrl+0. Set Windows'
separate large taskbar icon, standardise runtime icons to 256 px, and give shortcuts an explicit
versioned ICO. Reserve installer 0.1.5 and add Windows icon/fullscreen/restore smoke probes.

Validation: 97 desktop tests and 824 UI tests pass; workspace Clippy, WASM/plugins, layers and
asset attribution pass. Inspect real Xvfb GUI captures for conversation/user bubbles, a mocked
CLI's progress/completion, script formatting and global zoom, cross-video drops, image drops
into FigCraft and FilmCraft, library PNG export and fullscreen/restore with a window manager.
Live ChatGPT inference is not exercised; the selected-CLI/auth/JSON protocol and stop path use
a local deterministic CLI fixture. Full CI retains the existing board-table row-height failure
(1391 other engine tests pass). No clean full CI pass is claimed.

## 2026-10-29 — M-PURPOSE-VIDEO: FilmCraft editor, history page and P branding

Replace the basic FFmpeg timeline with FilmCraft 0.14.0 at pinned revision
8d925e9df32d8755f22bd2635e400a6b3e93f69c. Vendor its egui frontend, retain licences/credits,
and pin the native engine/codecs/renderers. Let the host own window lifecycle, fonts and scale;
disable the embedded upstream binary updater. Attach native audio I/O, media clipboard,
file dialogs, GPU preview/export hooks and the complete editor workspace. Use one edit.fcproj
per video, migrate old clips/audio gain/captions, fit incoming visual clips to frame, autosave,
and retain immutable project snapshots for history restoration. Save clean externally opened
projects into the current video's project too. Share library media with the native project.

Open History as a main browser page: searchable all-video table, channel/time/count columns,
per-video versions and restoration of script/library plus native edit project. Keep browser
back/forward navigation. Use the owner's unmodified P PNG for sidebar/runtime branding and
its ICO conversions for Windows executable, installer and uninstall icon. Include FilmCraft
and font notices in the installer. Publish only setup.exe, with an installed-GUI open/close check.

Validation: 87 desktop tests, 824 UI tests, workspace Clippy, dependency layers, asset attribution and WASM/plugin gates pass.
Exercise history table/detail, native timeline/video/audio-meter/caption playback, export UI,
and design transitions in Xvfb. Decode the actual GUI-exported 1280×720 H.264/AAC MP4 with
ffprobe, and test native export pixels with the external FFmpeg decoder. Verify migration,
save/reopen, revision restoration and missing-project recovery. Full CI still reaches the
existing board-table row-growth failure (122 → 122; 1391 other engine tests pass).

## 2026-10-29 — M-PURPOSE-VIDEO: installer-only distribution

Stop generating the portable archive. Publish the tested setup EXE directly as a Purpose video
maker prerelease asset, with its commit and checksum in the release notes, so users can download
the installer without extracting an Actions artifact ZIP. Keep installer and bundled media/CLI
smoke checks. Reserve 0.1.4 for the next build; this configuration change skips CI publication.
Workflow YAML/output checks pass. Full CI passes format/Clippy and retains the documented
engine table-height failure (1391 other engine tests pass); no Rust code changes.

## 2026-10-29 — M-PURPOSE-VIDEO: workspace panels and Codex login

Pin History, Trash and Codex actions to the sidebar footer. Use one optional side view across
all sections, with explicit close controls. Add official CLI discovery, browser/device login,
status, cancellation and logout; bundle checksum-pinned official Windows CLI binaries and notices.
Require ChatGPT auth for chat. Persist independent video expansion and reversible video trash,
checkpoint script/assets/edit settings, and make the script title the video name. Reset the host
Ui style before drawing and keep embedded editor font/zoom changes from altering the outer shell.
Wait for the startup font scan and apply fonts before the first visible workspace frame.
Verify title propagation, independent expansion, panel toggle, design transition, history restore,
trash restore, and a synthetic CLI login/chat flow in Xvfb. Real account login needs the user.
Validation: 73 desktop tests and 824 UI tests pass, including the embedded host zoom regression;
Clippy, layers, assets and WASM pass. Full CI retains the documented engine table-height failure
(1391 other engine tests pass).


## 2026-10-28 — M-PURPOSE-VIDEO: desktop layout and freeform script

Rework the video maker shell with a compact browser bar, restrained document typography,
aligned navigation, muted surfaces and original code-drawn icons. Replace script blocks with
one editable title and a freeform body; migrate old headings, paragraphs, scenes and asset
references on load. Add a filename-only library list and a thumbnail tile view with bounded
background preview generation for images and videos. Restyle the edit preview, timeline and
controls, and update the Windows package to 0.1.2. Review the script, both library modes and
the edit tab in a running Linux GUI.

## 2026-10-28 — M-PURPOSE-VIDEO: browser navigation and window chrome

Give the video workspace session tabs with independent back/forward history, a new-tab chooser,
close and sidebar toggle. Its top bar now owns the frameless window controls and dragging on
Windows/Linux, while the embedded FigCraft editor omits its duplicate tab bar. Closing the
workspace saves its project and loaded design first. Verified the tab/history model and the
Linux GUI: new tab, back/forward, sidebar toggle, design view, window drag, maximize/restore,
minimize, and close. macOS retains its inset system title bar.
Both desktop binaries pass 67 tests each; format, workspace Clippy, layers, assets and WASM
checks pass. The full cloud CI remains blocked by the unrelated existing engine test
`board_tables_tests::a_double_click_types_in_the_cell_and_its_row_grows_with_the_text`
(1391 other engine tests passed).

## 2026-10-28 — M-PURPOSE-VIDEO: Windows distribution

Add a per-user Inno Setup installer and portable Windows x64 ZIP for Purpose video maker.
Package a checksum-pinned LGPL FFmpeg build, make the app resolve bundled media tools, and
select its OpenH264 encoder when available. Windows CI builds both packages and verifies
the installer, installed executable, and bundled H.264 encoding.

## 2026-10-28 — M-PURPOSE-VIDEO: video production workspace

Add a second desktop entry point for channel and video projects, keeping FigCraft's design
editor while hiding its Home entry in this mode. Copy local media into each video's library;
link it to block-based scripts and an editable video/audio timeline. Save project state locally,
export selected designs as library PNGs, and render trimmed video, mixed audio and subtitles
through FFmpeg. Add a read-only Codex CLI task pane whose response needs a user action before
entering the script. The first version offers clip order, trim, volume and caption timing;
continuous in-app playback and transition effects remain outside this milestone.
Verification: 64 desktop tests, Clippy and format checks, Linux GUI screenshots, a design-to-library
PNG, and an MP4 checked for H.264 video, AAC audio and a subtitle stream. Cloud Codex execution
was blocked by the instance's rejected ChatGPT authentication token; the CLI path awaits a valid
user login on a target machine.

## 2026-10-28 — M-JIHO-PAPIERWERK: outward paper border from .5

Re-establish the v0.11.2.5 source tree (273bdf51) without rewriting published history,
then port the pinned MIT Papierwerk border algorithm to Rust. Keep the source image
above paper and grow render/export bounds. Retain BiRefNet, Paper and sliders exactly
as in .5; do not reapply .6 custom-parameter localization or inward erosion. Attribute
the upstream code and adapt its premultiplied compositing, local noise and shadows.
Validate against pixel samples from the actual upstream JS and outward/preservation,
pan, native round-trip and undo regressions. Results: VALIDATION.md.

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


## 2026-10-30 · Purpose 0.1.10

Reworked history into expandable inline rows with paginated previews and restore, per-video time
buckets (5 minutes / 1 hour / 1 day over 7 / 14 / 30 days), protected restore points, and conservative
background collection of unreferenced native snapshots. Current, trash and recovery manifest refs
remain protected; media is never collected. The same cleanup runs in the owned backup directory.

Added persisted base text size, shared page/heading/menu metrics and single-edge physical-pixel
panel dividers. Bold selections use real bold or variable weight 700 and retain bold fallbacks.
Codex accepts library drag/drop, multiselection context attachment and a source picker, renders
removable thumbnail chips, persists message attachment refs and supplies CLI image inputs.

Fixed stored-relative-path source reuse and duplicate bin entries; the native editor defaults to
a reconciled library bin, retains archived timeline media, owns new imports through the background
copy service, and mirrors native source clear/undo/redo. Automatic bookkeeping preserves native
undo history. Ignored the previously recorded engine board-table test as requested by the owner.


## 2026-10-30 · Purpose 0.1.11

Package the missing official Codex code-mode host from the same 0.165.0 release, prefer a complete
bundled CLI over an incomplete custom path and include helper siblings on child PATH. Add a full
Video Downloader page with explicit target library, MP4/MP3, progress/cancel, background import,
retry on registration errors and temporary cleanup only after durable registration. Bundle pinned
Unlicense wheels with a PSF Python runtime and Deno; do not redistribute upstream's GPL-containing
PyInstaller executable. Installed Windows probes check the host, HTTP download and both codecs.

Replace the trash modal with a shared collection page and confirmed permanent deletion. Persist
cleanup before touching owned files, retry after restart, refuse symlink/traversal paths, remove
history/inline references and prevent native archived references from resurrecting purged sources.
Close deleted native sessions to prevent later autosaves from recreating removed files.

Separate text sizing from viewport zoom with a scoped UI font policy shared by both editors.
Root clear is opaque white, shell dividers have one owner, tab text/close/add share their vertical
center, and a one-pixel brand-row overflow no longer grows the sidebar each frame. Restore inset
search background/equal gaps, use the requested footer order, move Settings beside the brand,
increase document title top padding and add Design fullscreen while removing the breadcrumb Codex button.

Final native review also fixes breadcrumb text alignment inside its reserved full-width hit box;
the fullscreen control keeps its own reserved space when a video title is long.

Native MP3 review also keeps PATH-based FFmpeg discovery when no bundled binary exists;
yt-dlp only receives --ffmpeg-location for an existing file. Windows retains its bundled path.
