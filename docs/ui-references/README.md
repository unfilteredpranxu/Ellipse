# Ellipse post-login UI — canonical visual references

These three user-approved reference screenshots are the **primary visual specification** for Ellipse's post-Google-login experience. Use the individual JPEGs at full 1536 × 1024 resolution for comparisons rather than their filenames or summaries.

| UI mode | Reference screenshot | Visual character |
| --- | --- | --- |
| **New Tab** | [new-tab.jpeg](new-tab.jpeg) | Near-black bento dashboard, knight hero, large customizable clock, subtle blurred panels, palette and compact control cards |
| **Tablet** | [tablet-mode.jpeg](tablet-mode.jpeg) | Light/off-white editorial canvas, oversized bold title, left navigation rail, asymmetrical imagery and image crops |
| **Magazine** | [magazine-mode.jpeg](magazine-mode.jpeg) | Dramatic dark editorial composition, giant Ellipse wordmark, tall portrait/detail crop, curated content and aligned lower modules |

These are three **user-selectable UI modes** for the same authenticated Ellipse application, **not** separate browsers/apps and **not** a restriction that Tablet Mode only works on tablets.

## Essential contract

- Maintain visual distinction, composition, alignment, image treatment, typography hierarchy, spacing, and mood of all three screenshots.
- Match design rather than artificially stretching a 1536 × 1024 static picture. Build real semantic, interactive Leptos components.
- Screen references show example labels and placeholder content. They **do not establish** real subscription status, 2h34m focus tracking, 21% growth, synchronized spaces, collections, or working shortcuts. Use real data or state honestly that a feature is not available; do not fabricate.
- The real time clock belongs in New Tab. Its appearance should be configurable (opacity, blur, typeface, size, time format, etc.) and persisted. No static “9:41.”
- Keep Google auth state, sign-out, loading/error behavior, and security intact. Post-login UI is gated by resolved authentication.
- Reuse the existing quote-free monochrome knight artwork from `site/assets/ellipse-knight-clean.jpg`. The screenshot files are **references** and must never be used as a giant fake UI background.
- No browser chrome, omnibox, search bar, fake tabs, or invented feature claims.
- Build usable responsive layouts for phone portrait/landscape, tablets, laptop and desktop, preserving each mode's distinct identity. Test safe-area insets on iOS, Android navigation, touch hit targets, and reduced motion.
- Preserve source under a **PRIVATE** GitHub repository. Build web as Rust/Leptos WASM; Tauri 2 is the native shell.

## Implementation/acceptance

See [CODEX_TASK.md](CODEX_TASK.md) for an action-oriented coding brief. Before delivery, compile Rust/WASM, exercise authentication state and mode switching, test settings persistence, run relevant repo checks, take screenshots across device sizes, and compare them to all three reference images. The source comparison remains authoritative if implementation details need to change as Ellipse develops.

## Provenance

These files are provided by the Ellipse project owner as visual design references. Preserve their filenames and intent. Do not optimize them destructively, replace them with new generated imagery, or expose them in a public repository.
