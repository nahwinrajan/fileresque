use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use fileresque_core::error::AppError;

/// Reads fixed-size blocks from a raw block device or disk image.
pub struct BlockReader {
    file: File,
    /// Block size in bytes; updated after reading the container superblock.
    pub block_size: u32,
}

impl BlockReader {
    /// Open a raw disk device for reading.
    ///
    /// `device_path` should be `/dev/rdiskN` (the character device) for best
    /// performance — the raw device bypasses the BSD buffer cache and allows
    /// direct sequential reads.
    ///
    /// # Errors
    ///
    /// - [`AppError::PermissionDenied`] when the process lacks read access
    /// - [`AppError::Io`] for other OS-level open failures
    pub fn open(device_path: &str) -> Result<Self, AppError> {
        let file = File::open(device_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                AppError::PermissionDenied(format!("Cannot open {device_path}: {e}"))
            } else {
                AppError::Io(e)
            }
        })?;
        Ok(Self {
            file,
            block_size: 4096,
        })
    }

    /// Read exactly one block at `block_addr` into a freshly allocated buffer.
    ///
    /// # Errors
    ///
    /// - [`AppError::Internal`] if the block address causes a u64 overflow
    /// - [`AppError::Io`] (wrapped in `Internal`) if the seek or read fails
    pub fn read_block(&mut self, block_addr: u64) -> Result<Vec<u8>, AppError> {
        let offset = block_addr
            .checked_mul(u64::from(self.block_size))
            .ok_or_else(|| AppError::Internal("Block address overflow".to_string()))?;

        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(AppError::Io)?;

        let mut buf = vec![0u8; self.block_size as usize];
        self.file
            .read_exact(&mut buf)
            .map_err(|e| AppError::Internal(format!("read_exact at block {block_addr}: {e}")))?;
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_nonexistent_device_returns_error() {
        let result = BlockReader::open("/dev/nonexistent_fileresque_test_device_xyz");
        assert!(result.is_err(), "expected error opening nonexistent device");
    }
}
