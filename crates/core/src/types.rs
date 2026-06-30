use serde::{Deserialize, Serialize};

// Filesystem and drive names are industry-standard abbreviations that must retain
// their conventional uppercase form (APFS, NTFS, SSD, etc.).
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DriveType {
    SSD,
    HDD,
    NVMe,
    USB,
    Virtual,
    Unknown,
}

// Filesystem and drive names are industry-standard abbreviations that must retain
// their conventional uppercase form (APFS, NTFS, FAT32, etc.).
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystem {
    APFS,
    HFSPlus,
    NTFS,
    FAT32,
    ExFAT,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProbabilityTier {
    High,
    Medium,
    Low,
}

/// Metadata describing a physical or logical disk visible to the OS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// OS-assigned identifier, e.g. `"disk0"` (macOS) or `"PhysicalDrive0"` (Windows).
    pub id: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub drive_type: DriveType,
    pub filesystem: FileSystem,
    pub mount_points: Vec<String>,
    pub encrypted: bool,
    pub trim_enabled: bool,
    /// Hardware serial number used for same-disk detection in preflight checks.
    pub serial: Option<String>,
}

/// A deleted file entry discovered during a filesystem scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedFileEntry {
    pub inode_id: u64,
    pub name: Option<String>,
    pub size_bytes: u64,
    /// Deletion timestamp, serialised as seconds since UNIX epoch.
    #[serde(with = "system_time_serde")]
    pub deleted_at: Option<std::time::SystemTime>,
    /// Block extents: each tuple is `(block_offset, block_count)`.
    pub extents: Vec<(u64, u64)>,
    pub filesystem: FileSystem,
}

/// Recovery probability assessment for a single deleted file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityReport {
    pub tier: ProbabilityTier,
    pub free_blocks_pct: f32,
    pub trim_active: bool,
    pub blocks_zeroed: bool,
    pub estimated_recoverable_bytes: u64,
    pub warnings: Vec<String>,
}

/// Result of destination pre-flight checks before a recovery operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResult {
    pub ok: bool,
    pub errors: Vec<PreflightError>,
}

/// Errors that can occur during pre-flight destination validation.
///
/// Internally tagged (`{ "kind": "SameDisk" }`) to match the discriminated-union
/// shape the frontend `PreflightError` type expects (see `src/lib/types.ts`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PreflightError {
    /// Source and destination reside on the same physical disk — unsafe for recovery.
    SameDisk,
    /// Destination volume has insufficient free space for the selected files.
    InsufficientSpace {
        required: u64,
        available: u64,
    },
    DestinationNotWritable,
    SourceNotReadable,
}

/// Custom serde module for `Option<std::time::SystemTime>`.
///
/// Serialises as `Option<u64>` (seconds since UNIX epoch). Times before the epoch
/// are clamped to zero. Precision of one second is sufficient for file-deletion
/// timestamps discovered via filesystem metadata.
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    // `&Option<T>` is required here by serde's `with` attribute mechanism;
    // `clippy::ref_option` is suppressed at the function level only.
    #[allow(clippy::ref_option)]
    pub fn serialize<S>(time: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let secs = time.map(|t| {
            // Pre-epoch times are clamped to zero — acceptable for deletion timestamps.
            t.duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs()
        });
        secs.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs: Option<u64> = Option::deserialize(deserializer)?;
        Ok(secs.map(|s| UNIX_EPOCH + Duration::from_secs(s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Case<I, O> {
        name: &'static str,
        input: I,
        expected: O,
    }

    #[test]
    fn disk_info_serializes() {
        let cases = vec![Case {
            name: "happy_path_full_disk_info",
            input: DiskInfo {
                id: "disk0".to_string(),
                display_name: "Macintosh HD".to_string(),
                size_bytes: 500_000_000_000,
                drive_type: DriveType::SSD,
                filesystem: FileSystem::APFS,
                mount_points: vec!["/".to_string()],
                encrypted: false,
                trim_enabled: true,
                serial: Some("ABC123".to_string()),
            },
            expected: true, // JSON must contain the disk id
        }];

        for case in cases {
            // JUSTIFIED: test-only; serde_json serialisation of valid DiskInfo is infallible
            let json =
                serde_json::to_string(&case.input).expect("DiskInfo serialisation must not fail");
            let contains_id = json.contains("disk0");
            assert_eq!(contains_id, case.expected, "FAILED case: {}", case.name);
        }
    }

    #[test]
    fn deleted_file_entry_round_trips_system_time() {
        use std::time::{Duration, UNIX_EPOCH};

        let cases = vec![
            Case {
                name: "happy_path_with_timestamp",
                input: DeletedFileEntry {
                    inode_id: 42,
                    name: Some("lost_file.txt".to_string()),
                    size_bytes: 1024,
                    deleted_at: Some(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
                    extents: vec![(0, 2), (4, 1)],
                    filesystem: FileSystem::APFS,
                },
                expected: true,
            },
            Case {
                name: "branch_none_timestamp",
                input: DeletedFileEntry {
                    inode_id: 99,
                    name: None,
                    size_bytes: 0,
                    deleted_at: None,
                    extents: vec![],
                    filesystem: FileSystem::NTFS,
                },
                expected: true,
            },
        ];

        for case in cases {
            // JUSTIFIED: test-only; serde_json operations on valid structs are infallible
            let json = serde_json::to_string(&case.input)
                .expect("DeletedFileEntry serialisation must not fail");
            let round_tripped: DeletedFileEntry = serde_json::from_str(&json)
                .expect("DeletedFileEntry deserialisation must not fail");
            let ids_match = round_tripped.inode_id == case.input.inode_id;
            assert_eq!(ids_match, case.expected, "FAILED case: {}", case.name);
        }
    }
}
