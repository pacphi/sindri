//! Component ordering for collision resolution.
//!
//! Components are sorted by `path_prefix` priority class first
//! (`:shared` core components apply earliest, then alphabetic by prefix),
//! with the component metadata name as a deterministic tiebreaker. This
//! mirrors the v3 `ordering.rs` approach: lowest "weight" first, alpha
//! tiebreak.

use sindri_core::component::ComponentManifest;
use sindri_core::registry::SHARED_PATH_PREFIX;

/// Sort components in apply order.
///
/// Stable sort; preserves caller's relative order when keys collide.
pub fn order(components: &mut [(ComponentManifest, &str)]) {
    components.sort_by(|a, b| {
        let ka = sort_key(&a.0);
        let kb = sort_key(&b.0);
        ka.cmp(&kb)
    });
}

/// Apply-order key for a single component.
///
/// Tuple of `(priority_class, prefix_or_empty, name)`:
/// - `priority_class`: `0` for `:shared`, `1` for component-scoped.
/// - `prefix_or_empty`: the literal prefix (or `""` if absent).
/// - `name`: alphabetic tiebreak.
fn sort_key(m: &ComponentManifest) -> (u8, String, String) {
    let prefix = m
        .capabilities
        .collision_handling
        .as_ref()
        .map(|c| c.path_prefix.clone())
        .unwrap_or_default();
    let class = if prefix == SHARED_PATH_PREFIX { 0 } else { 1 };
    (class, prefix, m.metadata.name.clone())
}
