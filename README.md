# Ellipse

**Ellipse** is a private, proprietary, Rust-first cross-platform personal workspace. Copyright © 2026 unfilteredpranxu. All rights reserved. See [LICENSE](LICENSE).

- **GitHub:** private source repository \`unfilteredpranxu/Ellipse\`.
- **Cloudflare:** https://ellipse-workspace.pages.dev/
- **Rust UI:** \`web/\` — Leptos CSR + WebAssembly.
- **Native application:** \`native/\` — Tauri 2 Rust shell targeting Windows/macOS/iOS/Android.
- **Rust shared logic:** \`crates/ellipse-core/\`.
- **Fallback static Cloudflare site:** \`site/\`, temporarily used while the Rust build/deploy pipeline is validated.

## Visual requirements
Original monochrome artwork, full-screen edge-to-edge on phone, glass Google-only button anchored above safe area, no white card, no email/password/OTP, no quotation section. The same UI source is shared by Tauri and browser when Leptos builds.

## Build the Rust web UI
Install Rust and Trunk, then:
\`\`\`bash
bash scripts/build-pages.sh
\`\`\`
Output: \`dist/\`. A Cloudflare Pages Git integration can use build command \`bash scripts/build-pages.sh\` with output directory \`dist\` after the build is validated.

## Build native applications
Install Node.js, Rust, platform SDKs, then:
\`\`\`bash
cd native && npm install
npm run tauri -- build                  # Windows/macOS desktop on each respective OS
npm run tauri -- android init
npm run tauri -- android build          # Android SDK+NDK required
npm run tauri -- ios init
npm run tauri -- ios build              # macOS + Xcode only
\`\`\`
The iOS/macOS tools must run on macOS. Windows installer builds run on Windows. Android needs SDK/NDK/JDK. Native app source and a compilable shell are **not** evidence that APK/IPA/EXE/DMG builds or native Google login have been validated.

## Authentication
\`web/auth.js\` and \`site/auth.js\` implement real Firebase Google sign-in for supported browsers. Set Firebase Authentication → Authorized domains to include the active Pages hostname. Web popups may be blocked on some mobile browsers. Native OAuth must use a platform-appropriate browser/redirect provider; webview popup auth is explicitly disabled pending that implementation.

## Quality/security
Auth providers only: Google. Browser files are inspectable even though this repository is private. Never commit client secrets. App Store/Play Store purchases, promo plans, entitlements and account deletion remain separate future work.
