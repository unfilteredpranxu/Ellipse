# Ellipse Notes implementation and evidence

## Entry points and privacy

Signed-in website users open **Notes** from the Ellipse workspace. The website reads explicitly chosen local PDFs and Ellipse notebook JSON backups. The open library is transient and has no connection to a private installed library. Web authoring and native storage IPC are unavailable on the public origin.

The installed app offers **Open Notes on this device** at its signed-out screen. This is an explicit device-local vault under the OS app profile. It does not create a Firebase session, unlock the authenticated workspace, or expose an account's data. Native Google OAuth remains unavailable. Native commands accept the main packaged app origin and the configured loopback development origin in debug builds. Native artwork uses a separate device-local media namespace. Documents, text, audio data, images and PDFs have no automatic upload path.

## Data format and recovery

- Portable Rust model in `ellipse-core/src/notes/`. Vault schema 2 includes productivity data and reads legacy schema 1. The vault uses stable unsigned identifiers covering notebooks, folders, pages, text boxes and strokes. Independent dimensions and paper attributes live on each page. Stroke points are in top-left-origin document space. Each sample records position, normalized pressure, tilt magnitude and azimuth derived from pointer events.
- SQLite schema 2 in `notes/device/notes.sqlite3` under Tauri's local app data directory. Initialization and the schema 1 upgrade use an immediate transaction, WAL and FULL synchronous mode. Snapshots contain typed JSON with SHA-256 digests, monotonic revisions, and up to four checkpoints. Future database and Vault versions are rejected. Commits validate IDs, parentage, geometry and referenced PDF assets before writing.
- Completed edits are serialized in the WebView. Native saves first commit a pending payload with the exact base revision, then promote it to a checkpoint in a second transaction. Promotion removes the pending payload atomically. Reopening after an interrupted promotion offers to recover the pending payload only if its checksum and base revision match. A retry of the same already-committed payload returns the existing revision. The library unlocks and **Saved on this device** appears only after acknowledgment.
- Background suspension has no guaranteed flush deadline. Sampled ink can be committed on a delivered pointer interruption, while cancelled erasing and shape gestures are discarded. Force termination before the native pending transaction commits can still lose the stroke or edit. Edits queued in the WebView behind an in-flight save can also be lost. Out-of-space errors keep the in-memory edit available for retry while the app stays open.
- Recovery searches from the newest checkpoint back and skips malformed or oversized rows before allocating their payloads. A valid checksum with a future Vault schema stops recovery and leaves the existing database intact. The library stays locked while a recovered checkpoint is being saved, and remains locked on failure until the user retries successfully. If no committed or staged checkpoint is readable, the recovery screen offers restore from a vault backup. The restore uses the observed revision and preserves unreadable rows until normal checkpoint retention removes them. Whole-database SQLite damage that prevents opening the database still requires manual recovery. Copy the database together with its `-wal` and `-shm` files before attempting manual repair. The current app has no SQLite page-level repair interface.
- Original PDFs are copied into a hash-addressed BLOB table. Annotation layers are separate in the Vault. Reimporting an identical PDF can repair a damaged BLOB while preserving its identity and all annotation references. An interrupted import can leave an unreferenced BLOB. Cover and wallpaper binaries use the app media store and are deduplicated there. The 24 MiB PDF and 32 MiB Vault snapshot bounds are enforced on writes and database reads. Restoring or deleting a library item changes metadata. Retained checkpoints and BLOBs can still contain earlier content.
- Versioned `.ellipse.json` notebook backups contain editable pages, text and ink and omit device-bound cover artwork. PDF originals can be downloaded separately. Annotated PDF export writes a new copy. The separate full-vault backup includes all referenced originals and does not include discarded checkpoints or unrelated workspace artwork.

## Full vault backup

Open **Folders → Vault backup** in installed Notes. **Save vault backup** writes a new numbered file to `Documents/Ellipse Backups`. The file is written and synced under a temporary name before publication. Existing backups are never replaced. On iOS, Files sharing is enabled for the Documents directory. Android resolves Documents inside its app-specific external directory. Access through the device file manager still needs validation on supported Android versions.

Version 1 of `.ellipsevault` contains the complete committed Vault JSON, followed by deduplicated PDF originals, covers and wallpaper in deterministic order and a SHA-256 checksum. Original artwork bytes, crop data, folders, trash, editable ink and study records are included. The import validates structure, sizes, asset signatures and both archive and asset hashes. The 96 MiB total and 24 MiB per-asset limits bound the memory used by this version's import. The checksum detects corruption. The archive is unencrypted and should be copied only to storage you trust.

