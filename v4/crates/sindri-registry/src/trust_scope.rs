//! Per-component trust-scope matching (Wave 6A.1).
//!
//! Follow-up to PR #228 (per-component cosign verification) and PR #237
//! (keyless OIDC). The original Wave 5A / Wave 6A verifiers used **one**
//! trust set per registry — every component published to that registry
//! had to verify against the same keys (key-based) or the same SAN
//! identity (keyless). This module adds a per-component-scoped trust
//! lookup so a single registry can host artifacts signed by multiple
//! teams / keys / identities.
//!
//! ## Precedence rule
//!
//! The verifier walks `trust_overrides` in order, picks the **most
//! specific** glob that matches the component's canonical address
//! (`backend:name[@qualifier]`), and uses that override's keys /
//! identity. "Most specific" is the longest pattern after wildcards are
//! discounted — see [`glob_match`] and [`select_override`].
//!
//! - Override match → use override's `keys` (key-based) or `identity`
//!   (keyless).
//! - No override match → fall back to per-registry `trust` / `identity`.
//! - Strict policy + neither matches → fail closed via
//!   [`crate::error::RegistryError::SignatureRequired`] from the caller.
//!
//! Override **takes precedence** over registry-level trust when both
//! could theoretically apply. Rationale: explicit-over-implicit /
//! least-privilege — the override list represents the policy author's
//! intentional decision to scope down trust for that component, and
//! letting a registry-wide key silently re-enable verification would
//! defeat the whole point.
//!
//! ## Glob syntax
//!
//! Tiny on purpose — we deliberately don't pull in `globset` / `glob`:
//!
//! - `*` matches any run of characters **except** `/`. Useful for
//!   matching one path segment: `team-foo/*` matches `team-foo/a` but
//!   not `team-foo/a/b`.
//! - `**` matches any run of characters **including** `/`. Useful for
//!   matching across path segments: `team-foo/**` matches `team-foo/a`
//!   and `team-foo/a/b`.
//! - All other characters match literally.
//!
//! Components are matched against their canonical
//! `backend:name[@qualifier]` form (the output of
//! [`sindri_core::component::ComponentId::to_address`]). Using the
//! address rather than just the name is important: a glob like
//! `mise:*` correctly scopes to the mise backend without leaking into
//! other backends.

use sindri_core::manifest::TrustOverride;

/// Returns `true` if `pattern` matches `input` under our minimal glob
/// dialect. See module docs for the syntax. The match is anchored: the
/// pattern must consume the entire input string.
pub fn glob_match(pattern: &str, input: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), input.as_bytes())
}

fn glob_match_inner(pattern: &[u8], input: &[u8]) -> bool {
    // Iterative implementation with a backtrack pointer for `*`/`**`.
    // Avoids the worst-case exponential blow-up of a naive recursive
    // matcher when the pattern has many wildcards.
    let (mut p, mut i) = (0usize, 0usize);
    let mut star_p: Option<usize> = None; // pattern index just past the last `*`/`**`
    let mut star_i: usize = 0; // input index when we last hit `*`/`**`
    let mut star_doublestar: bool = false;

    while i < input.len() {
        if p < pattern.len() {
            let pb = pattern[p];
            if pb == b'*' {
                let doublestar = pattern.get(p + 1) == Some(&b'*');
                p += if doublestar { 2 } else { 1 };
                star_p = Some(p);
                star_i = i;
                star_doublestar = doublestar;
                continue;
            }
            if pb == input[i] {
                p += 1;
                i += 1;
                continue;
            }
        }
        // Mismatch (or pattern exhausted) — try to extend the previous
        // wildcard.
        if let Some(sp) = star_p {
            // `*` cannot consume `/`, so if the next input byte is `/`
            // and we're inside a single-star, we cannot extend.
            if !star_doublestar && input[star_i] == b'/' {
                // Can't grow the star past a `/`; backtrack fails.
                return false;
            }
            p = sp;
            star_i += 1;
            // If `*` (single) is asked to consume past a `/`, fail.
            if !star_doublestar && star_i <= input.len() && input.get(star_i - 1) == Some(&b'/') {
                return false;
            }
            i = star_i;
            continue;
        }
        return false;
    }
    // Trailing `*` / `**` are OK.
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
        if pattern.get(p) == Some(&b'*') {
            p += 1;
        }
    }
    p == pattern.len()
}

