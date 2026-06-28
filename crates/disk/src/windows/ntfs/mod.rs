pub mod structs;

// Scanner is compiled on Windows (production) and all platforms in test mode.
// The file-level #![cfg] in scanner.rs gates the entire module.
#[cfg(any(target_os = "windows", test))]
pub(crate) mod scanner;

#[cfg(target_os = "windows")]
use fileresque_core::{error::AppError, types::DeletedFileEntry};
#[cfg(target_os = "windows")]
use tokio::sync::mpsc;

/// Synchronous NTFS scan. Opens the raw device at `device_path` (e.g.
/// `\\.\PhysicalDrive0`), reads the MFT, and sends each deleted file entry
/// over `tx`. Stops early when the receiver is dropped.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if the device cannot be opened
/// - [`AppError::Internal`] on VBR/MFT parse failures
#[cfg(target_os = "windows")]
pub fn scan_ntfs_sync(
    device_path: &str,
    tx: mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    scanner::scan_mft(device_path, &tx)
}

/// Async NTFS scan. Wraps [`scan_ntfs_sync`] in `tokio::task::spawn_blocking`.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if the device cannot be opened
/// - [`AppError::Internal`] on parse failures or `spawn_blocking` panics
#[cfg(target_os = "windows")]
pub async fn scan_ntfs(
    device_path: String,
    tx: mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || scan_ntfs_sync(&device_path, tx))
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking panic: {e}")))?
}