Restore explicitly replaces the active library at the revision selected before import. It refuses to replace a concurrently advanced vault or unresolved pending checkpoint. PDF assets and vault metadata commit in one SQLite transaction. Content-addressed artwork is written and synced before that commit. An interruption can leave an unreferenced image or a temporary file without replacing existing notes or published backups. Backups and restores stay in the device-local Notes namespace.

## Interaction and rendering

The cream library follows the supplied composition with a centered title and filters, adjacent CSS-rendered book spines, an expanded rotated cover, and understated controls. Mouse drag, wheel, touch-native scrolling and keyboard arrows work. Covers support cloth and simple geometric treatments, color and local photos with framing. Search covers notebook metadata and typed text. Folders can be nested and selected. Notebook details cover subject, collection, tags, favorite, move, rename, duplicate, archive and trash actions. Themes and wallpaper affect the Notes library, independently of note paper and the Ellipse workspace.

Opening a book or PDF replaces the library with an editor. Returning restores the library's mounted-session filters, selection and stored horizontal offset. Each book stores page, zoom, pen and the document scroll offset when leaving. The page navigator can jump to a page and show its bookmark. Ink uses two Canvas layers. A pointer gesture updates the transient layer on animation frames, and a completed gesture redraws committed vector content and queues a checkpoint. Each transparent stroke uses a single joined path. Coalesced events are used when exposed. Finger writing is off until enabled, so touch can scroll by default. Hardware hover, buttons and palm rejection need platform validation. Erasers remove vector segments or whole strokes. Lasso and rectangle selection operate on ink, with copy, cut, paste, duplicate, rotate, scale and incremental move. Undo history is capped by size and level count and cleared when closing the editor.

PDF.js renders selected pages at a bounded backing resolution. PDF search lists matching pages and reports image-only files accurately. Native markup can highlight, underline or strike matching text runs, plus add freehand ink, shapes and typed text. A text run can be longer than the search term. PDF.js view transforms are stored at scale 1 and inverted by Rust for export. pdf-lib copies original pages and adds vector ink and text. Notebook PDF export includes vector stationery from the same Rust geometry as the writing canvas, as well as paper color, ink and text. Pages sharing stationery geometry reuse one compressed PDF form and retain individual colors. PDF text export uses Helvetica-family encoding. The editable notebook backup preserves Unicode text and paper settings. PDF navigation does not yet expose outlines or raster thumbnails.

Study controls begin hidden in the editor. Sessions save wall-clock interval boundaries and can be paused, resumed, hidden or finished independently of display. A restarted active timer reconstructs elapsed time from persisted intervals and applies preset caps. The library shows completed daily, weekly, monthly and subject totals. Totals split completed intervals at the device's local calendar boundaries. Background time counts as recorded elapsed time. This version has no idle detection or reliable correction for manual clock changes.

### Calendar, Focus and Study

These are distinct workspace pages backed by the native vault. The browser planner uses a separate Firebase-UID-scoped local record. Notes authoring stays native. Calendar day, week and month views project civil-date tasks and UTC study intervals. The same active session appears in Focus and the optional Notes timer. See the [planner data model](../study/CALENDAR_TASK_TIMER_SCHEMA.md) and [feature matrix](../study/FEATURE_MATRIX.md).

## Evidence and limitations

| Area | Status | Evidence or open work |
| --- | --- | --- |
| Storage and recovery | Implemented and tested on Linux | SQLite commit/reopen, stale revision conflict, rollback, killed child before commit, malformed and oversized rows, future schema handling, earlier checkpoint recovery, storage root isolation, corrupt and missing PDF assets, and repair with identical source bytes. Process termination after journal commit, stale journal rejection, schema upgrade, archive round trip, interrupted publication, corrupt import, and SQLite full rollback are covered. Browser fixtures test failed-save retry and gate editing until the recovered checkpoint is acknowledged. |
| Library | Implemented and browser-tested | Cream shelf compared with source, empty and populated states, 13 viewport sizes, keyboard selection, search/layout, folder nesting, rename, cover, wallpaper, trash/restore. Folder reparenting and bulk operations, real preview images on unselected spines, spring physics and virtualized shelf rendering remain incomplete. |
| Ink and pages | Implemented with device validation pending | Pressure curves, clip-and-split eraser, undo, paper templates, insert/duplicate/move/delete, editable text, shapes, lasso and transforms. The browser checks full-screen editor controls at eight phone, tablet and desktop viewport sizes. Physical pressure, hover, palm rejection and latency remain unverified. No imported paper, page images, continuous page flow or OS clipboard interchange. |
| PDF | Implemented and browser-tested within limits | Real PDF.js worker, search and page jumps, rotated copy with vector markup rendered by Poppler, source-byte preservation and text extraction, and independent verification of notebook stationery geometry and per-page colors. No raster thumbnails, OCR, PDF outlines, password prompt or arbitrary page rotation tools. |
| Study | Partially implemented and tested | Hidden-by-default persisted timer, pause and suspension math, subject summaries and completed-interval totals. No session corrections, idle handling, goals, heatmap or clock-jump reconciliation. |
| Advanced features | Missing | Cross-platform recognition, correction, OCR, audio recording/playback/transcription, text-to-speech, page images, split view, tabs, multiple windows and full package import/export. |
| Native shell | Blocked in this Linux environment | Shared frontend build available. Full Tauri check stops at missing GLib/WebKitGTK system dependencies. GitHub workflows target macOS, Windows, Android and unsigned iOS builds. The iOS job requests a Simulator .app and an unsigned device IPA through Tauri. Real devices still need testing. |
| Cloudflare | Unverified | The Pages WASM build writes the source revision to `dist/version.json`. Production and GitHub Actions API requests from this cloud environment returned 403. |

