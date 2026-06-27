use fileresque_core::error::AppError;

/// Check whether the process has Full Disk Access (FDA) by attempting to open
/// `/dev/disk0` with read-only access. FDA is required to read `/dev/rdiskN`
/// for raw sector scanning (Phase 2).
///
/// Returns `Ok(true)` if FDA is granted, `Ok(false)` if denied.
///
/// # Errors
///
/// This function currently does not return `Err` — all `open(2)` error codes
/// are classified into the `bool` result. The `Result` wrapper exists so the
/// Tauri command layer can use `?` consistently and the signature can be
/// extended (e.g. to log audit events in Phase 4) without a breaking change.
pub fn has_full_disk_access() -> Result<bool, AppError> {
    Ok(check_dev_disk_readable())
}

/// Open `/dev/disk0` read-only and classify the outcome as granted / denied.
///
/// Returns `true` if FDA is granted, `false` if denied.
/// Called from [`has_full_disk_access`] and dispatched via `spawn_blocking`
/// in the Tauri command layer so the async executor is never stalled.
fn check_dev_disk_readable() -> bool {
    // We use std::fs::File::open which internally calls open(2) with O_RDONLY.
    // EACCES / EPERM → FDA not granted.
    // EBUSY → disk is locked by the system but we have access; FDA IS granted.
    // Other errors → assume access; disk enumeration will surface the real error.
    match std::fs::File::open("/dev/disk0") {
        Ok(_) => true,
        Err(e) => classify_open_error(&e),
    }
}

/// Classify a `/dev/disk0` open error as "access denied" (`false`) or
/// "access assumed" (`true`).
///
/// Extracted as `pub(crate)` so unit tests can exercise the classification
/// logic without needing a real `/dev/disk0` device node.
pub(crate) fn classify_open_error(err: &std::io::Error) -> bool {
    match err.kind() {
        // EACCES (13): explicit permission denial — FDA not granted.
        // EPERM (1): maps to PermissionDenied on Rust ≥ 1.75; handled by
        //   the raw_os_error fallback below for older toolchain compatibility.
        std::io::ErrorKind::PermissionDenied => false,
        // EBUSY (16): disk is locked by the system but we are allowed to open
        // it — FDA IS granted. Disk enumeration will still succeed.
        std::io::ErrorKind::ResourceBusy => true,
        _ => {
            // EPERM (os error 1) also signals FDA denial on macOS. On Rust
            // versions prior to 1.75, EPERM maps to ErrorKind::Other rather
            // than PermissionDenied, so we check the raw OS error code here
            // as a defensive fallback.
            if err.raw_os_error() == Some(1) {
                return false;
            }
            // All other unexpected errors (e.g. ENOENT if run outside macOS):
            // assume access is granted. The real disk enumeration call will
            // surface a more descriptive error if access is actually blocked.
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Case {
        name: &'static str,
        os_error: i32,
        expected: bool,
    }

    #[test]
    fn test_classify_open_error() {
        let cases = vec![
            Case {
                name: "permission_denied_returns_false",
                // EACCES — maps to ErrorKind::PermissionDenied
                os_error: 13,
                expected: false,
            },
            Case {
                name: "eperm_returns_false",
                // EPERM — maps to PermissionDenied on Rust ≥1.75, or hits raw fallback
                os_error: 1,
                expected: false,
            },
            Case {
                name: "ebusy_returns_true",
                // EBUSY — disk locked by system but accessible; FDA IS granted
                os_error: 16,
                expected: true,
            },
            Case {
                name: "other_error_returns_true",
                // ENOENT — unexpected; assume access granted
                os_error: 2,
                expected: true,
            },
        ];

        for case in cases {
            let err = std::io::Error::from_raw_os_error(case.os_error);
            let actual = classify_open_error(&err);
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }
}
