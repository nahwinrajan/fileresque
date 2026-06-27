use fileresque_core::{error::AppError, types::DeletedFileEntry};

/// Recover a single deleted file to the specified destination path.
///
/// Returns the SHA-256 hex digest of the recovered file on success.
///
/// This is a stub implementation. Full recovery logic is in P4-T02.
///
/// # Errors
///
/// Returns [`AppError::Io`] on read/write failure or [`AppError::Cancelled`]
/// if the operation is cancelled by the user.
// `async` is intentional: the real implementation in P4-T02 will await disk reads.
// The stub body has no awaits yet, hence the allow.
#[allow(clippy::unused_async)]
pub async fn recover_file(
    _entry: &DeletedFileEntry,
    _source_disk_id: &str,
    _dest_path: &std::path::Path,
) -> Result<String, AppError> {
    // TODO(P4-T02): implement recovery engine
    Err(AppError::Internal("not yet implemented".to_string()))
}
