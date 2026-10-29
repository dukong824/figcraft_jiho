# FilmCraft embedded frontend

Source: https://github.com/storytold/filmcraft/tree/8d925e9df32d8755f22bd2635e400a6b3e93f69c/crates/ui-egui
Revision: `8d925e9df32d8755f22bd2635e400a6b3e93f69c` (0.14.0), MIT OR Apache-2.0. Original notices and contributor credits retained.
The engine, codecs, renderers and project model are pinned Git dependencies from the same revision.
Local adaptations: host-managed fonts/zoom/window lifecycle and branding; embedded editor entry point; compact header while the host side view is open.
No upstream trademark artwork or font files are copied into this directory.

Native host adapters in `apps/figcraft/src/video_maker/film_audio.rs` and
`film_audio_in.rs` originate in upstream `apps/filmcraft/src/audio.rs` and
`audio_in.rs` at the same revision. Clipboard/open-path and GPU export hooks
also follow that desktop host. The original copyright and dual licence apply.
The frontend disables the upstream binary updater when embedded and opens the
fork's Releases page instead. Standalone tests/benchmarks are retained as source.

The embedded fullscreen actions now request a native-window toggle from the host so both
editor controls and F11/Esc act on the same window.
