//! Recovery engine (P4-T02): carve a deleted file's block extents off the
//! source device and write them to a destination directory.
//!
//! Design goals, all driven by the security gate on this task:
//!
//! - **No untrusted path ever touches the filesystem unsanitised.** The
//!   filename comes from raw disk metadata; [`sanitize_filename`] strips it to a
//!   single, safe basename before it is used (no traversal, no separators, no
//!   device names).
//! - **Crash-safe writes.** Output goes to a `.partial` temp file guarded by
//!   [`PartialGuard`], which deletes it on any early return, error, cancel, or
//!   panic. Only a fully written file is atomically `rename`d into place.
//! - **Bad sectors never abort the whole file.** A failed block read is
//!   zero-filled (preserving file offsets) and counted, and recovery continues.
//! - **Testable without a real disk.** All reads go through the [`ExtentReader`]
//!   trait; unit tests use an in-memory reader that can inject bad sectors.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use fileresque_core::{error::AppError, types::DeletedFileEntry};
use sha2::{Digest, Sha256};

/// Number of leading file bytes inspected for magic-byte type inference.
const HEADER_LEN: usize = 16;

/// Cap on the sanitised filename length (bytes), leaving room for a dedupe
/// suffix and extension within typical 255-byte filesystem limits.
const MAX_NAME_LEN: usize = 200;

/// Source of file data, abstracted so the engine is testable without a raw
/// device. The production adapter wraps `fileresque_disk`'s `BlockReader`.
pub trait ExtentReader {
    /// Filesystem block size in bytes.
    fn block_size(&self) -> u64;

    /// Read exactly one block at `block_addr`.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] on a read failure. The engine treats *any* error
    /// here as a bad sector: it is logged, the block is zero-filled, and
    /// recovery continues — the error does not propagate.
    fn read_block(&mut self, block_addr: u64) -> Result<Vec<u8>, AppError>;
}

/// What to recover and where.
pub struct RecoveryRequest<'a> {
    pub entry: &'a DeletedFileEntry,
    pub dest_dir: &'a Path,
}

/// Live progress, emitted once per block processed.
#[derive(Debug, Clone, Copy)]
pub struct RecoveryProgress {
    pub bytes_written: u64,
    pub blocks_done: u64,
    pub blocks_skipped: u64,
}

/// Result of a completed recovery.
#[derive(Debug, Clone)]
pub struct RecoveryOutcome {
    /// Inode of the recovered source entry (for audit correlation).
    pub inode_id: u64,
    /// Original filename from disk metadata, if any (pre-sanitisation).
    pub original_name: Option<String>,
    /// Final path the recovered file was atomically renamed to.
    pub final_path: PathBuf,
    /// Lowercase hex SHA-256 of the written file content.
    pub sha256: String,
    pub blocks_read: u64,
    /// Blocks that could not be read and were zero-filled (bad sectors).
    pub blocks_skipped: u64,
    pub bytes_written: u64,
}

impl RecoveryOutcome {
    /// True when at least one block was a bad sector.
    #[must_use]
    pub fn had_bad_sectors(&self) -> bool {
        self.blocks_skipped > 0
    }
}

/// Recover the file described by `req` using `reader`.
///
/// `progress` is invoked after each block; `should_cancel` is polled before each
/// block and, when it returns true, the recovery aborts with
/// [`AppError::Cancelled`] and the partial file is removed.
///
/// # Errors
///
/// - [`AppError::Cancelled`] if `should_cancel` fires
/// - [`AppError::Io`] on a write/rename failure (read failures do not abort)
pub fn recover<R: ExtentReader>(
    req: &RecoveryRequest,
    reader: &mut R,
    progress: &mut dyn FnMut(RecoveryProgress),
    should_cancel: &dyn Fn() -> bool,
) -> Result<RecoveryOutcome, AppError> {
    let mut guard = PartialGuard::new(req.dest_dir.join(provisional_name(req.entry)));
    let file = File::create(guard.path())?;
    let mut writer = BufWriter::new(file);

    let acc = stream_extents(req.entry, reader, &mut writer, progress, should_cancel)?;
    writer.flush()?;

    let final_path = unique_path(
        req.dest_dir,
        &final_file_name(req.entry, &acc.header, &acc.sha_hex),
    );
    fs::rename(guard.path(), &final_path)?;
    guard.commit();

    Ok(RecoveryOutcome {
        inode_id: req.entry.inode_id,
        original_name: req.entry.name.clone(),
        final_path,
        sha256: acc.sha_hex,
        blocks_read: acc.blocks_read,
        blocks_skipped: acc.blocks_skipped,
        bytes_written: acc.bytes_written,
    })
}

