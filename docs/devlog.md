# Local extension devlog

## 2026-10-27 — M-PAPER: native Paper Liquid Metal

Port the original Apache-2.0 GLSL and alpha/Poisson preprocessing into the native renderer,
add serialized effect parameters and Design-panel controls, and propagate animation time through
the canvas worker, retained renderer and prototype image clock. SVG/PDF rasterize this material;
unsupported exchange formats report the omission. See `paper-liquid-metal.md` for scope,
provenance, resampling differences and the native-only GPU limitation.

Verification results are recorded in the delivered `VALIDATION.md`.
