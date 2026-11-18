use base64::{engine::general_purpose::STANDARD, Engine as _};
use ellipse_core::notes::{Vault, MAX_PDF};
use ellipse_notes_store::{Snapshot, Store};
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
