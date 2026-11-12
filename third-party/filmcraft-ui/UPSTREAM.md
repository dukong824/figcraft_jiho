# FilmCraft embedded frontend

Source: https://github.com/storytold/filmcraft/tree/25ceb25f3e30659a04ade27727083b9932d97c04/crates/ui-egui
Revision: `25ceb25f3e30659a04ade27727083b9932d97c04` (0.17.0), MIT OR Apache-2.0. Original notices and contributor credits retained.
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

The embedded header retains its fullscreen button in narrow workspaces, with menu clipping
reserving its hit area; other compact-header quick actions remain in their menus.

Purpose mode uses the host-enabled, shared Lucide SVG icon renderer (ISC / Feather MIT).
The original standalone drawings and icon action identifiers remain available. Lucide sources
and notices are pinned in `assets/ui/lucide/` and shipped in the installer.

Updated by a three-way merge of the 0.14 frontend, the Purpose adaptations, and upstream 0.17.
Host lifecycle/fullscreen, font scaling, Lucide glyphs, branded assets and library integration
remain local adaptations; the latest upstream engine, project and frontend logic are retained.

When running this frontend as a standalone test workspace, use the parent Cargo.toml
`patch.crates-io` entries and dev profiles, and seed its temporary lockfile from the parent
Cargo.lock. This exercises the same egui/winit fork and dependency versions as the app.
