//! Portable, uncompressed Notes archive. Asset names are derived from the validated vault.
use super::{Result, Snapshot, Store, hash, sqlite};
use ellipse_core::notes::{MAX_PDF, MAX_STATE, Vault, valid_pdf};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAGIC: &[u8] = b"EllipseVault\x01";
const CHECKSUM: usize = 32;
pub const MAX_BACKUP: usize = 96 * 1024 * 1024;
const INVALID: &str = "This Notes backup is incomplete, invalid, or from an unsupported version.";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Pdf,
    Artwork,
}

fn references(vault: &Vault) -> BTreeSet<(Kind, &str)> {
    let pdfs = vault
        .books
        .iter()
        .filter_map(|book| book.pdf.as_ref())
        .map(|pdf| (Kind::Pdf, pdf.asset.as_str()));
    let artwork = vault
        .books
        .iter()
        .filter_map(|book| book.cover_art.as_ref())
        .chain(vault.appearance.wallpaper.iter())
        .map(|art| (Kind::Artwork, art.asset.as_str()));
    pdfs.chain(artwork).collect()
}

fn artwork_signature(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\xff\xd8\xff")
        || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
        || (bytes.get(4..8) == Some(b"ftyp")
            && bytes.get(8..12).is_some_and(|brand| {
                matches!(
                    brand,
                    b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"
                )
            }))
}

fn valid_asset(kind: Kind, id: &str, bytes: &[u8]) -> bool {
    bytes.len() <= MAX_PDF
        && match kind {
            Kind::Pdf => valid_pdf(bytes),
            Kind::Artwork => artwork_signature(bytes),
        }
        && hash(bytes) == id
}

fn get_art(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|_| "A cover or wallpaper could not be read.")?;
    let mut bytes = Vec::new();
    file.take(MAX_PDF as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "A cover or wallpaper could not be read.")?;
    Ok(bytes)
}

fn install_art(directory: &Path, id: &str, bytes: &[u8]) -> Result<()> {
    let target = directory.join(id);
    if get_art(&target).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    let io_error = |_| "Artwork could not be restored. Check available space.";
    fs::create_dir_all(directory).map_err(io_error)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(io_error)?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(io_error)?;
    temporary
        .persist(target)
        .map_err(|_| "Artwork could not be restored. Check available space.")?;
    #[cfg(unix)]
    File::open(directory)
        .and_then(|dir| dir.sync_all())
        .map_err(io_error)?;
    Ok(())
}

fn push(out: &mut Vec<u8>, data: &[u8]) -> Result<()> {
    if out.len() + 4 + data.len() + CHECKSUM > MAX_BACKUP {
        return Err("The Notes vault exceeds the 96 MiB backup limit.".into());
    }
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
    Ok(())
}

fn take<'a>(input: &mut &'a [u8], limit: usize) -> Result<&'a [u8]> {
    let (length, bytes) = input.split_at_checked(4).ok_or(INVALID)?;
    let length = u32::from_be_bytes(length.try_into().map_err(|_| INVALID)?) as usize;
    if length > limit {
        return Err(INVALID.into());
    }
    let (result, rest) = bytes.split_at_checked(length).ok_or(INVALID)?;
    *input = rest;
    Ok(result)
}

fn parse(bytes: &[u8]) -> Result<(Vault, &[u8])> {
    if bytes.len() > MAX_BACKUP {
        return Err(INVALID.into());
    }
    let (body, checksum) = bytes
        .split_at_checked(bytes.len().saturating_sub(CHECKSUM))
        .ok_or(INVALID)?;
    if &Sha256::digest(body)[..] != checksum {
        return Err(INVALID.into());
    }
    let mut input = body.strip_prefix(MAGIC).ok_or(INVALID)?;
    let vault: Vault = serde_json::from_slice(take(&mut input, MAX_STATE)?).map_err(|_| INVALID)?;
    vault.validate()?;
    let mut assets = input;
    for (kind, id) in references(&vault) {
        if !valid_asset(kind, id, take(&mut assets, MAX_PDF)?) {
            return Err("A PDF or image in the Notes backup failed its integrity check.".into());
        }
    }
    if !assets.is_empty() {
        return Err(INVALID.into());
    }
    Ok((vault, input))
}

