# Notes storage and recovery

SQLite schema 2 provides bounded checksummed snapshots and a durable pending write at its expected base revision. Acknowledgment follows promotion of the pending write. A later valid revision always takes precedence over a stale pending write. A valid staged edit takes precedence over restoring a backup. If all checkpoints are unreadable and SQLite can still open, the recovery screen offers a confirmed backup restore guarded by the observed revision.

Vault schema 2 includes Calendar, personal exam progress, and linked focus sessions. Legacy Vault schema 1 continues to load without rewriting pending payload hashes. The first planner or extended timer edit upgrades it. Older readers recognize the new schema as a future version. Standalone notebook JSON remains at format 1.

The archive format remains version 1. The versioned vault JSON inside the archive captures the new data alongside original asset bytes. Native artwork and PDFs remain content addressed. The existing 96 MiB total and 24 MiB per asset limits apply. An archive checksum verifies integrity. The archive is plaintext.

No UI state can protect a completed stroke that has yet to reach SQLite staging. Interrupted live drawing follows the pointer cancellation policy. Low-storage failures can leave unsaved in-memory edits, and preserving the last acknowledged revision takes priority. Before manually repairing a damaged database, copy the SQLite database with its matching WAL and SHM files.

Further details and tests are recorded in [the implementation record](IMPLEMENTATION.md).
