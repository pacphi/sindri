use sindri_core::version::{Version, VersionSpec};
use crate::error::ResolverError;

/// Resolve a VersionSpec against a list of available versions (ADR-004)
pub fn resolve_version(
    spec: &VersionSpec,
    available: &[Version],
) -> Result<Version, ResolverError> {
    if available.is_empty() {
        return Err(ResolverError::NotFound("No versions available".into()));
    }

    match spec {
        VersionSpec::Latest => {
            // Return the last entry (registries list versions oldest-first by convention)
            Ok(available.last().unwrap().clone())
        }
        VersionSpec::Exact(v) => {
            available
                .iter()
                .find(|a| a.0 == *v)
                .cloned()
                .ok_or_else(|| {
                    ResolverError::NotFound(format!("Exact version {} not available", v))
                })
        }
        VersionSpec::Range(range) => {
            // Simple semver-like range matching: ">=1.0, <2.0", "^1.2", "~1.2.3"
            // For Sprint 3: match prefix ranges and exact. Full semver in Sprint 3 hardening.
            let matched = available
                .iter().rfind(|v| version_satisfies_range(&v.0, range)) // take highest satisfying
                .cloned();
            matched.ok_or_else(|| {
                ResolverError::NotFound(format!("No version satisfies range {}", range))
            })
        }
    }
}

/// Simple range check — handles ^, ~, >=, <= and exact
fn version_satisfies_range(version: &str, range: &str) -> bool {
    let range = range.trim();

    // Exact match
    if !range.starts_with(['^', '~', '>', '<', '=']) {
        return version == range;
    }

    // Caret: ^1.2.3 means >=1.2.3 <2.0.0
    if let Some(req) = range.strip_prefix('^') {
        let v = parse_semver(version);
        let r = parse_semver(req);
        if v.0 != r.0 {
            return false;
        }
        if v.1 < r.1 {
            return false;
        }
        if v.1 == r.1 && v.2 < r.2 {
            return false;
        }
        return true;
    }

    // Tilde: ~1.2.3 means >=1.2.3 <1.3.0
    if let Some(req) = range.strip_prefix('~') {
        let v = parse_semver(version);
        let r = parse_semver(req);
        return v.0 == r.0 && v.1 == r.1 && v.2 >= r.2;
    }

    // >= and <= comparisons
    if let Some(req) = range.strip_prefix(">=") {
        let v = parse_semver(version);
        let r = parse_semver(req.trim());
        return (v.0, v.1, v.2) >= (r.0, r.1, r.2);
    }
    if let Some(req) = range.strip_prefix("<=") {
        let v = parse_semver(version);
        let r = parse_semver(req.trim());
        return (v.0, v.1, v.2) <= (r.0, r.1, r.2);
    }
    if let Some(req) = range.strip_prefix('>') {
        let v = parse_semver(version);
        let r = parse_semver(req.trim());
        return (v.0, v.1, v.2) > (r.0, r.1, r.2);
    }
    if let Some(req) = range.strip_prefix('<') {
        let v = parse_semver(version);
        let r = parse_semver(req.trim());
        return (v.0, v.1, v.2) < (r.0, r.1, r.2);
    }

    false
}

fn parse_semver(s: &str) -> (u64, u64, u64) {
    let parts: Vec<u64> = s
        .split('.')
        .map(|p| p.parse::<u64>().unwrap_or(0))
        .collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_version_matches() {
        let avail = vec![Version::new("1.0.0"), Version::new("1.1.0"), Version::new("2.0.0")];
        let v = resolve_version(&VersionSpec::Exact("1.1.0".into()), &avail).unwrap();
        assert_eq!(v.0, "1.1.0");
    }

    #[test]
    fn latest_returns_last() {
        let avail = vec![Version::new("1.0.0"), Version::new("2.0.0")];
        let v = resolve_version(&VersionSpec::Latest, &avail).unwrap();
        assert_eq!(v.0, "2.0.0");
    }

    #[test]
    fn caret_range() {
        let avail = vec![
            Version::new("1.0.0"),
            Version::new("1.2.0"),
            Version::new("2.0.0"),
        ];
        let v = resolve_version(&VersionSpec::Range("^1.0.0".into()), &avail).unwrap();
        assert_eq!(v.0, "1.2.0");
    }
}
