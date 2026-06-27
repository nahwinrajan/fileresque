use fileresque_core::{error::AppError, types::DiskInfo};

/// Enumerate physical disks visible to the OS on macOS.
///
/// This is a stub implementation. Full implementation is in P1-T01 via `diskutil`
/// and `IOKit` `IOServiceMatching("IOBlockStorageDriver")`.
///
/// # Errors
///
/// Returns [`AppError::PermissionDenied`] if Full Disk Access has not been granted.
// `async` is intentional: the real implementation in P1-T01 will await disk I/O.
// The stub body has no awaits yet, hence the allow.
#[allow(clippy::unused_async)]
pub async fn list_disks() -> Result<Vec<DiskInfo>, AppError> {
    // TODO(P1-T01): implement via diskutil + IOKit
    Ok(vec![])
}
