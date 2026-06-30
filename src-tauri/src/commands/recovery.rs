// Tauri commands always have their return value consumed by the IPC mechanism.
#![allow(clippy::must_use_candidate)]

use fileresque_core::{
    error::AppError,
    types::{DeletedFileEntry, DiskInfo, PreflightResult, ProbabilityReport},
};
use fileresque_recovery::probability::assess_probability;
use fileresque_recovery::{audit, engine};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

/// Assess recovery probability for a single deleted file (P3-T02).
///
/// Takes the full `entry` and `disk` (DECISION-015) because the probability
/// engine samples the file's block extents — data that lives only in the
/// `DeletedFileEntry`, not derivable from an inode id alone.
///
/// Disk reads run on a blocking thread (DECISION-005); this command never
/// stalls the async executor.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] if the raw device cannot be opened
/// - [`AppError::Internal`] on `spawn_blocking` join failure or read errors
#[tauri::command]
pub async fn check_probability(
    entry: DeletedFileEntry,
    disk: DiskInfo,
) -> Result<ProbabilityReport, AppError> {
    tokio::task::spawn_blocking(move || assess_sync(&entry, &disk))
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking join error: {e}")))?
}

/// Synchronous assessment dispatched via `spawn_blocking`.
fn assess_sync(entry: &DeletedFileEntry, disk: &DiskInfo) -> Result<ProbabilityReport, AppError> {
    #[cfg(target_os = "macos")]
    {
        let mut probe = device_probe::DeviceProbe::open(&disk.id)?;
        assess_probability(entry, disk, &mut probe)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows raw-device probe is not yet wired (NTFS allocation bitmap is a
        // later task). Use a metadata-only probe: allocation state unknown, no
        // device reads — the engine returns a conservative (Medium-capped)
        // report with an "allocation could not be confirmed" warning.
        let mut probe = metadata_probe::MetadataProbe;
        assess_probability(entry, disk, &mut probe)
    }
}

// ── P4-T01: destination picker & pre-flight ──────────────────────────────────

/// Open the native folder picker and return the chosen destination path.
///
/// Returns `Ok(None)` when the user cancels the dialog — a normal outcome, not
/// an error. The dialog is driven through the (Rust-only) dialog plugin via a
/// callback, bridged to async with a oneshot channel so the executor never
/// blocks on user interaction.
///
/// # Errors
///
/// - [`AppError::Internal`] if the dialog callback channel closes unexpectedly
#[tauri::command]
pub async fn pick_destination_folder(app: AppHandle) -> Result<Option<String>, AppError> {
    let (tx, rx) = oneshot::channel();
    app.dialog().file().pick_folder(move |picked| {
        let _ = tx.send(picked);
    });
    let picked = rx
        .await
        .map_err(|e| AppError::Internal(format!("dialog channel closed: {e}")))?;
    Ok(picked.map(|p| p.to_string()))
}

/// Run all recovery pre-flight checks for the selected `entries` recovered from
/// `source` into `dest_path` (P4-T01).
///
/// Gathering touches the filesystem (free space, writability, source liveness),
/// so it runs on a blocking thread (DECISION-005). The decision itself is the
/// pure `preflight::evaluate`.
///
/// # Errors
///
/// - [`AppError::Io`] if the destination cannot be stat-ed
/// - [`AppError::Internal`] on `spawn_blocking` join failure
#[tauri::command]
pub async fn preflight_recovery(
    entries: Vec<DeletedFileEntry>,
    source: DiskInfo,
    dest_path: String,
) -> Result<PreflightResult, AppError> {
    tokio::task::spawn_blocking(move || preflight_sync(&entries, &source, &dest_path))
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking join error: {e}")))?
}

/// Synchronous pre-flight: gather platform facts, then evaluate.
fn preflight_sync(
    entries: &[DeletedFileEntry],
    source: &DiskInfo,
    dest_path: &str,
) -> Result<PreflightResult, AppError> {
    use fileresque_recovery::preflight::{evaluate, required_bytes, PreflightFacts};

    let dest = Path::new(dest_path);
    let info = fileresque_disk::fsinfo::dest_info(dest)?;

    let facts = PreflightFacts {
        same_disk: fileresque_disk::fsinfo::same_disk(&info.device, &source.id),
        available_bytes: info.available_bytes,
        dest_writable: fileresque_disk::fsinfo::is_writable(dest),
        source_readable: source_readable(&source.id),
    };

    Ok(evaluate(required_bytes(entries), &facts))
}

/// Best-effort liveness check on the recovery source: can its raw device be
/// opened for reading? Failure (disconnected, no Full Disk Access) → not
/// readable, which the pre-flight surfaces as [`SourceNotReadable`].
///
/// [`SourceNotReadable`]: fileresque_core::types::PreflightError::SourceNotReadable
#[cfg(target_os = "macos")]
fn source_readable(disk_id: &str) -> bool {
    device_probe::raw_device_path(disk_id).is_ok_and(|path| std::fs::File::open(path).is_ok())
}

#[cfg(target_os = "windows")]
fn source_readable(disk_id: &str) -> bool {
    std::fs::File::open(format!("\\\\.\\{disk_id}")).is_ok()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn source_readable(_disk_id: &str) -> bool {
    false
}

// ── P4-T02 / P4-T04: recovery engine driver + audit ──────────────────────────

/// Shared cancellation handle for the active recovery batch. Managed by Tauri.
pub struct RecoveryState {
    cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl RecoveryState {
    pub fn new() -> Self {
        Self {
            cancel: Mutex::new(None),
        }
    }
}

/// Emit progress at most once per this many blocks, plus on every bad sector,
/// so multi-GB files do not flood the IPC channel.
const PROGRESS_STRIDE: u64 = 64;

/// Begin recovering `entries` (carved from `source`) into `dest_path`. Returns
/// immediately; per-file progress and results stream via Tauri events:
///
/// - `recovery:progress`      — `{ inode_id, file_index, total_files, bytes_written, blocks_done, blocks_skipped }`
/// - `recovery:file_complete` — `{ inode_id, status, final_path, sha256, blocks_read, blocks_skipped, bytes_written }`
/// - `recovery:complete`      — `{ recovered, partial, failed, cancelled, total }`
///
/// Call [`preflight_recovery`] first — this command assumes the destination is
/// already validated.
///
/// # Errors
///
/// Returns `Err` only when the batch cannot be dispatched (state lock poisoned).
#[tauri::command]
pub async fn recover_files(
    app: AppHandle,
    entries: Vec<DeletedFileEntry>,
    source: DiskInfo,
    dest_path: String,
    state: State<'_, RecoveryState>,
) -> Result<(), AppError> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut guard = state
            .cancel
            .lock()
            .map_err(|_| AppError::Internal("recovery state lock poisoned".to_string()))?;
        *guard = Some(Arc::clone(&cancel));
    }

    tokio::spawn(async move {
        run_recovery(app, entries, source, PathBuf::from(dest_path), cancel).await;
    });
    Ok(())
}

/// Signal the active recovery to stop after the current block. No-op when idle.
///
/// # Errors
///
/// Returns `Err` only when the state lock is poisoned.
#[tauri::command]
pub async fn cancel_recovery(state: State<'_, RecoveryState>) -> Result<(), AppError> {
    let guard = state
        .cancel
        .lock()
        .map_err(|_| AppError::Internal("recovery state lock poisoned".to_string()))?;
    if let Some(flag) = guard.as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

/// Running tally over a recovery batch.
#[derive(Default)]
struct BatchTally {
    recovered: u64,
    partial: u64,
    failed: u64,
    cancelled: bool,
}

/// Drive the batch: recover each entry on a blocking thread, emit its result,
/// write an audit record, and emit a final summary.
async fn run_recovery(
    app: AppHandle,
    entries: Vec<DeletedFileEntry>,
    source: DiskInfo,
    dest_dir: PathBuf,
    cancel: Arc<AtomicBool>,
) {
    let base_dir = app.path().app_data_dir().ok();
    let total = entries.len();
    let mut tally = BatchTally::default();

    for (idx, entry) in entries.into_iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            tally.cancelled = true;
            break;
        }
        let job = RecoverJob {
            app: app.clone(),
            entry,
            source: source.clone(),
            dest_dir: dest_dir.clone(),
            cancel: Arc::clone(&cancel),
            idx,
            total,
        };
        let result = tokio::task::spawn_blocking(move || job.run()).await;
        if finalize(&app, base_dir.as_deref(), &source.id, result, &mut tally) {
            break; // cancelled mid-file
        }
    }

    let _ = app.emit(
        "recovery:complete",
        serde_json::json!({
            "recovered": tally.recovered,
            "partial": tally.partial,
            "failed": tally.failed,
            "cancelled": tally.cancelled,
            "total": total,
        }),
    );
}

