// Tauri commands always have their return value consumed by the IPC mechanism.
#![allow(clippy::must_use_candidate)]

use fileresque_core::{
    error::AppError,
    types::{DeletedFileEntry, FileSystem},
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, oneshot};

// ── Shared scan state ────────────────────────────────────────────────────────

/// Shared state holding a cancellation channel for the active scan.
/// Managed by Tauri; accessible from any command via `State<ScanState>`.
pub struct ScanState {
    cancel_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            cancel_tx: Mutex::new(None),
        }
    }
}

// ── Public Tauri commands ────────────────────────────────────────────────────

/// Begin scanning `disk_id` for deleted files. Returns immediately; results
/// are streamed to the frontend via Tauri events:
///
/// - `scan:file_found` — one `DeletedFileEntry` per file discovered
/// - `scan:progress`   — periodic progress update (every 50 files)
/// - `scan:complete`   — scan finished `{ total_found, duration_ms }`
/// - `scan:error`      — unrecoverable scan error `{ message, recoverable }`
///
/// Calling `start_scan` while one is already running replaces the cancel
/// handle, so the old scan drains naturally.
///
/// # Errors
///
/// Returns `Err` synchronously only when the scan cannot even be dispatched
/// (unknown filesystem, lock poisoned, unsupported platform).
#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    disk_id: String,
    state: State<'_, ScanState>,
) -> Result<(), AppError> {
    let (entry_tx, entry_rx) = mpsc::channel::<DeletedFileEntry>(128);
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

    {
        let mut guard = state
            .cancel_tx
            .lock()
            .map_err(|_| AppError::Internal("scan state lock poisoned".to_string()))?;
        *guard = Some(cancel_tx);
    }

    let filesystem = resolve_filesystem(&disk_id).await?;

    tokio::spawn(async move {
        run_scan_loop(app, disk_id, filesystem, entry_tx, entry_rx, cancel_rx).await;
    });

    Ok(())
}

/// Cancel the active scan. No-op when no scan is running.
///
/// # Errors
///
/// Returns `Err` only when the state lock is poisoned (should never happen).
#[tauri::command]
pub async fn cancel_scan(state: State<'_, ScanState>) -> Result<(), AppError> {
    let mut guard = state
        .cancel_tx
        .lock()
        .map_err(|_| AppError::Internal("scan state lock poisoned".to_string()))?;
    if let Some(tx) = guard.take() {
        let _ = tx.send(());
    }
    Ok(())
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Determine the filesystem type for `disk_id` without starting a scan.
/// On macOS, calls `list_disks()` to look up the `DiskInfo` entry.
/// On Windows, always returns `NTFS` (the only supported Windows FS).
async fn resolve_filesystem(disk_id: &str) -> Result<FileSystem, AppError> {
    #[cfg(target_os = "macos")]
    {
        let disks = fileresque_disk::macos::enumerate::list_disks().await?;
        disks
            .into_iter()
            .find(|d| d.id == disk_id)
            .map(|d| d.filesystem)
            .ok_or_else(|| AppError::DiskNotFound(disk_id.to_string()))
    }
    #[cfg(target_os = "windows")]
    {
        let _ = disk_id;
        Ok(FileSystem::NTFS)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = disk_id;
        Err(AppError::UnsupportedFilesystem(
            "unsupported platform".to_string(),
        ))
    }
}

/// Dispatch a synchronous scan to the correct platform-specific scanner.
/// Called from inside `spawn_blocking`.
// JUSTIFIED: Windows arm moves entry_tx into scan_ntfs_sync; macOS arms borrow it.
// Taking by value is required to support both arms from a single owned value.
#[allow(clippy::needless_pass_by_value)]
fn dispatch_scan_sync(
    filesystem: FileSystem,
    disk_id: &str,
    entry_tx: mpsc::Sender<DeletedFileEntry>,
) -> Result<(), AppError> {
    match filesystem {
        #[cfg(target_os = "macos")]
        FileSystem::APFS => fileresque_disk::macos::apfs::scan_apfs_sync(disk_id, &entry_tx),
        #[cfg(target_os = "macos")]
        FileSystem::HFSPlus => {
            fileresque_disk::macos::hfsplus::scan_hfsplus_sync(disk_id, &entry_tx)
        }
        #[cfg(target_os = "windows")]
        FileSystem::NTFS => {
            // Windows device path: \\.\PhysicalDriveN
            let device_path = format!("\\\\.\\{disk_id}");
            fileresque_disk::windows::ntfs::scan_ntfs_sync(&device_path, entry_tx)
        }
        fs => Err(AppError::UnsupportedFilesystem(format!("{fs:?}"))),
    }
}

/// Drive the scan event loop: start the blocking scan, collect entries, emit
/// Tauri events, and handle cancellation.
///
/// When the cancel signal fires, `entry_rx` is dropped, which causes the next
/// `blocking_send` in the scan thread to fail and the scan thread stops.
async fn run_scan_loop(
    app: AppHandle,
    disk_id: String,
    filesystem: FileSystem,
    entry_tx: mpsc::Sender<DeletedFileEntry>,
    mut entry_rx: mpsc::Receiver<DeletedFileEntry>,
    cancel_rx: oneshot::Receiver<()>,
) {
    let start = std::time::Instant::now();
    let mut total_found = 0u64;

    // Watch for the source disk being pulled mid-scan (P5-T03).
    let watch_stop = Arc::new(AtomicBool::new(false));
    crate::disk_watch::spawn(app.clone(), disk_id.clone(), Arc::clone(&watch_stop));

    let scan_handle =
        tokio::task::spawn_blocking(move || dispatch_scan_sync(filesystem, &disk_id, entry_tx));

    let mut cancel_rx = cancel_rx;

    'collect: loop {
        tokio::select! {
            biased;
            _ = &mut cancel_rx => break 'collect,
            entry_opt = entry_rx.recv() => {
                match entry_opt {
                    Some(entry) => {
                        total_found += 1;
                        let _ = app.emit("scan:file_found", &entry);
                        if total_found.is_multiple_of(50) {
                            let _ = app.emit(
                                "scan:progress",
                                serde_json::json!({
                                    "scanned_bytes": 0u64,
                                    "total_bytes": 0u64,
                                    "files_found": total_found,
                                }),
                            );
                        }
                    }
                    None => break 'collect,
                }
            }
        }
    }

    // Dropping entry_rx signals the scan thread to stop via blocking_send Err.
    drop(entry_rx);

    // Scan is finished; retire the disconnection watcher.
    watch_stop.store(true, Ordering::SeqCst);

    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    match scan_handle.await {
        Ok(Ok(())) => {
            let _ = app.emit(
                "scan:complete",
                serde_json::json!({
                    "total_found": total_found,
                    "duration_ms": duration_ms,
                }),
            );
        }
        Ok(Err(e)) => {
            let _ = app.emit(
                "scan:error",
                serde_json::json!({
                    "message": e.user_message(),
                    "recoverable": false,
                }),
            );
        }
        Err(_join_err) => {
            let _ = app.emit(
                "scan:error",
                serde_json::json!({
                    "message": "The scan stopped unexpectedly. Please try again.",
                    "recoverable": false,
                }),
            );
        }
    }
}
