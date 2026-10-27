# Local extension devlog

## 2026-10-27 — M-JIHO: personal fork and release maintenance

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
