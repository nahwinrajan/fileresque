# P3-T01 — Block Status Checker

**Phase:** 3
**Status:** In Progress
**Security Gate:** No (no `unsafe`; pure scoring + trait-mediated I/O)
**Agent:** Developer (🔵)

---

## Task Scope

Given a `DeletedFileEntry` (with extent list) and the `DiskInfo` of its source
disk, assess how likely the file is recoverable. Produce a `ProbabilityReport`
{ tier, free_blocks_pct, trim_active, blocks_zeroed, estimated_recoverable_bytes,
warnings }.

The space-manager / allocation-bitmap cross-reference that the Phase 2 scanners
explicitly deferred (see `apfs/scanner.rs` line ~450) lands here.

---

## Approach

Split the engine into two layers so the scoring is fully unit-testable without
real disk hardware:

1. **`BlockProbe` trait** — abstracts the per-block facts the engine needs:
   - `block_size()` — filesystem block size in bytes
   - `is_free(block_addr) -> Option<bool>` — is the block still marked FREE in
     the allocation bitmap? `None` = allocation state unknown (no bitmap yet).
   - `read_head(block_addr, len)` — leading bytes of the block, for zero-fill
     detection.

2. **`assess_probability<P: BlockProbe>(entry, disk, probe)`** — pure scoring.
   Synchronous; the Tauri command (P3-T02) runs it inside `spawn_blocking`
   (DECISION-005).

Scoring walks the file's extents, samples up to `MAX_BLOCKS_SAMPLED` blocks
(evenly spaced, to bound I/O on multi-GB files), tallies free vs zeroed, then
classifies into High / Medium / Low.

The recovery crate stays dependency-free (no `fileresque-disk` coupling). The
real probe over a raw device lives in the Tauri layer (P3-T02), where
`BlockReader` is already available.

### Classification rules

| Condition | Tier |
|-----------|------|
| no extents | Low |
| majority of sampled blocks zeroed | Low |
| TRIM active (SSD/NVMe + `trim_enabled`) | Low |
| allocation state unknown (no bitmap) | Medium |
| free_blocks_pct ≥ 90 | High |
| free_blocks_pct ≥ 50 | Medium |
| otherwise (blocks reused) | Low |

Thresholds: `HIGH_FREE_PCT = 90.0`, `MEDIUM_FREE_PCT = 50.0`,
`SAMPLE_HEAD_BYTES = 512`, `MAX_BLOCKS_SAMPLED = 4096`.

`estimated_recoverable_bytes` = `entry.size_bytes` scaled by the observed
recoverable fraction (non-zero blocks, gated by free fraction when known),
capped at `size_bytes`.

---

## Function Signatures

```rust
// crates/recovery/src/probability.rs
pub trait BlockProbe {
    fn block_size(&self) -> u64;
    fn is_free(&mut self, block_addr: u64) -> Result<Option<bool>, AppError>;
    fn read_head(&mut self, block_addr: u64, len: usize) -> Result<Vec<u8>, AppError>;
}

pub fn assess_probability<P: BlockProbe>(
    entry: &DeletedFileEntry,
    disk: &DiskInfo,
    probe: &mut P,
) -> Result<ProbabilityReport, AppError>;
```

Helpers (each ≤ 15 cognitive complexity): `classify_tier`, `estimate_bytes`,
`sample_block_addrs`.

---

## Edge Cases

- Empty `extents` → Low, warning "No block extents recorded".
- Extent `(_, 0)` (zero-count) → contributes no blocks.
- Huge file (> `MAX_BLOCKS_SAMPLED` blocks) → evenly sampled, not fully read.
- `is_free` returns `None` for all blocks → `free_blocks_pct` reported as 0.0
  with a "could not confirm allocation state" warning; tier capped at Medium.
- `read_head` returns fewer bytes than requested → treat as readable; only
  all-zero buffers count as zeroed.
- Probe I/O error → propagated as `AppError`.

---

## Test Matrix (table-driven, mock probe)

