#![forbid(unsafe_code)]
mod artwork;
mod auth;
mod notes;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(auth::Pairing::default())
        .setup(|app| {
            notes::record_native_panics(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::begin_google_sign_in,
            auth::finish_google_sign_in,
            artwork::store_artwork,
            artwork::read_artwork,
            artwork::hash_artwork,
            notes::notes_load,
            notes::notes_save,
            notes::notes_store_pdf,
            notes::notes_read_pdf,
            notes::notes_backup,
            notes::notes_restore,
            notes::notes_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Ellipse");
}
