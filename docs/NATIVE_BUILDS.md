# Ellipse cross-platform Rust/Tauri builds

The private GitHub repo contains the shared Leptos Rust WebAssembly frontend and Tauri 2 Rust shell.

From an iPad, visit GitHub > Ellipse > Actions > **Ellipse — native packages (manual)** > Run workflow. Select Windows, macOS, Android or iOS compile-check. After a successful run, download the artifact from the workflow run. These workflows are manual to avoid unexpectedly spending private repository CI minutes.

On a computer with Rust, Node and the platform SDK:
```bash
cd native
npm install
npm run icons
npm run tauri -- build
npm run tauri -- android init --ci
npm run tauri -- android build --ci --apk --target aarch64
npm run tauri -- ios init --ci
npm run tauri -- ios build --ci
```

Run the desktop build on Windows or macOS as appropriate. iOS requires a macOS Xcode runner and Apple signing/provisioning to produce a device-installable IPA. The iOS compile-check job **does not** produce an IPA. Android requires Java/Android SDK/NDK. Unsigned results are not store-ready.

**Auth status:** Real Google Firebase popup sign-in works in supported browsers, subject to live testing. Native Google OAuth is not implemented. The native WebView deliberately disables embedded popup authentication until a proper external-browser OAuth handoff is built. Do not claim native auth works yet.

**Store caveat:** Google-only account login may need an exception under Apple App Review Guideline 4.8. Store privacy, account deletion, signing and entitlements must be addressed before distribution.
