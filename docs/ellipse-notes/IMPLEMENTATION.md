# Ellipse Notes implementation and evidence

## Entry points and privacy

Signed-in website users open **Notes** from the Ellipse workspace. The website reads explicitly chosen local PDFs and Ellipse notebook JSON backups. The open library is transient and has no connection to a private installed library. Web authoring and native storage IPC are unavailable on the public origin.

The installed app offers **Open Notes on this device** at its signed-out screen. This is an explicit device-local vault under the OS app profile. It does not create a Firebase session, unlock the authenticated workspace, or expose an account's data. Native Google OAuth remains unavailable. Native commands accept the main packaged app origin and the configured loopback development origin in debug builds. Native artwork uses a separate device-local media namespace. Documents, text, audio data, images and PDFs have no automatic upload path.

## Data format and recovery

- Portable Rust model in `ellipse-core/src/notes/`. Vault schema 1 uses stable unsigned identifiers covering notebooks, folders, pages, text boxes and strokes. Independent dimensions and paper attributes live on each page. Stroke points are in top-left-origin document space. Each sample records position, normalized pressure, tilt magnitude and azimuth derived from pointer events.
- SQLite schema 1 in `notes/device/notes.sqlite3` under Tauri's local app data directory. Initialization uses an immediate transaction, WAL and FULL synchronous mode. Snapshots contain typed JSON with SHA-256 digests, monotonic revisions, and up to four checkpoints. Future database and Vault versions are rejected. Commits validate IDs, parentage, geometry and referenced PDF assets before writing.
- Saves are serialized with revision comparison. A completed stroke or other operation queues persistence. An edit arriving during a write causes a subsequent snapshot. **Saved on this device** appears after the matching revision is acknowledged. The editor waits before returning to the library and offers retry after a failed write. If the app is killed, an in-progress stroke and the latest unacknowledged edit can be lost.
- Recovery searches from the newest checkpoint back. A corrupt latest payload is preserved while the recovery screen asks whether to commit the earlier version. A database with no valid checkpoint is opened only for recovery diagnostics. Copy the database together with its `-wal` and `-shm` files before attempting manual repair. The current app has no SQLite page-level repair interface.
- Original PDFs are copied into a hash-addressed immutable BLOB table. Annotation layers are separate in the Vault. An interrupted import can leave an unreferenced BLOB. Cover and wallpaper binaries use the app media store and are deduplicated there. The 24 MiB PDF and 32 MiB Vault snapshot bounds are enforced. Restoring or deleting a library item changes metadata. Retained checkpoints and BLOBs can still contain earlier content.
- Versioned `.ellipse.json` notebook backups contain editable pages, text and ink and omit device-bound cover artwork. PDF originals can be downloaded separately. Annotated PDF export writes a new copy. A complete vault backup, ZIP package, asset collection, and encryption beyond the OS app sandbox are future work.

## Interaction and rendering

The cream library follows the supplied composition with a centered title and filters, adjacent CSS-rendered book spines, an expanded rotated cover, and understated controls. Mouse drag, wheel, touch-native scrolling and keyboard arrows work. Covers support cloth and simple geometric treatments, color and local photos with framing. Search covers notebook metadata and typed text. Folders can be nested and selected. Notebook details cover subject, collection, tags, favorite, move, rename, duplicate, archive and trash actions. Themes and wallpaper affect the Notes library, independently of note paper and the Ellipse workspace.

Opening a book or PDF replaces the library with an editor. Returning restores the library's mounted-session filters, selection and stored horizontal offset. Each book stores page, zoom, pen and the document scroll offset when leaving. The page navigator can jump to a page and show its bookmark. Ink uses two Canvas layers. A pointer gesture updates the transient layer on animation frames, and a completed gesture redraws committed vector content and queues a checkpoint. Each transparent stroke uses a single joined path. Coalesced events are used when exposed. Finger writing is off until enabled, so touch can scroll by default. Hardware hover, buttons and palm rejection need platform validation. Erasers remove vector segments or whole strokes. Lasso and rectangle selection operate on ink, with copy, cut, paste, duplicate, rotate, scale and incremental move. Undo history is capped by size and level count and cleared when closing the editor.

PDF.js renders selected pages at a bounded backing resolution. PDF search lists matching pages and reports image-only files accurately. Native markup can highlight, underline or strike matching text runs, plus add freehand ink, shapes and typed text. A text run can be longer than the search term. PDF.js view transforms are stored at scale 1 and inverted by Rust for export. pdf-lib copies original pages and adds vector ink and text. Notebook PDF export includes backgrounds, ink and text. It presently omits stationery patterns and supports Helvetica-family encoding. The editable notebook backup preserves Unicode text and paper settings. PDF navigation does not yet expose outlines or raster thumbnails.

