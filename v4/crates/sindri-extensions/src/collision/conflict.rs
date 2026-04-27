//! Conflict-rule helpers for collision resolution.
//!
//! When [`super::detection`] reports a [`super::detection::PrefixOverlap`],
//! the resolver consults [`ConflictRule`] to decide whether the overlap is
//! tolerable (e.g. one component declared
//! [`super::scenarios::Scenario::Skip`]) or whether to surface a hard error.
//!
//! Wave 2B keeps this minimal: any overlap between two components claiming
//! distinct prefixes that share a path-segment ancestor is rejected unless
//! one component is from the core registry (which acts as the arbiter).

/// Outcome of consulting the conflict rules for an overlap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictRule {
    /// Hard reject — surface a `CollisionViolation` error.
    Reject {
        /// Why the overlap was rejected.
        reason: String,
    },
    /// Tolerated — log a warning but proceed.
    Tolerate {
        /// Reason for the warning log.
        reason: String,
    },
}
