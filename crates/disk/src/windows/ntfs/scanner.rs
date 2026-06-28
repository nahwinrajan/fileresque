// Compiled on Windows production builds and on all platforms during tests.
// Parsing logic is pure byte manipulation with no OS-specific code; only
// `scan_mft` (further gated with `cfg(target_os = "windows")`) performs I/O.
#![cfg(any(target_os = "windows", test))]

use byteorder::{ReadBytesExt, LE};
use fileresque_core::error::AppError;
use std::io::Cursor;

use super::structs::{
    ATTR_END_MARKER, ATTR_FILE_NAME, FILE_NAME_DOS, FILE_NAME_POSIX, FILE_NAME_WIN32,
    FILE_NAME_WIN32_AND_DOS, MFT_RECORD_IN_USE, MFT_RECORD_IS_DIRECTORY, MFT_RECORD_MAGIC,
};

pub use super::structs::{AttrHeader, FileNameAttr, MftRecord, NtfsVbr};

// ─── VBR ─────────────────────────────────────────────────────────────────────

/// Parse the NTFS Volume Boot Record from a 512-byte buffer.
///
/// Validates the OEM ID ("NTFS    ") and extracts BPB fields needed for MFT
/// location and record-size computation.
///
/// # Errors
///
/// Returns [`AppError::Internal`] when:
/// - `buf` is shorter than 512 bytes
/// - OEM ID at offset 3–10 is not `b"NTFS    "`
/// - Any field cannot be read from the cursor
pub fn parse_vbr(buf: &[u8]) -> Result<NtfsVbr, AppError> {
    if buf.len() < 512 {
        return Err(AppError::Internal("VBR buffer too small".to_string()));
    }

    let oem_id = &buf[3..11];
    if oem_id != b"NTFS    " {
        let display = std::str::from_utf8(oem_id).unwrap_or("<invalid>");
        return Err(AppError::Internal(format!(
            "Not an NTFS volume: OEM ID = {display:?}"
        )));
    }

    let mut cur = Cursor::new(buf);

    cur.set_position(11);
    let bytes_per_sector = cur
        .read_u16::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let sectors_per_cluster = cur
        .read_u8()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    cur.set_position(56);
    let mft_cluster = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let mft_mirror_cluster = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let clusters_per_mft_record = cur
        .read_i8()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    cur.set_position(80);
    let volume_serial = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(NtfsVbr {
        bytes_per_sector,
        sectors_per_cluster,
        mft_cluster,
        mft_mirror_cluster,
        clusters_per_mft_record,
        volume_serial,
    })
}

/// Compute the MFT record size in bytes from the VBR.
///
/// When `clusters_per_mft_record` is negative, the record size is
/// `2^|clusters_per_mft_record|` bytes (typically 1 KB = `2^10`).
/// When positive, the record size is `sectors_per_cluster × bytes_per_sector × value`.
#[must_use]
pub fn mft_record_size(vbr: &NtfsVbr) -> usize {
    let cpmr = vbr.clusters_per_mft_record;
    if cpmr < 0 {
        // Negative value encodes a power of 2: record_size = 2^|cpmr|
        1usize << cpmr.unsigned_abs()
    } else {
        // Positive: record spans `cpmr` clusters
        usize::from(vbr.sectors_per_cluster)
            * usize::from(vbr.bytes_per_sector)
            * usize::from(cpmr.unsigned_abs())
    }
}

// ─── MFT record header ───────────────────────────────────────────────────────

/// Parse the MFT record header from the start of `buf`.
///
/// NTFS MFT record header layout (offsets in bytes):
/// ```text
///  0  4  magic ("FILE")
///  4  2  update_sequence_array_offset
///  6  2  update_sequence_array_count
///  8  8  $LogFile LSN
/// 16  2  sequence_number
/// 18  2  link_count
/// 20  2  attrs_offset
/// 22  2  flags
/// 24  4  bytes_in_use
/// 28  4  bytes_allocated
/// ```
///
/// # Errors
///
/// Returns [`AppError::Internal`] when:
/// - `buf` is shorter than 48 bytes
/// - The 4-byte magic does not equal `MFT_RECORD_MAGIC`
pub fn parse_mft_record(buf: &[u8], record_number: u64) -> Result<MftRecord, AppError> {
    if buf.len() < 48 {
        return Err(AppError::Internal(
            "MFT record buffer too small".to_string(),
        ));
    }

    let mut cur = Cursor::new(buf);

    let magic = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if magic != MFT_RECORD_MAGIC {
        return Err(AppError::Internal(format!(
            "MFT record magic mismatch: {magic:#010x}"
        )));
    }

    // Jump to offset 16 (skip update_sequence fields and $LogFile LSN)
    cur.set_position(16);
    let sequence_number = cur
        .read_u16::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Advance past link_count (2 bytes) to reach attrs_offset at offset 20
    let _link_count = cur
        .read_u16::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let attrs_offset = cur
        .read_u16::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let flags = cur
        .read_u16::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let bytes_in_use = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let bytes_allocated = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(MftRecord {
        record_number,
        sequence_number,
        flags,
        attrs_offset,
        bytes_allocated,
        bytes_in_use,
    })
}

