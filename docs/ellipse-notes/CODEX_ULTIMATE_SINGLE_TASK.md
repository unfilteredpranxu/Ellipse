# ELLIPSE NOTES — THE ONE ULTIMATE CODEX IMPLEMENTATION TASK

## Mandate

You are the lead Rust document-engine architect, cross-platform Tauri engineer, stylus/graphics engineer, storage/recovery engineer, interaction designer, and QA owner for **Ellipse Notes**. This task is a single coordinated end-to-end implementation effort, not a request for a plan, prototype, code snippets, or a UI-only demo.

**Repository (PRIVATE):** https://github.com/unfilteredpranxu/Ellipse  
**Website:** https://ellipse-workspace.pages.dev/  
**Upstream functional reference:** https://github.com/owaisazmal/OwlLuna  
**One task, no user-facing milestone follow-up questions:** Audit → design mapping → implement → test → fix → integrate → commit → push → CI → deploy → report. Work as deeply as tooling and execution budget allow; when environment/time/hardware prevents work, checkpoint durable code and a precise truthful status rather than saying it is finished. Never invent tested features, native device results, or zero-crash guarantees.

### Read FIRST — do not skip inspecting the actual images

- AGENTS.md
- docs/ui-references/new-tab.jpeg
- docs/ui-references/tablet-mode.jpeg
- docs/ui-references/magazine-mode.jpeg
- docs/ellipse-notes/references/README.md
- docs/ellipse-notes/references/01-library-cream.jpeg
- docs/ellipse-notes/references/02-owlluna-ink-editor.jpeg
- docs/ellipse-notes/references/03-owlluna-library-toolbar.jpeg
- docs/ellipse-notes/references/04-owlluna-library-sidebar.jpeg

**Reference priority:** Image 01 is the VISUAL AUTHORITY for the Notes LIBRARY. The three OwlLuna screenshots are **FUNCTIONAL BEHAVIOR** references for pen bar, undo/redo, note canvas, sorting, search, selection, folder sidebar and recently deleted. They are NOT a license to clone OwlLuna's dark aesthetics. The existing Ellipse three-home-mode references govern the rest of the host application, which must remain intact. In particular, reject the earlier AI-style black-bento-book-gallery designs as the Notes Library baseline. Make the library feel like the CREAM editorial photographed physical-book collection, with selected full cover, book spines in one horizontal film-strip and abundant negative space. Give it custom-brand personality without dashboard slop.

## 0 — Grounding, safety and boundaries

1. Read actual current branch HEAD and git status; inspect all relevant Rust workspace manifests, current Leptos UI, existing Firebase browser auth, Tauri native shell, routes, settings persistence, Cloudflare Pages scripts, GitHub Actions, content-security controls and third-party licenses.
2. Re-read OwlLuna's README, LICENSE, Canvas/, Editor/, Storage/, PDF, input, UI and tests (pin upstream SHA), to understand behavior, failure cases, model designs, inking workflows, pen configuration, undo/redo, notebooks, templates, PDF and recovery. OwlLuna is **Swift/SwiftUI/PencilKit**; study it, do NOT claim it is Rust, and do NOT blindly transplant Apple-only classes to Android/Windows.
3. OwlLuna's LICENSE is MIT (© 2024–2026 Owais Khan); if code/substantial portions are reused, retain notices in source/distribution per MIT. Audit separately licensed typefaces, cover artwork, sound libraries and third-party dependencies. User screenshots are private *reference images*, not site assets. No public source repository, no public forks, no exposing secret tokens or user documents. Ellipse stays proprietary, with required notices for components we actually reuse.
4. Keep current Ellipse authenticated home modes, real Google/Firebase browser sign-in, native packaging, artwork customization, tablet-only existing dashboard palette behavior, navigation and settings intact. Existing platform OAuth gaps must be documented rather than bypassed.
5. No invented analytics, fake notes, fake subscriptions, fabricated promises of iCloud/Drive sync, or UI buttons that pretend to perform features they don't. All example data used in visual tests must be clearly test fixtures only.
6. Keep native notes documents LOCAL by default. Do not upload PDFs, handwriting, voice recordings, covers or other personal content to Cloudflare, Firebase or Drive without separately implemented consent and authorization.

## 1 — Product architecture and route behavior

Implement an integrated **Ellipse Notes** first-class section inside authenticated Ellipse (not a standalone unrelated web app).

