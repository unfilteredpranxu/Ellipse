//! Transactional local Notes storage. The caller supplies a private application directory.
use ellipse_core::{
    artwork::valid_asset_id,
    notes::{MAX_STATE, Vault, valid_pdf},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
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
                 PRAGMA user_version = 1;",
            ))?,
            1 => {},
            _ => return Err("This Notes database requires a newer version of Ellipse.".into()),
        }
        sqlite(tx.commit())?;
        Ok(db)
    }

    pub fn load(&self) -> Result<Snapshot> {
        let db = self.open()?;
        let mut query = sqlite(db.prepare(
            "SELECT revision, payload, digest FROM snapshots ORDER BY revision DESC LIMIT 4",
        ))?;
        let mut rows = sqlite(query.query([]))?;
        let mut head = None;
        while let Some(row) = sqlite(rows.next())? {
            let revision: u32 = sqlite(row.get(0))?;
            let head = *head.get_or_insert(revision);
            let data: Vec<u8> = sqlite(row.get(1))?;
            let digest: String = sqlite(row.get(2))?;
            if data.len() <= MAX_STATE
                && hash(&data) == digest
                && let Ok(vault) = serde_json::from_slice::<Vault>(&data)
                && vault.validate().is_ok()
            {
                return Ok(Snapshot {
                    head,
                    vault,
                    recovered: revision != head,
                });
            }
        }
        if head.is_some() {
            return Err(
                "Notes could not recover a valid checkpoint. The database was preserved.".into(),
            );
        }
        Ok(Snapshot {
            head: 0,
            vault: Vault::default(),
            recovered: false,
        })
    }

    pub fn save(&self, head: u32, vault: &Vault) -> Result<u32> {
        vault.validate()?;
        let payload = serde_json::to_vec(vault).map_err(|_| "Could not encode Notes.")?;
        if payload.len() > MAX_STATE {
            return Err("The Notes library exceeds this version’s storage limit. Export or remove material before saving.".into());
        }
        let digest = hash(&payload);
        let mut db = self.open()?;
        let tx = sqlite(db.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let current: u32 = sqlite(tx.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM snapshots",
            [],
            |r| r.get(0),
        ))?;
        if current != head {
            return Err("Notes changed in another window. Close the other window and reopen the library before editing.".into());
        }
        for pdf in vault.books.iter().filter_map(|book| book.pdf.as_ref()) {
            let exists: bool = sqlite(tx.query_row(
                "SELECT EXISTS (SELECT 1 FROM assets WHERE id = ?1)",
                [&pdf.asset],
                |row| row.get(0),
            ))?;
            if !exists {
                return Err("A PDF referenced by this save is missing.".into());
            }
        }
        let next = current
            .checked_add(1)
            .ok_or("Notes revision limit reached.")?;
        sqlite(tx.execute(
            "INSERT INTO snapshots VALUES (?1, ?2, ?3)",
            params![next, payload, digest],
        ))?;
        sqlite(tx.execute(
            "DELETE FROM snapshots WHERE revision <= ?1",
            [next.saturating_sub(4)],
        ))?;
        sqlite(tx.commit())?;
        Ok(next)
    }

    pub fn store_pdf(&self, bytes: &[u8]) -> Result<String> {
        if !valid_pdf(bytes) {
            return Err("Select a complete PDF up to 24 MiB.".into());
        }
        let id = hash(bytes);
        let db = self.open()?;
        sqlite(db.execute(
            "INSERT OR IGNORE INTO assets VALUES (?1, ?2)",
            params![id, bytes],
        ))?;
        Ok(id)
    }

    pub fn read_pdf(&self, id: &str) -> Result<Vec<u8>> {
        if !valid_asset_id(id) {
            return Err("Invalid PDF reference.".into());
        }
        let db = self.open()?;
        let bytes: Vec<u8> = sqlite(
            db.query_row("SELECT content FROM assets WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional(),
        )?
        .ok_or("The original PDF could not be found.")?;
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
        assert!(store.save(1, &vault).is_err());
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
    }
}
