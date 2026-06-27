# Feature Breakdown & Development Phases
**Version:** 1.0  
**Format:** Agentic AI consumption  
**Stack:** Tauri 2 · Rust backend · Svelte frontend  
**Platforms:** macOS (primary), Windows (secondary)

---

## Notation

- `[DEV]` → Developer agent task
- `[QA]` → QA agent task (required before marking complete)
- `[SEC]` → Security agent review gate
- `[DES]` → Designer agent task
- `[TPM]` → TPM coordination / decision gate
- `BLOCKS:` → Cannot start until listed task IDs are complete
- `PLANNING:` → Agent must write plan to `docs/agent-planning/<seq>-<phase>-<subtask>.md`

---

## Phase 0 — Foundation & Tooling
*Goal: Runnable Tauri 2 shell, CI pipeline, code quality gates, design system stub.*

### P0-T01 — Project Scaffold
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/01-p0-project-scaffold.md`
- Tasks:
  - `cargo create-tauri-app` with Svelte + TypeScript frontend
  - Set `edition = "2021"` in `Cargo.toml`
  - Configure workspace: `crates/core`, `crates/disk`, `crates/recovery`
  - `.cargo/config.toml`: `[build] rustflags = ["-D", "warnings"]`
  - `#![deny(clippy::all, clippy::pedantic, clippy::cognitive_complexity)]` in each crate root
  - `clippy.toml`: `cognitive-complexity-threshold = 15`
  - `rustfmt.toml`: `max_width = 100`, `edition = "2021"`
  - `.editorconfig`, `.gitignore`, `Makefile` (targets: dev, build, test, lint, coverage)
- **QA Gate:** `[QA]` — `cargo clippy` passes zero warnings; `cargo fmt --check` passes; Tauri dev window opens

### P0-T02 — CI Pipeline
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/02-p0-ci-pipeline.md`
- Tasks:
  - GitHub Actions: `.github/workflows/ci.yml`
  - Jobs: `lint`, `test`, `coverage`, `build-mac`, `build-win`
  - `cargo-llvm-cov` for coverage; fail if < 80%
  - Tauri build matrix: `macos-latest`, `windows-latest`
  - Cache: `~/.cargo/registry`, `~/.cargo/git`, `node_modules`
- **QA Gate:** `[QA]` — CI passes on main branch; coverage gate enforced

### P0-T03 — Design System Stub
- **Agent:** `[DES]`
- **PLANNING:** `docs/agent-planning/03-p0-design-system.md`
- Tasks:
  - CSS custom properties: colour palette, typography scale, spacing tokens
  - Component stubs: `<Button>`, `<Card>`, `<Badge>`, `<ProgressBar>`, `<Modal>`, `<DiskList>`, `<FileTable>`
  - Dark mode as default (system preference respected)
  - Svelte component library structure: `src/lib/components/`
- **NOTE:** Designer agent operates via chat first to establish brand identity before this task runs
- **QA Gate:** `[QA]` — Visual regression baseline captured; components render in Storybook or isolated test

### P0-T04 — Tauri Entitlements & Permissions
- **Agent:** `[DEV]` + `[SEC]`
- **PLANNING:** `docs/agent-planning/04-p0-entitlements.md`
- Tasks:
  - `src-tauri/entitlements.mac.plist`: `com.apple.security.files.all` (Full Disk Access)
  - `src-tauri/tauri.conf.json` → `bundle.macOS.entitlements`
  - Windows UAC: `requestedExecutionLevel = requireAdministrator` in manifest
  - Tauri capability files: `src-tauri/capabilities/` — only expose required IPC commands
  - Tauri `allowlist` → deny all by default; explicitly allow per command
- **SEC Gate:** `[SEC]` — Review entitlement scope; confirm no over-permission
- **QA Gate:** `[QA]` — App launches without crash on both platforms with correct permissions

---

## Phase 1 — Disk Discovery
*Goal: List all physical disks and logical volumes; detect filesystem type; detect encryption state.*

### P1-T01 — Disk Enumeration (macOS)
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/05-p1-disk-enum-macos.md`
- BLOCKS: `P0-T01`, `P0-T04`
- Tasks:
  - Crate: `crates/disk/src/macos/enumerate.rs`
  - Use `ioreg` via `libc::ioctl` or parse `diskutil list -plist` output via `std::process::Command`
  - Return: `Vec<DiskInfo>` — `{ id, name, size_bytes, disk_type (SSD|HDD|NVMe|USB|Virtual), mount_points, filesystem (APFS|HFS+|FAT32|ExFAT|Unknown), encrypted: bool, trim_enabled: bool }`
  - APFS container → volume hierarchy: container → volume list
  - Detect TRIM via `IOKit` `DKIOCGETFEATURES`
