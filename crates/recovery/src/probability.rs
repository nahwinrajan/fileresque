use fileresque_core::{
    error::AppError,
    types::{DeletedFileEntry, DiskInfo, DriveType, ProbabilityReport, ProbabilityTier},
};

/// Leading bytes of each block read for zero-fill detection.
const SAMPLE_HEAD_BYTES: usize = 512;

/// Upper bound on blocks sampled per file. Bounds I/O on multi-GB files whose
/// extent counts are attacker-influenced (read from raw disk metadata).
const MAX_BLOCKS_SAMPLED: u64 = 4096;

/// Free-block percentage at or above which a file is rated [`ProbabilityTier::High`].
const HIGH_FREE_PCT: f32 = 90.0;

/// Free-block percentage at or above which a file is rated [`ProbabilityTier::Medium`].
const MEDIUM_FREE_PCT: f32 = 50.0;

/// Abstraction over the on-disk facts the probability engine needs.
///
/// The production implementation (P3-T02, in the Tauri layer) reads the raw
/// block device; unit tests use an in-memory mock. Keeping I/O behind this
/// trait lets the scoring logic be table-tested without disk hardware.
pub trait BlockProbe {
    /// Filesystem block size in bytes.
    fn block_size(&self) -> u64;

    /// Whether `block_addr` is still marked FREE in the filesystem allocation
    /// bitmap (i.e. not yet reused by a live file).
    ///
    /// Returns `Ok(None)` when the allocation state is unknown — e.g. no
    /// free-space bitmap has been parsed for this volume yet.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] if the underlying bitmap read fails.
    fn is_free(&mut self, block_addr: u64) -> Result<Option<bool>, AppError>;

    /// Read up to `len` leading bytes of `block_addr` for zero-fill detection.
    ///
    /// # Errors
    ///
    /// Returns [`AppError`] if the block read fails.
    fn read_head(&mut self, block_addr: u64, len: usize) -> Result<Vec<u8>, AppError>;
}

/// Running tally accumulated while sampling a file's blocks.
#[derive(Default)]
struct Tally {
    sampled: u64,
    free_known: u64,
    free_count: u64,
    zeroed_count: u64,
}

/// Derived facts about a file's blocks, summarised from a [`Tally`]. Grouping
/// the flags into one struct keeps helper signatures small (avoids passing many
/// loose booleans).
// Private summary DTO: each flag is an independent observation, not a state
// machine, so distinct booleans read clearer than two-variant enums here.
#[allow(clippy::struct_excessive_bools)]
struct Facts {
    has_blocks: bool,
    alloc_known: bool,
    trim_active: bool,
    blocks_zeroed: bool,
    partial_reuse: bool,
    free_blocks_pct: f32,
}

/// Assess the recovery probability for a single deleted file.
///
/// Walks the file's extents, samples up to [`MAX_BLOCKS_SAMPLED`] blocks via
/// `probe`, and classifies the result into a [`ProbabilityTier`].
///
/// # Errors
///
/// Returns [`AppError`] if `probe` fails during block status checks.
pub fn assess_probability<P: BlockProbe>(
    entry: &DeletedFileEntry,
    disk: &DiskInfo,
    probe: &mut P,
) -> Result<ProbabilityReport, AppError> {
    let trim_active = trim_active(disk);
    let block_addrs = sample_block_addrs(&entry.extents);

    let mut tally = Tally::default();
    for addr in &block_addrs {
        tally.sampled += 1;
        if let Some(free) = probe.is_free(*addr)? {
            tally.free_known += 1;
            if free {
                tally.free_count += 1;
            }
        }
        let head = probe.read_head(*addr, SAMPLE_HEAD_BYTES)?;
        if is_all_zero(&head) {
            tally.zeroed_count += 1;
        }
    }

    Ok(build_report(entry, &tally, trim_active))
}

/// TRIM discards freed blocks on SSD/NVMe; an active TRIM means freed data is
/// likely already gone even if a sampled block has not yet been zeroed.
fn trim_active(disk: &DiskInfo) -> bool {
    matches!(disk.drive_type, DriveType::SSD | DriveType::NVMe) && disk.trim_enabled
}

