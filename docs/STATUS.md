# Ellipse M02 — evidence status

## Implemented in source
- Private/proprietary repository stays the source of truth.
- Original artwork extracted byte-for-byte from the prior self-contained HTML to site/assets/ellipse-art.jpg.
- New full-screen dark welcome with no white form, no quote box, only the bottom glass Google button.
- Firebase Google-only web sign-in bridge using the existing ellipse-84ac1 project (existing authorized Pages domain).
- Rust Leptos client-side rendered welcome screen in web/src/main.rs, using Trunk to build WebAssembly.
- Tauri 2 native shell consumes the compiled Leptos frontend when built.
- Cloudflare Pages keeps serving site/ until its build command and output directory are deliberately switched to scripts/build-pages.sh and dist/.

## Still require test/evidence
- Browser Google popup, Firebase session persistence, browser and platform QA.
- Rust/Leptos WASM and all Tauri native build outputs must pass actual CI/build tests.
- Google native OAuth for iOS/Android/Windows/macOS is **NOT IMPLEMENTED**. Firebase web popup is intentionally blocked inside Tauri webviews; do not misrepresent it as native OAuth.
- Apple/Google developer credentials, app signing and store compliance for native releases.
- Subscription backend and entitlements, cloud sync, full application workspace.

## Production caution
Only web client Firebase configuration is checked in; it is public by Firebase design. Do not put any OAuth client secrets, Firebase Admin service credentials, Apple signing identities or paid entitlement authority in static/browser code.
