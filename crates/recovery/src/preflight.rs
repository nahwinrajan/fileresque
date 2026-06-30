//! Pre-flight validation for a recovery operation (P4-T01).
//!
//! All four checks from the feature breakdown are evaluated here as **pure
//! logic** over a [`PreflightFacts`] struct that the caller gathers from the
//! platform (free space, destination device, writability, source liveness).
//! Keeping the decision pure makes every branch table-testable without touching
//! real disks — the platform-specific gathering lives in `crates/disk/fsinfo`
//! and the Tauri command layer.

use fileresque_core::types::{DeletedFileEntry, PreflightError, PreflightResult};

/// Facts gathered from the OS that the pre-flight decision depends on.
///
/// Each field answers exactly one of the four required checks, so the decision
/// function is a straight translation of facts → errors with no I/O.
#[derive(Debug, Clone)]
pub struct PreflightFacts {
    /// Destination resides on the same physical disk as the recovery source.
    /// Writing there could overwrite the very free blocks being recovered.
    pub same_disk: bool,
    /// Free bytes available on the destination volume.
    pub available_bytes: u64,
    /// A probe write to the destination succeeded.
    pub dest_writable: bool,
    /// The source disk is still connected and its raw device is readable.
    pub source_readable: bool,
}

/// Total bytes the selected entries require, plus a 10% safety buffer.
///
/// Uses `size_bytes` (the full file size) rather than the probability engine's
/// `estimated_recoverable_bytes`: `size_bytes >= estimated`, so reserving the
/// full size guarantees the recovered (possibly complete) file always fits. The
/// buffer is computed integer-only (`n + n/10`) to avoid float casts.
#[must_use]
pub fn required_bytes(entries: &[DeletedFileEntry]) -> u64 {
    let total: u64 = entries
        .iter()
        .fold(0u64, |acc, e| acc.saturating_add(e.size_bytes));
    total.saturating_add(total / 10)
}

/// Evaluate all pre-flight checks. `ok` is true only when every check passes;
/// otherwise `errors` lists each failure (checks are independent, so more than
/// one can fire at once).
#[must_use]
pub fn evaluate(required: u64, facts: &PreflightFacts) -> PreflightResult {
    let mut errors = Vec::new();

    if facts.same_disk {
        errors.push(PreflightError::SameDisk);
    }
    if facts.available_bytes < required {
        errors.push(PreflightError::InsufficientSpace {
            required,
            available: facts.available_bytes,
        });
    }
    if !facts.dest_writable {
        errors.push(PreflightError::DestinationNotWritable);
    }
    if !facts.source_readable {
        errors.push(PreflightError::SourceNotReadable);
    }

    PreflightResult {
        ok: errors.is_empty(),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fileresque_core::types::FileSystem;

    fn entry(size: u64) -> DeletedFileEntry {
        DeletedFileEntry {
            inode_id: 1,
            name: Some("f.bin".to_string()),
            size_bytes: size,
            deleted_at: None,
            extents: vec![],
            filesystem: FileSystem::APFS,
        }
    }

    fn ok_facts() -> PreflightFacts {
        PreflightFacts {
            same_disk: false,
            available_bytes: u64::MAX,
            dest_writable: true,
            source_readable: true,
        }
    }

    #[test]
    fn required_bytes_adds_ten_percent_buffer() {
        let cases = vec![
            ("single", vec![entry(1000)], 1100),
            ("sum_two", vec![entry(1000), entry(2000)], 3300),
            ("empty", vec![], 0),
        ];
        for (name, entries, expected) in cases {
            assert_eq!(required_bytes(&entries), expected, "FAILED case: {name}");
        }
    }

    #[test]
    fn required_bytes_saturates_on_overflow() {
        let huge = vec![entry(u64::MAX), entry(u64::MAX)];
        assert_eq!(
            required_bytes(&huge),
            u64::MAX,
            "must not panic on overflow"
        );
    }

    #[test]
    fn evaluate_all_pass_is_ok() {
        let result = evaluate(1100, &ok_facts());
        assert!(result.ok, "all checks pass → ok");
        assert!(result.errors.is_empty());
    }

    #[test]
    fn evaluate_flags_each_failure() {
        struct Case {
            name: &'static str,
            facts: PreflightFacts,
            required: u64,
            expect: fn(&PreflightError) -> bool,
        }

        let cases = vec![
            Case {
                name: "same_disk",
                facts: PreflightFacts {
                    same_disk: true,
                    ..ok_facts()
                },
                required: 0,
                expect: |e| matches!(e, PreflightError::SameDisk),
            },
            Case {
                name: "insufficient_space",
                facts: PreflightFacts {
                    available_bytes: 500,
                    ..ok_facts()
                },
                required: 1100,
                expect: |e| {
                    matches!(
                        e,
                        PreflightError::InsufficientSpace {
                            required: 1100,
                            available: 500
                        }
                    )
                },
            },
            Case {
                name: "read_only_dest",
                facts: PreflightFacts {
                    dest_writable: false,
                    ..ok_facts()
                },
                required: 0,
                expect: |e| matches!(e, PreflightError::DestinationNotWritable),
            },
            Case {
                name: "source_gone",
                facts: PreflightFacts {
                    source_readable: false,
                    ..ok_facts()
                },
                required: 0,
                expect: |e| matches!(e, PreflightError::SourceNotReadable),
            },
        ];

        for case in cases {
            let result = evaluate(case.required, &case.facts);
            assert!(!result.ok, "FAILED case: {} — expected not ok", case.name);
            assert!(
                result.errors.iter().any(case.expect),
                "FAILED case: {} — expected error variant missing from {:?}",
                case.name,
                result.errors
            );
        }
    }

    #[test]
    fn evaluate_reports_multiple_failures_at_once() {
        let facts = PreflightFacts {
            same_disk: true,
            available_bytes: 0,
            dest_writable: false,
            source_readable: false,
        };
        let result = evaluate(1000, &facts);
        assert!(!result.ok);
        assert_eq!(result.errors.len(), 4, "all four checks should fail");
    }
}
