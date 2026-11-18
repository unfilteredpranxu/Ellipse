# Ellipse — repository instructions for Codex and other coding agents

This repository is **private, proprietary, and Rust-first**. Preserve its licensing, source privacy, architecture, and existing functionality.

## Mandatory post-login UI visual references

Before editing the authenticated application shell, home page, mode selector, clock, navigation, or post-login responsive layouts, **open and inspect each of these repository-owned screenshots**:

1. [New Tab reference](docs/ui-references/new-tab.jpeg) — black/charcoal bento dashboard, artwork, large clock, customization controls, and card grid.
2. [Tablet Mode reference](docs/ui-references/tablet-mode.jpeg) — light editorial layout, huge typography, left rail, asymmetric artwork grid.
3. [Magazine Mode reference](docs/ui-references/magazine-mode.jpeg) — dark magazine layout, giant wordmark, editorial grid, collections and modules.

Treat these as the primary **visual acceptance references**, not as executable code, verified app features, real metrics, licensed fonts, or actual user data. See [the UI design contract](docs/ui-references/README.md) and [the implementation brief](docs/ui-references/CODEX_TASK.md).

## Non-negotiable implementation rules

- Keep the existing Rust/Leptos WebAssembly front end, Tauri 2 native shell, Google/Firebase auth integration, private GitHub repository, and Cloudflare deployment architecture.
- Show the new home modes **only after authenticated session resolution**. Do not replace genuine authentication with sample credentials or pretend native OAuth works.
- Build **three switchable styles**: New Tab, Tablet, Magazine. They are visual modes selectable on any supported device, not hard-coded platform categories.
- No browser or search bar. New Tab is a personalized workspace, not a web browser.
- A **real** customizable clock on New Tab: local device time, opacity, blur, font styles, format, size, and saved settings. Keep the rest of the screen coherent with its reference.
- Support mobile iOS/Android, iPad, desktop browsers, and Tauri variants with responsive layout, safe areas, accessible controls, no horizontal overflow or clipped controls.
- Preserve the clean knight art in `site/assets/ellipse-knight-clean.jpg`; do not regenerate it or commit screenshots in place of functional interfaces.
- The images include *illustrative* statistics, activity graphs, and items; **never misrepresent these as real user metrics or shipped functionality**. Hook to actual data or clearly label placeholders/disable unimplemented actions.
- Keep build, lint, test, and screenshot checks for Rust/WASM and target viewports. Verify GitHub CI and Cloudflare against the actual commit before claiming success.
- Change production code only when tasked; reference files are assets for development review and must not accidentally be published as user-facing functional content.