/// Everything one blocking recovery needs, bundled to keep `run` self-contained.
struct RecoverJob {
    app: AppHandle,
    entry: DeletedFileEntry,
    source: DiskInfo,
    dest_dir: PathBuf,
    cancel: Arc<AtomicBool>,
    idx: usize,
    total: usize,
}

impl RecoverJob {
    /// Open the source device and run the engine, streaming throttled progress.
    fn run(self) -> Result<engine::RecoveryOutcome, AppError> {
        let mut reader = open_source_reader(&self.source)?;
        let req = engine::RecoveryRequest {
            entry: &self.entry,
            dest_dir: &self.dest_dir,
        };

        let app = self.app.clone();
        let inode = self.entry.inode_id;
        let (idx, total) = (self.idx, self.total);
        let mut last_emit = 0u64;
        let mut progress = move |p: engine::RecoveryProgress| {
            let stride_hit = p.blocks_done.saturating_sub(last_emit) >= PROGRESS_STRIDE;
            if p.blocks_done == 1 || stride_hit {
                last_emit = p.blocks_done;
                let _ = app.emit(
                    "recovery:progress",
                    serde_json::json!({
                        "inode_id": inode,
                        "file_index": idx,
                        "total_files": total,
                        "bytes_written": p.bytes_written,
                        "blocks_done": p.blocks_done,
                        "blocks_skipped": p.blocks_skipped,
                    }),
                );
            }
        };

        let cancel = Arc::clone(&self.cancel);
        let should_cancel = move || cancel.load(Ordering::SeqCst);
        engine::recover(&req, &mut reader, &mut progress, &should_cancel)
    }
}

