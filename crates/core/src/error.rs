use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Disk not found: {0}")]
    DiskNotFound(String),
    #[error("Unsupported filesystem: {0}")]
    UnsupportedFilesystem(String),
    #[error("Scan cancelled")]
    Cancelled,
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Plain-English, actionable text for display in the UI (P5-T03).
    ///
    /// Never includes raw OS error strings, file paths, or debug payloads — the
    /// `Display` impl (used for logs) keeps that detail; this is the only form
    /// that crosses the IPC boundary to the frontend.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            AppError::Io(_) => {
                "A disk read or write error occurred. The drive may be failing or busy."
            }
            AppError::PermissionDenied(_) => {
                "Permission denied. On macOS, grant FileResque Full Disk Access in \
                 System Settings → Privacy & Security, then reopen the app."
            }
            AppError::DiskNotFound(_) => {
                "The selected disk could not be found. It may have been disconnected."
            }
            AppError::UnsupportedFilesystem(_) => {
                "This disk uses a file system FileResque does not support."
            }
            AppError::Cancelled => "The operation was cancelled.",
            AppError::Internal(_) => "Something went wrong inside FileResque. Please try again.",
        }
        .to_string()
    }
}

impl serde::Serialize for AppError {
    /// Serialise the *friendly* message, not `Display` — raw OS errors must
    /// never reach the UI (P5-T03).
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.user_message())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn user_message_io_hides_os_error() {
        let err = AppError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "no such file (os error 2)",
        ));
        let msg = err.user_message();
        assert!(!msg.contains("os error"), "raw OS text leaked: {msg}");
        assert!(!msg.is_empty());
    }

    #[test]
    fn user_message_permission_mentions_fda() {
        let msg = AppError::PermissionDenied("/dev/rdisk0".to_string()).user_message();
        assert!(msg.contains("Full Disk Access"));
        assert!(!msg.contains("/dev/rdisk0"), "raw path leaked: {msg}");
    }

    #[test]
    fn user_message_all_variants_clean() {
        let cases = [
            AppError::Io(io::Error::other("x")),
            AppError::PermissionDenied("p".into()),
            AppError::DiskNotFound("disk9".into()),
            AppError::UnsupportedFilesystem("ext4".into()),
            AppError::Cancelled,
            AppError::Internal("lock poisoned".into()),
        ];
        for err in &cases {
            let msg = err.user_message();
            assert!(!msg.is_empty(), "{err:?} produced empty message");
            // No struct/debug noise and no carried payload leaks into UI text.
            assert!(
                !msg.contains('{'),
                "{err:?} message looks like debug output"
            );
        }
        // Variants that carry a payload must not echo it verbatim.
        assert!(!AppError::DiskNotFound("disk9".into())
            .user_message()
            .contains("disk9"));
        assert!(!AppError::UnsupportedFilesystem("ext4".into())
            .user_message()
            .contains("ext4"));
    }

    #[test]
    fn serialize_emits_friendly_string() {
        let err = AppError::Internal("scan state lock poisoned".to_string());
        let json = serde_json::to_string(&err).expect("serialise");
        // JSON string value equals the friendly message, not the Display form.
        assert_eq!(json, format!("{:?}", err.user_message()));
        assert!(
            !json.contains("lock poisoned"),
            "internal detail leaked: {json}"
        );
    }
}
