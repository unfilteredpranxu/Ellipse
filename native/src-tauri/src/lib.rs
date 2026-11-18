#![forbid(unsafe_code)]
mod artwork;
mod notes;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            artwork::store_artwork,
            artwork::read_artwork,
            artwork::hash_artwork,
            notes::notes_load,
            notes::notes_save,
            notes::notes_store_pdf,
            notes::notes_read_pdf
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Ellipse");
}
