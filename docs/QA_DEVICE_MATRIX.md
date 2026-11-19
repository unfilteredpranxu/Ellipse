# Device and build verification

| Environment | Evidence |
| --- | --- |
| Linux Rust | Unit and SQLite integration suites, including legacy conversion, checkpoint and archive tests. |
| Chromium | Automated Notes, home modes, Calendar, syllabus and Focus workflows. Fixtures provide native IPC and Firebase responses. PDF loading and rendering, browser-local planner persistence and pointer input use the compiled frontend and real browser APIs. |
| Ubuntu 24.04 | Local container regression uses Playwright with a compatible Chromium executable and checks PDF outputs with Poppler. |
| iOS Simulator, iPad, iPhone | A previous hosted run at `7d88175` produced an unsigned IPA, as reported by the owner. The workflow requests package checks, a Simulator launch, a codesign report and SHA-256 digests. Hosted run and artifact verification from this environment returned HTTP 403. Physical Apple Pencil, force-quit, Files and runtime checks are unverified. |
| Android, Windows, macOS | Existing GitHub native workflows cover packaging. Current hosted results and installed app behavior are unverified. |
| Cloudflare | Local production build emits the Git SHA in version.json. Hosted verification returned HTTP 403. |

See [iPad installation and signing](ellipse-notes/IPAD_INSTALL_AND_SIGNING.md) and the [implementation record](ellipse-notes/IMPLEMENTATION.md) before physical-device checks.
