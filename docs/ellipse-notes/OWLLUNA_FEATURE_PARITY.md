# OwlLuna source and Ellipse parity

Reviewed upstream commit [7ea2a8c7a4acb0f30541d778483d746b309b4dfa](https://github.com/owaisazmal/OwlLuna/tree/7ea2a8c7a4acb0f30541d778483d746b309b4dfa). Its MIT license names Owais Khan, 2024–2026. The Xcode project sets iOS 18 and macOS 26 where applicable. No OwlLuna images, fonts, dependencies, app identifiers or entitlements are integrated.

| Upstream source | Ellipse integration and verification |
| --- | --- |
| `Canvas/InkLasso.swift`, `OwlLunaTests/InkLassoTests.swift` | The straight-diagonal outline adaptation runs in the portable Rust lasso. Same-page selection, transformations and undo use Ellipse strokes. Cross-page PencilKit pasteboard support is absent. |
| `Canvas/InkVectors.swift`, `Canvas/PencilGestures.swift`, `Models/ToolPreset.swift` | Upstream draws native PKStroke paths and recognizes scribble and circle gestures. Ellipse renders vector outlines from portable samples and handles cancellation. Native PKDrawing conversion, system Pencil gestures and physical behavior remain unverified or absent. |
| `Canvas/PDFMarks.swift`, `Services/NotebookExporter.swift` | Upstream uses PDFKit page links and PDF export. Ellipse retains original PDF bytes, separate annotations and vector export verified by Poppler. Native PDFKit, outlines and OCR are absent. |
| `Storage/NotebookPackage.swift`, `Storage/LibraryIndex.swift`, `Editor/EditorSession.swift` | Upstream coordinates package files, recovers orphaned files and saves manifest/ink atomically. Ellipse SQLite serializes native writes and confirms recovered checkpoints. Rust process-death and archive tests exercise its own format. |
| `Library/LibraryStore.swift`, `Canvas/PaperTemplates.swift`, `Editor/PenRoll.swift` | Ellipse retains its own cream shelf, original artwork, document-first view, independent paper styles and pen controls. Browser tests exercise responsive layout and local library operations. |
| `Editor/NotebookRecorder.swift`, `Services/InkText.swift`, `App/OwlLunaApp.swift` | Upstream uses AVAudioRecorder/AVAudioPlayer, local recognition and SwiftUI WindowGroup. Their Apple frameworks are unsupported on Windows/Android. Ellipse has no recording, recognition, split-editor or native multiwindow implementation. |

The shared engine remains the editable source of truth. A new Apple editor would require measured integration and lossless source-byte retention before activation. Linux has no iOS SDK or PencilKit, and the hosted API returned 403.
