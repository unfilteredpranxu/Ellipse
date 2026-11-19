#![forbid(unsafe_code)]
mod artwork;
mod notes;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            notes::record_native_panics(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
