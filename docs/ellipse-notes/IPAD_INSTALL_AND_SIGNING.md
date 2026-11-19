# iPad builds and installation

The macOS workflow asks Tauri to produce a Simulator app and an unsigned arm64 IPA. The job checks the device bundle identifier, architecture and minimum OS and attempts to launch the Simulator app when a compatible device is available. The unsigned IPA has no device provisioning profile. It cannot be installed as delivered. A successful hosted job publishes the packages with an IPA payload report and SHA-256 manifest. Run `shasum -a 256 -c ellipse-ios-sha256.txt` after downloading and extracting the iOS build artifacts.

## Run on a registered iPad

1. On a Mac with Xcode and the appropriate iOS SDK, clone the private Ellipse repository. Install the Rust WASM and device targets with `rustup target add wasm32-unknown-unknown aarch64-apple-ios`. Install Trunk and run `npm install` from `native`.
2. Run `npm run icons`, then `npm run tauri -- ios init`. Connect the iPad, turn on Developer Mode, and register its device ID with the Apple team.
3. Run `npm run tauri -- ios dev --open` to open the generated Xcode project through Tauri. Select the application target, enable automatic signing with the authorized team, choose the iPad, and run the app. The team must control the configured bundle identifier `app.ellipse.workspace`, or use a documented separate development identifier for its own build.
4. For an IPA, use a configured Apple Development certificate and development profile for the registered iPad and build with `npm run tauri -- ios build --target aarch64 --export-method debugging`. Verify `codesign --verify --deep --strict` on the unpacked app and confirm its embedded profile matches the app identifier, team and device. Use Xcode's Devices and Simulators window or Apple Configurator to install the signed output. TestFlight requires a distribution build and an App Store Connect record.

Keep any certificate, private key and provisioning materials in the approved Xcode keychain or a protected signing environment. Do not add them to Git or chat. This checkout has no verified signing setup.

If the startup screen opens during troubleshooting, use **Save local diagnostics** and find the report under **On My iPad → Ellipse → Ellipse Backups**. For an app that never reaches its startup screen, use the crash report in Xcode’s Devices and Simulators window. If available, include the iPadOS version and build SHA.

Follow the [physical device recovery and writing checks](IMPLEMENTATION.md#ios-signing-and-ipad-validation) using a disposable vault. Simulator launch cannot verify Pencil pressure, hover, palm rejection, Files interaction or force-quit durability on an actual device.