- **QA Gate:** `[QA]` — Table tests: internal SSD, USB HDD, encrypted volume, APFS container with multiple volumes; all fields populated correctly
- **PLANNING required tests:** `docs/agent-planning/05-p1-disk-enum-macos.md` must include test matrix

### P1-T02 — Disk Enumeration (Windows)
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/06-p1-disk-enum-windows.md`
- BLOCKS: `P0-T01`, `P0-T04`
- Tasks:
  - Crate: `crates/disk/src/windows/enumerate.rs`
  - `DeviceIoControl` with `IOCTL_DISK_GET_DRIVE_LAYOUT_EX`
  - WMI or `SetupDiGetDeviceRegistryProperty` for drive type
  - Return same `DiskInfo` struct (shared in `crates/disk/src/types.rs`)
- **QA Gate:** `[QA]` — Same test matrix as macOS equivalent

### P1-T03 — Disk List UI
- **Agent:** `[DES]` + `[DEV]`
- **PLANNING:** `docs/agent-planning/07-p1-disk-list-ui.md`
- BLOCKS: `P0-T03`, `P1-T01`
- Tasks:
  - Svelte component: `<DiskList />` — renders `Vec<DiskInfo>` from Tauri command
  - Show: disk icon (SSD/HDD/USB), name, size, filesystem badge, encryption lock icon
  - Disabled state for encrypted/system volumes
  - Empty state: no disks found, permission guidance
  - Tauri command: `get_disks() -> Result<Vec<DiskInfo>, AppError>`
- **QA Gate:** `[QA]` — Vitest component tests; all states rendered; keyboard accessible

### P1-T04 — Permission Onboarding Flow
- **Agent:** `[DES]` + `[DEV]`
- **PLANNING:** `docs/agent-planning/08-p1-permission-onboarding.md`
- BLOCKS: `P0-T03`, `P0-T04`
- Tasks:
  - On launch: check TCC Full Disk Access status via Tauri command
  - If denied: show modal with step-by-step screenshots guide to System Settings → Privacy → Full Disk Access
  - macOS: `AXIsProcessTrusted()` equivalent for FDA check via `libc`
  - Windows: check admin token via `IsUserAnAdmin()`
  - Retry button re-checks permission without app restart (where possible)
- **QA Gate:** `[QA]` — Simulate denied permission; modal appears; granted permission; modal dismissed

---

## Phase 2 — File System Scanning
*Goal: Scan selected disk for deleted files; return structured metadata; stream results progressively.*

### P2-T01 — APFS Parser (macOS)
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/09-p2-apfs-parser.md`
- BLOCKS: `P1-T01`
- Tasks:
  - Crate: `crates/disk/src/macos/apfs/`
  - Parse APFS container superblock (`NX_Superblock`) from raw `/dev/rdiskN`
  - Traverse Object Map B-tree → locate Checkpoint Descriptor Area
  - Parse File System Tree for records with `DREC_TYPE_FILE` where inode is not in active catalog
  - Cross-reference Space Manager free-space bitmap
  - Detect APFS snapshots; enumerate snapshot volumes as secondary scan targets
  - Return stream: `tokio::sync::mpsc::Sender<DeletedFileEntry>` (async streaming)
  - `DeletedFileEntry`: `{ inode_id, name: Option<String>, size_bytes, deleted_at: Option<SystemTime>, extents: Vec<(u64, u64)>, filesystem: FsType }`
  - **Fuzz target:** `crates/disk/fuzz/fuzz_targets/apfs_parser.rs`
