use fileresque_core::{
    error::AppError,
    types::{DeletedFileEntry, DiskInfo, ProbabilityReport, ProbabilityTier},
};

/// Assess the recovery probability for a single deleted file.
///
/// This is a stub implementation. Full block-level analysis is in P3-T01.
///
/// # Errors
///
/// Returns [`AppError`] if disk I/O fails during block status checks.
// `async` is intentional: the real implementation in P3-T01 will await block I/O.
// The stub body has no awaits yet, hence the allow.
#[allow(clippy::unused_async)]
pub async fn assess_probability(
    _entry: &DeletedFileEntry,
    _disk: &DiskInfo,
) -> Result<ProbabilityReport, AppError> {
    // TODO(P3-T01): implement block status check via filesystem-specific parser
    Ok(ProbabilityReport {
        tier: ProbabilityTier::Medium,
        free_blocks_pct: 0.0,
        trim_active: false,
        blocks_zeroed: false,
        estimated_recoverable_bytes: 0,
        warnings: vec![],
    })
}
