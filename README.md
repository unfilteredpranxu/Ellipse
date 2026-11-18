# Ellipse

[![License: Proprietary](https://img.shields.io/badge/License-Proprietary-1a1a1a?style=flat-square)](LICENSE)
![Source: Private](https://img.shields.io/badge/Source-Private-555555?style=flat-square)
![Status: M01](https://img.shields.io/badge/Milestone-M01-555555?style=flat-square)

**Ellipse** is a proprietary, Rust-first, offline-first personal workspace. Our intended products include an iOS/iPadOS PWA, web app, Android APK, Windows desktop app, and potential macOS build.

## Try the live welcome-screen preview

**[Open Ellipse on Cloudflare Pages](https://ellipse-workspace.pages.dev/)**

The public URL hosts only browser-delivered client files from the **private** GitHub repository. Source-code, secrets and future license verification services must stay private. Browser-delivered JavaScript/HTML/CSS remain inspectable.

### What works today

- The supplied monochrome art and M01 welcome screen, at phone and desktop sizes.
- Interactive sign-in/register/recovery **UI**, without real cloud authentication yet.
- **Enter locally**: opens a basic local workspace with notes stored in browser localStorage.
- PWA manifest and offline shell caching after installation/first load.
- Cloudflare Pages deploys automatically from `main` with no build command and output directory `site`.
- Rust core under `crates/ellipse-core` with unit tests; native Tauri/Rust shell scaffold under `native/`.
- GitHub Actions runs Rust tests and a static preview acceptance check.

### What does NOT work yet

**Google sign-in, purchases, premium licensing, Drive sync, native APK/installers, and fully Rust-rendered web UI are not implemented/validated.** Don't treat demo UI as production features.

The current visual web UI is an HTML/CSS/JS prototype preserving the original design; the native shell is Rust (Tauri). The next architecture task is a shared Leptos/Rust WebAssembly UI while maintaining screenshot fidelity.

## Project layout

- `site/` — deployable M01 preview and PWA assets (temporary frontend until shared Rust UI migration).
- `crates/ellipse-core/` — Rust domain logic and tests.
- `native/` — Rust/Tauri desktop/mobile shell scaffold sharing the web frontend.
- `scripts/verify_site.py` — static preview acceptance checks.
- `docs/STATUS.md` — implementation truth table.
- `.github/workflows/verify.yml` — automated tests.

## Test the Rust foundation

```bash
cargo test --workspace --all-targets
```

## Licensing

Copyright © 2026 unfilteredpranxu. All rights reserved. This repository is **closed-source**, not open-source. See [LICENSE](LICENSE). Third-party libraries retain their own licenses. Any compiled-app end-user license will be separate.

> Future commercial publication of uploaded reference artwork depends on applicable rights and permissions.
