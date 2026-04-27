//! File-level collision scenarios (ported from v3).
//!
//! v3 supported three resolution strategies when two components attempted to
//! write to the same on-disk path:
//!
//! - [`Stop`](Scenario::Stop) — abort the apply.
//! - [`Skip`](Scenario::Skip) — leave the existing file untouched.
//! - [`Proceed`](Scenario::Proceed) — overwrite (the later component wins).
//!
//! For Wave 2B these enum variants are present and dispatched by
//! [`crate::collision::CollisionResolver`], but the on-disk **detection probe**
//! (filesystem walks comparing two components' staged outputs) is **not yet
//! ported**. Today, "collision detection" in v4 means "two components claim
//! overlapping path prefixes" (see [`super::detection`]). When the v3 file
//! probe is ported in a future wave, this enum is the dispatch target.

/// File-level collision strategy declared by a component.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// Hard abort: surface a `CollisionViolation` error.
    #[default]
    Stop,
    /// Soft skip: log a warning, do not write the colliding file.
    Skip,
    /// Proceed (overwrite): the later component wins.
    Proceed,
}
