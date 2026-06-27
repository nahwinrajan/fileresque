use fileresque_core::{error::AppError, types::DiskInfo};

/// Enumerate physical disks visible to the OS on Windows.
///
/// This is a stub implementation. Full implementation is in P1-T02 via
/// `DeviceIoControl` with `IOCTL_STORAGE_QUERY_PROPERTY`.
///
/// # Errors
///
/// Returns [`AppError::PermissionDenied`] if the process is not running as Administrator.
// `async` is intentional: the real implementation in P1-T02 will await disk I/O.
// The stub body has no awaits yet, hence the allow.
#[allow(clippy::unused_async)]
pub async fn list_disks() -> Result<Vec<DiskInfo>, AppError> {
    // TODO(P1-T02): implement via DeviceIoControl
    Ok(vec![])
}
