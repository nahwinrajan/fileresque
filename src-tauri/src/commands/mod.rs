// Tauri commands always have their return value consumed by the IPC mechanism.
#![allow(clippy::must_use_candidate)]

use fileresque_core::{error::AppError, types::DiskInfo};

/// Return the list of physical disks visible to the OS.
///
/// Delegates to the platform-specific enumeration crate. The heavy disk I/O
/// is performed on a blocking thread inside the crate — this command never
/// blocks the async executor.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if the OS denies access (Full Disk Access
///   not granted on macOS, not running as Administrator on Windows)
/// - [`AppError::Internal`] on unexpected parsing or runtime failures
/// - [`AppError::UnsupportedFilesystem`] on unsupported platforms
#[tauri::command]
pub async fn get_disks() -> Result<Vec<DiskInfo>, AppError> {
    #[cfg(target_os = "macos")]
    {
        fileresque_disk::macos::enumerate::list_disks().await
    }
    #[cfg(target_os = "windows")]
    {
        fileresque_disk::windows::enumerate::list_disks().await
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(AppError::UnsupportedFilesystem(
            "This platform is not supported".to_string(),
        ))
    }
}

/// Return `true` if the app has Full Disk Access on macOS.
///
/// On platforms where this check is not applicable (e.g. Windows, which
/// elevates at launch via UAC), always returns `true` — no onboarding needed.
///
/// The blocking `open(2)` syscall is dispatched via `spawn_blocking` so the
/// async executor is never stalled.
///
/// # Errors
///
/// - [`AppError::Internal`] if the `spawn_blocking` task is dropped unexpectedly
#[tauri::command]
pub async fn check_disk_access() -> Result<bool, AppError> {
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(fileresque_disk::macos::permissions::has_full_disk_access)
            .await
            .map_err(|e| AppError::Internal(format!("spawn_blocking join error: {e}")))?
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows elevates at launch via UAC; no FDA-style runtime check needed.
        Ok(true)
    }
}