- Distinguish NOTES LIBRARY, FOLDER/SHELF, NEW NOTEBOOK, NOTEBOOK EDITOR, PDF READER/ANNOTATOR, SPLIT WORKSPACE, STUDY/FOCUS, APPEARANCE and BACKUP/EXPORT.
- When user opens a particular NOTEBOOK OR PDF, replace the library with a full-screen focusable document editor. No persistent shelf, carousel, gigantic title, dashboards or sidebar from library. Optional page thumbs, toolbar and timer are collapsible; chrome auto-hides where useful without hiding important safety status. On back, restore selected book, carousel offset, current collection, filters, folder, search and scroll without losing unsaved data.
- Timer is HIDDEN BY DEFAULT. Toggling timer visibility never stops a running session. Provide portable compact timer pill/floating dock on demand, and hide/move/resize/dock as supported.
- Website product policy: read-only Notes browser/PDF preview, not full creation/inking/annotation. Website may open a user-selected local PDF without uploading it, and show only genuinely shared/synced notes if an actual secure sync exists. No access to native sandbox from website. Native installed apps do real authoring.
- Keep shared portable document model and core business logic in Rust; Tauri 2 wraps iOS/iPadOS, Android, macOS, Windows; Leptos renders UI. Evaluate and implement dedicated native stylus hooks or graphics surfaces where webview pointer events or latency demand it. Do not claim "pure Rust" eliminates OS-specific input bridges.
- Architect for multiple windows and efficient memory usage. Use typed events, single clear state owner per document, and avoid full frontend rerender on each stylus sample.

## 2 — Notes Library: visual fidelity and beautiful usable motion

Replicate the visual composition of the supplied CREAM image, precisely for desktop at ~1536×1024:
- Off-white/cream quiet canvas; restrained paper texture if it remains crisp and doesn't fake image degradation. Minimal top line: Ellipse / Notes identity at left, editorial-centered Library / Folders / Subjects / Collections (adaptive), small search and account.
- Centered “My Notebooks” title, small real item-count, restrained All/Notebooks/PDFs/Favorites/Recent segmented filter.
- Dominant middle zone: an edge-to-edge row of tall, authentic-looking 3D book spines (varying real thickness, user-customized cover/spine design). Selecting a book moves it forward, exposes the full cover, shifts neighboring spines smoothly, subtly changes perspective/shadows and synchronizes title, type, page count and contextual actions beneath. Preserve generous blank space and understated edge arrows.
- Cohesive authentic book appearance: faithful geometry, covers, material, wraparound edges, text along spines, bevel, slight wear where applicable. Customizable designs and image covers; don't use one big flattened image to fake functionality.
- Cards, settings sliders, colorful charts and random callouts MUST NOT compete with the central books. Practical create/import, sort/list, folder tree are present through menus, trays and optional side panels, not through a thick permanent dashboard.
- Motion: pointer hover (desktop), click/tap, inertial mouse/trackpad wheel, touch swipe, keyboard left/right and Enter, velocity-dependent spring settling, snapping, reversible/interrupted animation, selected-spine projection and restrained book-opening transition into an actual editor route. Do not overshoot so far selection is lost. Make pointer vs touch interactions intentional. Respect prefers-reduced-motion and reduce expensive effects during low-power mode.
- Desktop large library: efficient visible-range virtualization/windowing, quick search/sort/list mode for 1k+ items, accessibility, clean empty states.
- iPad landscape/portrait: maintain strong physical carousel and large touch targets with actual iPad layout, safe-area support.
- iPhone and Android: truly designed mobile adaptation with swipeable books, no squeezed microscopic full-width desktop carousel; folders/search/Create accessible. Explicit orientation behavior. Dark appearance selectable; cream is default reference.

## 3 — Real library functionality

Implement complete persisted folder/shelf library with nested folders; notebooks/PDFs; tags; subjects; favorite/pin; smart Recent; archive; Recently Deleted/trash with reversible restore policy and explicit permanent delete; list/grid alternate to carousel; sort/name/created/modified; full-text search of available typed/indexed text and PDF text if imported; safe rename, move, duplicate, merge where supported, drag/drop or touch-friendly equivalent; resilient bulk select/action menus; keyboard shortcuts; breadcrumb navigation; stable IDs/relationships with cycle-prevention for folder moves.

Imported PDFs remain distinguishable from notebooks and keep original bytes. No placeholder-only actions. Notebook creation asks name, subject/folder, cover/spine and initial paper; can start quickly with defaults. Book-opening preserves selected page, zoom, last-used pen and view state.

Notebook covers: custom local artwork, built-in vector patterns, typography, spine title/orientation, book thickness according to pages if useful, color/material, cover preview, image crop/position/zoom, cancel/reset, preview and upload limits. Assets deduplicated by hash/ref; no unnecessary duplicate megabyte images. Preserve copyright notices if applicable.