Study controls begin hidden in the editor. Sessions save wall-clock interval boundaries and can be paused, resumed, hidden or finished independently of display. A restarted active timer reconstructs elapsed time from persisted intervals and applies preset caps. The library shows completed daily, weekly, monthly and subject totals. Totals split completed intervals at the device's local calendar boundaries. Background time counts as recorded elapsed time. This version has no idle detection or reliable correction for manual clock changes.

### Future Calendar contract

Read completed sessions as a projection of `Vault.study.history`. Each record exposes subject, optional stable notebook ID, UTC epoch milliseconds for start and finish, and an ordered list of half-open active intervals. Query display totals by intersecting the active intervals with the requested timezone's calendar boundaries using `Session::milliseconds_in`. Calendar clients should retain source UTC values when users change timezone. The current format keeps all records in the local store and exposes no sync endpoint.

## Evidence and limitations

| Area | Status | Evidence or open work |
| --- | --- | --- |
| Storage and recovery | Implemented and tested on Linux | SQLite commit/reopen, stale revision conflict, rollback, killed child before commit, earlier checkpoint recovery, storage root isolation, corrupt and missing PDF assets. Browser fixture also exercises failed-save retry. |
| Library | Implemented and browser-tested | Cream shelf compared with source, empty and populated states, 13 viewport sizes, keyboard selection, search/layout, folder nesting, rename, cover, wallpaper, trash/restore. Folder reparenting and bulk operations, real preview images on unselected spines, spring physics and virtualized shelf rendering remain incomplete. |
| Ink and pages | Implemented with device validation pending | Pressure curves, clip-and-split eraser, undo, paper templates, insert/duplicate/move/delete, editable text, shapes, lasso and transforms. Physical pressure, hover, palm rejection and latency remain unverified. No imported paper, page images, continuous page flow or OS clipboard interchange. |
| PDF | Implemented and browser-tested within limits | Real PDF.js worker, search and page jumps, rotated copy with vector markup rendered by Poppler, source-byte preservation and text extraction. No raster thumbnails, OCR, PDF outlines, password prompt or arbitrary page rotation tools. |
| Study | Partially implemented and tested | Hidden-by-default persisted timer, pause and suspension math, subject summaries and completed-interval totals. No session corrections, idle handling, goals, heatmap or clock-jump reconciliation. |
| Advanced features | Missing | Cross-platform recognition, correction, OCR, audio recording/playback/transcription, text-to-speech, page images, split view, tabs, multiple windows and full package import/export. |
| Native shell | Blocked in this Linux environment | Shared frontend build available. Full Tauri check stops at missing GLib/WebKitGTK system dependencies. GitHub workflows target macOS, Windows, Android and the iOS compile check. Real devices still need testing. |
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

Run `scripts/verify_notes_pdf.py` with the browser suite's exported `annotated-rotated-fixture.pdf` and Poppler installed. The checker requires embedded source text, two pages, a landscape rotated page, and distinct ink and highlight pixels. A single optimized Linux run of 1,000 notebooks containing 2,000 strokes took 43 ms to commit, 26 ms to reload, and 1.0 ms to generate 96,000 outline vertices. These timings exclude WebView input, compositing and hardware latency. In one headless Chromium run, the 300-page image-only fixture reached its first rendered canvas in 647 ms, including fixture serialization and browser test overhead.

The browser suite substitutes Firebase responses and native IPC for controlled workflows. Native attachment IO, login, device exports and physical stylus behavior still need packaged-app tests. Testers should write with a physical Apple Pencil, Android stylus and Windows pen, rotate during a gesture, suspend with an active timer, interrupt an import and reopen, and check storage recovery with controlled low-space fixtures.

## Dependencies and design references

OwlLuna was reviewed at `7ea2a8c7a4acb0f30541d778483d746b309b4dfa` (MIT, © 2024–2026 Owais Khan). No upstream implementation code, images, or typefaces were incorporated. Notes uses the owner's cream screenshot as a design comparison. The screenshot stays in the repository's private documentation.

Pinned `pdfjs-dist` and `pdf-lib` packages are installed from the lockfile for the build. The bundled runtime includes upstream license notices and license files shipped with PDF.js resources. PDF.js scripting and eval are disabled. The original PDF files remain in local app storage or browser memory.
