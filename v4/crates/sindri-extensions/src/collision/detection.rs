//! Path-prefix-level collision detection.
//!
//! Wave 2B scope: detect two components claiming **overlapping path prefixes**
//! at manifest level. We do NOT walk the filesystem here — that is the v3
//! file-level probe and is deferred (see [`super::scenarios`]).
//!
//! Two prefixes "overlap" iff one is a path-segment-prefix of the other (or
//! they are equal). This is a stricter rule than substring containment so
//! that `nodejs/` does not collide with `nodejs-other/`.

use sindri_core::component::ComponentManifest;

/// A detected overlap between two components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixOverlap {
    /// First component's metadata name.
    pub a: String,
    /// First component's prefix.
    pub a_prefix: String,
    /// Second component's metadata name.
    pub b: String,
    /// Second component's prefix.
    pub b_prefix: String,
}

/// Detect all prefix overlaps in `components`.
///
/// Components without a `collision_handling.path_prefix` are skipped. The
/// `:shared` sentinel is treated as non-overlapping with anything (it has no
/// concrete prefix to compare).
pub fn detect_overlaps(components: &[(ComponentManifest, &str)]) -> Vec<PrefixOverlap> {
    let prefixes: Vec<(&str, &str)> = components
        .iter()
        .filter_map(|(m, _)| {
            m.capabilities.collision_handling.as_ref().and_then(|c| {
                let p = c.path_prefix.as_str();
                if p == sindri_core::registry::SHARED_PATH_PREFIX {
                    None
                } else {
                    Some((m.metadata.name.as_str(), p))
                }
            })
        })
        .collect();

    let mut out = Vec::new();
    for i in 0..prefixes.len() {
        for j in (i + 1)..prefixes.len() {
            let (na, pa) = prefixes[i];
            let (nb, pb) = prefixes[j];
            if path_prefix_overlap(pa, pb) {
                out.push(PrefixOverlap {
                    a: na.to_string(),
                    a_prefix: pa.to_string(),
                    b: nb.to_string(),
                    b_prefix: pb.to_string(),
                });
            }
        }
    }
    out
}

/// `true` iff `a` and `b` claim an overlapping prefix tree (segment-aware).
fn path_prefix_overlap(a: &str, b: &str) -> bool {
    let a = a.trim_end_matches('/');
    let b = b.trim_end_matches('/');
    if a == b {
        return true;
    }
    is_segment_prefix(a, b) || is_segment_prefix(b, a)
}

/// `true` iff `parent` is a path-segment prefix of `child`.
///
/// `"nodejs"` is a segment prefix of `"nodejs/conf"` but NOT of `"nodejs-x"`.
fn is_segment_prefix(parent: &str, child: &str) -> bool {
    if let Some(rest) = child.strip_prefix(parent) {
        rest.starts_with('/')
    } else {
        false
    }
}
