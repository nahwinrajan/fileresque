// Tauri commands always have their return value consumed by the IPC mechanism.
#![allow(clippy::must_use_candidate)]

use fileresque_core::{
    error::AppError,
    types::{DeletedFileEntry, DiskInfo, ProbabilityReport},
};
use fileresque_recovery::probability::assess_probability;

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
    fn raw_device_path(disk_id: &str) -> Result<String, AppError> {
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
