use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use tauri::{ipc::Response, AppHandle, Manager};

const MAX_BYTES: usize = 24 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn path(app: &AppHandle, account: &str, key: &str) -> Result<PathBuf, String> {
    let valid = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    };
    if !valid(account) || !valid(key) {
        return Err("Invalid local artwork reference.".into());
    }
    app.path()
        .app_local_data_dir()
        .map(|root| root.join("artwork-v1").join(account).join(key))
        .map_err(|_| "App artwork storage is unavailable.".into())
}

fn decode(encoded: &str) -> Result<Vec<u8>, String> {
    if encoded.is_empty() || encoded.len() > MAX_BYTES.div_ceil(3) * 4 {
        return Err("Choose an image no larger than 24 MiB.".into());
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| "Invalid artwork bytes.")?;
    if bytes.len() > MAX_BYTES || bytes.is_empty() {
        return Err("Choose an image no larger than 24 MiB.".into());
    }
    Ok(bytes)
}

#[tauri::command(async)]
pub fn store_artwork(
    app: AppHandle,
    account: String,
    key: String,
    encoded: String,
) -> Result<(), String> {
    let target = path(&app, &account, &key)?;
    let bytes = decode(&encoded)?;
    let dir = target
        .parent()
        .ok_or("App artwork storage is unavailable.")?;
    let save = || -> std::io::Result<()> {
        fs::create_dir_all(dir)?;
        if target.is_file() {
            return Ok(());
        }
        let temp = dir.join(format!(
            ".{}-{}.part",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let result = file.write_all(&bytes).and_then(|_| file.sync_all());
        drop(file);
        let result = result.and_then(|_| fs::rename(&temp, &target));
        if result.is_err() {
            let _ = fs::remove_file(&temp);
            if target.is_file() {
                return Ok(());
            }
        }
        result
    };
    save().map_err(|_| "The app could not save the image. Check available storage.".into())
}

#[tauri::command(async)]
pub fn read_artwork(app: AppHandle, account: String, key: String) -> Result<Response, String> {
    let path = path(&app, &account, &key)?;
    let file = fs::File::open(path).map_err(|_| "This image is unavailable on this device.")?;
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "This artwork file could not be read.")?;
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err("This artwork file is invalid.".into());
    }
    Ok(Response::new(bytes))
}

#[tauri::command(async)]
pub fn hash_artwork(encoded: String) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    Ok(format!("{:x}", Sha256::digest(decode(&encoded)?)))
}
