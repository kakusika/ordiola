// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // =========================================================================
        // Wayland / WebKitGTK 4.1 Compatibility Fix
        // =========================================================================
        // Context:
        // On Linux Wayland compositors (especially wlroots-based ones like Hyprland),
        // WebKitGTK 4.1's hardware accelerated DMA-BUF renderer can trigger:
        //   "Gdk-Message: Error 71 (プロトコルエラー) dispatching to Wayland display."
        //
        // Solution:
        // Disabling the DMA-BUF renderer forces WebKitGTK to fall back to safe
        // shared-memory (SHM) buffer exchange, eliminating protocol error crashes.
        //
        // SAFETY:
        // In Rust 2024 edition, `set_var` is marked unsafe due to potential data
        // races in multithreaded environments. Calling it here before spawning any
        // threads or initializing GTK/Tauri runtimes is completely safe.
        //
        // TODO(tauri-v3):
        // When upgrading to Tauri v3 (which ports Linux runtime from GTK3 / WebKitGTK 4.1
        // to GTK4 / WebKitGTK 6.0), GTK4 natively handles Wayland accelerated compositing.
        // Test removing this environment variable once Tauri v3 is adopted.
        // =========================================================================
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            // SAFETY: Process is single-threaded at the start of main.
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
    }

    // Parse optional vault path from command line arguments
    let initial_vault = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .map(std::path::PathBuf::from);

    telaion_lib::run(initial_vault)
}