/// True when the buffer is empty or contains only zero bytes.
fn is_all_zero(buf: &[u8]) -> bool {
    buf.iter().all(|&b| b == 0)
}

/// Even-spaced sample of block addresses across all extents, capped at
/// [`MAX_BLOCKS_SAMPLED`]. Each extent is `(block_offset, block_count)`.
// Casts are bounded: counts and indices fit comfortably in the target widths
// after the MAX_BLOCKS_SAMPLED cap; precision loss on the stride is irrelevant.
#[allow(clippy::cast_possible_truncation)]
fn sample_block_addrs(extents: &[(u64, u64)]) -> Vec<u64> {
    let mut all: Vec<u64> = Vec::new();
    for &(offset, count) in extents {
        for i in 0..count {
            all.push(offset.saturating_add(i));
        }
    }

    let total = all.len() as u64;
    if total <= MAX_BLOCKS_SAMPLED {
        return all;
    }

    // Stride sampling keeps the set evenly distributed across the file.
    let stride = total / MAX_BLOCKS_SAMPLED;
    all.into_iter()
        .step_by(stride.max(1) as usize)
        .take(MAX_BLOCKS_SAMPLED as usize)
        .collect()
}

/// Summarise a [`Tally`] into derived [`Facts`].
// Casts: tallies are bounded by MAX_BLOCKS_SAMPLED; f32 precision is ample for a
// percentage that is only compared against coarse thresholds.
#[allow(clippy::cast_precision_loss)]
fn derive_facts(tally: &Tally, trim_active: bool) -> Facts {
    let has_blocks = tally.sampled > 0;
    let alloc_known = tally.free_known > 0;
    let free_blocks_pct = if alloc_known {
        (tally.free_count as f32 / tally.free_known as f32) * 100.0
    } else {
        0.0
    };
    Facts {
        has_blocks,
        alloc_known,
        trim_active,
        blocks_zeroed: has_blocks && tally.zeroed_count * 2 >= tally.sampled,
        partial_reuse: alloc_known && tally.free_count < tally.free_known,
        free_blocks_pct,
    }
}

/// Assemble the report: percentages, derived flags, tier, warnings.
fn build_report(entry: &DeletedFileEntry, tally: &Tally, trim_active: bool) -> ProbabilityReport {
    let facts = derive_facts(tally, trim_active);
    let tier = classify_tier(&facts);
    let estimated_recoverable_bytes = estimate_bytes(entry, tally, &tier);
    let warnings = build_warnings(&facts);

    ProbabilityReport {
        tier,
        free_blocks_pct: facts.free_blocks_pct,
        trim_active: facts.trim_active,
        blocks_zeroed: facts.blocks_zeroed,
        estimated_recoverable_bytes,
        warnings,
    }
}

/// Map block facts to a probability tier. See planning doc for the rule table.
fn classify_tier(facts: &Facts) -> ProbabilityTier {
    if !facts.has_blocks || facts.blocks_zeroed || facts.trim_active {
        return ProbabilityTier::Low;
    }
    if !facts.alloc_known {
        // Cannot confirm blocks are still free — stay conservative.
        return ProbabilityTier::Medium;
    }
    if facts.free_blocks_pct >= HIGH_FREE_PCT {
        ProbabilityTier::High
    } else if facts.free_blocks_pct >= MEDIUM_FREE_PCT {
        ProbabilityTier::Medium
    } else {
        ProbabilityTier::Low
    }
}

/// Estimate recoverable bytes: file size scaled by the observed recoverable
/// fraction (non-zero blocks, gated by the free fraction when known).
// Casts: bounded tallies → f32 fraction in [0,1]; the product is clamped to
// `size_bytes`, so truncation/sign loss cannot produce an invalid value.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn estimate_bytes(entry: &DeletedFileEntry, tally: &Tally, tier: &ProbabilityTier) -> u64 {
    if tally.sampled == 0 || matches!(tier, ProbabilityTier::Low) {
        return 0;
    }

    let non_zero = (tally.sampled - tally.zeroed_count) as f32 / tally.sampled as f32;
    let fraction = if tally.free_known > 0 {
        let free = tally.free_count as f32 / tally.free_known as f32;
        non_zero.min(free)
    } else {
        non_zero
    };

    let estimate = (entry.size_bytes as f32 * fraction) as u64;
    estimate.min(entry.size_bytes)
}