/// Return `true` when the MFT record describes a deleted (free) file.
#[must_use]
pub fn is_deleted(record: &MftRecord) -> bool {
    record.flags & MFT_RECORD_IN_USE == 0
}

// ─── Attribute parsing ───────────────────────────────────────────────────────

/// Parse an NTFS attribute header at `offset` within `buf`.
///
/// Returns `Ok(None)` when:
/// - There are fewer than 4 bytes remaining at `offset` (end of attribute area)
/// - The attribute type equals `ATTR_END_MARKER` (0xFFFF_FFFF)
///
/// NTFS resident attribute header layout (offsets relative to attribute start):
/// ```text
/// 0  4  attr_type
/// 4  4  length (total, including header and value)
/// 8  1  non_resident flag
/// 9  1  name_length
/// 10 2  name_offset / value_offset
/// ```
///
/// # Errors
///
/// Returns [`AppError::Internal`] when:
/// - `length` is zero
/// - The attribute would extend beyond `buf`
/// - A field cannot be read
pub fn parse_attr_header(buf: &[u8], offset: usize) -> Result<Option<AttrHeader>, AppError> {
    if offset.saturating_add(4) > buf.len() {
        return Ok(None);
    }

    let mut cur = Cursor::new(&buf[offset..]);

    let attr_type = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if attr_type == ATTR_END_MARKER {
        return Ok(None);
    }

    let length = cur
        .read_u32::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let total_end = offset
        .checked_add(length as usize)
        .ok_or_else(|| AppError::Internal("Attribute length overflows usize".to_string()))?;

    if length == 0 || total_end > buf.len() {
        return Err(AppError::Internal(
            "Attribute length out of bounds".to_string(),
        ));
    }

    let non_resident = cur
        .read_u8()
        .map_err(|e| AppError::Internal(e.to_string()))?
        != 0;
    let name_length = cur
        .read_u8()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let value_offset = cur
        .read_u16::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Some(AttrHeader {
        attr_type,
        length,
        non_resident,
        name_length,
        offset: value_offset,
    }))
}

/// Parse a $FILE_NAME attribute value from a byte slice.
///
/// `data` must start at the attribute value (i.e., after the attribute header).
///
/// $FILE_NAME value layout (offsets in bytes):
/// ```text
///  0  8  parent_mft_reference
///  8  8  creation_time (FILETIME)
/// 16  8  modification_time (FILETIME)
/// 24  8  mft_modification_time (skipped)
/// 32  8  access_time (skipped)
/// 40  8  allocated_size
/// 48  8  file_size
/// 56  4  flags (skipped)
/// 60  4  reparse_tag (skipped)
/// 64  1  file_name_length (in UTF-16 code units)
/// 65  1  namespace
/// 66  …  file_name (UTF-16LE)
/// ```
///
/// # Errors
///
/// Returns [`AppError::Internal`] when:
/// - `data` is shorter than 66 bytes
/// - The file-name code units extend beyond `data`
/// - The UTF-16 sequence is invalid
pub fn parse_file_name_attr(data: &[u8]) -> Result<FileNameAttr, AppError> {
    if data.len() < 66 {
        return Err(AppError::Internal(
            "$FILE_NAME attribute too small".to_string(),
        ));
    }

    let mut cur = Cursor::new(data);

    let parent_ref = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let creation_time = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let modification_time = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Skip mft_modification_time (8 bytes) and access_time (8 bytes)
    cur.set_position(cur.position() + 16);

    let allocated_size = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let file_size = cur
        .read_u64::<LE>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Skip flags (4 bytes) and reparse_tag (4 bytes)
    cur.set_position(cur.position() + 8);

    let name_len = cur
        .read_u8()
        .map_err(|e| AppError::Internal(e.to_string()))? as usize;
    let namespace = cur
        .read_u8()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Name starts at byte 66 (cursor is now at position 66)
    let name_start = cur.position() as usize;
    let name_byte_len = name_len
        .checked_mul(2)
        .ok_or_else(|| AppError::Internal("$FILE_NAME name length overflow".to_string()))?;
    let name_end = name_start
        .checked_add(name_byte_len)
        .ok_or_else(|| AppError::Internal("$FILE_NAME name end overflow".to_string()))?;

    if name_end > data.len() {
        return Err(AppError::Internal(
            "$FILE_NAME name extends beyond attribute".to_string(),
        ));
    }

    let name_utf16: Vec<u16> = data[name_start..name_end]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();

    let name = String::from_utf16(&name_utf16)
        .map_err(|_| AppError::Internal("$FILE_NAME contains invalid UTF-16".to_string()))?;

    Ok(FileNameAttr {
        parent_ref,
        file_size,
        allocated_size,
        creation_time,
        modification_time,
        namespace,
        name,
    })
}

