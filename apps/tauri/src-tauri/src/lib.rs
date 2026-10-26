use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};
use telaion_config::{AppConfig, VaultEntry};
use telaion_core::{
    FileEntry, ProseMirrorDoc, prosemirror_to_tomet, scan_workspace, tomet_to_prosemirror,
};

/// Global application state managed by Tauri
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub current_vault: Mutex<PathBuf>,
}

#[derive(Debug, serde::Serialize)]
pub struct VaultState {
    pub current_path: String,
    pub current_name: String,
    pub recent_vaults: Vec<VaultEntry>,
}

#[derive(Debug, serde::Serialize)]
pub struct VaultSwitchResult {
    pub state: VaultState,
    pub files: Vec<FileEntry>,
}

#[tauri::command]
fn get_vault_state(state: State<'_, AppState>) -> Result<VaultState, String> {
    let current = state.current_vault.lock().map_err(|e| e.to_string())?;
    let config = state.config.lock().map_err(|e| e.to_string())?;

    let name = current
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("vault")
        .to_string();

    Ok(VaultState {
        current_path: current.to_string_lossy().to_string(),
        current_name: name,
        recent_vaults: config.recent_vaults.clone(),
    })
}

#[tauri::command]
fn switch_vault(path: &str, state: State<'_, AppState>) -> Result<VaultSwitchResult, String> {
    let target_path = PathBuf::from(path);
    let canonical = target_path
        .canonicalize()
        .map_err(|e| format!("無効な Vault パスです: {e}"))?;

    if !canonical.is_dir() {
        return Err(format!(
            "指定されたパスはディレクトリではありません: {}",
            canonical.display()
        ));
    }

    // Update current vault
    {
        let mut current = state.current_vault.lock().map_err(|e| e.to_string())?;
        *current = canonical.clone();
    }

    // Update recent vaults in config and persist
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.add_recent_vault(&canonical);
        let _ = config.save();
    }

    // Scan files in new vault
    let files = scan_workspace(&canonical).map_err(|e| e.to_string())?;
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("vault")
        .to_string();

    Ok(VaultSwitchResult {
        state: VaultState {
            current_path: canonical.to_string_lossy().to_string(),
            current_name: name,
            recent_vaults: config.recent_vaults.clone(),
        },
        files,
    })
}

#[tauri::command]
fn open_workspace(
    path: Option<&str>,
    state: State<'_, AppState>,
) -> Result<Vec<FileEntry>, String> {
    let target_path = match path {
        Some(p) if !p.is_empty() && p != "." => PathBuf::from(p),
        _ => {
            let current = state.current_vault.lock().map_err(|e| e.to_string())?;
            current.clone()
        }
    };
    scan_workspace(&target_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_tomet_document(path: &str, state: State<'_, AppState>) -> Result<ProseMirrorDoc, String> {
    let file_path = resolve_document_path(path, &state)?;
    let source = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("ファイル読み込み失敗 ({}): {e}", file_path.display()))?;
    tomet_to_prosemirror(&source).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_tomet_document(
    path: &str,
    doc: ProseMirrorDoc,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let file_path = resolve_document_path(path, &state)?;
    let source = prosemirror_to_tomet(&doc).map_err(|e| e.to_string())?;
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file_path, source)
        .map_err(|e| format!("ファイル保存失敗 ({}): {e}", file_path.display()))
}

fn resolve_document_path(path: &str, state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        let current = state.current_vault.lock().map_err(|e| e.to_string())?;
        Ok(current.join(p))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_with_vault(None);
}

pub fn run_with_vault(initial_vault: Option<PathBuf>) {
    let mut config = AppConfig::load();

    // Determine initial active vault
    let active_vault = if let Some(cli_path) = initial_vault {
        cli_path.canonicalize().unwrap_or(cli_path)
    } else if let Some(last) = config.last_vault.clone() {
        if last.exists() {
            last
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let active_vault = active_vault.canonicalize().unwrap_or(active_vault);
    config.add_recent_vault(&active_vault);
    let _ = config.save();

    let app_state = AppState {
        config: Mutex::new(config),
        current_vault: Mutex::new(active_vault),
    };

    tauri::Builder::default()
        .runtime(tauri_runtime_wry::Wry::default())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(url) = window.url() {
                    eprintln!(">>> TELAION WEBVIEW URL: {}", url);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_vault_state,
            switch_vault,
            open_workspace,
            read_tomet_document,
            write_tomet_document
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