/// Handle one finished job: emit `recovery:file_complete`, write the audit
/// record, update `tally`. Returns true when the batch should stop (cancelled).
fn finalize(
    app: &AppHandle,
    base_dir: Option<&Path>,
    source_id: &str,
    result: Result<Result<engine::RecoveryOutcome, AppError>, tokio::task::JoinError>,
    tally: &mut BatchTally,
) -> bool {
    match result {
        Ok(Ok(outcome)) => {
            let status = if outcome.had_bad_sectors() {
                tally.partial += 1;
                audit::RecoveryStatus::Partial
            } else {
                tally.recovered += 1;
                audit::RecoveryStatus::Success
            };
            emit_file_complete(app, &outcome, status);
            write_audit(base_dir, source_id, &outcome, status);
            false
        }
        Ok(Err(AppError::Cancelled)) => {
            tally.cancelled = true;
            true
        }
        Ok(Err(e)) => {
            tally.failed += 1;
            let _ = app.emit(
                "recovery:file_complete",
                serde_json::json!({ "status": "failed", "error": e.to_string() }),
            );
            true
        }
        Err(join_err) => {
            tally.failed += 1;
            let _ = app.emit(
                "recovery:file_complete",
                serde_json::json!({ "status": "failed", "error": format!("recovery task panicked: {join_err}") }),
            );
            false
        }
    }
}

