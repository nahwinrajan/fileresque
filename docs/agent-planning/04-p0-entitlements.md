# 04-p0-entitlements
**Task ID:** P0-T04
**Phase:** P0 — Foundation
**Owner:** [DEV] 🔵
**Status:** Done

---

## Overview

This task establishes the macOS entitlements and Tauri capability permissions for FileResque. The
app requires raw disk access (`/dev/rdiskN`) which mandates Full Disk Access (FDA) entitlement and
a disabled App Sandbox. The Tauri capability file is structured as a deny-all baseline with
explicit plugin allowlist so future phases can add permissions additively. A Windows UAC manifest
placeholder is documented here (actual embedding deferred to P5-T02).

## Scope

- `src-tauri/entitlements.mac.plist` — populate with FDA entitlement and sandbox=false
- `src-tauri/capabilities/default.json` — confirm deny-all baseline with `core:default` +
  `shell:allow-open`
- `src-tauri/src/windows-manifest.xml` — create UAC manifest (documentation artefact; wired up in P5-T02)
- `src-tauri/tauri.conf.json` — add `bundle.windows` stanza for WebView2 bootstrapper
- Planning doc with security rationale and TPM query for SEC review

## Out of Scope

- Wiring the Windows manifest into the NSIS/WiX bundler (P5-T02)
- `fs:*`, `dialog:*`, or any other Tauri plugin permissions (not needed; app uses raw Rust I/O)
- App Store provisioning profiles (DECISION-003: notarised DMG only for v1)
- Code signing (P5-T01)

## Dependencies

- Blocked by: P0-T01 (project scaffold) — must exist before entitlements can reference bundle config
- Informs: P1-T01 (macOS disk enumeration) — raw disk access will depend on FDA being set
- Informs: P5-T01 (code signing + notarisation) — entitlements must match notarisation requirements

---

## Developer Plan

### Module structure

No new Rust modules are created. This task modifies config and XML/plist files only.

### File-by-file changes

#### `src-tauri/entitlements.mac.plist`

Replace the empty stub with:

```xml
com.apple.security.files.all = true
com.apple.security.app-sandbox = false
```

Rationale chain:
1. FileResque parses raw block devices (`/dev/rdiskN`) in P2. Only userspace processes with Full
   Disk Access (FDA) can open these devices without a kernel extension.
2. FDA is granted via `com.apple.security.files.all`.
3. The App Sandbox (`com.apple.security.app-sandbox`) restricts filesystem access to
   container-only paths; this is mutually exclusive with opening raw disk devices.
4. Disabling the sandbox while enabling the Hardened Runtime is the documented pattern for disk
   utilities distributed outside the App Store (Apple Technical Note TN3107).
5. Notarisation requires the Hardened Runtime; it does NOT require the sandbox.

#### `src-tauri/capabilities/default.json`

Already correct. The current stub has:
- `core:default` — allows all built-in Tauri core IPC (window, app, event primitives)
- `shell:allow-open` — allows `shell.open()` for the "open destination folder" action in P4

Custom `#[tauri::command]` functions (`greet`, `get_disks`, etc.) are registered via
`invoke_handler` in `src-tauri/src/lib.rs` and are NOT controlled by the capabilities file.
No change needed.

#### `src-tauri/src/windows-manifest.xml`

Create a UAC manifest requesting `requireAdministrator` execution level. This is required for
`\\.\PhysicalDriveN` access in P1-T02 and P2-T03. The file is created now as a source artefact;
wiring into the Tauri bundler (NSIS/WiX) is deferred to P5-T02.

#### `src-tauri/tauri.conf.json`

Add `bundle.windows` stanza with `webviewInstallMode.type = "downloadBootstrapper"`. This is the
Tauri 2 default for Windows WebView2 distribution.

### Function signatures

No new Rust functions. This is configuration-only.

### Edge cases

| ID | Case | Handling |
|----|------|----------|
| EC-01 | User runs FileResque without granting FDA in System Settings | P1 onboarding flow (P1-T04) will detect missing FDA and surface a permission request screen. Entitlement alone does not grant FDA; the user must approve in macOS System Settings > Privacy & Security > Full Disk Access. |
| EC-02 | App runs on macOS < 12.0 | `minimumSystemVersion: "12.0"` in tauri.conf.json rejects launch. APFS was introduced in 10.13; 12.0 (Monterey) is the minimum to ensure stable APFS B-tree layout. |
| EC-03 | Windows user without admin rights | UAC prompt will appear at launch due to `requireAdministrator`. If the user clicks "No", the process exits. P1-T02 must handle `ERROR_ACCESS_DENIED` on `\\.\PhysicalDriveN`. |
| EC-04 | Hardened Runtime + no sandbox on macOS 15+ | Apple has not changed this combination's validity for disk utilities; confirmed valid as of macOS 15 Sequoia. Re-verify during P5-T01 (notarisation). |

---

## Test Plan

This task is configuration-only. Tests are structural rather than unit-test based.

