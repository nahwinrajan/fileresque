# System Architecture — FileResque

**Version:** 1.0  
**Owners:** 🔵 Developer + 🟠 TPM  
**Updated:** 2026-06-26

---

## High-Level Component Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│                        FileResque Application                         │
│                                                                        │
│  ┌───────────────────────────────────────────────────────────────┐   │
│  │                     WebView (Svelte + TS)                      │   │
│  │                                                                 │   │
│  │  ┌──────────┐  ┌───────────┐  ┌────────────┐  ┌───────────┐  │   │
│  │  │ DiskList │  │ FileTable │  │  Probability│  │ Recovery  │  │   │
│  │  │ Component│  │ Component │  │  Panel      │  │ Modal     │  │   │
│  │  └────┬─────┘  └─────┬─────┘  └──────┬─────┘  └─────┬─────┘  │   │
│  │       │              │               │               │          │   │
│  │  ┌────▼──────────────▼───────────────▼───────────────▼──────┐  │   │
│  │  │              Svelte Stores (app state)                     │  │   │
│  │  └────────────────────────┬──────────────────────────────────┘  │   │
│  └───────────────────────────│─────────────────────────────────────┘   │
│                               │ tauri::invoke / tauri::listen           │
│  ┌────────────────────────────▼─────────────────────────────────────┐  │
│  │                    Tauri Core (src-tauri)                         │  │
│  │                                                                    │  │
│  │  ┌──────────────────────────────────────────────────────────┐    │  │
│  │  │              IPC Command Handlers (thin layer)            │    │  │
│  │  │  get_disks | start_scan | check_probability |            │    │  │
│  │  │  run_preflight | start_recovery | cancel_*              │    │  │
│  │  └──────────────────┬───────────────────────────────────────┘    │  │
│  │                      │  calls into crates                          │  │
│  │  ┌───────────────────▼───────────────────────────────────────┐   │  │
│  │  │                   crates/core                              │   │  │
│  │  │   DiskInfo, DeletedFileEntry, ProbabilityReport,          │   │  │
│  │  │   traits: DiskScanner, FsParser, RecoveryEngine           │   │  │
│  │  └──────┬───────────────────────────┬──────────────────────┘    │  │
│  │         │                           │                             │  │
│  │  ┌──────▼──────┐           ┌────────▼──────┐                    │  │
│  │  │ crates/disk │           │crates/recovery│                    │  │
│  │  │             │           │               │                    │  │
│  │  │ ┌─────────┐ │           │ ┌───────────┐ │                    │  │
│  │  │ │macos/   │ │           │ │probability│ │                    │  │
│  │  │ │  apfs/  │ │           │ │  .rs      │ │                    │  │
│  │  │ │  hfsplus│ │           │ ├───────────┤ │                    │  │
│  │  │ ├─────────┤ │           │ │engine.rs  │ │                    │  │
│  │  │ │windows/ │ │           │ ├───────────┤ │                    │  │
│  │  │ │  ntfs/  │ │           │ │audit.rs   │ │                    │  │
│  │  │ └─────────┘ │           │ └───────────┘ │                    │  │
│  │  └──────┬──────┘           └───────┬───────┘                    │  │
│  └─────────│──────────────────────────│─────────────────────────────┘  │
│            │                          │                                  │
│  ┌─────────▼──────────────────────────▼──────────────────────────────┐  │
│  │                      Operating System Layer                         │  │
│  │                                                                      │  │
│  │   macOS: /dev/rdiskN (raw device)    Windows: \\.\PhysicalDriveN  │  │
│  │   ioctl / IOKit                      DeviceIoControl / ReadFile    │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Crate Dependency Graph

```
tauri app (src-tauri)
    └── crates/core       (types, traits, errors — no OS deps)
    └── crates/disk       (OS-gated: macos | windows)
        └── crates/core
    └── crates/recovery   (probability + engine)
        └── crates/core
        └── crates/disk   (reads block status)
```

**Rule:** `crates/core` has zero OS-specific dependencies. `crates/disk` uses `#[cfg(target_os = "macos")]` and `#[cfg(target_os = "windows")]` to gate platform code. `crates/recovery` is platform-agnostic — it consumes `DeletedFileEntry` and trait objects, not OS APIs directly.

