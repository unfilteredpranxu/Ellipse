# Ellipse

**Ellipse** is a private, proprietary, Rust-first cross-platform personal workspace. Copyright © 2026 unfilteredpranxu. All rights reserved. See [LICENSE](LICENSE).

- **GitHub:** private source repository `unfilteredpranxu/Ellipse`.
- **Cloudflare:** https://ellipse-workspace.pages.dev/
- **Rust UI:** `web/` — Leptos CSR + WebAssembly.
- **Native application:** `native/` — Tauri 2 Rust shell targeting Windows/macOS/iOS/Android.
- **Rust shared logic:** `crates/ellipse-core/`.
- **Fallback static Cloudflare site:** `site/`, temporarily used while the Rust build/deploy pipeline is validated.

## Visual requirements
Original monochrome artwork, full-screen edge-to-edge on phone, glass Google-only button anchored above safe area, no white card, no email/password/OTP, no quotation section. The same UI source is shared by Tauri and browser when Leptos builds.

## Build the Rust web UI
Install Rust and Trunk, then:
```bash
bash scripts/build-pages.sh
```
Output: `dist/`. A Cloudflare Pages Git integration can use build command `bash scripts/build-pages.sh` with output directory `dist` after the build is validated. The build writes `dist/version.json` with the source commit SHA for deployment verification.

## Build native applications
Install Node.js, Rust, Trunk, platform SDKs, then:
```bash
cd native && npm install
npm run tauri -- build                  # Windows/macOS desktop on each respective OS
npm run tauri -- android init
npm run tauri -- android build          # Android SDK+NDK required
npm run tauri -- ios init
npm run tauri -- ios build              # macOS + Xcode only
```
The iOS/macOS tools must run on macOS. Windows installer builds run on Windows. Android needs SDK/NDK/JDK. Native app source and a compilable shell are **not** evidence that APK/IPA/EXE/DMG builds or native Google login have been validated.

## Authentication
`web/auth.js` and `site/auth.js` implement real Firebase Google sign-in for supported browsers. Set Firebase Authentication → Authorized domains to include the active Pages hostname. Web popups may be blocked on some mobile browsers. Native OAuth must use a platform-appropriate browser/redirect provider; webview popup auth is explicitly disabled pending that implementation.

## Quality/security
Auth providers only: Google. Browser files are inspectable even though this repository is private. Never commit client secrets. App Store/Play Store purchases, promo plans, entitlements and account deletion remain separate future work.

## Workspace appearance

After signing in, use the home mode control to switch between New Tab, Tablet, and Magazine. New Tab has a live local clock. Clock customisation, wallpapers, and changes to New Tab blocks are saved in this browser or app's storage for the current Firebase account. The Presets panel saves named snapshots of the current workspace.

In the browser, the Presets panel can back up or restore those snapshots using the hidden Google Drive app data folder. The Google Cloud project used by Firebase Google sign-in must have the Drive API enabled and its OAuth consent configuration must permit the `drive.appdata` scope. Ellipse requests that scope when a Drive action is confirmed. Each action reauthenticates the currently signed-in user and uses a short-lived access token in memory.

To run the Chromium acceptance tests from a built checkout, run `bash scripts/build-pages.sh`, then run `npm ci`, `npx playwright install chromium`, and `npm test` from `tests/browser`. Set `CHROMIUM_PATH` to use a system Chromium. The tests stub the Firebase and Drive endpoints and exercise the actual release WASM. Live Google account and native-device tests need their respective environments.
