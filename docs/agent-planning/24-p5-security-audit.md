# 24-p5-security-audit
**Task ID:** P5-T05
**Phase:** Phase 5 — Polish & Hardening
**Owner:** [SEC]
**Status:** Security Review (gated on CI security job — see Verdict)

---

## Overview
Final pre-launch security audit (🔴 Security, veto power). Reviews the whole codebase against FileResque's hard rules: zero network, every `unsafe` justified, raw-disk input sanitised before it touches the filesystem, and a minimal IPC/permission surface. Manual + static checks are complete and pass; the two automated supply-chain gates (CVE + licence) could not run in this offline environment and have been wired into CI to run where the advisory DB and network are available.

## Scope
- All `unsafe` blocks carry a `// SAFETY:` comment.
- No network code path exists in any production crate.
- IPC command surface = only the expected commands; Tauri capability/permission set is minimal.
- Raw-disk-derived filenames are sanitised before filesystem use.
- Dependency audit: CVEs (cargo-audit / `cargo deny check advisories`) and licence policy (`cargo deny check licenses`).
- Fuzz corpus review.

## Dependencies
- Blocked by: all Phase 0–4 tasks complete (they are; P5-T03/T04 also Done).

---

## Findings

### 1. `unsafe` blocks — ✅ PASS
12 `unsafe` blocks total, all annotated:
- `crates/disk/src/fsinfo.rs` ×2 — `statfs(2)` (macOS) and `GetDiskFreeSpaceExW` (Windows). Both zero-init the OUT struct, pass a NUL-terminated path that outlives the call, check the return code before reading. SAFETY comments accurate.
- `crates/disk/src/windows/ioctl.rs` ×10 — `CreateFileW`, `GetLastError`, `CloseHandle`, two `DeviceIoControl` IOCTLs, `mem::zeroed` on POD structs, `LARGE_INTEGER.QuadPart` union read, and a `&*ptr.cast::<STORAGE_DEVICE_DESCRIPTOR>()`. The pointer cast (line 258) is the sharpest edge: it is **bounds-checked** (`buf.len() >= size_of::<…>()` before the cast) and the SAFETY comment reasons explicitly about Windows heap alignment guarantees (8/16-byte) vs the struct's 4-byte requirement. Sound.

### 2. Network — ✅ PASS
- No `reqwest`/`hyper`/`ureq`/`isahc`/`surf`, no `std::net`/`tokio::net`/`TcpStream`/sockets in any crate (grep over `crates/`, `src-tauri/` returns only doc-comment URLs and the `SourceNotReadable` identifier).
- Dependency graph carries no HTTP **client** — only the `http` *types* crate (v1.4.2), pulled transitively by Tauri's custom protocol; it performs no I/O.
- No `tauri-plugin-http`, no updater plugin.

### 3. IPC / capability surface — ✅ PASS
- `invoke_handler` exposes exactly 9 commands: `get_disks`, `check_disk_access`, `start_scan`, `cancel_scan`, `check_probability`, `pick_destination_folder`, `preflight_recovery`, `recover_files`, `cancel_recovery`. All expected; no debug/test command leaks.
- `src-tauri/capabilities/default.json` is a deny-all baseline granting only `core:default` + `shell:allow-open`.
- `tauri-plugin-fs` is present in the lockfile (transitive via `tauri-plugin-dialog`) but is **not** initialised in `run()` and **no `fs:` permission** is granted — the frontend cannot reach arbitrary filesystem APIs. The folder picker runs entirely in Rust (`pick_destination_folder`), so no frontend `dialog:` capability is needed either (DECISION-018).

### 4. Filename sanitisation — ✅ PASS
`crates/recovery/src/engine.rs::sanitize_filename` strips to the final path component (defeats `../` and `..\` traversal), removes control chars, maps reserved chars (`/\:*?"<>|`) to `_`, trims dots/spaces, truncates to `MAX_NAME_LEN` bytes, and rejects empty strings + Windows reserved device names (CON/PRN/AUX/NUL/COM#/LPT#). Table-tested incl. `../../etc/passwd → passwd`, `evil\..\boot.ini → boot.ini`, `CON → None`. The Windows descriptor string read (`extract_offset_string`) is also bounds-checked (`offset == 0 || offset >= buf.len() → None`).

### 5. Dependency CVE / licence audit — ⚠️ DEFERRED TO CI (gating)
- `cargo-audit` and `cargo-deny` are **not installed** in this environment, and the RustSec advisory DB needs network (unavailable here) — so the "zero high/critical CVE" and licence checks **could not be executed locally**.
- **Remediation landed this task:** added `deny.toml` (advisories=deny+yanked, permissive licence allow-list, wildcard-deny, crates.io-only sources) and a `security` job to `.github/workflows/ci.yml` running `cargo deny check` + `cargo audit --deny warnings`.
- **Gate:** T05 cannot be signed Done until that CI job is observed green on the branch.

### 6. Fuzz corpus — ⚠️ GAP (recommendation)
- No `fuzz/` harness or `corpus/` exists. The Phase-2 filesystem parsers (APFS / HFS+ / NTFS) consume **untrusted on-disk structures** and are the highest-value fuzz targets in the codebase.
- They are defensively written (bounds checks, graceful `None`/error on malformed input) and unit-tested with synthetic fixtures, but there is no continuous fuzzing.
- **Recommendation:** add `cargo-fuzz` targets for each parser entry point (superblock / catalog / MFT record parse) before GA. Logged as a follow-up; not a code defect.

### 7. Hardening note — 🔧 tokio features
`tokio = { features = ["full"] }` (workspace) pulls in the `net` capability the app never uses. Cargo feature-unification means Tauri likely keeps `net` on regardless, so narrowing our declaration may not shrink the final binary — but requesting only what we use (`rt-multi-thread`, `macros`, `sync`, `time`) is correct hygiene for a zero-network app. Recommended, non-blocking.

---

## Verdict
**Code-level audit: PASS.** `unsafe` fully justified, zero network, minimal IPC surface, robust raw-input sanitisation, bounds-checked parsing.

**Conditional on CI:** the automated CVE + licence gates (items 5) are newly wired into CI and **must be green** before 🔴 Security signs T05 Done. Items 6 (fuzzing) and 7 (tokio features) are recommendations, not blockers.

**🔴 Security sign-off:** _withheld pending the CI `security` job passing on this branch._ All code-level gates approved.

## Open Questions / TPM Queries
- [TPM] Should continuous fuzzing (item 6) block GA, or ship MVP with the recommendation tracked as a fast-follow? Non-blocking for this task either way.
