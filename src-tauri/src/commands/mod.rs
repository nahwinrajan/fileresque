// Tauri commands always have their return value consumed by the IPC mechanism.
#![allow(clippy::must_use_candidate)]

/// Temporary greeting command. Replaced in P1-T01 with `get_disks`.
///
/// This exists solely to verify that the Tauri IPC pipeline is wired up correctly
/// during the P0 scaffold phase.
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}! FileResque is initialising.")
}
