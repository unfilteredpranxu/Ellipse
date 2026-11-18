# Ellipse — platform UI and sign-in acceptance (2026-11-18)

## Responsive sign-in layout
- iPhone, Android phones and mobile browsers: the knight artwork fills the screen edge to edge. There is **no** white card, overlay quote, or text panel. Only the Google button floats above the bottom safe area. The button alone gets a translucent backdrop blur and short press transitions.
- iPad apps and iPad web (portrait or landscape), Windows, macOS and desktop browsers: use the user's second reference image as the composition: clean warm-white left panel with Ellipse branding, "Welcome back", "Sign in to pick up where you left off." and non-blurred outlined Google button; monochrome knight artwork fills the right panel. No image filters or texture regeneration.
- At widths under 701px, and low-height coarse-pointer phone landscape, switch to mobile composition. Tablet split-view narrow windows may use the phone layout.
- Shared implementation: `web/src/main.rs` (Leptos/Rust), `web/style.css`; `site/` is a static fallback for comparison only.
- The black-and-white artwork committed as `site/assets/ellipse-art.jpg` predates the new reference supplied on 2026-11-18; exact replacement of this binary with `IMG_0043(1).png` is a separate asset operation. Do not claim those image bytes are identical.

## Auth — what works and what doesn't
- Browser: Firebase Auth GoogleAuthProvider using `signInWithPopup`, persistence via Firebase SDK and `onAuthStateChanged`. No Google login result has been manually tested on a real account in this task. The Firebase Authentication authorized domains need `ellipse-workspace.pages.dev` and any other deployment host.
- Tauri iOS/Android/macOS/Windows: **native Google OAuth remains unimplemented**. The web JS bridge currently shows a clear explanatory error inside Tauri instead of attempting disallowed embedded-webview OAuth.
- Production native sign-in design: Google OAuth in a system browser or platform auth session using PKCE and platform OAuth client IDs; validate state/nonce, use securely registered app/loopback callbacks, exchange credentials with Firebase in the native application, and store refresh/session tokens in platform protected storage. No tokens in URL fragments, logs, or localStorage. Require independent iOS, Android, macOS, and Windows integration tests.
- Apple App Store Review Guideline 4.8 normally requires an equivalent privacy-focused sign-in option for a consumer app using Google as its primary-account sign-in; narrow exceptions must be evaluated before release.

## Build and evidence
- Browser/WebAssembly: `bash scripts/build-pages.sh` builds Rust/Leptos using Trunk; Cloudflare Pages serves `dist`.
- Tauri native: GitHub Actions `native-packages.yml` has manual platform builds, and this workflow's first push triggers a four-target build smoke test. It produces packages only after successful job completion.
- iOS job is Xcode project + Rust target compile check, **not** a signed installable IPA; App Store distribution requires Apple developer team/signing/provisioning.
- Android and desktop packages require platform signing/distribution before production release.
- Browser source bootstrap HTML is unavoidable; compiled Rust UI appears as `.wasm`. Tauri uses Rust host code plus the same Rust-built webview UI, not a pure native widget renderer.