/// Mutable tallies accumulated while streaming a file's extents.
struct Acc {
    hasher: Sha256,
    header: [u8; HEADER_LEN],
    header_filled: usize,
    blocks_read: u64,
    blocks_skipped: u64,
    bytes_written: u64,
    sha_hex: String,
}

impl Acc {
    fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            header: [0u8; HEADER_LEN],
            header_filled: 0,
            blocks_read: 0,
            blocks_skipped: 0,
            bytes_written: 0,
            sha_hex: String::new(),
        }
    }

    /// Capture leading bytes for magic-byte inference until the header is full.
    fn fill_header(&mut self, data: &[u8]) {
        if self.header_filled >= HEADER_LEN {
            return;
        }
        for &b in data {
            if self.header_filled >= HEADER_LEN {
                break;
            }
            self.header[self.header_filled] = b;
            self.header_filled += 1;
        }
    }
}

/// Walk every block of every extent, writing (and hashing) at most `size_bytes`
/// of content. Each block is polled for cancellation and contributes one
/// progress tick. Returns the finalised accumulator (with `sha_hex` set).
fn stream_extents<R: ExtentReader>(
    entry: &DeletedFileEntry,
    reader: &mut R,
    writer: &mut BufWriter<File>,
    progress: &mut dyn FnMut(RecoveryProgress),
    should_cancel: &dyn Fn() -> bool,
) -> Result<Acc, AppError> {
    let block_size = reader.block_size();
    // `Some(n)` caps output at the known file size; `None` writes full blocks
    // when the size is unknown (size_bytes == 0 but extents exist).
    let cap = (entry.size_bytes > 0).then_some(entry.size_bytes);
    let mut acc = Acc::new();

    for &(offset, count) in &entry.extents {
        for i in 0..count {
            if should_cancel() {
                return Err(AppError::Cancelled);
            }
            let block = read_or_zero(reader, offset.saturating_add(i), block_size, &mut acc);
            write_capped(writer, &block, cap, &mut acc)?;
            progress(RecoveryProgress {
                bytes_written: acc.bytes_written,
                blocks_done: acc.blocks_read + acc.blocks_skipped,
                blocks_skipped: acc.blocks_skipped,
            });
        }
    }

    acc.sha_hex = hex_lower(&acc.hasher.clone().finalize());
    Ok(acc)
}

/// Read one block; on any read error, count a bad sector and substitute a
/// zero-filled block so downstream file offsets stay correct.
fn read_or_zero<R: ExtentReader>(
    reader: &mut R,
    addr: u64,
    block_size: u64,
    acc: &mut Acc,
) -> Vec<u8> {
    if let Ok(data) = reader.read_block(addr) {
        acc.blocks_read += 1;
        data
    } else {
        acc.blocks_skipped += 1;
        vec![0u8; usize::try_from(block_size).unwrap_or(0)]
    }
}

/// Write `block` to `writer`, never exceeding `cap` total bytes; update the
/// hash, header capture, and byte tally with exactly what was written.
fn write_capped(
    writer: &mut BufWriter<File>,
    block: &[u8],
    cap: Option<u64>,
    acc: &mut Acc,
) -> Result<(), AppError> {
    let take = match cap {
        Some(limit) => {
            let remaining = limit.saturating_sub(acc.bytes_written);
            usize::try_from(remaining)
                .unwrap_or(usize::MAX)
                .min(block.len())
        }
        None => block.len(),
    };
    if take == 0 {
        return Ok(());
    }
    let slice = &block[..take];
    writer.write_all(slice)?;
    acc.hasher.update(slice);
    acc.fill_header(slice);
    acc.bytes_written += take as u64;
    Ok(())
}

/// Lowercase hex encoding of a digest, without pulling in a hex crate.
fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        // JUSTIFIED: writing to a String never fails; the Result is unobservable.
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Provisional `.partial` temp filename — process- and time-unique so two
/// concurrent recoveries never collide on the temp file.
fn provisional_name(entry: &DeletedFileEntry) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!(
        ".fileresque_recover_{}_{}_{}.partial",
        std::process::id(),
        entry.inode_id,
        nanos
    )
}