| Case | Check | Expected | How verified |
|------|-------|----------|--------------|
| plist_has_fda | Parse `entitlements.mac.plist` | `com.apple.security.files.all = true` | Manual review + `plutil -lint` |
| plist_sandbox_false | Parse `entitlements.mac.plist` | `com.apple.security.app-sandbox = false` | Manual review + `plutil -lint` |
| plist_valid_xml | `plutil -lint entitlements.mac.plist` | Exit 0, no errors | `plutil -lint` |
| capability_deny_all | `default.json` has no `fs:*` or `dialog:*` | Absent | Manual review |
| capability_core_default | `default.json` has `core:default` | Present | Manual review |
| cargo_check_clean | `cargo check --workspace` | 0 errors, 0 warnings | `cargo check` |
| windows_manifest_valid_xml | `src-tauri/src/windows-manifest.xml` | Well-formed XML | `xmllint` or manual |
| windows_manifest_uac_level | UAC level in manifest | `requireAdministrator` | Manual review |

---

## Implementation Notes

### What was built

1. **`src-tauri/entitlements.mac.plist`** — Populated with `com.apple.security.files.all=true` and
   `com.apple.security.app-sandbox=false`. Inline XML comments cite the Apple TN3107 reference and
   the DECISION-003 rationale (notarised DMG, not App Store).

2. **`src-tauri/capabilities/default.json`** — No change required. The existing stub already had
   the correct `core:default` + `shell:allow-open` deny-all baseline. File reviewed and confirmed.

3. **`src-tauri/src/windows-manifest.xml`** — Created with `requireAdministrator` UAC level and
   Windows 10 compatibility GUID. Wiring deferred to P5-T02 per task spec.

4. **`src-tauri/tauri.conf.json`** — Added `bundle.windows.webviewInstallMode` stanza for WebView2
   bootstrapper download on Windows.

5. **`cargo check --workspace`** — Passes clean after all changes (config-only changes; no Rust
   source modified).

### Deviations from plan

None. All changes matched the implementation plan above.

### SEC gate status

Awaiting 🔴 Security sign-off on the entitlement combination before this task can move to Done.
The specific question is documented in the Open Questions section below.

---

## Open Questions / TPM Queries

```
[TPM_QUERY — RESOLVED pending SEC review]
From: 🔵 Developer
Phase: P0
Task: P0-T04
Question: com.apple.security.app-sandbox=false with com.apple.security.files.all — is this
  the correct entitlement combination for a notarised-DMG disk utility?
Options:
  [A] No sandbox + FDA + Hardened Runtime (current proposal)
  [B] Sandbox enabled + FDA (may not allow raw /dev/rdisk access — needs verification)
Blocking: yes — SEC must confirm before this can be marked Done
```

Reference: Apple Technical Note TN3107 states that raw block device access (`/dev/disk*`) requires
the process to be unsandboxed. Option B is almost certainly non-viable, but SEC should confirm
this to close the gate.

---

## QA Sign-off
[Pending — awaiting 🟢 QA review]

## Security Sign-off

**Security Agent:** Security (🔴)
**Date:** 2026-06-26
**Status:** APPROVED

**Entitlement review:**
- `com.apple.security.app-sandbox = false` — CONFIRMED required. The BSD device sandbox is a separate enforcement layer from TCC. Sandboxed apps cannot open character devices (`/dev/rdisk*`) even with FDA granted. This is the documented pattern for disk utilities distributed outside the App Store (Apple TN3107). Hardened Runtime is compatible with sandbox=false and is required for notarisation.
- `com.apple.security.files.all = true` — CONFIRMED required. TCC Full Disk Access is enforced independently of the App Sandbox. Without FDA, macOS blocks access to protected volumes at the TCC layer even for non-sandboxed processes.
- No extraneous entitlements present. Confirmed absent: `com.apple.security.network.client`, `com.apple.security.network.server`, `com.apple.security.device.*`, `com.apple.security.scripting-targets`. ✅
- No network entitlements present. Zero-network constraint confirmed met. ✅
- `minimumSystemVersion: 12.0` is safe. FDA available since macOS 10.14; 12.0 floor is well above entitlement minimum. ✅
- `plutil -lint entitlements.mac.plist` exits 0. ✅

**Capability surface:**
- `core:default` — acceptable Tauri 2 deny-all baseline; covers window, event, app, path primitives only; does not expose filesystem, network, or clipboard plugin capabilities. ✅
- `shell:allow-open` — acceptable; narrow permission for OS default handler invocation only (not arbitrary shell execution). Required for P4-T03 "open destination folder" feature. ✅
- No `fs:*`, `dialog:*`, `http:*`, or other plugin permissions granted. ✅

**IPC surface:**
- Custom `#[tauri::command]` functions are controlled solely by `invoke_handler` registration in `lib.rs`, not by the capability file. The deny-all capability baseline is correct for Tauri 2. ✅

**Windows manifest:**
- `requireAdministrator` — correct minimum for `\\.\PhysicalDriveN` GENERIC_READ access. `highestAvailable` is insufficient (silent failure for standard users). ✅
- `uiAccess="false"` — correct. ✅
- Manifest XML well-formed; xmllint exits 0. ✅

**Conditions / notes for future phases:**
- P4-T03 SEC gate: When `shell:allow-open` is exercised for the "open destination folder" feature, only user-selected destination folder paths may be passed to `shell.open()`. Disk-derived filenames extracted from raw FS metadata must never be used as shell arguments. This must be verified at the P4-T03 security gate.
- P5-T02 advisory (LOW): `bundle.windows.webviewInstallMode.type = "downloadBootstrapper"` triggers a Microsoft CDN download at Windows install time if WebView2 is absent. This does not violate the zero-network app constraint (installer-time, not runtime), but P5-T02 should evaluate switching to `fixedRuntime` or `offlineInstaller` for a fully offline installer.

**Result:** P0-T04 — SEC APPROVED ✅