- **SEC Gate:** `[SEC]` — All `unsafe` blocks reviewed; buffer overread protection; sector-aligned reads only
- **QA Gate:** `[QA]` — Unit tests with captured binary fixtures of APFS metadata blocks; fuzz CI passes

### P2-T02 — HFS+ Parser (macOS)
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/10-p2-hfsplus-parser.md`
- BLOCKS: `P1-T01`
- Tasks:
  - Crate: `crates/disk/src/macos/hfsplus/`
  - Parse Volume Header → locate Catalog File B*-tree
  - Iterate leaf nodes for records with `kHFSFileRecord` type flagged deleted
  - Cross-reference Allocation File bitmap
  - Same stream interface as APFS parser

### P2-T03 — NTFS Parser (Windows)
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/11-p2-ntfs-parser.md`
- BLOCKS: `P1-T02`
- Tasks:
  - Crate: `crates/disk/src/windows/ntfs/`
  - Open `\\.\PhysicalDriveN` → read MFT (`$MFT`) via sector-aligned `ReadFile`
  - Iterate MFT records; filter `IN_USE` flag == 0 (deleted)
  - Parse `$STANDARD_INFORMATION`, `$FILE_NAME`, `$DATA` attributes
  - `$UsnJrnl` ($EXTEND\$UsnJrnl) scan for recently deleted entries
  - **Fuzz target:** `crates/disk/fuzz/fuzz_targets/ntfs_parser.rs`
- **SEC Gate:** `[SEC]` — Windows path traversal prevention; handle `ERROR_SHARING_VIOLATION` gracefully
- **QA Gate:** `[QA]` — Binary fixtures; table tests; fuzz passes

### P2-T04 — Scan Progress Streaming
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/12-p2-scan-progress.md`
- BLOCKS: `P2-T01` OR `P2-T03` (can parallel)
- Tasks:
  - Tauri event: `scan:progress` → `{ scanned_bytes, total_bytes, files_found: u64, current_path: Option<String> }`
  - Tauri event: `scan:file_found` → `DeletedFileEntry` (streamed one by one)
  - Tauri event: `scan:complete` → `{ total_found, duration_ms }`
  - Tauri event: `scan:error` → `{ message, recoverable: bool }`
  - Cancel: `scan:cancel` command → graceful abort via `CancellationToken`
  - State machine: `Idle → Scanning → Complete | Cancelled | Error`

### P2-T05 — Scan Results UI
- **Agent:** `[DES]` + `[DEV]`
- **PLANNING:** `docs/agent-planning/13-p2-scan-results-ui.md`
- BLOCKS: `P0-T03`, `P2-T04`
- Tasks:
  - `<FileTable />` — virtual/windowed list (handles 10k+ entries without layout thrash)
  - Columns: icon (inferred type), name, size, deleted date, probability badge (post P3)
  - Sort: by name, size, date, probability
  - Filter: by file type, date range, recoverable only
  - Scan progress bar with live file count and bytes scanned
  - Cancel button
- **QA Gate:** `[QA]` — Render 10,000 rows without jank; filter/sort correctness; cancel terminates scan

---

## Phase 3 — Recovery Probability Assessment
*Goal: On-click assessment of how likely a specific file is recoverable.*

### P3-T01 — Block Status Checker
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/14-p3-block-checker.md`
- BLOCKS: `P2-T01` OR `P2-T03`
- Tasks:
  - Crate: `crates/recovery/src/probability.rs`
  - Input: `DeletedFileEntry` with extent list
  - For each extent: check allocation bitmap → is block still free?
  - Read first 512 bytes of each block → detect null-fill (TRIM zeroed) or pattern (OS zeroed)
  - Detect SSD + TRIM active → downgrade probability
  - Drive type lookup from `DiskInfo` (from Phase 1)
  - Score function: `fn assess_probability(entry: &DeletedFileEntry, disk: &DiskInfo) -> ProbabilityReport`
  - `ProbabilityReport`: `{ tier: High|Medium|Low, free_blocks_pct: f32, trim_active: bool, blocks_zeroed: bool, estimated_recoverable_bytes: u64, warnings: Vec<String> }`