/// Choose the final filename: the sanitised original if usable, else a
/// `recovered_<hash8>.<ext>` name derived from the digest and magic bytes.
fn final_file_name(entry: &DeletedFileEntry, header: &[u8; HEADER_LEN], sha_hex: &str) -> String {
    if let Some(name) = entry.name.as_deref().and_then(sanitize_filename) {
        return name;
    }
    let stem: String = sha_hex.chars().take(8).collect();
    let stem = if stem.is_empty() { "unknown" } else { &stem };
    format!("recovered_{stem}.{}", infer_extension(header))
}

/// Ensure the chosen name does not clobber an existing file by appending
/// `_1`, `_2`, … to the stem until a free path is found.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = split_ext(name);
    for n in 1u32.. {
        let alt = match ext {
            Some(e) => format!("{stem}_{n}.{e}"),
            None => format!("{stem}_{n}"),
        };
        let path = dir.join(alt);
        if !path.exists() {
            return path;
        }
    }
    // JUSTIFIED: the 1..=u32::MAX loop always returns before exhausting; this
    // line is unreachable but satisfies the non-Option return type.
    dir.join(name)
}

/// Split `name` into `(stem, Some(ext))` on the last `.`; no extension → `None`.
fn split_ext(name: &str) -> (&str, Option<&str>) {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => (stem, Some(ext)),
        _ => (name, None),
    }
}

/// **Security boundary.** Reduce a filename from raw disk metadata to a single
/// safe basename, or `None` if nothing safe remains (caller then uses a
/// generated name).
///
/// Guarantees on the returned string: it is a single path component (no `/` or
/// `\\`), is not `.`/`..`, contains no NUL or ASCII control characters, does not
/// collide with a Windows reserved device name, and is at most
/// [`MAX_NAME_LEN`] bytes.
#[must_use]
pub fn sanitize_filename(raw: &str) -> Option<String> {
    // Keep only the final component, defeating any embedded path/traversal.
    let base = raw.rsplit(['/', '\\']).next().unwrap_or(raw);

    let mut cleaned: String = base
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if is_reserved_char(c) { '_' } else { c })
        .collect();

    cleaned = cleaned.trim().trim_matches('.').trim().to_string();
    truncate_bytes(&mut cleaned, MAX_NAME_LEN);

    if cleaned.is_empty() || is_reserved_device_name(&cleaned) {
        return None;
    }
    Some(cleaned)
}

/// Characters illegal in a filename on at least one supported platform.
fn is_reserved_char(c: char) -> bool {
    matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
}

/// Windows reserved device names (case-insensitive, ignoring any extension).
fn is_reserved_device_name(name: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    RESERVED.contains(&stem.as_str())
}

/// Truncate `s` in place to at most `max` bytes without splitting a UTF-8 char.
fn truncate_bytes(s: &mut String, max: usize) {
    if s.len() <= max {
        return;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
}

/// Infer a file extension from up to 16 leading bytes. Best-effort, covering the
/// common cases named in the spec; unknown content yields `"bin"`.
#[must_use]
pub fn infer_extension(header: &[u8; HEADER_LEN]) -> &'static str {
    match header {
        [0xFF, 0xD8, 0xFF, ..] => "jpg",
        [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => "png",
        [b'%', b'P', b'D', b'F', ..] => "pdf",
        // PK\x03\x04 — ZIP container (also DOCX/XLSX/PPTX; not distinguishable
        // from the header alone, so reported as the container type).
        [b'P', b'K', 0x03, 0x04, ..] => "zip",
        _ => infer_ftyp(header),
    }
}

/// ISO base-media files (MP4/MOV) carry `ftyp` at byte offset 4.
fn infer_ftyp(header: &[u8; HEADER_LEN]) -> &'static str {
    if &header[4..8] != b"ftyp" {
        return "bin";
    }
    match &header[8..11] {
        b"qt " => "mov",
        _ => "mp4",
    }
}

/// RAII cleanup for the `.partial` temp file: removes it on drop unless
/// [`commit`](PartialGuard::commit) was called after a successful rename.
struct PartialGuard {
    path: Option<PathBuf>,
}

