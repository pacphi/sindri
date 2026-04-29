//! Deprecated alias for the legacy `LocalRegistry` type.
//!
//! The implementation moved to [`crate::source::local_path::LocalPathSource`]
//! as part of Phase 1 of the source-modes refactor (DDD-08, ADR-028). This
//! module retains the old name for one release so existing call sites keep
//! compiling — see `v4/docs/plan/source-modes-implementation.md` §1.2.

/// Backwards-compatible alias for [`crate::source::local_path::LocalPathSource`].
///
/// New code should refer to `LocalPathSource` directly. This alias exists
/// solely so call sites that imported `sindri_registry::LocalRegistry`
/// continue to compile during the v4 source-modes rollout (Phase 1).
#[deprecated(
    since = "4.0.0-alpha.2",
    note = "use sindri_registry::source::LocalPathSource instead"
)]
pub type LocalRegistry = crate::source::local_path::LocalPathSource;
