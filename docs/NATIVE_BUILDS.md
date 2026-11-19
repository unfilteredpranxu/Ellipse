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

## Installed Google sign-in

1. Select **Continue with Google** in the installed app. Ellipse opens its Cloudflare pairing page in your default browser.
2. Continue with Google on that page, copy the encrypted response, and return to the same open instance of Ellipse.
3. Paste it into **Sign-in response** and select **Finish Google sign-in**.

The pairing page uses the existing Firebase Google provider. Its short-lived browser session stays in memory. The Google credential is encrypted to the installed app's ephemeral P-256 key with HKDF-SHA-256 and AES-GCM. The clipboard carries ciphertext. The app accepts a response once within ten minutes, then uses Firebase to establish an account session in the packaged WebView. Signing out clears that session. The device-local Notes vault remains available from the installed app independently of the account.

Deploy `/native-sign-in/` with the Pages output before distributing an updated native build. The production Pages origin must be authorized for the existing Firebase Google provider. Verify first sign-in, relaunch, sign-out and account switching on each installed platform. The local browser tests use Firebase and native IPC fixtures.

**Store caveat:** Google-only account login may need an exception under Apple App Review Guideline 4.8. Store privacy, account deletion, signing and entitlements must be addressed before distribution.
