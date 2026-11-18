# Codex task — implement Ellipse's three post-login UI modes

You are working in the existing **private** `unfilteredpranxu/Ellipse` GitHub repository. **First inspect `AGENTS.md`, `docs/ui-references/README.md`, and each of the three screenshot JPEGs in this directory.**

## Deliverable

Implement a fully responsive, functional **Rust/Leptos authenticated home page** with three rapidly switchable, persisted visual modes:

- **New Tab:** faithfully reproduce [new-tab.jpeg](new-tab.jpeg)'s black/charcoal bento dashboard and its large, actual local-time **customizable clock**. Implement adjustable clock opacity, backdrop blur, font choice/style, size, formatting (12/24-hour, optional seconds and date), live preview and reset. Persist controls per account locally. No browser search or address bar.
- **Tablet Mode:** faithfully reproduce [tablet-mode.jpeg](tablet-mode.jpeg)'s light editorial layout, oversized title, left sidebar, and image/content grid. This mode remains available on phones and desktops with deliberate responsive compositions.
- **Magazine Mode:** faithfully reproduce [magazine-mode.jpeg](magazine-mode.jpeg)'s dramatic dark editorial typographic hierarchy and curated grid, adapted carefully across mobile and desktop widths.

Implement shared Rust application state, navigation/mode selection, real control handlers, persistence, accessibility, and preference restoration. Reuse the existing knight artwork rather than regenerating it or painting an image into a fake UI background. The screenshot's metrics and content are examples; do not assert such data is real or implement fake subscriptions/progress.

**Authentication:** preserve the existing real Firebase Google browser flow and session resolution. The three home modes must only render after authentication. Existing native OAuth may be incomplete: assess it honestly and do not bypass or claim success without test evidence.

**Platform support:** iOS/Android phones, iPad, desktop web, Windows/macOS Tauri. Handle portrait/landscape, safe-area insets, Dynamic Island/notch, Android navigation bars, touch targets, reduced-motion, no horizontal overflow, no clipped text. Test representative widths 320, 360, 390, 430, 768, 820, 1024, 1280, 1440 CSS px.

## Technical requirements

Keep the established Rust/Leptos, Trunk/WASM, Tauri 2, Firebase, CI and Cloudflare architecture. No React/Next/Vue rewrite or HTML-only substitute. No regression to the existing login experience. Keep all secrets out of git.

Investigate the current repository and plans before editing. Implement and iterate based on real build/test results. Run Rust formatting, tests and WASM release compilation. Test UI state and screenshot fidelity with browser automation when available. Validate native builds on real runners where possible and clearly distinguish compile checks from signed device-ready builds.

Once tests pass, commit and **push** the implementation to the private repository without force-pushing or overwriting concurrent edits. Allow the existing Cloudflare Pages Git integration to deploy it; verify the production deployment status and commit SHA, and smoke-test `https://ellipse-workspace.pages.dev/`. Fix detected errors. Do not claim a release or cross-platform runtime works without evidence.

Report final commit, URLs, visual screenshots, tests run/results, deploy status, and unresolved blockers. **Do the engineering, not only a plan.**

## Reference priority

When UI decisions conflict, the screenshots are the primary visual specification; this brief establishes real behavior, authentication, and responsive requirements. See `docs/ui-references/README.md` for the interpretation contract.
