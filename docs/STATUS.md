# Ellipse M02 — evidence status

## Implemented in source
- Private/proprietary repository stays the source of truth.
- Replaced the legacy quotation-embedded JPEG with the user's clean standalone monochrome artwork at site/assets/ellipse-knight-clean.jpg; no retouching or quote overlay. The former image is removed from the current tree.
- Phone layout: edge-to-edge monochrome knight, frosted Google button only; iPad/desktop layout: unblurred split-panel welcome and clean image.
- Firebase Google-only web sign-in bridge using the existing ellipse-84ac1 project (existing authorized Pages domain).
- Rust Leptos client-side rendered welcome screen in web/src/main.rs, using Trunk to build WebAssembly.
- Tauri 2 native shell consumes the compiled Leptos frontend when built.
- Cloudflare Pages is configured to build and serve the Rust/Leptos WASM via scripts/build-pages.sh into dist/. Confirm deployment success and visually smoke-test the newest production URL.

## Still require test/evidence
- Browser Google popup, Firebase session persistence, browser and platform QA.
- Rust/Leptos WASM and all Tauri native build outputs must pass actual CI/build tests.
- Google native OAuth for iOS/Android/Windows/macOS is **NOT IMPLEMENTED**. Firebase web popup is intentionally blocked inside Tauri webviews; do not misrepresent it as native OAuth.
- Apple/Google developer credentials, app signing and store compliance for native releases.
- Subscription backend and entitlements, cloud sync, full application workspace.

## Production caution
Only web client Firebase configuration is checked in; it is public by Firebase design. Do not put any OAuth client secrets, Firebase Admin service credentials, Apple signing identities or paid entitlement authority in static/browser code.