Independent Notes Library appearance: choose local wallpaper, fit/position/zoom, blur, brightness, opacity, frosted amount; appropriate contrast and preview/apply/cancel/reset; per-account persistence. NEVER change other Ellipse main UI themes or the paper/ink content implicitly. Limit expensive blur, avoid "fancy glass", excessive gradients and card stacks.

## 4 — Durable storage foundation (before complex drawing)

Design a robust, versioned portable Ellipse notebook format:
- SQLite with WAL/appropriate platform-safe transactions for metadata, folders, tags, subjects, thumbnails/indices, session logs and migrations.
- Notebook package storing manifest/version, pages/IDs, vector ink/segments, immutable original PDFs, PDF annotation layers, text objects, photos/assets, audio recordings, thumbnails and recovery metadata. Choose clear portable representations (e.g. serde/CBOR/JSON appropriately); document canonical coordinate system.
- Per-account sandboxed storage including local offline access policies. Signing out or switching accounts must never leak data. Prevent path traversal on import; reject ZIP bombs, unsupported input, malicious SVG/PDF content as applicable; no executing embedded scripts. Validate MIME and file signatures, limits, checksums and image orientation.
- Save journals/checkpoints, atomic replacement and fsync/durability behavior appropriate to each OS. Do not write on every input point; buffer safely with bounded loss window. Autosave correct dirty/saving/saved/error status. Never falsely report Saved. Restore from partial writes or quarantine corruption without overwriting last-good copies. Transactionally update deleted/moved objects, dedupe assets, and repair references.
- Ensure imported PDF and original image/audio assets are preserved; ink is editable vector data, not just screenshots. Document schema migration and backup/restore/export; test unexpected termination during save, import, bulk delete and restore.
- Native-first creation/inking uses secure Tauri filesystem operations and app directories. Browser view may use transient local PDF input without native private-file access. Don't rely on localStorage for actual native documents. Account data remains private unless user explicitly exports it.

## 5 — Full document editor: precise, beautiful and predictable

Make editor a dedicated full-screen page with calm paper filling the available area, minimal refined top chrome, compact pen bar/tool drawer, optional thumbnails, optional page strip and optional timer. Do not display the library while editing. Selected tool/page and Saving status always clear without big overlays.

Ink model: stable stroke IDs and points in document coordinates, pressure, tilt/azimuth where available, timestamp (if needed), width/opacity/color/brush curve, transforms, layer ordering, bounding boxes; convert device pointer coordinates correctly through scroll, page pan/zoom, viewport resize, DPI and rotation. Handle raw/predicted/coalesced samples when supported. Input events robust across touch vs stylus, pencil hover/double-tap/squeeze where available, eraser tip, palm rejection, pointer capture/cancel, pressure and tilt. Validate on actual hardware; provide mouse/touch fallback; no unverified "Apple Pencil Pro fully supported" promises.

Pen presets: ballpoint, fountain, fineliner, pencil, brush/calligraphy, marker/highlighter; genuinely distinct stroke physics and rendering. Adjustable width/pressure curve/smoothing/taper, opacity, color, texture where appropriate, saved presets and favorites; pen preview in compact picker, never permanent sprawling controls. Use non-destructive realtime adaptive smoothing, avoid lag/jitter/ink gaps, zoom invariant source geometry. Profile high-DPI and long-stroke rendering with CPU/GPU as appropriate. Prioritize stylus latency over page animations.

Erasers: whole-stroke and true partial/vector-segment eraser, spatial hit-testing, correct splitting geometry, layers, boundaries and transforms, undo restoring exact originals; never fake erasing by painting paper color. Test crossing highlights/ink/transparency/zoom and small near neighbors.

Lasso/selection: freeform + rectangular, move/scale/rotate/cut/copy/paste/duplicate/deletion, multiple vector strokes, text, shapes and images. Clipboard image/text integration and move across compatible pages. Clear pointer affordances/hit areas; group changes logically.

History: intuitive unlimited-in-concept but memory-bounded undo/redo with checkpoints, meaningful gesture groupings (one stroke per action), support erasure, transformations, object reorder, text, page reorder/delete/add, template changes and PDF annotations. Split view should not mix documents' undo states. Document reload should never silently discard saved state.

Pages: add/insert before/after/duplicate/reorder/delete/restore, page thumbnails, virtualization, scroll/page turn, continuous vs single page; blank, ruled, dots, square grid, graph, Cornell, planner and imported template; each page own size/orientation/paper/pattern/pattern color/spacing/opacity; explicit Apply to All; non-destructive changes. Support custom light/dark paper without automatically changing actual ink colors on theme change. Crisp template at any zoom. Smooth pinch/trackpad/wheel zoom, fit width, fit page, jump-to-page, proper coordinate mapping.