/// Emit a `recovery:file_complete` event for a finished outcome.
fn emit_file_complete(
    app: &AppHandle,
    outcome: &engine::RecoveryOutcome,
    status: audit::RecoveryStatus,
) {
    let status_str = match status {
        audit::RecoveryStatus::Partial => "partial",
        _ => "success",
    };
    let _ = app.emit(
        "recovery:file_complete",
        serde_json::json!({
            "status": status_str,
            "inode_id": outcome.inode_id,
            "final_path": outcome.final_path.to_string_lossy(),
            "sha256": outcome.sha256,
            "blocks_read": outcome.blocks_read,
            "blocks_skipped": outcome.blocks_skipped,
            "bytes_written": outcome.bytes_written,
        }),
    );
}

/// Append a recovery record to the audit log. Best-effort: a logging failure
/// must never fail the recovery itself.
fn write_audit(
    base_dir: Option<&Path>,
    source_id: &str,
    outcome: &engine::RecoveryOutcome,
    status: audit::RecoveryStatus,
) {
    let Some(dir) = base_dir else { return };
    let mut entry = audit::AuditEntry::now(source_id.to_string(), outcome.inode_id, status);
    entry.original_name.clone_from(&outcome.original_name);
    entry.dest_path = Some(outcome.final_path.to_string_lossy().to_string());
    entry.sha256_dest = Some(outcome.sha256.clone());
    entry.blocks_read = outcome.blocks_read;
    entry.bytes_written = outcome.bytes_written;
    let _ = audit::append(dir, &entry);
}

/// Open the source disk's raw device with the correct filesystem block size.
///
/// The returned adapter bridges `fileresque_disk`'s `BlockReader` to the
/// engine's `ExtentReader` trait (orphan rule: the wrapper type is local here).
#[cfg(target_os = "macos")]
fn open_source_reader(source: &DiskInfo) -> Result<DeviceExtentReaderImpl, AppError> {
    use fileresque_disk::macos::apfs::{reader::BlockReader, scanner::parse_nx_superblock};
    let path = device_probe::raw_device_path(&source.id)?;
    let mut reader = BlockReader::open(&path)?;
    // Fix the block size from the APFS container superblock when available.
    if let Ok(block0) = reader.read_block(0) {
        if let Ok(nx) = parse_nx_superblock(&block0) {
            reader.block_size = nx.block_size;
        }
    }
    Ok(DeviceExtentReaderImpl { reader })
}

#[cfg(target_os = "macos")]
struct DeviceExtentReaderImpl {
    reader: fileresque_disk::macos::apfs::reader::BlockReader,
}

#[cfg(target_os = "macos")]
impl engine::ExtentReader for DeviceExtentReaderImpl {
    fn block_size(&self) -> u64 {
        u64::from(self.reader.block_size)
    }
    fn read_block(&mut self, block_addr: u64) -> Result<Vec<u8>, AppError> {
        self.reader.read_block(block_addr)
    }
}

