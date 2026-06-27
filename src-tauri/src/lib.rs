#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod commands;

/// Run the Tauri application event loop.
///
/// # Panics
///
/// Panics if the Tauri runtime fails to initialise. This is intentional and
/// unrecoverable — if the application cannot start, the process must exit.
/// The `expect` call is covered by the `// JUSTIFIED:` comment in the source.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![commands::greet])
        .run(tauri::generate_context!())
        // JUSTIFIED: unrecoverable — Tauri runtime failure means the process cannot continue
        .expect("error while running tauri application");
}