// ─── Time conversion ─────────────────────────────────────────────────────────

/// Convert a Windows FILETIME (100 ns intervals since 1601-01-01 UTC) to a
/// [`std::time::SystemTime`].
///
/// Returns `None` when `filetime` predates the Unix epoch (1970-01-01), since
/// [`std::time::SystemTime`] cannot represent times before [`UNIX_EPOCH`].
#[must_use]
pub fn filetime_to_system_time(filetime: u64) -> Option<std::time::SystemTime> {
    // Difference between Windows epoch (1601-01-01) and Unix epoch (1970-01-01)
    // expressed in 100-ns intervals: 116444736000000000
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    let unix_100ns = filetime.checked_sub(EPOCH_DIFF)?;
    let secs = unix_100ns / 10_000_000;
    let nanos = (unix_100ns % 10_000_000) * 100;
    std::time::UNIX_EPOCH.checked_add(std::time::Duration::new(secs, nanos as u32))
}

// ─── Deleted-entry extraction helpers ────────────────────────────────────────

/// Return a numeric priority for a $FILE_NAME namespace, used to pick the
/// most user-visible name when multiple $FILE_NAME attributes exist.
///
/// Priority order (highest first): Win32AndDos → Win32 → POSIX → DOS → unknown
fn namespace_priority(ns: u8) -> u8 {
    match ns {
        FILE_NAME_WIN32_AND_DOS => 3,
        FILE_NAME_WIN32 => 2,
        FILE_NAME_POSIX => 1,
        FILE_NAME_DOS => 0,
        _ => 0,
    }
}

/// Walk the attributes of an MFT record and return the best $FILE_NAME attribute.
///
/// "Best" means the namespace with the highest priority (Win32AndDos > Win32 >
/// POSIX > DOS), because Win32 names are the user-visible filenames.
/// Non-resident $FILE_NAME attributes are skipped (NTFS never makes $FILE_NAME
/// non-resident in practice, but we guard defensively).
pub(crate) fn find_best_file_name(buf: &[u8], record: &MftRecord) -> Option<FileNameAttr> {
    let mut offset = usize::from(record.attrs_offset);
    let mut best: Option<FileNameAttr> = None;

    loop {
        let header = match parse_attr_header(buf, offset) {
            Ok(Some(h)) => h,
            _ => break,
        };

        if header.attr_type == ATTR_FILE_NAME && !header.non_resident {
            let value_start = offset.saturating_add(usize::from(header.offset));
            let value_end = offset.saturating_add(header.length as usize);

            if value_start < value_end && value_end <= buf.len() {
                if let Ok(attr) = parse_file_name_attr(&buf[value_start..value_end]) {
                    let is_better = best
                        .as_ref()
                        .map_or(true, |b| namespace_priority(attr.namespace) > namespace_priority(b.namespace));
                    if is_better {
                        best = Some(attr);
                    }
                }
            }
        }

        // Advance to the next attribute; guard against zero-length loops
        let next = offset.saturating_add(header.length as usize);
        if next <= offset || next >= buf.len() {
            break;
        }
        offset = next;
    }

    best
}

