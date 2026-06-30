//! Filesystem facts about a *destination* path, used by recovery pre-flight
//! (P4-T01). Three concerns:
//!
//! - [`dest_info`] — free space + the backing BSD device of a path (platform).
//! - [`is_writable`] — a real probe write (pure std, cross-platform).
//! - [`same_disk`] / [`normalize_disk_id`] — compare a destination device to the
//!   recovery source's disk id (pure, fully unit-tested).
//!
//! The platform call is isolated so the comparison logic stays testable without
//! mounting anything.

use std::path::Path;

use fileresque_core::error::AppError;

/// Free space and backing device for a destination path.
#[derive(Debug, Clone)]
pub struct DestInfo {
    /// Bytes available to a non-privileged writer on the destination volume.
    pub available_bytes: u64,
    /// Backing device, normalized to its whole-disk id (e.g. `"disk3"` on
    /// macOS, `"C:"` on Windows). Empty when it could not be determined.
    pub device: String,
}

/// Probe whether `dir` is writable by creating and removing a unique temp file.
///
/// This is the spec's "attempt temp file write" check — a real write catches
/// read-only mounts, permission gaps, and full volumes that a metadata-only
/// check would miss. The probe file is always removed.
#[must_use]
pub fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(
        ".fileresque_write_probe_{}_{}",
        std::process::id(),
        nanos()
    ));
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            // Best-effort cleanup; a leftover empty probe file is harmless.
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Monotonic-ish suffix to keep concurrent probe filenames distinct.
fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

/// True when a destination `device` and a source `disk_id` resolve to the same
/// whole disk. Both are normalized first, so `/dev/disk3s1s1` and `disk3` match.
#[must_use]
pub fn same_disk(device: &str, source_disk_id: &str) -> bool {
    let a = normalize_disk_id(device);
    let b = normalize_disk_id(source_disk_id);
    !a.is_empty() && a == b
}

/// Reduce any device spelling to its whole-disk id.
///
/// macOS: strips a leading `/dev/` and an `r` (raw) prefix, then keeps
/// `disk<N>` and drops the partition/slice suffix — `/dev/rdisk3s1s2` →
/// `disk3`. Other strings (e.g. a Windows `"C:"`) are returned trimmed and
/// lower-cased unchanged so the comparison is still exact.
#[must_use]
pub fn normalize_disk_id(raw: &str) -> String {
    let s = raw.trim().trim_start_matches("/dev/");
    let s = s
        .strip_prefix('r')
        .filter(|r| r.starts_with("disk"))
        .unwrap_or(s);

    if let Some(rest) = s.strip_prefix("disk") {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            return format!("disk{digits}");
        }
    }
    s.to_ascii_lowercase()
}

/// Gather free space and backing device for `path`.
///
/// # Errors
///
/// Returns [`AppError`] when the platform query fails (e.g. the path does not
/// exist) or the platform is unsupported.
#[cfg(target_os = "macos")]
pub fn dest_info(path: &Path) -> Result<DestInfo, AppError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| AppError::Internal(format!("path contains NUL byte: {e}")))?;

    // SAFETY: `statfs` is zero-initialised before the call and fully populated by
    // a successful `statfs(2)`. `c_path` is a valid NUL-terminated C string that
    // outlives the call. We check the return code and only read the struct on 0.
    let stat = unsafe {
        let mut stat: libc::statfs = std::mem::zeroed();
        if libc::statfs(c_path.as_ptr(), &raw mut stat) != 0 {
            return Err(AppError::Io(std::io::Error::last_os_error()));
        }
        stat
    };

    let available_bytes = stat.f_bavail.saturating_mul(u64::from(stat.f_bsize));
    let device = device_from_mntfromname(&stat.f_mntfromname);

    Ok(DestInfo {
        available_bytes,
        device,
    })
}

/// Decode the C `f_mntfromname` field (`/dev/disk3s1s1`) into a normalized
/// whole-disk id.
#[cfg(target_os = "macos")]
fn device_from_mntfromname(raw: &[libc::c_char]) -> String {
    let bytes: Vec<u8> = raw
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c.cast_unsigned())
        .collect();
    normalize_disk_id(&String::from_utf8_lossy(&bytes))
}

#[cfg(target_os = "windows")]
pub fn dest_info(path: &Path) -> Result<DestInfo, AppError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut free_to_caller: u64 = 0;

    // SAFETY: `wide` is a valid NUL-terminated UTF-16 path that outlives the
    // call; `free_to_caller` is a live local we pass by mutable pointer. The
    // other two OUT params are optional per the Win32 contract and passed null.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_to_caller,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(AppError::Io(std::io::Error::last_os_error()));
    }

    // Drive letter as the device id (e.g. "C:"); best-effort same-disk proxy on
    // Windows where physical-disk mapping needs a heavier IOCTL.
    let device = path
        .components()
        .next()
        .map(|c| {
            c.as_os_str()
                .to_string_lossy()
                .trim_end_matches('\\')
                .to_string()
        })
        .unwrap_or_default();

    Ok(DestInfo {
        available_bytes: free_to_caller,
        device,
    })
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn dest_info(_path: &Path) -> Result<DestInfo, AppError> {
    Err(AppError::UnsupportedFilesystem(
        "destination free-space query unsupported on this platform".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_dev_raw_and_slice() {
        let cases = vec![
            ("/dev/disk3s1s1", "disk3"),
            ("/dev/rdisk3", "disk3"),
            ("disk0", "disk0"),
            ("/dev/disk12s4", "disk12"),
            ("rdisk5s2", "disk5"),
        ];
        for (input, expected) in cases {
            assert_eq!(normalize_disk_id(input), expected, "input: {input}");
        }
    }

    #[test]
    fn normalize_passes_through_non_disk_strings() {
        assert_eq!(normalize_disk_id("C:"), "c:");
        assert_eq!(normalize_disk_id(""), "");
    }

    #[test]
    fn same_disk_matches_across_spellings() {
        assert!(same_disk("/dev/disk3s1s1", "disk3"), "slice vs whole disk");
        assert!(same_disk("/dev/rdisk0", "disk0"), "raw vs buffered");
        assert!(!same_disk("/dev/disk4s1", "disk3"), "different disks");
        assert!(!same_disk("", "disk3"), "empty device never matches");
    }

    #[test]
    fn is_writable_true_for_temp_dir() {
        assert!(
            is_writable(&std::env::temp_dir()),
            "temp dir must be writable"
        );
    }

    #[test]
    fn is_writable_false_for_nonexistent_dir() {
        let bogus = std::env::temp_dir().join("fileresque_nonexistent_xyz_123/sub");
        assert!(!is_writable(&bogus), "missing dir is not writable");
    }
}
