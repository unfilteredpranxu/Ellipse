//! Transactional local Notes storage. The caller supplies a private application directory.
mod backup;
pub use backup::MAX_BACKUP;
use ellipse_core::{
    artwork::valid_asset_id,
    notes::{MAX_PDF, MAX_STATE, SCHEMA, Vault, valid_pdf},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Store(PathBuf);

#[derive(Serialize)]
pub struct Snapshot {
    pub head: u32,
    pub vault: Vault,
    pub recovered: bool,
    pub restore_required: bool,
}

#[derive(Deserialize)]
struct CheckpointVersion {
    schema: u32,
}

type Result<T> = std::result::Result<T, String>;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sqlite<T>(result: rusqlite::Result<T>) -> Result<T> {
    result.map_err(|_| "Notes storage is unavailable. Check device space and retry.".into())
}

impl Store {
    pub fn new(dir: &Path) -> Result<Self> {
        fs::create_dir_all(dir).map_err(|_| "The Notes folder could not be accessed.")?;
        Ok(Self(dir.join("notes.sqlite3")))
    }

    fn open(&self) -> Result<Connection> {
        let mut db = sqlite(Connection::open(&self.0))?;
        sqlite(db.busy_timeout(Duration::from_secs(5)))?;
        sqlite(db.pragma_update(None, "journal_mode", "WAL"))?;
        sqlite(db.pragma_update(None, "synchronous", "FULL"))?;
        let tx = sqlite(db.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let version: u32 = sqlite(tx.pragma_query_value(None, "user_version", |row| row.get(0)))?;
        match version {
            0 => sqlite(tx.execute_batch(
                "CREATE TABLE snapshots (revision INTEGER PRIMARY KEY, payload BLOB NOT NULL, digest TEXT NOT NULL);
                 CREATE TABLE assets (id TEXT PRIMARY KEY, content BLOB NOT NULL);
                 CREATE TABLE pending (base INTEGER PRIMARY KEY, payload BLOB NOT NULL, digest TEXT NOT NULL);
                 PRAGMA user_version = 2;",
            ))?,
            1 => sqlite(tx.execute_batch(
                "CREATE TABLE pending (base INTEGER PRIMARY KEY, payload BLOB NOT NULL, digest TEXT NOT NULL);
                 PRAGMA user_version = 2;",
            ))?,
            2 => {},
            _ => return Err("This Notes database requires a newer version of Ellipse.".into()),
        }
        sqlite(tx.commit())?;
        Ok(db)
    }

    pub fn load(&self) -> Result<Snapshot> {
        let mut db = self.open()?;
        let tx = sqlite(db.transaction())?;
        let snapshot = Self::committed(&tx)?;
        if let Some(vault) = Self::pending(&tx, snapshot.head)? {
            return Ok(Snapshot {
                head: snapshot.head,
                vault,
                recovered: true,
                restore_required: false,
            });
        }
        if !snapshot.restore_required && Self::has_pending(&tx, snapshot.head)? {
            return Ok(Snapshot {
                recovered: true,
                ..snapshot
            });
        }
        Ok(snapshot)
    }

    fn has_pending(db: &Connection, head: u32) -> Result<bool> {
        sqlite(db.query_row(
            "SELECT EXISTS(SELECT 1 FROM pending WHERE base=?1)",
            [head],
            |row| row.get(0),
        ))
    }

    fn pending(db: &Connection, head: u32) -> Result<Option<Vault>> {
        let row = sqlite(
            db.query_row(
                "SELECT payload, CAST(digest AS BLOB) FROM pending WHERE base=?1
             AND typeof(payload)='blob' AND length(payload)<=?2
             AND typeof(digest)='text' AND length(CAST(digest AS BLOB))=64",
                params![head, MAX_STATE],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional(),
        )?;
        if let Some((payload, digest)) = row {
            return Self::decode_checkpoint(&payload, &digest);
        }
        Ok(None)
    }

    fn decode_checkpoint(payload: &[u8], digest: &[u8]) -> Result<Option<Vault>> {
        if hash(payload).as_bytes() != digest {
            return Ok(None);
        }
        match serde_json::from_slice::<Vault>(payload) {
            Ok(vault) if vault.schema > SCHEMA => {
                Err("These Notes require a newer version of Ellipse.".into())
            }
            Ok(vault) if vault.validate().is_ok() => Ok(Some(vault)),
            Err(_)
                if serde_json::from_slice::<CheckpointVersion>(payload)
                    .is_ok_and(|version| version.schema > SCHEMA) =>
            {
                Err("These Notes require a newer version of Ellipse.".into())
            }
            _ => Ok(None),
        }
    }

    fn committed(db: &Connection) -> Result<Snapshot> {
        let mut query = sqlite(db.prepare(
            "SELECT revision,
             CASE WHEN typeof(payload) = 'blob' AND length(payload) <= ?1 THEN payload END,
             CASE WHEN typeof(digest) = 'text' AND length(CAST(digest AS BLOB)) = 64
             THEN CAST(digest AS BLOB) END
             FROM snapshots ORDER BY revision DESC LIMIT 4",
        ))?;
        let mut rows = sqlite(query.query([MAX_STATE]))?;
        let mut head = None;
        while let Some(row) = sqlite(rows.next())? {
            let revision: u32 = sqlite(row.get(0))?;
            let head = *head.get_or_insert(revision);
            let data: Option<Vec<u8>> = sqlite(row.get(1))?;
            let digest: Option<Vec<u8>> = sqlite(row.get(2))?;
            let (Some(data), Some(digest)) = (data, digest) else {
                continue;
            };
            if let Some(vault) = Self::decode_checkpoint(&data, &digest)? {
                return Ok(Snapshot {
                    head,
                    vault,
                    recovered: revision != head,
                    restore_required: false,
                });
            }
        }
        Ok(Snapshot {
            head: head.unwrap_or(0),
            vault: Vault::default(),
            recovered: false,
            restore_required: head.is_some(),
        })
    }

    pub fn save(&self, head: u32, vault: &Vault) -> Result<u32> {
        let payload = Self::encode(vault)?;
        let digest = hash(&payload);
        let mut db = self.open()?;
        let tx = sqlite(db.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let current = Self::head(&tx)?;
        if Some(current) == head.checked_add(1)
            && sqlite(tx.query_row(
                "SELECT payload=?2 AND digest=?3 FROM snapshots WHERE revision=?1",
                params![current, payload, digest],
                |r| r.get::<_, bool>(0),
            ))?
        {
            return Ok(current);
        }
        Self::stage(&tx, head, vault, &payload, &digest)?;
        sqlite(tx.commit())?;

        let tx = sqlite(db.transaction_with_behavior(TransactionBehavior::Immediate))?;
        Self::check_head(&tx, head)?;
        let next = Self::write_checkpoint(&tx, head, &payload, &digest)?;
        sqlite(tx.execute("DELETE FROM pending WHERE base=?1", [head]))?;
        sqlite(tx.commit())?;
        Ok(next)
    }

    fn encode(vault: &Vault) -> Result<Vec<u8>> {
        vault.validate()?;
        let mut payload = Vec::new();
        serde_json::to_writer(&mut payload, vault).map_err(|_| "Could not encode Notes.")?;
        if payload.len() > MAX_STATE {
            return Err("The Notes library exceeds this version’s storage limit. Export or remove material before saving.".into());
        }
        Ok(payload)
    }

    fn head(db: &Connection) -> Result<u32> {
        sqlite(db.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM snapshots",
            [],
            |r| r.get(0),
        ))
    }

    fn check_head(db: &Connection, head: u32) -> Result<()> {
        if Self::head(db)? != head {
            return Err(
                "Notes changed in another window. Reopen the library before editing.".into(),
            );
        }
        Ok(())
    }

    fn stage(
        db: &Connection,
        head: u32,
        vault: &Vault,
        payload: &[u8],
        digest: &str,
    ) -> Result<()> {
        Self::check_head(db, head)?;
        Self::check_pdfs(db, vault)?;
        if Self::has_pending(db, head)? && Self::pending(db, head)?.is_none() {
            sqlite(db.execute("DELETE FROM pending WHERE base=?1", [head]))?;
        }
        let inserted = sqlite(db.execute(
            "INSERT OR IGNORE INTO pending VALUES (?1, ?2, ?3)",
            params![head, payload, digest],
        ))?;
        let matches = inserted == 1
            || sqlite(db.query_row(
                "SELECT payload = ?2 AND digest = ?3 FROM pending WHERE base=?1",
                params![head, payload, digest],
                |row| row.get(0),
            ))?;
        if !matches {
            return Err(
                "An earlier Notes write needs recovery. Reopen Notes before saving further edits."
                    .into(),
            );
        }
        Ok(())
    }

    fn check_pdfs(db: &Connection, vault: &Vault) -> Result<()> {
        for pdf in vault.books.iter().filter_map(|book| book.pdf.as_ref()) {
            let exists: bool = sqlite(db.query_row(
                "SELECT EXISTS (SELECT 1 FROM assets WHERE id = ?1)",
                [&pdf.asset],
                |row| row.get(0),
            ))?;
            if !exists {
                return Err("A PDF referenced by this save is missing.".into());
            }
        }
        Ok(())
    }

    fn write_checkpoint(db: &Connection, head: u32, payload: &[u8], digest: &str) -> Result<u32> {
        let next = head.checked_add(1).ok_or("Notes revision limit reached.")?;
        sqlite(db.execute(
            "INSERT INTO snapshots VALUES (?1, ?2, ?3)",
            params![next, payload, digest],
        ))?;
        sqlite(db.execute(
            "DELETE FROM snapshots WHERE revision <= ?1",
            [next.saturating_sub(4)],
        ))?;
        Ok(next)
    }

    pub fn store_pdf(&self, bytes: &[u8]) -> Result<String> {
        if !valid_pdf(bytes) {
            return Err("Select a complete PDF up to 24 MiB.".into());
        }
        let id = hash(bytes);
        let db = self.open()?;
        sqlite(db.execute(
            "INSERT INTO assets VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET content=excluded.content
             WHERE assets.content != excluded.content",
            params![id, bytes],
        ))?;
        Ok(id)
    }

    pub fn read_pdf(&self, id: &str) -> Result<Vec<u8>> {
        if !valid_asset_id(id) {
            return Err("Invalid PDF reference.".into());
        }
        let db = self.open()?;
        let bytes: Option<Vec<u8>> = sqlite(
            db.query_row(
                "SELECT CASE WHEN typeof(content) = 'blob' AND length(content) <= ?2 THEN content END
                 FROM assets WHERE id=?1",
                params![id, MAX_PDF],
                |row| row.get(0),
            )
            .optional(),
        )?
        .ok_or("The original PDF could not be found.")?;
        let bytes = bytes.ok_or("The original PDF failed its integrity check.")?;
        if !valid_pdf(&bytes) || hash(&bytes) != id {
            return Err("The original PDF failed its integrity check.".into());
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ellipse_core::notes::Paper;

    #[test]
    fn commits_conflicts_rollback_recovery_and_separate_roots() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("local");
        let store = Store::new(&root).unwrap();
        let mut vault = store.load().unwrap().vault;
        let book = vault.new_book("First", 11, Paper::default());
        vault.books.push(book);
        assert_eq!(store.save(0, &vault).unwrap(), 1);
        vault.books[0].title = "Renamed".into();
        assert_eq!(store.save(1, &vault).unwrap(), 2);
        assert_eq!(store.save(1, &vault).unwrap(), 2);
        let mut stale = vault.clone();
        stale.books[0].title = "A stale edit".into();
        assert!(store.save(1, &stale).is_err());
        {
            let mut connection = store.open().unwrap();
            let uncommitted = connection.transaction().unwrap();
            uncommitted.execute("DELETE FROM snapshots", []).unwrap();
            // Dropping a connection before commit must not change the last saved state.
        }
        let reopened = Store::new(&root).unwrap();
        assert_eq!(reopened.load().unwrap().vault, vault);
        reopened
            .open()
            .unwrap()
            .execute("UPDATE snapshots SET payload=x'00' WHERE revision=2", [])
            .unwrap();
        let recovered = reopened.load().unwrap();
        assert_eq!(recovered.head, 2);
        assert!(recovered.recovered);
        assert_eq!(recovered.vault.books[0].title, "First");
        assert!(
            Store::new(&fixture.path().join("different"))
                .unwrap()
                .load()
                .unwrap()
                .vault
                .books
                .is_empty()
        );
        assert_eq!(reopened.save(recovered.head, &recovered.vault).unwrap(), 3);
        assert!(!reopened.load().unwrap().recovered);
    }

    #[test]
    fn legacy_pending_payload_can_be_acknowledged_before_vault_upgrade() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::new(root.path()).unwrap();
        let mut legacy = Vault {
            schema: 1,
            ..Vault::default()
        };
        assert!(legacy.study.start("Physics", None, None, 1000));
        let old = Store::encode(&legacy).unwrap();
        let json = std::str::from_utf8(&old).unwrap();
        assert!(!json.contains(r#""planner""#));
        assert!(!json.contains(r#""kind""#));
        store
            .open()
            .unwrap()
            .execute(
                "INSERT INTO pending VALUES (0, ?1, ?2)",
                params![old, hash(&old)],
            )
            .unwrap();
        let observed = store.load().unwrap();
        assert!(observed.recovered);
        assert_eq!(observed.vault, legacy);
        assert_eq!(store.save(0, &observed.vault).unwrap(), 1);
        legacy.schema = SCHEMA;
        legacy.planner.weekly_goal_minutes = Some(240);
        assert_eq!(store.save(1, &legacy).unwrap(), 2);
        assert_eq!(store.load().unwrap().vault, legacy);
    }

    #[test]
    fn upgrades_the_existing_schema_without_replacing_notes() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::new(root.path()).unwrap();
        let mut vault = Vault::default();
        let book = vault.new_book("Pre-upgrade notebook", 1, Paper::default());
        vault.books.push(book);
        store.save(0, &vault).unwrap();
        store
            .open()
            .unwrap()
            .execute_batch("DROP TABLE pending; PRAGMA user_version=1;")
            .unwrap();
        let snapshot = store.load().unwrap();
        assert_eq!(snapshot.head, 1);
        assert_eq!(snapshot.vault, vault);
        assert_eq!(store.save(1, &vault).unwrap(), 2);
    }

    #[test]
    fn interrupted_process_preserves_last_acknowledged_save() {
        const CHILD_ROOT: &str = "ELLIPSE_NOTES_INTERRUPT_TEST_DIR";
        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            let path = PathBuf::from(root);
            let store = Store::new(&path).unwrap();
            let mut db = store.open().unwrap();
            let transaction = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            transaction.execute("DELETE FROM snapshots", []).unwrap();
            fs::write(path.join("uncommitted"), b"ready").unwrap();
            std::thread::park_timeout(Duration::from_secs(30));
            drop(transaction);
            return;
        }

        let fixture = tempfile::tempdir().unwrap();
        let store = Store::new(fixture.path()).unwrap();
        let mut vault = Vault::default();
        let book = vault.new_book("Before interrupted write", 1, Paper::default());
        vault.books.push(book);
        store.save(0, &vault).unwrap();
        let mut process = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::interrupted_process_preserves_last_acknowledged_save",
            ])
            .env(CHILD_ROOT, fixture.path())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(8);
        while !fixture.path().join("uncommitted").exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let ready = fixture.path().join("uncommitted").exists();
        let _ = process.kill();
        let _ = process.wait();
        assert!(ready, "child should have entered an uncommitted write");
        assert_eq!(store.load().unwrap().vault, vault);
    }

    #[test]
    fn killed_after_staging_recovers_completed_edit_and_rejects_stale_data() {
        const CHILD_ROOT: &str = "ELLIPSE_NOTES_STAGE_TEST_DIR";
        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            let path = PathBuf::from(root);
            let store = Store::new(&path).unwrap();
            let Snapshot {
                head, mut vault, ..
            } = store.load().unwrap();
            vault.books[0].title = "Completed edit".into();
            let payload = Store::encode(&vault).unwrap();
            let mut db = store.open().unwrap();
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            Store::stage(&tx, head, &vault, &payload, &hash(&payload)).unwrap();
            tx.commit().unwrap();
            fs::write(path.join("staged"), b"ready").unwrap();
            std::thread::park_timeout(Duration::from_secs(30));
            return;
        }

        let root = tempfile::tempdir().unwrap();
        let store = Store::new(root.path()).unwrap();
        let mut vault = Vault::default();
        let book = vault.new_book("Original", 1, Paper::default());
        vault.books.push(book);
        store.save(0, &vault).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::killed_after_staging_recovers_completed_edit_and_rejects_stale_data",
            ])
            .env(CHILD_ROOT, root.path())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(8);
        while !root.path().join("staged").exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let staged = root.path().join("staged").exists();
        let _ = child.kill();
        let _ = child.wait();
        assert!(staged);
        let pending = store.load().unwrap();
        assert_eq!(pending.head, 1);
        assert!(pending.recovered);
        assert_eq!(pending.vault.books[0].title, "Completed edit");
        assert!(store.save(1, &vault).is_err());
        assert_eq!(store.save(1, &pending.vault).unwrap(), 2);

        vault.books[0].title = "Newer acknowledged edit".into();
        assert_eq!(store.save(2, &vault).unwrap(), 3);
        let stale = Store::encode(&pending.vault).unwrap();
        store
            .open()
            .unwrap()
            .execute(
                "INSERT INTO pending VALUES (1, ?1, ?2)",
                params![stale, hash(&stale)],
            )
            .unwrap();
        assert_eq!(store.load().unwrap().vault, vault);
        assert!(store.save(1, &pending.vault).is_err());
    }

    #[test]
    #[ignore = "manual release-mode timing sample"]
    fn measure_large_checkpoint() {
        use ellipse_core::notes::{Pen, Point, Stroke, outline};
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path()).unwrap();
        let mut vault = Vault::default();
        for index in 0..1000 {
            let book = vault.new_book(&format!("Notebook {index}"), index, Paper::default());
            vault.books.push(book);
        }
        let mut ink = Vec::new();
        for index in 0..2000 {
            let id = vault.allocate();
            let points = (0..24)
                .map(|i| Point {
                    x: i as f32 * 2.,
                    y: index as f32 / 3.,
                    pressure: 0.6,
                    ..Point::default()
                })
                .collect();
            ink.push(Stroke {
                id,
                pen: Pen::default(),
                points,
            });
        }
        vault.books[0].pages[0].ink = ink;
        let start = std::time::Instant::now();
        store.save(0, &vault).unwrap();
        let save = start.elapsed();
        let start = std::time::Instant::now();
        let read = store.load().unwrap().vault;
        let load = start.elapsed();
        let start = std::time::Instant::now();
        let vertices: usize = read.books[0].pages[0]
            .ink
            .iter()
            .map(|s| outline(s).len())
            .sum();
        assert_eq!(vertices, 96_000);
        eprintln!(
            "1000 notebooks / 2000 strokes: save={save:?}, load={load:?}, outlines={:?}",
            start.elapsed()
        );
    }

    #[test]
    fn asset_identity_and_corruption() {
        let fixture = tempfile::tempdir().unwrap();
        let store = Store::new(fixture.path()).unwrap();
        let pdf = b"%PDF-1.7\n%%EOF";
        let id = store.store_pdf(pdf).unwrap();
        assert_eq!(store.store_pdf(pdf).unwrap(), id);
        assert_eq!(store.read_pdf(&id).unwrap(), pdf);
        assert!(store.read_pdf("../private").is_err());
        store
            .open()
            .unwrap()
            .execute("UPDATE assets SET content=x'00'", [])
            .unwrap();
        assert!(store.read_pdf(&id).is_err());
        assert_eq!(store.store_pdf(pdf).unwrap(), id);
        assert_eq!(store.read_pdf(&id).unwrap(), pdf);
        let db = store.open().unwrap();
        db.execute("UPDATE assets SET content=zeroblob(?1)", [MAX_PDF + 1])
            .unwrap();
        assert_eq!(
            store.read_pdf(&id).unwrap_err(),
            "The original PDF failed its integrity check."
        );
        assert_eq!(store.store_pdf(pdf).unwrap(), id);
        assert_eq!(store.read_pdf(&id).unwrap(), pdf);
    }

    #[test]
    fn skips_malformed_and_oversized_checkpoints_but_preserves_newer_formats() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path()).unwrap();
        let vault = Vault::default();
        store.save(0, &vault).unwrap();
        let db = store.open().unwrap();
        let digest = hash(b"placeholder");
        db.execute(
            "INSERT INTO snapshots VALUES (2, zeroblob(?1), ?2), (3, 'text instead of a BLOB', ?2)",
            params![MAX_STATE + 1, digest],
        )
        .unwrap();
        let recovered = store.load().unwrap();
        assert_eq!(recovered.head, 3);
        assert!(recovered.recovered);
        assert_eq!(recovered.vault, vault);

        // A valid digest with a newer schema blocks recovery even if fields changed.
        let future = br#"{"schema":99,"future_format":true}"#;
        db.execute(
            "INSERT INTO snapshots VALUES (4, ?1, ?2)",
            params![future, hash(future)],
        )
        .unwrap();
        assert_eq!(
            store.load().err().unwrap(),
            "These Notes require a newer version of Ellipse."
        );
        let count: u32 = db
            .query_row("SELECT count(*) FROM snapshots", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 4);
    }
}
