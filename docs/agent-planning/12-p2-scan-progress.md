# Planning Doc — P2-T04 Scan Progress Streaming

**Task ID:** P2-T04  
**Phase:** P2 — File System Scanning  
**Status:** Implemented  
**Agent:** 🔵 Developer  
**Security Gate:** No (no unsafe blocks; events are outbound only)

---

## Background

P2-T04 wires the platform FS parsers (APFS, HFS+, NTFS) to the Tauri event
bus so the frontend can receive live results while a scan runs.

---

## Design

### State

`ScanState` is managed by Tauri (`Builder::manage`). It holds a
`Mutex<Option<oneshot::Sender<()>>>` — the cancellation channel for the active
scan. Only one scan can run at a time; starting a new scan replaces the old
cancel handle.

### Command surface

| Command | Signature | Effect |
|---------|-----------|--------|
| `start_scan` | `(disk_id: String) → Result<(), AppError>` | Resolves FS type, spawns scan task, returns immediately |
| `cancel_scan` | `() → Result<(), AppError>` | Sends cancel signal; scan drains to stop |

### Event flow

```
[frontend] start_scan(disk_id)
    │
    ├─ resolve_filesystem(disk_id)   ← macOS: list_disks(); Windows: NTFS
    │
    ├─ tokio::task::spawn_blocking(dispatch_scan_sync)
    │       sends DeletedFileEntry → mpsc::channel(128)
    │
    └─ async loop (run_scan_loop)
           tokio::select! {
               cancel_rx → break (drop entry_rx → blocking_send fails → scan stops)
               entry_rx.recv() → emit scan:file_found, scan:progress (every 50)
               None (channel closed) → break
           }
           scan_handle.await → emit scan:complete | scan:error
```

### Filesystem routing

| Platform | Filesystem | Scanner |
|----------|-----------|---------|
| macOS | APFS | `macos::apfs::scan_apfs_sync` |
| macOS | HFSPlus | `macos::hfsplus::scan_hfsplus_sync` |
| Windows | NTFS | `windows::ntfs::scan_ntfs_sync` |
| any | other | `AppError::UnsupportedFilesystem` |

### Emitted events

| Event | Payload |
|-------|---------|
| `scan:file_found` | `DeletedFileEntry` |
| `scan:progress` | `{ scanned_bytes: 0, total_bytes: 0, files_found: u64 }` |
| `scan:complete` | `{ total_found: u64, duration_ms: u64 }` |
| `scan:error` | `{ message: String, recoverable: false }` |

`scanned_bytes` / `total_bytes` are stubbed to 0 in this MVP; the parsers
don't report byte-level progress. Phase 3 probability engine may update this.

---

## Files Changed

- `src-tauri/src/commands/scan.rs` — new file, all scan logic
- `src-tauri/src/commands/mod.rs` — added `pub mod scan;`
- `src-tauri/src/lib.rs` — added `.manage(ScanState::new())` + two new commands
- `crates/disk/src/windows/ntfs/mod.rs` — added `scan_ntfs_sync`, `scan_ntfs`

---

## Completion Checklist

- [x] `cargo clippy` — 0 warnings
- [x] `cargo fmt` — clean
- [x] Cognitive complexity ≤ 15 on all new functions
- [ ] Unit tests written — scan command is integration-level; no unit tests
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [x] All `unsafe` blocks have `// SAFETY:` (none — no unsafe)
- [x] Planning doc updated with implementation notes
- [ ] 🟢 QA sign-off
- [ ] 🔴 Security sign-off (N/A — no unsafe, events are outbound only)
