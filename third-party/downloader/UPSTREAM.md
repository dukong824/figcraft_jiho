# Purpose video downloader runtime

The Windows installer downloads the following original distributions with pinned SHA-256 checks.
No implementation code is vendored in this directory. `packaging/purpose-video-maker/downloader.ps1`
is the authoritative URL/hash list and keeps the upstream packages and complete notices together.

| Component | Version | Source | Licence |
| --- | --- | --- | --- |
| CPython Windows embeddable x64 | 3.14.10 | https://www.python.org/downloads/release/python-31410/ | PSF-2.0 and bundled notices |
| yt-dlp pure Python wheel | 2026.10.26 | https://pypi.org/project/yt-dlp/2026.10.26/ | Unlicense |
| yt-dlp-ejs wheel | 0.8.0 | https://pypi.org/project/yt-dlp-ejs/0.8.0/ | Unlicense, bundled ISC/MIT notices |
| Deno Windows x64 | 2.9.8 | https://github.com/denoland/deno/releases/tag/v2.9.8 | MIT and `deno license` third-party notices |

The PyInstaller `yt-dlp.exe` release includes optional GPL components. It is **not** distributed.
The official wheels above contain only Unlicense code (and permissive EJS notices); optional
packages such as mutagen, curl_cffi and PyCryptodome are not installed. The application launches
`python.exe -I -X utf8 -m yt_dlp` without plugins or user configuration. No handwritten Python or
JavaScript is shipped. Deno executes the upstream extractor's bundled EJS for supported sites.
The existing LGPL FFmpeg build performs MP4 remuxing and MP3 extraction.

Installed notices are under `licenses/downloader/`; each wheel's original `.dist-info` licence is
also retained, and the Python runtime retains its full `LICENSE.txt`. Nothing downloads or updates
itself at app startup. Updating extractor/runtime versions requires a new reviewed installer.

The app never passes URLs through a shell or supplies browser cookies. Website access restrictions,
login-only/DRM content and extraction changes can prevent individual links from downloading.