- **QA Gate:** `[QA]` — Table tests: HDD free blocks → High; SSD TRIM → Low; partial blocks → Medium; all branches covered

### P3-T02 — Probability UI
- **Agent:** `[DES]` + `[DEV]`
- **PLANNING:** `docs/agent-planning/15-p3-probability-ui.md`
- BLOCKS: `P0-T03`, `P3-T01`
- Tasks:
  - On row click → trigger `check_probability(inode_id)` command
  - Loading state on row (spinner)
  - Result: inline expandable panel below row
  - Show: probability tier badge (🟢/🟡/🔴), breakdown bars, warnings, estimated recoverable bytes
  - Tauri command: `check_probability(inode_id: u64, disk_id: String) -> Result<ProbabilityReport, AppError>`
- **QA Gate:** `[QA]` — Loading state renders; all three tiers display correctly; errors handled

---

## Phase 4 — File Recovery
*Goal: Recover selected file(s) from source disk to a user-selected destination.*

### P4-T01 — Destination Picker & Pre-flight
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/16-p4-destination-preflight.md`
- BLOCKS: `P3-T01`
- Tasks:
  - Tauri command: `pick_destination_folder() -> Result<String, AppError>` (native folder picker)
  - Pre-flight checks (ALL must pass before recovery starts):
    1. `destination != source` (same physical disk check — compare disk serial)
    2. `available_space(destination) >= file.estimated_recoverable_bytes * 1.1` (10% buffer)
    3. Destination path writable (attempt temp file write)
    4. Source disk still connected and readable
  - Return: `PreflightResult { ok: bool, errors: Vec<PreflightError> }`
- **QA Gate:** `[QA]` — All edge case checks: same disk → error; insufficient space → error; read-only dest → error

### P4-T02 — Recovery Engine
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/17-p4-recovery-engine.md`
- BLOCKS: `P4-T01`
- Tasks:
  - Crate: `crates/recovery/src/engine.rs`
  - Algorithm:
    1. Open source disk `O_RDONLY`
    2. For each extent in `DeletedFileEntry.extents`: read `block_size` bytes
    3. Write to `<dest>/<original_name_or_recovered_hash>.<ext>.partial` (temp name)
    4. On completion: `fs::rename()` to final name (atomic)
    5. On error/cancel: delete `.partial` file
  - Progress: emit `recovery:progress` event per block written
  - Bad sector handling: `[SEC]` reviewed `unsafe` read; on `SIGBUS`/`EIO` → skip block, log, continue
  - File naming: original name if available; else `recovered_<sha256_of_first_block_truncated>.<inferred_ext>`
  - Magic byte detection: `fn infer_extension(header: &[u8; 16]) -> &'static str` — covers JPEG, PNG, PDF, ZIP, DOCX, MP4, MOV common cases
  - `CancellationToken` supported
- **SEC Gate:** `[SEC]` — Path traversal on filename from disk metadata; sanitise all filenames from raw disk
- **QA Gate:** `[QA]` — Unit test with mock disk fixture; cancel mid-recovery cleans up; bad sector continues; atomic rename verified

### P4-T03 — Recovery Progress UI
- **Agent:** `[DES]` + `[DEV]`
- **PLANNING:** `docs/agent-planning/18-p4-recovery-ui.md`
- BLOCKS: `P0-T03`, `P4-T02`
- Tasks:
  - Recovery modal: confirmation with file list, sizes, destination, probability warnings
  - Progress view: per-file progress bar, overall progress, transfer speed, ETA
  - Completion view: success list, skipped (bad sectors), failed list with reasons
  - Cancel button → graceful abort
  - Open destination folder button on completion (Tauri `shell::open`)
- **QA Gate:** `[QA]` — All states (confirming, recovering, complete, error, cancelled) rendered and transition correctly