/// Collect human-readable warnings describing the assessment's caveats.
fn build_warnings(facts: &Facts) -> Vec<String> {
    let mut warnings = Vec::new();
    if !facts.has_blocks {
        warnings.push("No block extents recorded; only file metadata survives.".to_string());
        return warnings;
    }
    if facts.trim_active {
        warnings.push("TRIM is active on this SSD; freed blocks are likely discarded.".to_string());
    }
    if facts.blocks_zeroed {
        warnings.push("Most sampled blocks are zero-filled; data appears erased.".to_string());
    }
    if !facts.alloc_known {
        warnings
            .push("Allocation state could not be confirmed; estimate is conservative.".to_string());
    } else if facts.partial_reuse {
        warnings.push("Some blocks have been reused by live files.".to_string());
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use fileresque_core::types::FileSystem;
    use std::collections::HashMap;

    /// In-memory probe. `free` maps `block_addr` → allocation state; `zeroed`
    /// lists `block_addr`s whose head reads back all-zero.
    struct MockProbe {
        block_size: u64,
        free: HashMap<u64, Option<bool>>,
        zeroed: Vec<u64>,
        default_free: Option<bool>,
    }

    impl BlockProbe for MockProbe {
        fn block_size(&self) -> u64 {
            self.block_size
        }
        fn is_free(&mut self, block_addr: u64) -> Result<Option<bool>, AppError> {
            Ok(*self.free.get(&block_addr).unwrap_or(&self.default_free))
        }
        fn read_head(&mut self, block_addr: u64, len: usize) -> Result<Vec<u8>, AppError> {
            if self.zeroed.contains(&block_addr) {
                Ok(vec![0u8; len])
            } else {
                Ok(vec![0xABu8; len])
            }
        }
    }

    fn disk(drive: DriveType, trim: bool) -> DiskInfo {
        DiskInfo {
            id: "disk0".to_string(),
            display_name: "Test".to_string(),
            size_bytes: 1_000_000,
            drive_type: drive,
            filesystem: FileSystem::APFS,
            mount_points: vec![],
            encrypted: false,
            trim_enabled: trim,
            serial: None,
        }
    }

    fn entry(extents: Vec<(u64, u64)>, size: u64) -> DeletedFileEntry {
        DeletedFileEntry {
            inode_id: 1,
            name: Some("f.bin".to_string()),
            size_bytes: size,
            deleted_at: None,
            extents,
            filesystem: FileSystem::APFS,
        }
    }

    fn uniform_probe(free: Option<bool>, zeroed: Vec<u64>) -> MockProbe {
        MockProbe {
            block_size: 4096,
            free: HashMap::new(),
            zeroed,
            default_free: free,
        }
    }

    /// Build a probe where `[start, start+free_n)` are free and the rest reused.
    fn split_probe(start: u64, free_n: u64, total: u64) -> MockProbe {
        let mut free = HashMap::new();
        for a in start..start + total {
            free.insert(a, Some(a < start + free_n));
        }
        MockProbe {
            block_size: 4096,
            free,
            zeroed: vec![],
            default_free: Some(false),
        }
    }

    fn run(disk: &DiskInfo, entry: &DeletedFileEntry, mut p: MockProbe) -> ProbabilityReport {
        // JUSTIFIED: test-only; mock probe never errors
        assess_probability(entry, disk, &mut p).expect("mock probe must not fail")
    }

    #[test]
    fn assess_probability_tiers() {
        struct Case {
            name: &'static str,
            disk: DiskInfo,
            entry: DeletedFileEntry,
            probe: MockProbe,
            expected: ProbabilityTier,
        }

        let cases = vec![
            Case {
                name: "hdd_all_free_high",
                disk: disk(DriveType::HDD, false),
                entry: entry(vec![(100, 10)], 40_960),
                probe: uniform_probe(Some(true), vec![]),
                expected: ProbabilityTier::High,
            },
            Case {
                name: "ssd_trim_low",
                disk: disk(DriveType::SSD, true),
                entry: entry(vec![(100, 10)], 40_960),
                probe: uniform_probe(Some(true), vec![]),
                expected: ProbabilityTier::Low,
            },
            Case {
                name: "nvme_trim_low",
                disk: disk(DriveType::NVMe, true),
                entry: entry(vec![(100, 10)], 40_960),
                probe: uniform_probe(Some(true), vec![]),
                expected: ProbabilityTier::Low,
            },
            Case {
                name: "zeroed_blocks_low",
                disk: disk(DriveType::HDD, false),
                entry: entry(vec![(100, 4)], 16_384),
                probe: uniform_probe(Some(true), vec![100, 101, 102, 103]),
                expected: ProbabilityTier::Low,
            },
            Case {
                name: "no_extents_low",
                disk: disk(DriveType::HDD, false),
                entry: entry(vec![], 16_384),
                probe: uniform_probe(Some(true), vec![]),
                expected: ProbabilityTier::Low,
            },
            Case {
                name: "alloc_unknown_medium",
                disk: disk(DriveType::HDD, false),
                entry: entry(vec![(100, 10)], 40_960),
                probe: uniform_probe(None, vec![]),
                expected: ProbabilityTier::Medium,
            },
            Case {
                name: "hdd_reused_low",
                disk: disk(DriveType::HDD, false),
                entry: entry(vec![(100, 10)], 40_960),
                probe: split_probe(100, 1, 10), // 10% free
                expected: ProbabilityTier::Low,
            },
            Case {
                name: "hdd_partial_medium",
                disk: disk(DriveType::HDD, false),
                entry: entry(vec![(100, 10)], 40_960),
                probe: split_probe(100, 6, 10), // 60% free
                expected: ProbabilityTier::Medium,
            },
        ];

        for case in cases {
            let report = run(&case.disk, &case.entry, case.probe);
            assert_eq!(
                std::mem::discriminant(&report.tier),
                std::mem::discriminant(&case.expected),
                "FAILED case: {} — got {:?}",
                case.name,
                report.tier
            );
        }
    }

    #[test]
    fn estimate_never_exceeds_size() {
        let d = disk(DriveType::HDD, false);
        let e = entry(vec![(100, 10)], 40_960);
        let report = run(&d, &e, uniform_probe(Some(true), vec![]));
        assert!(
            report.estimated_recoverable_bytes <= e.size_bytes,
            "estimate must be capped at size_bytes"
        );
        assert!(
            report.estimated_recoverable_bytes > 0,
            "high-tier estimate must be non-zero"
        );
    }

    #[test]
    fn low_tier_estimates_zero() {
        let d = disk(DriveType::SSD, true); // trim → Low
        let e = entry(vec![(100, 10)], 40_960);
        let report = run(&d, &e, uniform_probe(Some(true), vec![]));
        assert_eq!(report.estimated_recoverable_bytes, 0);
        assert!(report.trim_active);
    }

    #[test]
    fn sample_caps_large_files() {
        let huge = vec![(0u64, MAX_BLOCKS_SAMPLED * 4)];
        let addrs = sample_block_addrs(&huge);
        assert!(
            addrs.len() as u64 <= MAX_BLOCKS_SAMPLED,
            "sample must be capped at MAX_BLOCKS_SAMPLED"
        );
    }

    #[test]
    fn warnings_present_for_edge_cases() {
        let d = disk(DriveType::HDD, false);
        let e = entry(vec![], 1024);
        let report = run(&d, &e, uniform_probe(Some(true), vec![]));
        assert!(
            !report.warnings.is_empty(),
            "expected metadata-only warning"
        );
    }
}