/// Build a [`fileresque_core::types::DeletedFileEntry`] from an MFT record's
/// $FILE_NAME attribute.
///
/// Returns `None` when no usable $FILE_NAME attribute is found.
/// `extents` is left empty for this MVP — NTFS data-run parsing is deferred.
// Called by scan_mft (Windows production); compiled in test mode on macOS.
#[allow(dead_code)]
pub(crate) fn extract_deleted_entry(
    buf: &[u8],
    record: &MftRecord,
    _vbr: &NtfsVbr,
) -> Option<fileresque_core::types::DeletedFileEntry> {
    let file_name = find_best_file_name(buf, record)?;
    let deleted_at = filetime_to_system_time(file_name.modification_time);

    Some(fileresque_core::types::DeletedFileEntry {
        inode_id: record.record_number,
        name: Some(file_name.name),
        size_bytes: file_name.file_size,
        deleted_at,
        // MVP: NTFS data-run (extent) parsing is deferred to a future task.
        extents: vec![],
        filesystem: fileresque_core::types::FileSystem::NTFS,
    })
}

// ─── MFT scan loop (Windows only) ────────────────────────────────────────────

/// Scan the MFT of an NTFS volume and send deleted-file entries to `tx`.
///
/// Opens `device_path` (e.g. `r"\\.\C:"`) with read-only access, reads the VBR
/// to locate the MFT, then iterates every MFT record sequentially, emitting a
/// [`DeletedFileEntry`] for each deleted non-directory file that has a
/// $FILE_NAME attribute.
///
/// Records with an unrecognised magic value (bad sectors, uninitialised records)
/// are silently skipped; scanning continues until EOF.
///
/// Send errors are silently ignored — they indicate that the receiver (UI layer)
/// has been dropped, i.e. the scan was cancelled.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] when the process lacks Administrator privileges.
/// - [`AppError::Io`] on unexpected I/O failures.
/// - [`AppError::Internal`] when VBR parsing fails (not an NTFS volume).
#[cfg(target_os = "windows")]
pub(crate) fn scan_mft(
    device_path: &str,
    tx: &tokio::sync::mpsc::Sender<fileresque_core::types::DeletedFileEntry>,
) -> Result<(), AppError> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    let mut f = File::open(device_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            AppError::PermissionDenied(format!("Cannot open {device_path}: {e}"))
        } else {
            AppError::Io(e)
        }
    })?;

    let mut vbr_buf = vec![0u8; 512];
    f.read_exact(&mut vbr_buf).map_err(AppError::Io)?;
    let vbr = parse_vbr(&vbr_buf)?;

    let cluster_size = u64::from(vbr.bytes_per_sector) * u64::from(vbr.sectors_per_cluster);
    let mft_offset = vbr.mft_cluster * cluster_size;
    let rec_size = mft_record_size(&vbr);

    f.seek(SeekFrom::Start(mft_offset)).map_err(AppError::Io)?;

    let mut record_number = 0u64;
    let mut record_buf = vec![0u8; rec_size];

    loop {
        match f.read_exact(&mut record_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(AppError::Io(e)),
        }

        let record = match parse_mft_record(&record_buf, record_number) {
            Ok(r) => r,
            Err(_) => {
                record_number = record_number.saturating_add(1);
                continue;
            }
        };

        if is_deleted(&record) && record.flags & MFT_RECORD_IS_DIRECTORY == 0 {
            if let Some(entry) = extract_deleted_entry(&record_buf, &record, &vbr) {
                // Ignore send errors — receiver dropped means scan was cancelled
                let _ = tx.blocking_send(entry);
            }
        }

        record_number = record_number.saturating_add(1);
    }

    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    // Helper: build a minimal valid 512-byte VBR buffer
    fn make_vbr_buf(
        bytes_per_sector: u16,
        sectors_per_cluster: u8,
        mft_cluster: u64,
        mft_mirror_cluster: u64,
        clusters_per_mft_record: i8,
        volume_serial: u64,
    ) -> Vec<u8> {
        let mut buf = vec![0u8; 512];
        buf[3..11].copy_from_slice(b"NTFS    ");
        buf[11..13].copy_from_slice(&bytes_per_sector.to_le_bytes());
        buf[13] = sectors_per_cluster;
        buf[56..64].copy_from_slice(&mft_cluster.to_le_bytes());
        buf[64..72].copy_from_slice(&mft_mirror_cluster.to_le_bytes());
        buf[72] = clusters_per_mft_record as u8;
        buf[80..88].copy_from_slice(&volume_serial.to_le_bytes());
        buf
    }

    // Helper: build a minimal valid 1024-byte MFT record buffer
    fn make_mft_record_buf(flags: u16, attrs_offset: u16) -> Vec<u8> {
        let mut buf = vec![0u8; 1024];
        buf[0..4].copy_from_slice(&MFT_RECORD_MAGIC.to_le_bytes());
        buf[16..18].copy_from_slice(&1u16.to_le_bytes()); // sequence_number
        // link_count at 18: leave as 0
        buf[20..22].copy_from_slice(&attrs_offset.to_le_bytes());
        buf[22..24].copy_from_slice(&flags.to_le_bytes());
        buf[24..28].copy_from_slice(&400u32.to_le_bytes()); // bytes_in_use
        buf[28..32].copy_from_slice(&1024u32.to_le_bytes()); // bytes_allocated
        buf
    }

    // ── parse_vbr ─────────────────────────────────────────────────────────────

    #[derive(Debug)]
    struct ParseVbrCase {
        name: &'static str,
        buf: Vec<u8>,
        expect_ok: bool,
        expected_bps: Option<u16>,
        expected_spc: Option<u8>,
        expected_mft: Option<u64>,
    }

    #[test]
    fn test_parse_vbr() {
        let cases = vec![
            ParseVbrCase {
                name: "happy_path_valid_ntfs_vbr",
                buf: make_vbr_buf(512, 8, 2, 0, -10, 0xDEAD_BEEF),
                expect_ok: true,
                expected_bps: Some(512),
                expected_spc: Some(8),
                expected_mft: Some(2),
            },
            ParseVbrCase {
                name: "branch_wrong_oem_id",
                buf: {
                    let mut b = make_vbr_buf(512, 8, 0, 0, 0, 0);
                    b[3..11].copy_from_slice(b"FAT32   ");
                    b
                },
                expect_ok: false,
                expected_bps: None,
                expected_spc: None,
                expected_mft: None,
            },
            ParseVbrCase {
                name: "branch_buffer_too_small",
                buf: vec![0u8; 100],
                expect_ok: false,
                expected_bps: None,
                expected_spc: None,
                expected_mft: None,
            },
        ];

        for case in cases {
            let result = parse_vbr(&case.buf);
            assert_eq!(
                result.is_ok(),
                case.expect_ok,
                "FAILED case: {} — result: {result:?}",
                case.name
            );
            if let Ok(vbr) = result {
                if let Some(bps) = case.expected_bps {
                    assert_eq!(vbr.bytes_per_sector, bps, "FAILED case: {}", case.name);
                }
                if let Some(spc) = case.expected_spc {
                    assert_eq!(vbr.sectors_per_cluster, spc, "FAILED case: {}", case.name);
                }
                if let Some(mft) = case.expected_mft {
                    assert_eq!(vbr.mft_cluster, mft, "FAILED case: {}", case.name);
                }
            }
        }
    }

    // ── mft_record_size ───────────────────────────────────────────────────────

    #[derive(Debug)]
    struct MftRecordSizeCase {
        name: &'static str,
        cpmr: i8,
        sectors_per_cluster: u8,
        bytes_per_sector: u16,
        expected: usize,
    }

    fn make_vbr_for_size(cpmr: i8, spc: u8, bps: u16) -> NtfsVbr {
        NtfsVbr {
            bytes_per_sector: bps,
            sectors_per_cluster: spc,
            mft_cluster: 0,
            mft_mirror_cluster: 0,
            clusters_per_mft_record: cpmr,
            volume_serial: 0,
        }
    }

    #[test]
    fn test_mft_record_size() {
        let cases = vec![
            MftRecordSizeCase {
                name: "happy_path_positive_cpmr",
                cpmr: 2,
                sectors_per_cluster: 8,
                bytes_per_sector: 512,
                expected: 8 * 512 * 2, // 8192
            },
            MftRecordSizeCase {
                name: "branch_negative_cpmr_yields_power_of_two",
                cpmr: -10,
                sectors_per_cluster: 8,
                bytes_per_sector: 512,
                expected: 1024, // 2^10
            },
            MftRecordSizeCase {
                name: "branch_negative_cpmr_minus_one",
                cpmr: -1,
                sectors_per_cluster: 1,
                bytes_per_sector: 512,
                expected: 2, // 2^1
            },
        ];

        for case in cases {
            let vbr = make_vbr_for_size(case.cpmr, case.sectors_per_cluster, case.bytes_per_sector);
            let actual = mft_record_size(&vbr);
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }

    // ── parse_mft_record ──────────────────────────────────────────────────────

    #[derive(Debug)]
    struct ParseMftCase {
        name: &'static str,
        buf: Vec<u8>,
        expect_ok: bool,
        expected_flags: Option<u16>,
    }

    #[test]
    fn test_parse_mft_record() {
        let cases = vec![
            ParseMftCase {
                name: "happy_path_valid_deleted_record",
                buf: make_mft_record_buf(0, 56),
                expect_ok: true,
                expected_flags: Some(0),
            },
            ParseMftCase {
                name: "branch_in_use_flag_set",
                buf: make_mft_record_buf(MFT_RECORD_IN_USE, 56),
                expect_ok: true,
                expected_flags: Some(MFT_RECORD_IN_USE),
            },
            ParseMftCase {
                name: "branch_wrong_magic",
                buf: {
                    let mut b = vec![0u8; 1024];
                    b[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
                    b
                },
                expect_ok: false,
                expected_flags: None,
            },
            ParseMftCase {
                name: "branch_buffer_too_small",
                buf: vec![0u8; 20],
                expect_ok: false,
                expected_flags: None,
            },
        ];

        for case in cases {
            let result = parse_mft_record(&case.buf, 42);
            assert_eq!(
                result.is_ok(),
                case.expect_ok,
                "FAILED case: {} — result: {result:?}",
                case.name
            );
            if let Ok(record) = result {
                if let Some(flags) = case.expected_flags {
                    assert_eq!(record.flags, flags, "FAILED case: {}", case.name);
                }
            }
        }
    }

    // ── is_deleted ────────────────────────────────────────────────────────────

    fn make_record(flags: u16) -> MftRecord {
        MftRecord {
            record_number: 0,
            sequence_number: 1,
            flags,
            attrs_offset: 56,
            bytes_allocated: 1024,
            bytes_in_use: 400,
        }
    }

    #[derive(Debug)]
    struct IsDeletedCase {
        name: &'static str,
        flags: u16,
        expected: bool,
    }

    #[test]
    fn test_is_deleted() {
        let cases = vec![
            IsDeletedCase {
                name: "happy_path_in_use_flag_zero_means_deleted",
                flags: 0,
                expected: true,
            },
            IsDeletedCase {
                name: "branch_in_use_flag_set_means_active",
                flags: MFT_RECORD_IN_USE,
                expected: false,
            },
            IsDeletedCase {
                name: "branch_directory_deleted",
                flags: MFT_RECORD_IS_DIRECTORY,
                expected: true,
            },
            IsDeletedCase {
                name: "branch_in_use_directory",
                flags: MFT_RECORD_IN_USE | MFT_RECORD_IS_DIRECTORY,
                expected: false,
            },
        ];

        for case in cases {
            let record = make_record(case.flags);
            let actual = is_deleted(&record);
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }

    // ── parse_attr_header ─────────────────────────────────────────────────────

    #[derive(Debug)]
    struct ParseAttrCase {
        name: &'static str,
        buf: Vec<u8>,
        offset: usize,
        expect_some: bool,
        expect_err: bool,
        expected_type: Option<u32>,
    }

    #[test]
    fn test_parse_attr_header() {
        let cases = vec![
            ParseAttrCase {
                name: "happy_path_end_marker_returns_none",
                buf: {
                    let mut b = vec![0u8; 16];
                    b[0..4].copy_from_slice(&ATTR_END_MARKER.to_le_bytes());
                    b
                },
                offset: 0,
                expect_some: false,
                expect_err: false,
                expected_type: None,
            },
            ParseAttrCase {
                name: "happy_path_valid_file_name_attribute",
                buf: {
                    let mut b = vec![0u8; 48];
                    b[0..4].copy_from_slice(&ATTR_FILE_NAME.to_le_bytes());
                    b[4..8].copy_from_slice(&48u32.to_le_bytes()); // length = full buf
                    b[8] = 0; // resident
                    b[9] = 0; // no name
                    b[10..12].copy_from_slice(&24u16.to_le_bytes()); // value_offset
                    b
                },
                offset: 0,
                expect_some: true,
                expect_err: false,
                expected_type: Some(ATTR_FILE_NAME),
            },
            ParseAttrCase {
                name: "branch_too_few_bytes_returns_none",
                buf: vec![0u8; 2],
                offset: 0,
                expect_some: false,
                expect_err: false,
                expected_type: None,
            },
            ParseAttrCase {
                name: "branch_length_zero_returns_err",
                buf: {
                    let mut b = vec![0u8; 16];
                    b[0..4].copy_from_slice(&ATTR_FILE_NAME.to_le_bytes());
                    b[4..8].copy_from_slice(&0u32.to_le_bytes()); // length = 0
                    b
                },
                offset: 0,
                expect_some: false,
                expect_err: true,
                expected_type: None,
            },
            ParseAttrCase {
                name: "branch_length_exceeds_buffer_returns_err",
                buf: {
                    let mut b = vec![0u8; 16];
                    b[0..4].copy_from_slice(&ATTR_FILE_NAME.to_le_bytes());
                    b[4..8].copy_from_slice(&1000u32.to_le_bytes()); // length > buf
                    b
                },
                offset: 0,
                expect_some: false,
                expect_err: true,
                expected_type: None,
            },
        ];

        for case in cases {
            let result = parse_attr_header(&case.buf, case.offset);
            if case.expect_err {
                assert!(result.is_err(), "FAILED case: {} — expected Err", case.name);
            } else {
                let opt = result.expect("FAILED case: unexpected Err");
                assert_eq!(
                    opt.is_some(),
                    case.expect_some,
                    "FAILED case: {}",
                    case.name
                );
                if let (Some(header), Some(ty)) = (opt, case.expected_type) {
                    assert_eq!(header.attr_type, ty, "FAILED case: {}", case.name);
                }
            }
        }
    }

    // ── parse_file_name_attr ──────────────────────────────────────────────────

    fn make_file_name_buf(name: &str, namespace: u8, file_size: u64) -> Vec<u8> {
        let name_units: Vec<u16> = name.encode_utf16().collect();
        let total = 66 + name_units.len() * 2;
        let mut buf = vec![0u8; total];
        // parent_ref at 0: 0
        // creation_time at 8: 0
        // modification_time at 16: 0
        // mft_mod_time at 24: 0
        // access_time at 32: 0
        // allocated_size at 40
        buf[40..48].copy_from_slice(&file_size.to_le_bytes());
        // file_size at 48
        buf[48..56].copy_from_slice(&file_size.to_le_bytes());
        // flags at 56: 0
        // reparse_tag at 60: 0
        buf[64] = name_units.len() as u8;
        buf[65] = namespace;
        for (i, &u) in name_units.iter().enumerate() {
            let pos = 66 + i * 2;
            buf[pos..pos + 2].copy_from_slice(&u.to_le_bytes());
        }
        buf
    }

    #[derive(Debug)]
    struct ParseFileNameCase {
        name: &'static str,
        buf: Vec<u8>,
        expect_ok: bool,
        expected_name: Option<&'static str>,
        expected_namespace: Option<u8>,
    }

    #[test]
    fn test_parse_file_name_attr() {
        let cases = vec![
            ParseFileNameCase {
                name: "happy_path_win32_name",
                buf: make_file_name_buf("test", FILE_NAME_WIN32, 1024),
                expect_ok: true,
                expected_name: Some("test"),
                expected_namespace: Some(FILE_NAME_WIN32),
            },
            ParseFileNameCase {
                name: "happy_path_empty_filename",
                buf: make_file_name_buf("", FILE_NAME_WIN32_AND_DOS, 0),
                expect_ok: true,
                expected_name: Some(""),
                expected_namespace: Some(FILE_NAME_WIN32_AND_DOS),
            },
            ParseFileNameCase {
                name: "branch_buffer_too_small",
                buf: vec![0u8; 40],
                expect_ok: false,
                expected_name: None,
                expected_namespace: None,
            },
            ParseFileNameCase {
                name: "branch_name_length_exceeds_buffer",
                buf: {
                    // 66-byte buffer but name_len says 20 characters (40 bytes beyond offset 66)
                    let mut b = vec![0u8; 66];
                    b[64] = 20; // 20 UTF-16 chars but no room
                    b[65] = FILE_NAME_WIN32;
                    b
                },
                expect_ok: false,
                expected_name: None,
                expected_namespace: None,
            },
        ];

        for case in cases {
            let result = parse_file_name_attr(&case.buf);
            assert_eq!(
                result.is_ok(),
                case.expect_ok,
                "FAILED case: {} — result: {result:?}",
                case.name
            );
            if let Ok(attr) = result {
                if let Some(name) = case.expected_name {
                    assert_eq!(attr.name, name, "FAILED case: {}", case.name);
                }
                if let Some(ns) = case.expected_namespace {
                    assert_eq!(attr.namespace, ns, "FAILED case: {}", case.name);
                }
            }
        }
    }

    // ── filetime_to_system_time ───────────────────────────────────────────────

    #[derive(Debug)]
    struct FiletimeCase {
        name: &'static str,
        filetime: u64,
        expected: Option<std::time::SystemTime>,
    }

    #[test]
    fn test_filetime_to_system_time() {
        let cases = vec![
            FiletimeCase {
                name: "happy_path_unix_epoch_in_filetime",
                filetime: 116_444_736_000_000_000,
                expected: Some(UNIX_EPOCH),
            },
            FiletimeCase {
                name: "branch_before_unix_epoch_returns_none",
                filetime: 0,
                expected: None,
            },
            FiletimeCase {
                name: "branch_one_second_after_unix_epoch",
                filetime: 116_444_736_000_000_000 + 10_000_000,
                expected: Some(UNIX_EPOCH + std::time::Duration::from_secs(1)),
            },
        ];

        for case in cases {
            let actual = filetime_to_system_time(case.filetime);
            assert_eq!(actual, case.expected, "FAILED case: {}", case.name);
        }
    }

    // ── find_best_file_name ───────────────────────────────────────────────────

    /// Build a minimal MFT record buffer with a single resident $FILE_NAME
    /// attribute at a fixed offset, using the given name and namespace.
    fn make_record_with_name(name: &str, namespace: u8) -> Vec<u8> {
        let attrs_offset: u16 = 56;
        let mut buf = make_mft_record_buf(0, attrs_offset);

        // Insert $FILE_NAME attribute at offset 56
        let fn_value = make_file_name_buf(name, namespace, 512);
        let value_offset_in_attr: u16 = 24; // standard resident attribute header size
        let attr_len = value_offset_in_attr as u32 + fn_value.len() as u32;

        let attr_start = usize::from(attrs_offset);
        buf[attr_start..attr_start + 4].copy_from_slice(&ATTR_FILE_NAME.to_le_bytes());
        buf[attr_start + 4..attr_start + 8].copy_from_slice(&attr_len.to_le_bytes());
        buf[attr_start + 8] = 0; // resident
        buf[attr_start + 9] = 0; // no attribute name
        buf[attr_start + 10..attr_start + 12]
            .copy_from_slice(&value_offset_in_attr.to_le_bytes());

        // Write value
        let val_start = attr_start + usize::from(value_offset_in_attr);
        buf[val_start..val_start + fn_value.len()].copy_from_slice(&fn_value);

        // End-of-attributes marker
        let end_pos = attr_start + attr_len as usize;
        if end_pos + 4 <= buf.len() {
            buf[end_pos..end_pos + 4].copy_from_slice(&ATTR_END_MARKER.to_le_bytes());
        }

        buf
    }

    #[derive(Debug)]
    struct FindBestCase {
        name: &'static str,
        file_name: &'static str,
        namespace: u8,
        expect_found: bool,
        expected_name: Option<&'static str>,
    }

    #[test]
    fn test_find_best_file_name() {
        let cases = vec![
            FindBestCase {
                name: "happy_path_win32_name_found",
                file_name: "hello.txt",
                namespace: FILE_NAME_WIN32,
                expect_found: true,
                expected_name: Some("hello.txt"),
            },
            FindBestCase {
                name: "happy_path_posix_name_found",
                file_name: "readme",
                namespace: FILE_NAME_POSIX,
                expect_found: true,
                expected_name: Some("readme"),
            },
        ];

        for case in cases {
            let buf = make_record_with_name(case.file_name, case.namespace);
            let record = parse_mft_record(&buf, 0).expect("make_record_with_name must produce valid record");
            let result = find_best_file_name(&buf, &record);
            assert_eq!(
                result.is_some(),
                case.expect_found,
                "FAILED case: {}",
                case.name
            );
            if let (Some(attr), Some(name)) = (result, case.expected_name) {
                assert_eq!(attr.name, name, "FAILED case: {}", case.name);
            }
        }
    }
}