impl Store {
    pub fn export_backup(&self, head: u32, artwork: &Path, directory: &Path) -> Result<PathBuf> {
        let bytes = self.backup(head, artwork)?;
        write_archive(directory, |file| file.write_all(&bytes))
    }

    pub fn backup(&self, head: u32, artwork: &Path) -> Result<Vec<u8>> {
        let mut db = self.open()?;
        let tx = sqlite(db.transaction())?;
        Self::check_head(&tx, head)?;
        let snapshot = Self::committed(&tx)?;
        if snapshot.restore_required || snapshot.recovered || Self::has_pending(&tx, head)? {
            return Err("Recover Notes before creating a backup.".into());
        }
        let mut out = MAGIC.to_vec();
        push(&mut out, &Self::encode(&snapshot.vault)?)?;
        for (kind, id) in references(&snapshot.vault) {
            let data = match kind {
                Kind::Pdf => sqlite(tx.query_row(
                    "SELECT content FROM assets WHERE id=?1 AND typeof(content)='blob' AND length(content)<=?2",
                    params![id, MAX_PDF], |row| row.get::<_, Vec<u8>>(0),
                ).optional())?.ok_or("A PDF original is missing or invalid.")?,
                Kind::Artwork => get_art(&artwork.join(id))?,
            };
            if !valid_asset(kind, id, &data) {
                return Err("A local PDF or image failed its integrity check.".into());
            }
            push(&mut out, &data)?;
        }
        let checksum = Sha256::digest(&out);
        out.extend_from_slice(&checksum);
        Ok(out)
    }

    pub fn restore(&self, head: u32, artwork: &Path, bytes: &[u8]) -> Result<Snapshot> {
        let (vault, assets) = parse(bytes)?;
        let head = self.restore_validated(&mut self.open()?, head, artwork, &vault, assets)?;
        Ok(Snapshot {
            head,
            vault,
            recovered: false,
            restore_required: false,
        })
    }

    fn restore_validated(
        &self,
        db: &mut Connection,
        head: u32,
        artwork: &Path,
        vault: &Vault,
        mut assets: &[u8],
    ) -> Result<u32> {
        let tx = sqlite(db.transaction_with_behavior(TransactionBehavior::Immediate))?;
        Self::check_head(&tx, head)?;
        // Reject newer-format data and protect a recoverable edit before changing anything.
        Self::committed(&tx)?;
        if Self::pending(&tx, head)?.is_some() {
            return Err("Recover pending Notes edits before restoring a backup.".into());
        }
        for (kind, id) in references(vault) {
            let data = take(&mut assets, MAX_PDF)?;
            match kind {
                Kind::Pdf => {
                    sqlite(tx.execute(
                        "INSERT INTO assets VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET content=excluded.content WHERE assets.content != excluded.content",
                        params![id, data],
                    ))?;
                }
                Kind::Artwork => install_art(artwork, id, data)?,
            }
        }
        let payload = Self::encode(vault)?;
        let next = Self::write_checkpoint(&tx, head, &payload, &hash(&payload))?;
        sqlite(tx.commit())?;
        Ok(next)
    }
}