Images: import/clipboard/drag/drop/camera/platform picker, EXIF orientation, position, resize/rotate/crop/zoom/layer/lock/copy/duplicate/delete, no distortion; store original plus preview where needed; handle unsupported HEIC gracefully if decoder absent. Never let a corrupt image crash the notebook. Simple shapes/arrows/connectors/snap/hold-to-shape; text boxes with font/size/color/bold/italic/underline/alignment/list and editable selection; LaTeX/equation insertion with preserved source where supported.

Handwriting recognition/autocorrect: opt-in, LOCAL/on-device first if supported; allow select → recognize → preview → correct → convert, retain original ink until confirmed, confidence/error indications when available, indexed recognized text search. Never silently "correct" handwritten mathematics or replace unknown script with guesses. If no correct cross-platform recognition backend, ship honest clear feature-disabled fallback and actionable documentation; do not assert flawless handwriting recognition.

## 6 — Premium PDF reader/annotation

Use a dependable PDF renderer compatible with targets (evaluate alternatives/license). Native import; immutable original PDF, page metadata, bookmarks/outline, thumbnails, text extraction/search/select if PDF contains text, zoom/fit/pan and large document virtualization. Scanned PDF text needs OCR; never promise native text select where no text layer exists. Support freehand ink, accurate text-space highlights (quadpoints), freehand highlight, underline/strike, text/shape/note/image overlay and undo/redo; separate editable annotations from original bytes. Robust coordinate conversions across rotation/cropbox/zoom, no annotation drifting. Export merged annotated PDF, original unchanged PDF, flattened optional PDF, image or portable notes export with accurate page dimensions, visible highlights and correct ordering. Validate externally rendered exported PDF (not only code success).

## 7 — Text, audio and speech

Rich-text objects support editing, keyboard layout, paste, undo/redo, multi-line, selectable/copyable, resizing/move; non-destructive integration with vector content. Audio recording with explicit permission, attach to notebook/page/subject, pause/resume, playback, save on interruption, clear duration, optional on-device transcription with labels and language handling; text-to-speech/read-aloud only where platform supports. Never record covertly or promise forbidden persistent background microphone sessions. Export attachments where intended and handle very long files without freezing.

## 8 — Multidocument workspace, windows, optional timer

Desktop/iPad: tabs, split view notebook + PDF, notebook + notebook, PDF + PDF where supported; resizable divider, independent zoom/pan/tool selection, remembered layout, safe separate document histories and lifecycle; native multiple windows where Tauri/platform supports. Phones use one fullscreen document and fast switching, not crushed 2-pane layout. Windows support mouse/keyboard/tablet pen, macOS trackpad/multiwindow, iPad Apple Pencil touch and iOS safe areas, Android scoped file storage/active stylus. Cross-platform differences should have graceful documented fallback.

Timer starts HIDDEN. User explicitly shows/starts it. Pomodoro 25/5, 50/10, custom focus/rest, stopwatch; subject and optional doc association, start/pause/resume/finish/stop/edit/correct, optional small floating resizable timer or pill, quick hide without stopping. Clear count semantics for breaks, inactivity, app background, and device sleep. Persist timestamps and active/paused intervals so app suspension and restart reconstruct accurately; avoid JS setInterval as authoritative time; use monotonic clock while running + persisted wall clock/timezone event reconciliation. Prevent concurrent double-counted sessions; sessions spanning midnight split correctly into daily totals. Do NOT pretend time in background proves studying; optional idle pause or manual correction. Calendar section is a separate future UI; expose documented calendar-ready event/query contract only.

Study page: actual focus records, day/week/month totals, subject breakdown, day heatmap, recent sessions, goals, document-linked sessions and real timestamps, editable corrections and exports; no hardcoded "2h34m"/fake heatmaps. Safely handle timezone/daylight savings/clock jumps.

## 9 — Responsive and accessible interaction

Build distinct tailored layouts for iPhone/Android phones, iPad portrait/landscape, Windows/macOS wide screen. Web is only read-only notes/PDF. Test narrow widths 320, 360, 375, 390, 430; tablet 768, 820, 1024; desktop 1280, 1440, 1536, 1920. Dynamic type, screen reader labels, correct keyboard semantics, large touch hit targets, reduced-motion, safe-area/gesture/home indicators. No clipped modals, horizontal viewport overflow, tiny toolbar targets, lost content on rotation, overlapping keyboard and page actions. Hover animations never gate touch. Avoid wallpaper blur over document ink; drawing on page must not inherit tinted tablet dashboard theme. Colors and contrast remain readable.