#[cfg(not(target_os = "macos"))]
fn open_source_reader(_source: &DiskInfo) -> Result<DeviceExtentReaderImpl, AppError> {
    Err(AppError::UnsupportedFilesystem(
        "recovery is implemented for macOS APFS in this build".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
struct DeviceExtentReaderImpl;

#[cfg(not(target_os = "macos"))]
impl engine::ExtentReader for DeviceExtentReaderImpl {
    fn block_size(&self) -> u64 {
        4096
    }
    fn read_block(&mut self, _block_addr: u64) -> Result<Vec<u8>, AppError> {
        Err(AppError::UnsupportedFilesystem("unsupported".to_string()))
    }
}

#[cfg(target_os = "macos")]
mod device_probe {
    use fileresque_core::error::AppError;
    use fileresque_disk::macos::apfs::{
        reader::BlockReader, scanner::parse_nx_superblock, spaceman::AllocationMap,
    };
    use fileresque_recovery::probability::BlockProbe;

    /// Real probe over a raw APFS block device. Reads block heads for zero-fill
    /// detection and answers allocation queries against the space-manager free
    /// bitmap when it could be loaded.
    pub struct DeviceProbe {
        reader: BlockReader,
        /// `None` when the free-space map could not be resolved/parsed; the
        /// engine then treats allocation state as unknown.
        alloc: Option<AllocationMap>,
    }

    impl DeviceProbe {
        /// Open `/dev/rdiskN` for `disk_id` (e.g. `"disk0"`) and load the
        /// container's free-space map.
        ///
        /// # Errors
        ///
        /// Returns [`AppError`] if `disk_id` is malformed or the device cannot
        /// be opened. A free-map that cannot be parsed is non-fatal: the probe
        /// degrades to "allocation unknown".
        pub fn open(disk_id: &str) -> Result<Self, AppError> {
            let path = raw_device_path(disk_id)?;
            let mut reader = BlockReader::open(&path)?;
            let alloc = load_allocation_map(&mut reader);
            Ok(Self { reader, alloc })
        }
    }

    /// Read the container superblock, fix the block size, and load the free map.
    /// Any failure yields `None` (allocation unknown) rather than an error.
    fn load_allocation_map(reader: &mut BlockReader) -> Option<AllocationMap> {
        let block0 = reader.read_block(0).ok()?;
        let nx_sb = parse_nx_superblock(&block0).ok()?;
        reader.block_size = nx_sb.block_size;
        AllocationMap::load(reader, &nx_sb).ok()
    }

    impl BlockProbe for DeviceProbe {
        fn block_size(&self) -> u64 {
            u64::from(self.reader.block_size)
        }

        fn is_free(&mut self, block_addr: u64) -> Result<Option<bool>, AppError> {
            match &mut self.alloc {
                Some(map) => map.is_free(&mut self.reader, block_addr),
                None => Ok(None),
            }
        }

        fn read_head(&mut self, block_addr: u64, len: usize) -> Result<Vec<u8>, AppError> {
            let mut buf = self.reader.read_block(block_addr)?;
            buf.truncate(len);
            Ok(buf)
        }
    }

    /// `"disk0"` → `"/dev/rdisk0"`. Validates the `disk` + digits pattern.
    pub(super) fn raw_device_path(disk_id: &str) -> Result<String, AppError> {
        const PREFIX: &str = "disk";
        let suffix = disk_id
            .strip_prefix(PREFIX)
            .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()));
        match suffix {
            Some(n) => Ok(format!("/dev/rdisk{n}")),
            None => Err(AppError::Internal(format!("Invalid disk_id: {disk_id}"))),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn raw_device_path_maps_valid_id() {
            // JUSTIFIED: test-only; valid input must map successfully
            let path = raw_device_path("disk2").expect("valid id maps");
            assert_eq!(path, "/dev/rdisk2");
        }

        #[test]
        fn raw_device_path_rejects_bad_id() {
            assert!(raw_device_path("sda").is_err());
            assert!(raw_device_path("disk").is_err());
            assert!(raw_device_path("diskX").is_err());
        }

        #[test]
        fn open_nonexistent_device_errs() {
            let result = DeviceProbe::open("disk99999");
            assert!(result.is_err(), "opening nonexistent device must error");
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod metadata_probe {
    use fileresque_core::error::AppError;
    use fileresque_recovery::probability::BlockProbe;

    /// Metadata-only probe: no device access. Allocation unknown; block heads
    /// reported as non-zero so the engine does not mistake "no data read" for
    /// "data erased". Yields a conservative Medium-capped report.
    pub struct MetadataProbe;

    impl BlockProbe for MetadataProbe {
        fn block_size(&self) -> u64 {
            4096
        }
        fn is_free(&mut self, _block_addr: u64) -> Result<Option<bool>, AppError> {
            Ok(None)
        }
        fn read_head(&mut self, _block_addr: u64, len: usize) -> Result<Vec<u8>, AppError> {
            Ok(vec![0xFF; len])
        }
    }
}
