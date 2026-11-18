use base64::{engine::general_purpose::STANDARD, Engine as _};
use ellipse_core::notes::{Vault, MAX_PDF};
use ellipse_notes_store::{Snapshot, Store, MAX_BACKUP};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::{ipc::Response, AppHandle, Manager, WebviewWindow};

fn local(window: &WebviewWindow, app: &AppHandle) -> Result<Store, String> {
    if window.label() != "main"
        || !window
            .url()
            .is_ok_and(|url| match (url.scheme(), url.host_str()) {
                ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost")) => true,
                ("http", Some("127.0.0.1" | "localhost")) => {
                    cfg!(debug_assertions) && url.port() == Some(5173)
                }
                _ => false,
            })
    {
        return Err("Local Notes is available in the installed Ellipse window.".into());
    }
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Local Notes storage could not be found.")?;
    Store::new(&root.join("notes").join("device"))
}

#[tauri::command(async)]
pub fn notes_load(window: WebviewWindow, app: AppHandle) -> Result<Snapshot, String> {
    local(&window, &app)?.load()
}

#[tauri::command(async)]
pub fn notes_save(
    window: WebviewWindow,
    app: AppHandle,
    head: u32,
    vault: Vault,
) -> Result<u32, String> {
    local(&window, &app)?.save(head, &vault)
}

#[tauri::command(async)]
pub fn notes_store_pdf(
    window: WebviewWindow,
    app: AppHandle,
    encoded: String,
) -> Result<String, String> {
    let store = local(&window, &app)?;
    if encoded.len() > MAX_PDF.div_ceil(3) * 4 {
        return Err("Choose a PDF up to 24 MiB.".into());
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| "The PDF could not be read.")?;
    store.store_pdf(&bytes)
}

#[tauri::command(async)]
pub fn notes_read_pdf(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
) -> Result<Response, String> {
    Ok(Response::new(local(&window, &app)?.read_pdf(&id)?))
}

fn artwork_directory(app: &AppHandle) -> Result<PathBuf, String> {
    // The device-local Notes namespace is independent of signed-in artwork.
    let account = format!("{:x}", Sha256::digest(b"ellipse-notes-device-v1"));
    app.path()
        .app_local_data_dir()
        .map(|root| root.join("artwork-v1").join(account))
        .map_err(|_| "Local Notes artwork could not be found.".into())
}

#[tauri::command(async)]
pub fn notes_backup(window: WebviewWindow, app: AppHandle, head: u32) -> Result<String, String> {
    let store = local(&window, &app)?;
    let documents = app
        .path()
        .document_dir()
        .map_err(|_| "The device’s Documents directory is unavailable.")?;
    let saved = store.export_backup(
        head,
        &artwork_directory(&app)?,
        &documents.join("Ellipse Backups"),
    )?;
    Ok(saved.display().to_string())
}

#[tauri::command(async)]
pub fn notes_restore(
    window: WebviewWindow,
    app: AppHandle,
    head: u32,
    encoded: String,
) -> Result<Snapshot, String> {
    let store = local(&window, &app)?;
    if encoded.len() > MAX_BACKUP.div_ceil(3) * 4 {
        return Err("Choose a Notes vault backup up to 96 MiB.".into());
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| "The Notes backup could not be read.")?;
    store.restore(head, &artwork_directory(&app)?, &bytes)
}