| Test | Disk | Blocks | Expected tier |
|------|------|--------|---------------|
| `hdd_all_free_high` | HDD, no trim | all free, non-zero | High |
| `ssd_trim_low` | SSD, trim_enabled | all free, non-zero | Low |
| `nvme_trim_low` | NVMe, trim_enabled | free | Low |
| `hdd_partial_medium` | HDD | 60% free | Medium |
| `hdd_reused_low` | HDD | 10% free | Low |
| `zeroed_blocks_low` | HDD | free but zeroed | Low |
| `no_extents_low` | HDD | none | Low |
| `alloc_unknown_medium` | HDD | is_free=None | Medium |
| `estimate_capped` | HDD | non-zero | est ≤ size_bytes |

Coverage target ≥ 80%.

---

## Security Notes

- No `unsafe`. All disk bytes treated as untrusted; only counted/zero-checked.
- Sampling cap bounds I/O — no unbounded read driven by attacker-controlled
  extent counts.

---

## Implementation Notes

Implemented in `crates/recovery/src/probability.rs`.

- `BlockProbe` trait with `block_size` / `is_free(-> Option<bool>)` / `read_head`.
  `is_free` returns `Option<bool>` so an unparsed allocation bitmap is modelled
  as "unknown" rather than guessed.
- `assess_probability` is **synchronous** (was an async stub). The async wrapper
  moved to the Tauri layer (`spawn_blocking`), matching the scan command pattern.
- Scoring split into small helpers — `derive_facts`, `classify_tier`,
  `estimate_bytes`, `build_warnings`, `sample_block_addrs`, `trim_active`,
  `is_all_zero` — each well under the cognitive-complexity-15 gate.
- Boolean flags grouped into a private `Facts` struct to keep helper signatures
  small (clippy `fn_params_excessive_bools`); `struct_excessive_bools` allowed on
  `Facts` with a reason (independent observations, not a state machine).
- Numeric casts (`u64`↔`f32`, stride `as usize`) carry function-level
  `#[allow(clippy::cast_*)]` with comments; ranges are bounded by
  `MAX_BLOCKS_SAMPLED` and the estimate is clamped to `size_bytes`.
- No `unsafe`. No `unwrap`/`expect` outside tests (test `expect`s carry
  `// JUSTIFIED:`).

### Verification

- `cargo clippy -p fileresque-recovery --all-targets` — 0 warnings.
- `cargo fmt --check` — clean.
- `cargo test -p fileresque-recovery` — 5 tests pass (8 tier scenarios table +
  estimate cap + low-tier-zero + sample cap + warnings).
- Coverage tool (`cargo-llvm-cov`) not installed in this environment, so the
  ≥80% number was not machine-measured here; every branch of every helper is
  exercised by the table tests (CI enforces the gate).

---

## Completion Checklist

- [x] `cargo clippy` — 0 warnings
- [x] `cargo fmt` — clean
- [x] Cognitive complexity ≤ 15 on all new functions
- [x] Unit tests written (table-driven); branch-complete (CI measures ≥ 80%)
- [x] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [x] All `unsafe` blocks have `// SAFETY:` (n/a — none)
- [x] Planning doc updated with `## Implementation Notes`
- [x] 🟢 QA sign-off appended to planning doc

---

## 🟢 QA Sign-off

**Agent:** QA (🟢) · **Date:** 2026-06-30 · **Status:** PASS (unit gate)

- Reviewed `assess_probability` and all helpers against the rule table in
  `## Approach`. Tier logic matches: HDD-free→High, SSD/NVMe-TRIM→Low,
  partial→Medium, reused→Low, zeroed→Low, no-extents→Low, alloc-unknown→Medium.
- `cargo test -p fileresque-recovery`: 5/5 pass. `cargo clippy --all-targets`:
  clean. `cargo fmt --check`: clean.
- Edge cases covered: empty extents, sample cap on oversized files, estimate
  clamped to `size_bytes`, low-tier → 0 estimate.
- No runtime/UI surface in this task (pure crate) → `make smoke` not applicable
  here; it applies to the P3-T02 UI task.

No security gate on this task (no `unsafe`, no OS calls, trait-mediated I/O).

### Addendum (2026-06-30) — allocation bitmap landed

The "check allocation bitmap → is block still free?" requirement that this task
listed (and that the Phase 2 APFS scanner explicitly deferred) is now
implemented in `crates/disk/src/macos/apfs/spaceman.rs` and consumed by the
P3-T02 `DeviceProbe`. See DECISION-017 (self-validating bitmap polarity). The
engine here was already built against the `BlockProbe::is_free -> Option<bool>`
contract, so it consumes the real bitmap with no change.
