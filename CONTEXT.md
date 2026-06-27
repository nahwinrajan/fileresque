# CONTEXT.md — FileResque Project Context for AI Agents

> **Purpose:** This file is the single source of truth for any AI agent entering this codebase. Read this before touching any file.

---

## Project Overview

**Name:** FileResque  
**Type:** Native desktop application  
**Stack:** Tauri 2 (Rust backend) + Svelte + TypeScript frontend  
**Target platforms:** macOS (primary), Windows (secondary)  
**Purpose:** File recovery — scan disks for deleted files, assess recovery probability, recover files to a safe destination.  
**Network:** Zero. This app never makes network calls. Fully offline.

---

## Repository Structure

```
fileresque/
├── .claude/
│   ├── settings.json              # Claude permissions and project config
│   └── agents/
│       ├── tpm.md                 # 🟠 TPM agent — coordination and decisions
│       ├── developer.md           # 🔵 Developer agent — Rust + Svelte implementation
│       ├── qa.md                  # 🟢 QA agent — verification and sign-off
│       ├── designer.md            # 🟣 Designer agent — UI/UX and design system
│       └── security.md            # 🔴 Security agent — audits and safety gates
├── docs/
│   ├── research/
│   │   └── research-analysis-report.md   # Technical feasibility, FS internals, risks
│   ├── features/
│   │   └── feature-breakdown-phases.md   # All tasks, phases, dependencies, agent gates
│   ├── architecture/
│   │   └── system-architecture.md        # Component diagram, data flow, IPC contracts
│   ├── design/
│   │   └── design-brief.md               # Brand identity, design tokens (created by [DES])
│   └── agent-planning/
│       ├── decisions.md                   # [TPM] decision log
│       └── <seq>-<phase>-<subtask>.md    # Per-task planning docs (created by agents)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                        # Tauri app entry point
│   │   ├── commands/                      # Thin Tauri IPC command handlers
│   │   └── lib.rs
│   ├── capabilities/                      # Tauri 2 capability files
│   ├── entitlements.mac.plist             # macOS entitlements
│   └── tauri.conf.json
├── crates/
│   ├── core/                              # Shared types, traits, errors
│   ├── disk/                              # Disk enumeration + FS parsing
│   │   └── src/
│   │       ├── macos/                     # APFS, HFS+ parsers
│   │       └── windows/                  # NTFS parser
│   └── recovery/                         # Probability engine + recovery engine
├── src/                                   # Svelte frontend
│   ├── lib/
│   │   ├── components/                   # UI components
│   │   └── stores/                       # Svelte stores
│   └── routes/                           # App views/pages
├── .github/
│   └── workflows/
│       └── ci.yml                         # CI: lint, test, coverage, build
├── Makefile                               # make dev | test | lint | coverage | build
├── README.md                              # Human setup guide
└── CONTEXT.md                             # This file
```

---

## Agent Roster & Colours

| Agent | Colour | Model | Responsibility |
|-------|--------|-------|---------------|
| TPM | 🟠 Orange | claude-sonnet-4-6 | Coordination, decisions, scope |
| Developer | 🔵 Blue | claude-sonnet-4-6 | Rust + Svelte implementation |
| QA | 🟢 Green | claude-haiku-4-5-20251001 | Testing, verification, sign-off |
| Designer | 🟣 Purple | claude-sonnet-4-6 | UI/UX, design system, specs |
| Security | 🔴 Red | claude-sonnet-4-6 | Audits, unsafe review, veto |

---

## Core Data Types

These are the canonical types. All agents must use these; do not create parallel type definitions.

```rust
// crates/core/src/types.rs

pub enum DriveType { SSD, HDD, NVMe, USB, Virtual, Unknown }
pub enum FileSystem { APFS, HFSPlus, NTFS, FAT32, ExFAT, Unknown }
pub enum ProbabilityTier { High, Medium, Low }

pub struct DiskInfo {
    pub id: String,           // e.g. "disk0" (mac), "PhysicalDrive0" (win)
    pub display_name: String,
    pub size_bytes: u64,
    pub drive_type: DriveType,
    pub filesystem: FileSystem,
    pub mount_points: Vec<String>,
    pub encrypted: bool,
    pub trim_enabled: bool,
    pub serial: Option<String>, // for same-disk detection
}

pub struct DeletedFileEntry {
    pub inode_id: u64,
    pub name: Option<String>,
    pub size_bytes: u64,
    pub deleted_at: Option<std::time::SystemTime>,
    pub extents: Vec<(u64, u64)>, // (block_offset, block_count)
    pub filesystem: FileSystem,
}

pub struct ProbabilityReport {
    pub tier: ProbabilityTier,
    pub free_blocks_pct: f32,
    pub trim_active: bool,
    pub blocks_zeroed: bool,
    pub estimated_recoverable_bytes: u64,
    pub warnings: Vec<String>,
}

pub struct PreflightResult {
    pub ok: bool,
    pub errors: Vec<PreflightError>,
}

pub enum PreflightError {
    SameDisk,
    InsufficientSpace { required: u64, available: u64 },
    DestinationNotWritable,
    SourceNotReadable,
}
```

