//! Append-only recovery audit log (P4-T04).
//!
//! Every recovery attempt — success, partial, cancelled, or failed — is written
//! as one JSON object per line (JSONL) to `audit.log` in the app data directory
//! (resolved by the Tauri layer and passed in as `base_dir`, so this module is
//! filesystem-pure and testable). The log self-rotates: when the active file
//! reaches [`MAX_LOG_BYTES`] it is rolled to `audit.log.1`, older archives shift
//! up, and only [`MAX_ARCHIVES`] archives are kept.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use fileresque_core::error::AppError;
use serde::Serialize;

/// Active-log size threshold that triggers a rotation (10 MB).
pub const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;

/// Number of rotated archives retained (`audit.log.1` … `audit.log.5`).
pub const MAX_ARCHIVES: u32 = 5;

/// Base filename of the active log.
const LOG_NAME: &str = "audit.log";

/// Outcome status recorded for a recovery attempt.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStatus {
    /// All blocks read; file fully recovered.
    Success,
    /// Recovered, but one or more bad sectors were zero-filled.
    Partial,
    /// User cancelled before completion.
    Cancelled,
    /// Aborted by a write/rename error.
    Failed,
}

/// One audit record. Field set matches the P4-T04 spec.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    /// Seconds since the Unix epoch.
    pub timestamp: u64,
    pub source_disk: String,
    pub inode_id: u64,
    pub original_name: Option<String>,
    pub dest_path: Option<String>,
    pub sha256_dest: Option<String>,
    pub status: RecoveryStatus,
    pub blocks_read: u64,
    pub bytes_written: u64,
    pub duration_ms: u64,
}

impl AuditEntry {
    /// Build an entry stamped with the current wall-clock time.
    #[must_use]
    pub fn now(source_disk: String, inode_id: u64, status: RecoveryStatus) -> Self {
        Self {
            timestamp: now_secs(),
            source_disk,
            inode_id,
            original_name: None,
            dest_path: None,
            sha256_dest: None,
            status,
            blocks_read: 0,
            bytes_written: 0,
            duration_ms: 0,
        }
    }
}

/// Seconds since the Unix epoch, clamped to 0 before the epoch.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Append `entry` to `base_dir/audit.log`, rotating first if the active log has
/// reached [`MAX_LOG_BYTES`].
///
/// # Errors
///
/// Returns [`AppError::Io`] if the directory, rotation, or write fails.
pub fn append(base_dir: &Path, entry: &AuditEntry) -> Result<(), AppError> {
    append_with_limit(base_dir, entry, MAX_LOG_BYTES)
}

/// [`append`] with an explicit rotation threshold (kept internal so tests can
/// exercise rotation without writing 10 MB).
fn append_with_limit(base_dir: &Path, entry: &AuditEntry, max_bytes: u64) -> Result<(), AppError> {
    fs::create_dir_all(base_dir)?;
    let log_path = base_dir.join(LOG_NAME);

    if current_size(&log_path) >= max_bytes {
        rotate(base_dir)?;
    }

    let line = serde_json::to_string(entry)
        .map_err(|e| AppError::Internal(format!("audit serialise failed: {e}")))?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Current size of `path` in bytes, or 0 if it does not exist.
fn current_size(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |m| m.len())
}

/// Roll `audit.log` → `audit.log.1`, shifting `.1`→`.2` … and dropping the
/// oldest beyond [`MAX_ARCHIVES`].
fn rotate(base_dir: &Path) -> Result<(), AppError> {
    // Drop the oldest archive so the shift has somewhere to go.
    let oldest = archive_path(base_dir, MAX_ARCHIVES);
    if oldest.exists() {
        fs::remove_file(&oldest)?;
    }
    // Shift .{n} → .{n+1} from the top down so nothing is overwritten in use.
    for n in (1..MAX_ARCHIVES).rev() {
        let src = archive_path(base_dir, n);
        if src.exists() {
            fs::rename(&src, archive_path(base_dir, n + 1))?;
        }
    }
    // Active log → .1.
    let active = base_dir.join(LOG_NAME);
    if active.exists() {
        fs::rename(&active, archive_path(base_dir, 1))?;
    }
    Ok(())
}

/// Path of the `n`th archive: `audit.log.{n}`.
fn archive_path(base_dir: &Path, n: u32) -> PathBuf {
    base_dir.join(format!("{LOG_NAME}.{n}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(status: RecoveryStatus) -> AuditEntry {
        AuditEntry {
            timestamp: 1_700_000_000,
            source_disk: "disk0".to_string(),
            inode_id: 42,
            original_name: Some("photo.jpg".to_string()),
            dest_path: Some("/Volumes/Backup/photo.jpg".to_string()),
            sha256_dest: Some("abc123".to_string()),
            status,
            blocks_read: 10,
            bytes_written: 40960,
            duration_ms: 250,
        }
    }

    #[test]
    fn appends_one_jsonl_line_per_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        append(dir.path(), &entry(RecoveryStatus::Success)).expect("append 1");
        append(dir.path(), &entry(RecoveryStatus::Partial)).expect("append 2");

        let content = fs::read_to_string(dir.path().join(LOG_NAME)).expect("read log");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2, "one line per entry");
        // Each line must be valid standalone JSON with all required fields.
        for line in lines {
            let v: serde_json::Value = serde_json::from_str(line).expect("valid json");
            for field in [
                "timestamp",
                "source_disk",
                "inode_id",
                "original_name",
                "dest_path",
                "sha256_dest",
                "status",
                "blocks_read",
                "bytes_written",
                "duration_ms",
            ] {
                assert!(v.get(field).is_some(), "missing field: {field}");
            }
        }
        assert!(content.contains("\"status\":\"success\""));
        assert!(content.contains("\"status\":\"partial\""));
    }

    #[test]
    fn rotates_when_threshold_reached() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Tiny threshold: the first entry already pushes the active log over it,
        // so the second append rotates the first into audit.log.1.
        append_with_limit(dir.path(), &entry(RecoveryStatus::Success), 1).expect("a");
        append_with_limit(dir.path(), &entry(RecoveryStatus::Failed), 1).expect("b");

        assert!(dir.path().join(LOG_NAME).exists(), "active log present");
        assert!(
            dir.path().join("audit.log.1").exists(),
            "first entry rotated to .1"
        );
    }

    #[test]
    fn keeps_at_most_max_archives() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Force a rotation on every append; write well past MAX_ARCHIVES.
        for _ in 0..(MAX_ARCHIVES + 4) {
            append_with_limit(dir.path(), &entry(RecoveryStatus::Success), 1).expect("append");
        }
        // Highest archive kept is .MAX_ARCHIVES; nothing beyond exists.
        assert!(
            dir.path()
                .join(format!("{LOG_NAME}.{MAX_ARCHIVES}"))
                .exists(),
            "oldest retained archive present"
        );
        assert!(
            !dir.path()
                .join(format!("{LOG_NAME}.{}", MAX_ARCHIVES + 1))
                .exists(),
            "archives beyond the cap must be dropped"
        );
    }

    #[test]
    fn no_rotation_below_threshold() {
        let dir = tempfile::tempdir().expect("tempdir");
        append(dir.path(), &entry(RecoveryStatus::Success)).expect("append");
        assert!(
            !dir.path().join("audit.log.1").exists(),
            "must not rotate under the 10MB default"
        );
    }
}