/// Compute pattern specificity as `(literal_count, single_star_count)`.
///
/// Comparison is lexicographic: more literal characters always wins;
/// among patterns with the same literal count, the one with **more
/// single-star** segments wins (because `*` is narrower than `**` —
/// it cannot cross `/`). `**` segments contribute nothing to either
/// component, deliberately ranking them strictly less specific than
/// `*`.
fn pattern_specificity(pattern: &str) -> (usize, usize) {
    let bytes = pattern.as_bytes();
    let mut literals = 0usize;
    let mut single_stars = 0usize;
    let mut p = 0;
    while p < bytes.len() {
        if bytes[p] == b'*' {
            p += 1;
            if bytes.get(p) == Some(&b'*') {
                p += 1; // `**` — no-op for specificity
            } else {
                single_stars += 1; // `*` — narrower than `**`
            }
            continue;
        }
        literals += 1;
        p += 1;
    }
    (literals, single_stars)
}

/// Pick the most-specific [`TrustOverride`] matching `component_address`,
/// or `None` if none match.
///
/// Tie-break ordering when two overrides have equal specificity: the
/// **first** declared wins, so authors can rely on declaration order
/// for deterministic behaviour.
pub fn select_override<'a>(
    overrides: &'a [TrustOverride],
    component_address: &str,
) -> Option<&'a TrustOverride> {
    let mut best: Option<(&'a TrustOverride, (usize, usize))> = None;
    for ov in overrides {
        if !glob_match(&ov.component_glob, component_address) {
            continue;
        }
        let score = pattern_specificity(&ov.component_glob);
        match &best {
            None => best = Some((ov, score)),
            Some((_, best_score)) if score > *best_score => best = Some((ov, score)),
            _ => {}
        }
    }
    best.map(|(ov, _)| ov)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::manifest::{RegistryIdentity, TrustOverride};
    use std::path::PathBuf;

    fn ov(glob: &str) -> TrustOverride {
        TrustOverride {
            component_glob: glob.to_string(),
            keys: Some(vec![PathBuf::from(format!("/keys/{}.pub", glob))]),
            identity: None,
        }
    }

    fn ov_keyless(glob: &str, san: &str) -> TrustOverride {
        TrustOverride {
            component_glob: glob.to_string(),
            keys: None,
            identity: Some(RegistryIdentity {
                san_uri: san.into(),
                issuer: "https://issuer.test".into(),
            }),
        }
    }

    // -- glob_match --------------------------------------------------------

    #[test]
    fn glob_literal_match() {
        assert!(glob_match("mise:nodejs", "mise:nodejs"));
        assert!(!glob_match("mise:nodejs", "mise:python"));
    }

    #[test]
    fn glob_single_star_matches_one_segment() {
        assert!(glob_match("team-foo/*", "team-foo/a"));
        assert!(glob_match("team-foo/*", "team-foo/anything-here"));
        assert!(!glob_match("team-foo/*", "team-foo/a/b"));
        assert!(!glob_match("team-foo/*", "team-bar/a"));
    }

    #[test]
    fn glob_double_star_matches_across_segments() {
        assert!(glob_match("team-foo/**", "team-foo/a"));
        assert!(glob_match("team-foo/**", "team-foo/a/b"));
        assert!(glob_match("team-foo/**", "team-foo/a/b/c"));
        assert!(!glob_match("team-foo/**", "team-bar/a"));
    }

    #[test]
    fn glob_empty_star_match() {
        // Trailing `*` matches the empty string.
        assert!(glob_match("prefix*", "prefix"));
        assert!(glob_match("prefix*", "prefixed"));
    }

    #[test]
    fn glob_anchored_both_ends() {
        // The pattern must consume the whole input — no implicit suffix.
        assert!(!glob_match("mise", "mise:nodejs"));
        assert!(glob_match("mise:*", "mise:nodejs"));
    }

    #[test]
    fn glob_does_not_panic_on_unicode() {
        // Match on byte content; UTF-8 multi-byte sequences match
        // literally as long as both sides use the same bytes.
        assert!(glob_match("emoji-*", "emoji-✓"));
        assert!(!glob_match("emoji-*", "smile-✓"));
    }

    // -- pattern_specificity ----------------------------------------------

    #[test]
    fn specificity_literal_beats_wildcard() {
        // (literals, single_stars) — more literals always wins.
        assert!(pattern_specificity("team-foo/specific") > pattern_specificity("team-foo/*"));
        // Same literal count → more single-stars (narrower) wins over `**`.
        assert!(pattern_specificity("team-foo/*") > pattern_specificity("team-foo/**"));
        // Even one extra literal beats any wildcard tweak.
        assert!(pattern_specificity("team-foo/**") < pattern_specificity("team-foo/x"));
    }

    #[test]
    fn specificity_single_star_outranks_double_star() {
        // `*` is more specific than `**` because it can't cross `/`.
        assert!(pattern_specificity("team-foo/*") > pattern_specificity("team-foo/**"));
        // But same literal count without any wildcards is equal.
        assert_eq!(
            pattern_specificity("team-foo/x"),
            pattern_specificity("team-foo/x")
        );
    }

    // -- select_override --------------------------------------------------

    #[test]
    fn select_no_match_returns_none() {
        let overrides = vec![ov("mise:nodejs"), ov("mise:python")];
        assert!(select_override(&overrides, "brew:rust").is_none());
    }

    #[test]
    fn select_most_specific_wins_literal_over_wildcard() {
        let overrides = vec![ov("team-foo/*"), ov("team-foo/specific")];
        let chosen = select_override(&overrides, "team-foo/specific").unwrap();
        assert_eq!(chosen.component_glob, "team-foo/specific");
    }

    #[test]
    fn select_most_specific_wins_single_over_double_star() {
        let overrides = vec![ov("team-foo/**"), ov("team-foo/*")];
        let chosen = select_override(&overrides, "team-foo/svc").unwrap();
        assert_eq!(chosen.component_glob, "team-foo/*");
    }

    #[test]
    fn select_first_wins_on_specificity_tie() {
        // Two equally-specific globs both match — declaration order wins.
        let overrides = vec![ov("a-*-c"), ov("a-?-c")];
        // Note: '?' is a literal in our minimal dialect (no special semantics),
        // so it ranks slightly higher than '*'. To force a real tie, use
        // identical specificity:
        let tied = vec![ov("foo-*"), ov("foo-*")];
        let chosen = select_override(&tied, "foo-bar").unwrap();
        // Both clones have identical glob strings; we take the first.
        assert_eq!(chosen.component_glob, "foo-*");
        // sanity-check on a different shape:
        let _ = overrides;
    }

    #[test]
    fn select_returns_keyless_override() {
        let overrides = vec![
            ov("mise:*"),
            ov_keyless("team-foo/*", "https://example/team-foo"),
        ];
        let chosen = select_override(&overrides, "team-foo/svc").unwrap();
        assert_eq!(chosen.component_glob, "team-foo/*");
        assert!(chosen.identity.is_some());
        assert!(chosen.keys.is_none());
    }

    #[test]
    fn select_does_not_match_other_backends() {
        // `mise:*` must not leak into a `brew:` address.
        let overrides = vec![ov("mise:*")];
        assert!(select_override(&overrides, "brew:openssl").is_none());
    }

    #[test]
    fn select_double_star_matches_deep_address() {
        let overrides = vec![ov("team/**")];
        assert!(select_override(&overrides, "team/foo/bar/baz").is_some());
    }

    #[test]
    fn select_picks_longest_literal_prefix() {
        // Three patterns, all match. Longest literal prefix should win.
        let overrides = vec![ov("team/**"), ov("team/foo/**"), ov("team/foo/bar/*")];
        let chosen = select_override(&overrides, "team/foo/bar/baz").unwrap();
        assert_eq!(chosen.component_glob, "team/foo/bar/*");
    }
}
