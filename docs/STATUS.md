# Ellipse implementation status — M01

## Completed and published
- Proprietary `LICENSE` on private GitHub repository.
- First-install welcome/sign-in visual preview using original user-supplied black-and-white artwork.
- Interactive local-only entry, notes and persistence in browser localStorage (subject to browser eviction).
- Cloudflare Pages project `ellipse-workspace` connected to private GitHub `main` branch for automatic deployments.
- PWA manifest and offline shell service worker in the `site/` preview.
- Initial Rust workspace `crates/ellipse-core` with domain logic and tests.
- Tauri 2 Rust native shell scaffold in `native/`. It currently uses the same frontend files as the web preview.

## Not yet implemented or validated
- Google OAuth: screens exist but real sign-in and managed account creation are **not configured**.
- Subscriptions, payment verification, coupons, cloud entitlements: **not implemented**.
- Google Drive sync: **not implemented**.
- Native APK/Windows installer: **not built or device-tested**.
- Browser interface is currently legacy HTML/CSS/JS, not Rust Leptos/Wasm. Migrate the UI to shared Leptos components in the next architecture milestone.
- Art licensing for public commercial distribution: must be verified.
- Automated end-to-end tests, synchronization, encryption and production auth: pending.

## Golden rule
One screen at a time. Do not claim preview-only functions are production-ready. Keep repository private and paid license verification server-side.