---

## IPC Command Surface

All Tauri commands. Frontend can only call these — all other Rust code is inaccessible from WebView.

| Command | Args | Returns | Phase |
|---------|------|---------|-------|
| `get_disks` | — | `Result<Vec<DiskInfo>, AppError>` | P1 |
| `start_scan` | `disk_id: String` | `Result<(), AppError>` (streams events) | P2 |
| `cancel_scan` | — | `Result<(), AppError>` | P2 |
| `check_probability` | `inode_id: u64, disk_id: String` | `Result<ProbabilityReport, AppError>` | P3 |
| `pick_destination` | — | `Result<String, AppError>` | P4 |
| `run_preflight` | `inode_ids: Vec<u64>, dest: String, disk_id: String` | `Result<PreflightResult, AppError>` | P4 |
| `start_recovery` | `inode_ids: Vec<u64>, dest: String, disk_id: String` | `Result<(), AppError>` (streams events) | P4 |
| `cancel_recovery` | — | `Result<(), AppError>` | P4 |

## Tauri Events (backend → frontend)

| Event | Payload | Phase |
|-------|---------|-------|
| `scan:progress` | `{ scanned_bytes: u64, total_bytes: u64, files_found: u64 }` | P2 |
| `scan:file_found` | `DeletedFileEntry` (serialised) | P2 |
| `scan:complete` | `{ total_found: u64, duration_ms: u64 }` | P2 |
| `scan:error` | `{ message: String, recoverable: bool }` | P2 |
| `recovery:progress` | `{ inode_id: u64, bytes_written: u64, total_bytes: u64 }` | P4 |
| `recovery:file_complete` | `{ inode_id: u64, dest_path: String, sha256: String }` | P4 |
| `recovery:error` | `{ inode_id: u64, message: String }` | P4 |
| `recovery:complete` | `{ succeeded: u64, failed: u64, duration_ms: u64 }` | P4 |
| `disk:disconnected` | `{ disk_id: String }` | P5 |

---

## Development Phases Summary

| Phase | Goal | Key Tasks |
|-------|------|-----------|
| P0 | Foundation | Scaffold, CI, design system, entitlements |
| P1 | Disk Discovery | Enumerate disks (mac + win), permission onboarding |
| P2 | FS Scanning | APFS, HFS+, NTFS parsers; streaming results |
| P3 | Probability | Block status check, recovery likelihood UI |
| P4 | Recovery | Pre-flight, engine, progress UI, audit log |
| P5 | Polish | Code signing, accessibility, error hardening, security audit |

Full task list with dependencies: `docs/features/feature-breakdown-phases.md`

---

## Absolute Rules (All Agents)

1. **No network calls.** Not in Rust, not in TypeScript, not in CI scripts that touch the app binary.
2. **No `unwrap()` / `expect()` without `// JUSTIFIED:`** comment in non-test code.
3. **No task is DONE without QA 🟢 sign-off.**
4. **No security gate task ships without Security 🔴 approval.**
5. **Cognitive complexity ≤ 15** per Rust function. Enforced by clippy.
6. **Coverage ≥ 80%** Rust, ≥ 70% frontend. Enforced by CI.
7. **All `unsafe` blocks have `// SAFETY:` comments.**
8. **Filenames from raw disk are always sanitised before use as filesystem paths.**
9. **Designer 🟣 is consulted in chat first** to establish design system before P0-T03 runs.
10. **All ambiguities go to TPM 🟠 via `[TPM_QUERY]`** — agents do not make unilateral scope decisions.

---

## How to Raise a Query

```
[TPM_QUERY]
From: <agent colour and name>
Phase: P<N>
Task: <task_id>
Question: <specific question>
Options: [A] ... [B] ... [C] ...
Blocking: yes | no
```

Post to the task's planning doc and tag `[TPM]`.

---

## Key References

- Research & risk analysis: `docs/research/research-analysis-report.md`
- Feature phases & task IDs: `docs/features/feature-breakdown-phases.md`
- TPM decisions: `docs/agent-planning/decisions.md`
- Design tokens & brand: `docs/design/design-brief.md`
- Human setup guide: `README.md`