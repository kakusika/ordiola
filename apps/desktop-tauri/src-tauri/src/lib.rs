use std::path::Path;
use telaion_core::{
    FileEntry, ProseMirrorDoc, prosemirror_to_tomet, scan_workspace, tomet_to_prosemirror,
};

#[tauri::command]
fn open_workspace(path: &str) -> Result<Vec<FileEntry>, String> {
    scan_workspace(Path::new(path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_tomet_document(path: &str) -> Result<ProseMirrorDoc, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    tomet_to_prosemirror(&source).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_tomet_document(path: &str, doc: ProseMirrorDoc) -> Result<(), String> {
    let source = prosemirror_to_tomet(&doc).map_err(|e| e.to_string())?;
    std::fs::write(path, source).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .runtime(tauri_runtime_wry::Wry::default())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(url) = window.url() {
                    eprintln!(">>> TELAION WEBVIEW URL: {}", url);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_workspace,
            read_tomet_document,
            write_tomet_document
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