UI motion standard: physically smooth, interruptible carousel; page transitions and toolbar animations concise; minimize animations when pen down, while exporting or under memory pressure. Use profiling to justify blur/springs, don't assert Rust alone guarantees 120 fps.

## 10 — Strict quality & verification gate

Before calling anything complete, test the actual behavior end-to-end:
- Rust fmt, clippy, unit/integration, workspace compilation, WASM build, Tauri native artifact/compile checks.
- Storage: create → save → kill process → reopen; rename/move/duplicate/delete/restore; corrupt files; migration; interrupted write; image/PDF/audio quota errors; account separation; rollback on failure.
- Ink: pressure/width samples, rapid writing, strokes vs transforms, whole/partial eraser exactness, undo/redo property/invariants, lasso, multi-page copy/paste; page zoom coordinate identity.
- PDFs: import/view/search/markup/rotation/export round-trip into external viewer, 100/300 page fixture, scanned PDF honest treatment.
- Study: background suspend and resume, midnight, daylight saving, manual edits, paused timers, multiple documents, accurate subject totals.
- Visual: screenshot actual 1536×1024 library compared against reference 01, plus iPad portrait/landscape, iPhone portrait/landscape and Android; verify cream physical book interaction and readable paper. Functionality references 02–04 for working pen tray, proper toolbar and folder/shelf actions.
- Browser automation (Playwright on supported runner) for website read-only experience, authenticated navigation, default empty-state, and no false creation UI. CI must not fail due to unsupported Ubuntu/Playwright mismatch; use supported runner and real tests.
- Xcode/macOS runners for iOS/iPadOS compile and package where configured, Android APK, Windows, macOS. **Compilation != device runtime, unsigned != signed IPA, simulated viewport != real device**. Hardware tests required for meaningful Apple Pencil hover/pressure/latency/palm rejection. Distinguish all outcomes.
- Performance benchmarks: input-to-display latency, worst-frame time for carousel, viewport memory, 1k notebook library, thousands of strokes/page, large PDF, repeated open/close; document measured values/hardware/bounds and remaining risks. No unverifiable "never crash" promises.

## 11 — Deliverable process, CI, deployment and reporting

Operate autonomously inside this SINGLE task; solve blockers where possible. Do not stop after writing plans. Do not ask user to repeat existing requirements. No cosmetic completion claims or unchecked checkboxes.

If necessary, stage implementation in coherent commits and keep durable progress docs so a single Codex task can keep progressing, but do not demand separate user prompts to move between internal phases. Make no irreversible changes to real user documents. Prefer meaningful smaller implemented surfaces over a broad fake implementation if execution limits prevent full delivery; failures must be explicitly marked.

Push to existing PRIVATE repo main only after protecting newer concurrent commits; never force push/overwrite. Preserve existing Google auth and home modes. Update AGENTS.md and docs/ellipse-notes/ with architecture, data format/migrations, third-party notices, input handling, storage recovery, platform tests, UX/motion decisions and acceptance matrix. Add a detailed unimplemented/features list with honest statuses.

Cloudflare Pages: preserve real Leptos Rust/WASM build and production address; site must continue working with *read-only* Notes browser preview and an honest native app install/open affordance. Verify deployment ID, deployed commit SHA, status, asset loads, auth redirect, and no accidental document upload. Native packages may be available via Actions artifacts—link only verified artifacts. Native Google sign-in must not be asserted production-ready without secure OAuth and actual runtime validation.

**Final mandatory report with evidence:** exact HEAD, source branch/commits, screenshot comparison, storage/ink/PDF/study feature matrix each marked Implemented+tested / Implemented-not-device-verified / Partial / Blocked / Missing, native platform results, CI logs/artifacts, Cloudflare deployment commit+ID, unsolved defects, performance measurements, and explicitly what a real-device tester still must do.

### FINAL PRODUCT STANDARD

It must feel like an ORIGINAL ELLIPSE product: physical-book carousel on cream editorial background, powerful native full-page notebook/PDF editor whose LIBRARY DISAPPEARS upon opening a note, precise pens/erasers/undo, per-page paper customizations, audio/text/objects, local-first reliable saves, optional hidden-by-default study timer, subject-based honest logs, user-controlled wallpaper/blur, minimal polish and confident responsive behavior.

**Start inspecting the current repository and the four attached repository image references now. Carry out the engineering and verify what is actually finished.**