impl PartialGuard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn path(&self) -> &Path {
        // JUSTIFIED: `path` is only `None` after `commit`, which consumes the
        // guard's responsibility; `path()` is never called post-commit.
        self.path.as_deref().unwrap_or(Path::new(""))
    }

    /// Mark the partial file as successfully renamed; suppress cleanup.
    fn commit(&mut self) {
        self.path = None;
    }
}

impl Drop for PartialGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            // Best-effort: a leftover .partial is recoverable; nothing to do on error.
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fileresque_core::types::FileSystem;
    use std::collections::HashMap;

    /// In-memory reader. `blocks` maps addr → content; `bad` addresses error.
    struct MemReader {
        block_size: u64,
        blocks: HashMap<u64, Vec<u8>>,
        bad: Vec<u64>,
    }

    impl ExtentReader for MemReader {
        fn block_size(&self) -> u64 {
            self.block_size
        }
        fn read_block(&mut self, addr: u64) -> Result<Vec<u8>, AppError> {
            if self.bad.contains(&addr) {
                return Err(AppError::Io(std::io::Error::other("bad sector")));
            }
            Ok(self
                .blocks
                .get(&addr)
                .cloned()
                .unwrap_or_else(|| vec![0u8; usize::try_from(self.block_size).unwrap_or(0)]))
        }
    }

    fn entry(name: Option<&str>, size: u64, extents: Vec<(u64, u64)>) -> DeletedFileEntry {
        DeletedFileEntry {
            inode_id: 7,
            name: name.map(str::to_string),
            size_bytes: size,
            deleted_at: None,
            extents,
            filesystem: FileSystem::APFS,
        }
    }

    fn no_progress() -> impl FnMut(RecoveryProgress) {
        |_| {}
    }

    #[test]
    fn recovers_file_and_renames_atomically() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut blocks = HashMap::new();
        blocks.insert(10, vec![0xAB; 4]);
        blocks.insert(11, vec![0xCD; 4]);
        let mut reader = MemReader {
            block_size: 4,
            blocks,
            bad: vec![],
        };
        let e = entry(Some("photo.bin"), 8, vec![(10, 2)]);
        let req = RecoveryRequest {
            entry: &e,
            dest_dir: dir.path(),
        };

        let out = recover(&req, &mut reader, &mut no_progress(), &|| false).expect("recover ok");

        assert_eq!(out.final_path, dir.path().join("photo.bin"));
        assert!(out.final_path.exists(), "final file must exist");
        assert_eq!(out.bytes_written, 8);
        assert_eq!(out.blocks_read, 2);
        assert!(!out.had_bad_sectors());
        let content = std::fs::read(&out.final_path).expect("read back");
        assert_eq!(
            content,
            vec![0xAB, 0xAB, 0xAB, 0xAB, 0xCD, 0xCD, 0xCD, 0xCD]
        );
        // No leftover .partial files in the dir.
        let leftovers = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains("partial"))
            .count();
        assert_eq!(leftovers, 0, "partial temp file must be cleaned up");
    }

    #[test]
    fn truncates_to_size_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut blocks = HashMap::new();
        blocks.insert(0, vec![0x11; 8]);
        let mut reader = MemReader {
            block_size: 8,
            blocks,
            bad: vec![],
        };
        // size 5 < one 8-byte block → only 5 bytes written.
        let e = entry(Some("f.bin"), 5, vec![(0, 1)]);
        let req = RecoveryRequest {
            entry: &e,
            dest_dir: dir.path(),
        };
        let out = recover(&req, &mut reader, &mut no_progress(), &|| false).expect("ok");
        assert_eq!(out.bytes_written, 5);
        assert_eq!(std::fs::read(&out.final_path).unwrap().len(), 5);
    }

    #[test]
    fn bad_sector_is_zero_filled_and_counted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut blocks = HashMap::new();
        blocks.insert(0, vec![0x11; 4]);
        blocks.insert(2, vec![0x33; 4]);
        let mut reader = MemReader {
            block_size: 4,
            blocks,
            bad: vec![1], // middle block unreadable
        };
        let e = entry(Some("f.bin"), 12, vec![(0, 3)]);
        let req = RecoveryRequest {
            entry: &e,
            dest_dir: dir.path(),
        };
        let out = recover(&req, &mut reader, &mut no_progress(), &|| false).expect("ok");
        assert!(out.had_bad_sectors());
        assert_eq!(out.blocks_skipped, 1);
        assert_eq!(out.blocks_read, 2);
        let content = std::fs::read(&out.final_path).unwrap();
        assert_eq!(&content[4..8], &[0, 0, 0, 0], "bad block zero-filled");
    }

    #[test]
    fn cancel_aborts_and_cleans_partial() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut reader = MemReader {
            block_size: 4,
            blocks: HashMap::new(),
            bad: vec![],
        };
        let e = entry(Some("f.bin"), 4096, vec![(0, 1000)]);
        let req = RecoveryRequest {
            entry: &e,
            dest_dir: dir.path(),
        };
        let err = recover(&req, &mut reader, &mut no_progress(), &|| true).unwrap_err();
        assert!(matches!(err, AppError::Cancelled));
        let any = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(any, 0, "no files left behind after cancel");
    }

    #[test]
    fn dedupes_existing_final_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("f.bin"), b"existing").expect("seed");
        let mut blocks = HashMap::new();
        blocks.insert(0, vec![0x11; 4]);
        let mut reader = MemReader {
            block_size: 4,
            blocks,
            bad: vec![],
        };
        let e = entry(Some("f.bin"), 4, vec![(0, 1)]);
        let req = RecoveryRequest {
            entry: &e,
            dest_dir: dir.path(),
        };
        let out = recover(&req, &mut reader, &mut no_progress(), &|| false).expect("ok");
        assert_eq!(out.final_path, dir.path().join("f_1.bin"));
    }

    #[test]
    fn generated_name_when_metadata_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut blocks = HashMap::new();
        blocks.insert(0, vec![0xFF, 0xD8, 0xFF, 0xE0]); // JPEG magic
        let mut reader = MemReader {
            block_size: 4,
            blocks,
            bad: vec![],
        };
        let e = entry(None, 4, vec![(0, 1)]);
        let req = RecoveryRequest {
            entry: &e,
            dest_dir: dir.path(),
        };
        let out = recover(&req, &mut reader, &mut no_progress(), &|| false).expect("ok");
        let name = out
            .final_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(name.starts_with("recovered_"), "got: {name}");
        assert!(
            out.final_path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg")),
            "magic-byte ext, got: {name}"
        );
    }

    #[test]
    fn sanitize_filename_blocks_traversal_and_separators() {
        let cases = vec![
            ("../../etc/passwd", Some("passwd")),
            ("a/b/c.txt", Some("c.txt")),
            ("evil\\..\\boot.ini", Some("boot.ini")),
            ("..", None),
            (".", None),
            ("", None),
            ("   ", None),
            ("na:me*?.txt", Some("na_me__.txt")),
            ("CON", None),
            ("nul.txt", None),
            ("normal_file.png", Some("normal_file.png")),
        ];
        for (input, expected) in cases {
            assert_eq!(
                sanitize_filename(input),
                expected.map(str::to_string),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn sanitize_truncates_overlong_names() {
        let long = "x".repeat(500);
        let out = sanitize_filename(&long).expect("non-empty");
        assert!(out.len() <= MAX_NAME_LEN, "len {} > max", out.len());
    }

    #[test]
    fn infer_extension_covers_common_types() {
        let cases: Vec<([u8; HEADER_LEN], &str)> = vec![
            (
                [0xFF, 0xD8, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "jpg",
            ),
            (
                [
                    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
                "png",
            ),
            (
                [b'%', b'P', b'D', b'F', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "pdf",
            ),
            (
                [b'P', b'K', 3, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "zip",
            ),
            (
                [
                    0, 0, 0, 0x18, b'f', b't', b'y', b'p', b'm', b'p', b'4', b'2', 0, 0, 0, 0,
                ],
                "mp4",
            ),
            (
                [
                    0, 0, 0, 0x14, b'f', b't', b'y', b'p', b'q', b't', b' ', 0, 0, 0, 0, 0,
                ],
                "mov",
            ),
            ([0; HEADER_LEN], "bin"),
        ];
        for (header, expected) in cases {
            assert_eq!(infer_extension(&header), expected, "header: {header:?}");
        }
    }
}