---

## Data Flow: Scan Operation

```
User selects disk
     │
     ▼
[Frontend] invoke("start_scan", { disk_id })
     │
     ▼
[Tauri Command] start_scan(disk_id) → validate input → get DiskInfo
     │
     ▼
tokio::spawn_blocking(|| {
     │
     ├── macOS: APFSParser::scan(disk_path, tx)
     │     └── open /dev/rdiskN (O_RDONLY)
     │     └── parse superblock → object map → file system tree
     │     └── for each deleted inode: tx.send(DeletedFileEntry)
     │
     └── Windows: NTFSParser::scan(disk_path, tx)
           └── open \\.\PhysicalDriveN
           └── read $MFT → filter IN_USE == 0
           └── for each record: tx.send(DeletedFileEntry)
})
     │
     ▼
Receiver loop: for entry in rx { app.emit("scan:file_found", entry) }
     │
     ▼
[Frontend] listens on "scan:file_found" → appends to reactive store → FileTable re-renders
```

---

## Data Flow: Recovery Operation

```
User selects files → confirms destination
     │
     ▼
[Frontend] invoke("run_preflight", { inode_ids, dest, disk_id })
     │
     ▼
[PreflightChecker]
     ├── same_disk_check(source_serial, dest_path)?
     ├── available_space(dest) >= required * 1.1?
     ├── writable_check(dest)?
     └── source_readable_check(disk_id)?
     │
     ▼ (if all pass)
[Frontend] invoke("start_recovery", { inode_ids, dest, disk_id })
     │
     ▼
tokio::spawn_blocking(|| RecoveryEngine::recover(entries, dest, event_tx))
     ├── for each entry:
     │   ├── open source O_RDONLY
     │   ├── write to dest/<name>.partial (streaming by block)
     │   ├── emit recovery:progress per block
     │   ├── on complete: fs::rename(.partial → final)
     │   ├── on error: delete .partial, emit recovery:error
     │   └── emit recovery:file_complete
     └── emit recovery:complete
     │
     ▼
[AuditLog::append(entry)] → ~/.../fileresque/audit.log (JSONL)
```

---

## Error Type Hierarchy

```
AppError (exposed to Tauri IPC — user-facing)
├── PermissionDenied { detail: String }
├── DiskNotFound { disk_id: String }
├── DiskDisconnected { disk_id: String }
├── ScanFailed { reason: String }
├── RecoveryFailed { inode_id: u64, reason: String }
├── PreflightFailed { errors: Vec<PreflightError> }
└── Internal { message: String }  // never exposes raw OS errors

DiskError (internal — crates/disk)
├── IoError(std::io::Error)
├── ParseError { offset: u64, detail: String }
├── UnsupportedFilesystem
└── EncryptedVolume

RecoveryError (internal — crates/recovery)
├── BadSector { offset: u64 }
├── PartialExtent { inode_id: u64 }
├── DestinationFull
└── SourceDisconnected
```

**Rule:** `DiskError` and `RecoveryError` are internal. They are mapped to `AppError` variants in Tauri command handlers. Raw OS error messages NEVER reach the frontend.

---

## Threading Model

```
Main thread (Tauri / WebView)
    └── async executor (Tokio)
          ├── Tauri command tasks (lightweight; call spawn_blocking for I/O)
          ├── Event emission tasks
          └── spawn_blocking pool (CPU-bound disk I/O)
                ├── Scan worker (one per active scan)
                └── Recovery worker (one per active recovery)
```

**Rule:** `spawn_blocking` for all `read()` / `write()` syscalls. Async `.await` only for channel operations and Tauri event emission.

---

## Tauri Capability Configuration

```
src-tauri/capabilities/
├── default.json     — base capabilities (window management)
├── disk.json        — allows: get_disks, start_scan, cancel_scan
├── recovery.json    — allows: check_probability, run_preflight, start_recovery, cancel_recovery, pick_destination
└── shell.json       — allows: shell::open (for "open folder" after recovery)
```

Frontend JavaScript can only invoke commands listed in the active capability files. This is enforced at the Tauri runtime level.