### P4-T04 — Audit Log
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/19-p4-audit-log.md`
- BLOCKS: `P4-T02`
- Tasks:
  - Write to `~/.local/share/fileresque/audit.log` (macOS: `~/Library/Application Support/fileresque/`)
  - Log format: JSONL (one JSON object per line)
  - Each entry: `{ timestamp, source_disk, inode_id, original_name, dest_path, sha256_dest, status, blocks_read, bytes_written, duration_ms }`
  - Rotation: max 10MB, keep last 5 files
- **QA Gate:** `[QA]` — Log written on recovery; rotation triggered at 10MB; fields all present

---

## Phase 5 — Polish & Hardening
*Goal: Production-ready. Crash resilience, code signing, packaging, accessibility.*

### P5-T01 — Code Signing & Notarisation (macOS)
- **Agent:** `[DEV]` + `[SEC]`
- **PLANNING:** `docs/agent-planning/20-p5-codesign.md`
- Tasks:
  - GitHub Actions secret: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_TEAM_ID`
  - Tauri CI: `tauri action` with signing config
  - Notarisation: `xcrun notarytool submit`
  - Verify with `spctl --assess`
- **SEC Gate:** `[SEC]` — Signing cert scope is minimal; no private key in repo

### P5-T02 — Windows Installer (NSIS / MSI)
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/21-p5-windows-installer.md`
- Tasks:
  - Tauri `bundle.windows.nsis` config
  - UAC elevation baked into installer and app manifest
  - Code signing via `signtool` in CI

### P5-T03 — Error Boundaries & Crash Recovery
- **Agent:** `[DEV]`
- **PLANNING:** `docs/agent-planning/22-p5-error-recovery.md`
- Tasks:
  - `std::panic::set_hook` in main → log panic to file before exit
  - All `AppError` variants mapped to user-friendly messages (no raw OS errors in UI)
  - Frontend: Svelte error boundary component wrapping each major view
  - Disconnection detection: poll source disk every 2s during scan/recovery; emit `disk:disconnected` event

### P5-T04 — Accessibility Audit
- **Agent:** `[DES]` + `[QA]`
- **PLANNING:** `docs/agent-planning/23-p5-accessibility.md`
- Tasks:
  - WCAG 2.1 AA compliance
  - Keyboard navigation: all actions reachable without mouse
  - ARIA labels on icon-only buttons
  - Screen reader test: VoiceOver (macOS), NVDA (Windows)
  - Colour contrast ≥ 4.5:1 for all text

### P5-T05 — Final Security Audit
- **Agent:** `[SEC]`
- **PLANNING:** `docs/agent-planning/24-p5-security-audit.md`
- BLOCKS: All Phase 0–4 tasks complete
- Tasks:
  - Review all `unsafe` blocks (must have `// SAFETY:` comment)
  - Verify no network code paths exist (`cargo audit` + manual check)
  - Verify IPC command surface: only expected commands exposed
  - Fuzz corpus review
  - Dependency audit: `cargo audit` — zero high/critical CVEs
  - `cargo deny` — licence compliance check

---

## Ambiguity Resolution Protocol

When any agent encounters a specification gap or decision point:

1. Agent writes ambiguity to its planning doc under `## Open Questions`
2. Agent emits a `[TPM_QUERY]` block:
   ```
   [TPM_QUERY]
   From: <agent>
   Phase: <phase>
   Task: <task_id>
   Question: <clear question>
   Options: [A] ... [B] ... [C] ...
   Blocking: yes|no
   ```
3. TPM responds in `docs/agent-planning/decisions.md` with decision + rationale
4. If security-adjacent → also route to `[SEC]`
5. If UX-adjacent → also route to `[DES]`

---

## Task Completion Checklist

Before any task is marked **DONE**, the responsible agent must confirm:

- [ ] Code passes `cargo clippy` with zero warnings
- [ ] `cargo fmt` applied
- [ ] Cognitive complexity ≤ 15 on all new functions
- [ ] Unit tests written (table-driven where applicable); coverage ≥ 80%
- [ ] Planning doc updated with actual implementation notes
- [ ] `[QA]` agent has run test suite and signed off
- [ ] `[SEC]` sign-off obtained on any task with security gate
- [ ] No `unwrap()` or `expect()` in non-test code without documented justification
- [ ] All `unsafe` blocks have `// SAFETY:` explanation comment