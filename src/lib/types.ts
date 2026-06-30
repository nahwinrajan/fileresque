/**
 * FileResque — shared TypeScript types
 *
 * These interfaces mirror the canonical Rust types in `crates/core/src/types.rs`.
 * Keep in sync with the Rust structs when the Rust side changes.
 * Serialisation: Tauri uses serde_json; field names are snake_case on both sides.
 */

export interface DiskInfo {
  /** Platform disk identifier, e.g. "disk0" (macOS) or "PhysicalDrive0" (Windows). */
  id: string;
  display_name: string;
  size_bytes: number;
  drive_type: 'SSD' | 'HDD' | 'NVMe' | 'USB' | 'Virtual' | 'Unknown';
  filesystem: 'APFS' | 'HFSPlus' | 'NTFS' | 'FAT32' | 'ExFAT' | 'Unknown';
  mount_points: string[];
  encrypted: boolean;
  trim_enabled: boolean;
  /** null when serial number is unavailable or redacted. */
  serial: string | null;
}

export interface DeletedFileEntry {
  inode_id: number;
  /** null when the directory entry for this inode has been zeroed. */
  name: string | null;
  size_bytes: number;
  /** Seconds since Unix epoch. null when deletion timestamp is unavailable. */
  deleted_at: number | null;
  /** Each tuple is (block_offset, block_count). */
  extents: [number, number][];
  filesystem: 'APFS' | 'HFSPlus' | 'NTFS' | 'FAT32' | 'ExFAT' | 'Unknown';
}

export type ProbabilityTier = 'High' | 'Medium' | 'Low';

export interface ProbabilityReport {
  tier: ProbabilityTier;
  /** Percentage of disk blocks that are free (0.0–100.0). */
  free_blocks_pct: number;
  trim_active: boolean;
  blocks_zeroed: boolean;
  estimated_recoverable_bytes: number;
  warnings: string[];
}

export interface PreflightResult {
  ok: boolean;
  errors: PreflightError[];
}

export type PreflightError =
  | { kind: 'SameDisk' }
  | { kind: 'InsufficientSpace'; required: number; available: number }
  | { kind: 'DestinationNotWritable' }
  | { kind: 'SourceNotReadable' };

// ── Recovery (P4-T02/T03) ─────────────────────────────────────────────────────
// Event payloads emitted by the `recover_files` command. Mirror the JSON shapes
// built in `src-tauri/src/commands/recovery.rs`.

/** `recovery:progress` — throttled, one stream per recovering file. */
export interface RecoveryProgressEvent {
  inode_id: number;
  file_index: number;
  total_files: number;
  bytes_written: number;
  blocks_done: number;
  blocks_skipped: number;
}

export type RecoveryFileStatus = 'success' | 'partial' | 'failed';

/** `recovery:file_complete` — one per finished (or failed) file. */
export interface RecoveryFileCompleteEvent {
  status: RecoveryFileStatus;
  inode_id?: number;
  final_path?: string;
  sha256?: string;
  blocks_read?: number;
  blocks_skipped?: number;
  bytes_written?: number;
  error?: string;
}

/** `recovery:complete` — batch summary. */
export interface RecoveryCompleteEvent {
  recovered: number;
  partial: number;
  failed: number;
  cancelled: boolean;
  total: number;
}

// ── Disconnection (P5-T03) ────────────────────────────────────────────────────

/** `disk:disconnected` — source device vanished during scan or recovery. */
export interface DiskDisconnectedEvent {
  disk_id: string;
  message: string;
}