### Reproduce the checks

```sh
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown -- -D warnings
bash scripts/build-pages.sh
npm ci --prefix tests/browser
npm --prefix tests/browser test
cargo test --release --locked -p ellipse-notes-store measure_large_checkpoint -- --ignored --nocapture
```

Run `scripts/verify_notes_pdf.py` with the browser suite's exported `annotated-rotated-fixture.pdf` and `notebook-stationery-fixture.pdf` and Poppler installed. The checker requires embedded source text, two pages, a landscape rotated page, and distinct ink and highlight pixels in the first fixture. It also checks the Cornell note and cue columns, summary line, and different stationery colors in the second fixture. A single optimized Linux run of 1,000 notebooks containing 2,000 strokes took roughly 180 ms to stage and commit, 28 ms to reload, and 1.4 ms to generate 96,000 outline vertices. These timings exclude WebView input, compositing and hardware latency. In one headless Chromium run, the 300-page image-only fixture reached its first rendered canvas in 647 ms, including fixture serialization and browser test overhead.

The browser suite substitutes Firebase responses and native IPC for controlled workflows. Native attachment IO, login, device exports and physical stylus behavior still need packaged-app tests. Testers should write with a physical Apple Pencil, Android stylus and Windows pen, rotate during a gesture, suspend with an active timer, interrupt an import and reopen, and check storage recovery with controlled low-space fixtures.

## iOS signing and iPad validation

The CI simulator zip contains a macOS Simulator app when the build succeeds. The device IPA is unsigned. To install on an iPad, build with an Apple Development certificate, a development provisioning profile for the bundle identifier and the registered iPad, and an Apple team selected in Xcode. Distribution through TestFlight requires an App Store Connect record and a signed distribution build. CI uses no signing credentials.

On a signed physical iPad build, use a disposable vault for these checks:

1. Draw in a notebook and annotate a PDF. Force-quit immediately after lifting the Pencil, repeat after **Saved on this device**, and relaunch. Confirm acknowledged edits remain and any offered checkpoint contains the staged edit. Repeat by backgrounding and resuming during a save.
2. Save a vault containing notebooks, nested folders, a PDF, a custom cover and a wallpaper. Copy the archive out through Files. Restore it into a fresh test vault and reopen every asset. Repeat with a truncated copy and under low available storage. The previous vault and previous backup must remain usable after failure.
3. Compare fast and slow pen strokes with light and heavy pressure. Check palm rejection, undo and erasing after rotation, and interrupted gestures when opening Control Center. Confirm a cancelled eraser changes no existing ink. Measure latency using real hardware. Simulator and browser tests do not establish stylus behavior.

## Dependencies and design references

OwlLuna was reviewed at `7ea2a8c7a4acb0f30541d778483d746b309b4dfa` (MIT, © 2024–2026 Owais Khan). The portable lasso now adapts the straight-diagonal outline behavior from upstream. Its MIT notice ships with the web and native frontend. See [the source parity map](OWLLUNA_FEATURE_PARITY.md). Notes uses the owner's cream screenshot as a design comparison. The screenshot stays in the repository's private documentation.

Pinned `pdfjs-dist` and `pdf-lib` packages are installed from the lockfile for the build. The bundled runtime includes upstream license notices and license files shipped with PDF.js resources. PDF.js scripting and eval are disabled. The original PDF files remain in local app storage or browser memory.
