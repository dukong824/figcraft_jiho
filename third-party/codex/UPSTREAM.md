# OpenAI Codex CLI

Windows packages bundle unmodified official x86_64 MSVC executables from
https://github.com/openai/codex/releases/tag/rust-v0.165.0 (Apache-2.0).
`packaging/purpose-video-maker/package.ps1` pins each upstream SHA-256 digest.
The CLI owns browser/device login and its credential store. The application never reads credentials.
