// Compiled on Windows production builds, on all platforms during tests,
// and when building fuzz targets (cargo-fuzz sets --cfg fuzzing).
#![cfg(any(target_os = "windows", test))]

/// NTFS MFT record magic: "FILE" encoded as a little-endian u32.
/// Bytes on disk: 0x46 0x49 0x4C 0x45 ('F','I','L','E').
pub const MFT_RECORD_MAGIC: u32 = 0x454C_4946;

/// Standard NTFS sector size in bytes.
pub const NTFS_SECTOR_SIZE: u16 = 512;

/// Standard NTFS MFT record size in bytes (used as buffer minimum).
pub const MFT_RECORD_SIZE: usize = 1024;

/// MFT record flag: record is in use (not deleted).
pub const MFT_RECORD_IN_USE: u16 = 0x0001;

/// MFT record flag: record describes a directory.
pub const MFT_RECORD_IS_DIRECTORY: u16 = 0x0002;

/// NTFS attribute type: $STANDARD_INFORMATION.
pub const ATTR_STANDARD_INFORMATION: u32 = 0x10;

/// NTFS attribute type: $FILE_NAME.
pub const ATTR_FILE_NAME: u32 = 0x30;

/// NTFS attribute type: $DATA.
pub const ATTR_DATA: u32 = 0x80;

/// NTFS attribute type: end-of-attribute-list sentinel.
pub const ATTR_END_MARKER: u32 = 0xFFFF_FFFF;

/// $FILE_NAME namespace: POSIX (case-sensitive).
pub const FILE_NAME_POSIX: u8 = 0;

/// $FILE_NAME namespace: Win32 (case-insensitive, user-visible).
pub const FILE_NAME_WIN32: u8 = 1;

/// $FILE_NAME namespace: DOS (8.3 short name).
pub const FILE_NAME_DOS: u8 = 2;

/// $FILE_NAME namespace: Win32 and DOS share one record.
pub const FILE_NAME_WIN32_AND_DOS: u8 = 3;

/// Parsed fields from the NTFS Volume Boot Record (VBR, first sector).
#[derive(Debug, Clone)]
pub struct NtfsVbr {
    /// Bytes per logical sector (almost always 512).
    pub bytes_per_sector: u16,
    /// Logical sectors per cluster.
    pub sectors_per_cluster: u8,
    /// Logical Cluster Number (LCN) of the Master File Table ($MFT).
    pub mft_cluster: u64,
    /// LCN of the MFT mirror ($MFTMirr).
    pub mft_mirror_cluster: u64,
    /// Clusters per MFT record. If negative: record size = `2^|value|` bytes.
    pub clusters_per_mft_record: i8,
    /// Volume serial number (from VBR offset 80).
    pub volume_serial: u64,
}

/// Parsed MFT record header fields.
#[derive(Debug)]
pub struct MftRecord {
    /// Index of this record in the MFT (0-based).
    pub record_number: u64,
    /// Sequence number incremented every time the record is reused.
    pub sequence_number: u16,
    /// Record flags (see `MFT_RECORD_*` constants).
    pub flags: u16,
    /// Byte offset from the start of the record to the first attribute.
    pub attrs_offset: u16,
    /// Total bytes allocated for the record.
    pub bytes_allocated: u32,
    /// Bytes of the record that are actually in use.
    pub bytes_in_use: u32,
}

/// Parsed NTFS attribute header (common to resident and non-resident forms).
#[derive(Debug)]
pub struct AttrHeader {
    /// Attribute type code (see `ATTR_*` constants).
    pub attr_type: u32,
    /// Total length of the attribute (header + value), in bytes.
    pub length: u32,
    /// `true` if the attribute value is stored outside the MFT record (run list).
    pub non_resident: bool,
    /// Length of the attribute name in characters (0 for unnamed attributes).
    pub name_length: u8,
    /// Byte offset from the start of the attribute to the value (resident)
    /// or to the data-run list (non-resident).
    pub offset: u16,
}

/// Parsed $FILE_NAME attribute value.
#[derive(Debug)]
pub struct FileNameAttr {
    /// MFT file reference of the parent directory.
    pub parent_ref: u64,
    /// File size in bytes.
    pub file_size: u64,
    /// Space allocated on disk for the file.
    pub allocated_size: u64,
    /// Windows FILETIME: creation time (100 ns intervals since 1601-01-01 UTC).
    pub creation_time: u64,
    /// Windows FILETIME: last modification time.
    pub modification_time: u64,
    /// Namespace code (see `FILE_NAME_*` constants).
    pub namespace: u8,
    /// File name decoded from UTF-16LE.
    pub name: String,
}

/// A deleted file candidate assembled from one MFT record.
#[derive(Debug)]
pub struct NtfsDeletedFile {
    /// MFT record number.
    pub record_number: u64,
    /// File name, if a $FILE_NAME attribute was found.
    pub name: Option<String>,
    /// File size in bytes from the $FILE_NAME attribute.
    pub file_size: u64,
    /// Last modification time as a Windows FILETIME, if available.
    pub modification_time: Option<u64>,
    /// Data runs: `(lcn_offset, run_length_in_clusters)` pairs.
    /// `lcn_offset` is relative (signed delta from the previous run's LCN).
    pub data_runs: Vec<(i64, u64)>,
}
