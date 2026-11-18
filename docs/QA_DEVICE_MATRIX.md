# Device and build verification

| Environment | Evidence |
| --- | --- |
| Linux Rust | Unit and SQLite integration suites, including legacy conversion, checkpoint and archive tests. |
| Chromium | Automated Notes, home modes, Calendar, syllabus and Focus workflows. The suite uses a native IPC fixture and a Firebase network fixture where applicable. |
| Ubuntu 24.04 | Local container regression uses Playwright with a compatible Chromium executable and checks PDF outputs with Poppler. |
| iOS Simulator, iPad, iPhone | Hosted workflow has a package and launch check. Current run access is blocked by HTTP 403. Physical Apple Pencil, force-quit, Files and runtime checks are unverified. |
| Android, Windows, macOS | Existing GitHub native workflows cover packaging. Current hosted results and installed app behavior are unverified. |
| Cloudflare | Local production build emits the Git SHA in version.json. Hosted verification returned HTTP 403. |

See [iPad installation and signing](ellipse-notes/IPAD_INSTALL_AND_SIGNING.md) and the [implementation record](ellipse-notes/IMPLEMENTATION.md) before physical-device checks.
