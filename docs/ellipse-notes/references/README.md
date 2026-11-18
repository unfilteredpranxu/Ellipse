# Ellipse Notes — canonical image reference pack (November 18, 2026)

These screenshots were supplied by the Ellipse project owner. They are **design/behavior references**, not application assets or evidence of implemented features. Keep them PRIVATE in this GitHub repository; never ship the screenshots as a flattened substitute for a working interface. Source previews have been encoded for repository reference storage.

## Priority and interpretation

| File | Status | What to learn |
| --- | --- | --- |
| [01-library-cream.jpeg](01-library-cream.jpeg) | **PRIMARY: visual source of truth for Notes Library** | Cream editorial layout; thin header; centered "My Notebooks" title, understated tabs; a continuous row of tall *physical book spines*; chosen notebook projected forward as its full cover; precise shadows and low-key bottom metadata/arrows; unhurried negative space. Reproduce this composition faithfully rather than showing generic cards or a dashboard. |
| [02-owlluna-ink-editor.jpeg](02-owlluna-ink-editor.jpeg) | **FUNCTIONAL REFERENCE, not a visual skin** | OwlLuna's real-world, distraction-free editor, full-page handwriting area, compact configurable pen bar, tool/selection state, undo/redo, page navigation. Do not copy OwlLuna's dark styling when implementing Ellipse Notes. |
| [03-owlluna-library-toolbar.jpeg](03-owlluna-library-toolbar.jpeg) | **FUNCTIONAL REFERENCE, not a visual skin** | Practical list/grid switch, sorting, selection, new-item action with menu, search. Integrate these capabilities gracefully into Ellipse's cream editorial UI. |
| [04-owlluna-library-sidebar.jpeg](04-owlluna-library-sidebar.jpeg) | **FUNCTIONAL REFERENCE, not a visual skin** | Notebook library, favorites, recently deleted, shelves, new-folder action. Preserve functional information architecture; restyle and support responsive drawers/sheets. |

**The person opening a notebook/PDF must no longer see the library.** A dedicated full-page editor takes over; any timer panel is **hidden by default** and can be shown/hidden independent of timer running state. Opening an existing item restores page/zoom; back restores selected book and carousel position.

OwlLuna upstream: https://github.com/owaisazmal/OwlLuna
- Actual SwiftUI/PencilKit iOS/macOS application (NOT a Rust or browser editor).
- Upstream LICENSE: MIT, copyright © 2024–2026 Owais Khan. If reusing any code/substantial portions, preserve the original MIT copyright and permission text in appropriate third-party notices. Inspect third-party art/fonts separately. Do not present OwlLuna as authored by Ellipse.
- The Apple Pencil/PencilKit engine cannot simply be compiled into a cross-platform Rust WASM/Tauri build. Use it to study behavior and edge cases; design a portable Rust document/ink model and narrow native bridges only as needed.
- Do NOT copy copyrighted UI illustrations or proprietary third-party assets into Ellipse without authorization.

## Visual acceptance

- [ ] On a 1536×1024 desktop viewport, library composition is demonstrably close to reference 01: whitespace, book-spine proportions, selection projection, typography, tabs, metadata, arrows.
- [ ] iPad/tablet touch carousel is fluid, books remain recognizable, sidebar/toolbar controls are discoverable.
- [ ] iPhone/Android layout is deliberately adapted rather than squashed; supports safe areas, screen reader, touch, reduced motion.
- [ ] Book covers/spines are real customizable, generated/rendered components from document metadata—not a single flattened picture.
- [ ] Functional OwlLuna-derived requirements (toolbar, shelves, pens) work while the UI consistently follows the Ellipse design system.
- [ ] Real editor opens separately with page/pen tools, saving, and optional focus timer.
- [ ] Any sample text/graphics in images are illustrative—not real user notebooks, study metrics, or evidence of shipped features.
- [ ] Screenshot tests, native tests, and source-backed evidence—not self-assessed "100% perfection".

Read [THE ONE MASTER CODEX TASK](../CODEX_ULTIMATE_SINGLE_TASK.md) before making changes.