fn write_archive(
    directory: &Path,
    write: impl FnOnce(&mut File) -> std::io::Result<()>,
) -> Result<PathBuf> {
    let storage_error = |_| "The backup could not be written. Check available space.";
    fs::create_dir_all(directory).map_err(storage_error)?;
    let mut pending = tempfile::NamedTempFile::new_in(directory).map_err(storage_error)?;
    write(pending.as_file_mut())
        .and_then(|_| pending.as_file().sync_all())
        .map_err(storage_error)?;
    for number in 1.. {
        let path = directory.join(format!("Ellipse Notes {number}.ellipsevault"));
        match pending.persist_noclobber(&path) {
            Ok(_) => {
                #[cfg(unix)]
                File::open(directory)
                    .and_then(|dir| dir.sync_all())
                    .map_err(storage_error)?;
                return Ok(path);
            }
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                pending = error.file
            }
            Err(_) => {
                return Err("The backup could not be published. Check available space.".into());
            }
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ellipse_core::notes::{Folder, LocalImage, Paper, Pdf, Pen, Point, Stroke};

    fn fixture(store: &Store, artwork: &Path) -> (Vault, Vec<u8>, Vec<u8>) {
        let mut pdf = b"%PDF-1.7\n".to_vec();
        pdf.resize(120_000, b' ');
        pdf.extend_from_slice(b"\n%%EOF");
        let id = store.store_pdf(&pdf).unwrap();
        let image = b"\x89PNG\r\n\x1a\nfixture";
        let asset = hash(image);
        install_art(artwork, &asset, image).unwrap();
        let picture = LocalImage {
            asset,
            frame: Default::default(),
        };
        let mut vault = Vault::default();
        let folder = vault.allocate();
        vault.folders.push(Folder {
            id: folder,
            title: "Research".into(),
            parent: None,
        });
        let mut book = vault.new_book("Annotated PDF", 12, Paper::default());
        book.folder = Some(folder);
        book.cover_art = Some(picture.clone());
        book.pdf = Some(Pdf {
            asset: id,
            title: "Original".into(),
            transforms: vec![[1., 0., 0., -1., 0., 792.]],
        });
        book.pages[0].ink.push(Stroke {
            id: vault.allocate(),
            pen: Pen::default(),
            points: vec![Point {
                x: 10.,
                pressure: 0.5,
                ..Point::default()
            }],
        });
        vault.books.push(book);
        vault.appearance.wallpaper = Some(picture);
        use ellipse_core::notes::planner::{Day, Profile, Task, Track};
        vault.planner.profile = Some(Profile {
            track: Track::Both,
            year: 2027,
        });
        let day = Day::new(2027, 1, 17).unwrap();
        let id = vault
            .planner
            .save_task(Task::new(day, "Read the annotated PDF", 20))
            .unwrap();
        vault.planner.tasks[0].set_done(day, true, 25);
        assert!(
            vault
                .study
                .start("Physics", Some(vault.books[0].id), None, 30)
        );
        vault.study.active.as_mut().unwrap().task = Some(id);
        assert!(vault.study.finish(90));
        let head = store.save(0, &vault).unwrap();
        let archive = store.backup(head, artwork).unwrap();
        (vault, pdf, archive)
    }

    #[test]
    fn vault_round_trip_and_scope() {
        let root = tempfile::tempdir().unwrap();
        let original = Store::new(&root.path().join("source")).unwrap();
        let artwork = root.path().join("source-art");
        let (vault, pdf, archive) = fixture(&original, &artwork);
        let destination = Store::new(&root.path().join("target")).unwrap();
        let other_artwork = root.path().join("target-art");
        let unrelated = root.path().join("different-account");
        assert!(destination.backup(1, &other_artwork).is_err());
        let restored = destination.restore(0, &other_artwork, &archive).unwrap();
        assert_eq!(restored.vault, vault);
        let pdf_id = &vault.books[0].pdf.as_ref().unwrap().asset;
        let image_id = &vault.appearance.wallpaper.as_ref().unwrap().asset;
        assert_eq!(destination.read_pdf(pdf_id).unwrap(), pdf);
        assert_eq!(
            get_art(&other_artwork.join(image_id)).unwrap(),
            get_art(&artwork.join(image_id)).unwrap()
        );
        assert!(!unrelated.exists());
        assert_eq!(destination.backup(1, &other_artwork).unwrap(), archive);
        assert!(destination.restore(0, &other_artwork, &archive).is_err());
        assert_eq!(destination.load().unwrap().vault, vault);
    }

    #[test]
    fn interrupted_corrupt_and_full_restores_keep_prior_vault() {
        let root = tempfile::tempdir().unwrap();
        let source = Store::new(&root.path().join("source")).unwrap();
        let (vault, _, archive) = fixture(&source, &root.path().join("art"));
        let destination = Store::new(&root.path().join("target")).unwrap();
        let mut before = Vault::default();
        let book = before.new_book("Existing notebook", 1, Paper::default());
        before.books.push(book);
        destination.save(0, &before).unwrap();
        let artwork = root.path().join("target-art");
        let original_len = archive.len();
        for interrupted in [
            &archive[..9],
            &archive[..original_len / 2],
            &archive[..original_len - 1],
        ] {
            assert!(destination.restore(1, &artwork, interrupted).is_err());
        }
        let mut damaged = archive.clone();
        damaged[original_len - CHECKSUM - 1] ^= 1;
        assert!(destination.restore(1, &artwork, &damaged).is_err());
        let digest = Sha256::digest(&damaged[..original_len - CHECKSUM]);
        damaged[original_len - CHECKSUM..].copy_from_slice(&digest);
        assert!(destination.restore(1, &artwork, &damaged).is_err());
        assert!(!artwork.exists());
        fs::write(&artwork, b"filesystem obstruction").unwrap();
        assert!(destination.restore(1, &artwork, &archive).is_err());
        fs::remove_file(&artwork).unwrap();
        assert_eq!(destination.load().unwrap().vault, before);
        assert!(
            destination
                .read_pdf(&vault.books[0].pdf.as_ref().unwrap().asset)
                .is_err()
        );

        let mut db = destination.open().unwrap();
        let pages: u32 = db
            .pragma_query_value(None, "page_count", |row| row.get(0))
            .unwrap();
        db.pragma_update(None, "max_page_count", pages).unwrap();
        let (vault, assets) = parse(&archive).unwrap();
        assert!(
            destination
                .restore_validated(&mut db, 1, &artwork, &vault, assets)
                .is_err()
        );
        assert_eq!(destination.load().unwrap().vault, before);
        assert_eq!(destination.load().unwrap().head, 1);
    }

    #[test]
    fn interrupted_backup_never_replaces_a_published_archive() {
        let root = tempfile::tempdir().unwrap();
        let path = write_archive(root.path(), |file| file.write_all(b"original")).unwrap();
        let failure = write_archive(root.path(), |file| {
            file.write_all(b"partial")?;
            Err(std::io::Error::from(std::io::ErrorKind::StorageFull))
        });
        assert!(failure.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        let second = write_archive(root.path(), |file| file.write_all(b"next")).unwrap();
        assert_ne!(path, second);
        assert_eq!(fs::read(path).unwrap(), b"original");
        assert_eq!(fs::read(second).unwrap(), b"next");
    }

    #[test]
    fn restore_from_unreadable_checkpoints_checks_the_observed_revision() {
        let root = tempfile::tempdir().unwrap();
        let original = Store::new(&root.path().join("source")).unwrap();
        let artwork = root.path().join("artwork");
        let (vault, _, archive) = fixture(&original, &artwork);
        let target = Store::new(&root.path().join("target")).unwrap();
        target.save(0, &Vault::default()).unwrap();
        let db = target.open().unwrap();
        db.execute("UPDATE snapshots SET payload=x'00'", [])
            .unwrap();
        let unreadable = target.load().unwrap();
        assert_eq!(unreadable.head, 1);
        assert!(unreadable.restore_required);
        assert!(!unreadable.recovered);
        assert!(target.backup(unreadable.head, &artwork).is_err());
        assert!(target.restore(1, &artwork, &archive[..20]).is_err());
        assert!(target.load().unwrap().restore_required);

        let staged = Store::encode(&vault).unwrap();
        db.execute(
            "INSERT INTO pending VALUES (1, ?1, ?2)",
            params![staged, hash(&staged)],
        )
        .unwrap();
        let recoverable = target.load().unwrap();
        assert!(recoverable.recovered);
        assert!(!recoverable.restore_required);
        assert_eq!(recoverable.vault, vault);
        assert!(target.restore(1, &artwork, &archive).is_err());
        db.execute("UPDATE pending SET payload=x'00' WHERE base=1", [])
            .unwrap();
        assert!(target.load().unwrap().restore_required);

        let restored = target.restore(unreadable.head, &artwork, &archive).unwrap();
        assert_eq!(restored.head, 2);
        assert_eq!(target.load().unwrap().vault, vault);
        assert!(!target.load().unwrap().restore_required);
        assert!(target.restore(unreadable.head, &artwork, &archive).is_err());

        let future = br#"{"schema":99}"#;
        db.execute(
            "INSERT INTO pending VALUES (2, ?1, ?2)",
            params![future, hash(future)],
        )
        .unwrap();
        assert!(target.load().is_err());
        assert!(target.restore(2, &artwork, &archive).is_err());
    }
}
