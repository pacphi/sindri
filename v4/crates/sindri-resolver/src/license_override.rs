//! `sindri resolve --allow <license>=<reason>` value parsing (F-POL-04).
//!
//! Accepts one occurrence per override. Splits on the first `=`; everything
//! after is the reason. SPDX identifiers do not contain `=`, so the split
//! is unambiguous. The reason is mandatory and non-empty after trimming —
//! flag-line overrides without a justification have no audit value.

use std::str::FromStr;

/// One operator-supplied license override. Carries both the SPDX id and a
/// free-form reason; the reason is logged to the ledger when an admission
/// matches the override (see `policy_ledger::emit_license_overrides`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseOverride {
    pub license: String,
    pub reason: String,
}

impl FromStr for LicenseOverride {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let (lic, reason) = raw.split_once('=').ok_or_else(|| {
            format!(
                "expected `<license>=<reason>` (got `{}`); reason is mandatory",
                raw
            )
        })?;
        let lic = lic.trim();
        let reason = reason.trim();
        if lic.is_empty() {
            return Err(format!(
                "license id is empty in `--allow {}=…`; supply an SPDX id",
                raw
            ));
        }
        if reason.is_empty() {
            return Err(format!(
                "reason is empty in `--allow {}=`; supply a justification",
                lic
            ));
        }
        Ok(LicenseOverride {
            license: lic.to_string(),
            reason: reason.to_string(),
        })
    }
}

/// Convenience: returns the override matching `license` (by exact string),
/// or `None`. Used by the resolver post-pass to decide whether to emit an
/// audit event for a resolved component.
pub fn find_override<'a>(
    license: &str,
    overrides: &'a [LicenseOverride],
) -> Option<&'a LicenseOverride> {
    overrides.iter().find(|o| o.license == license)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_override() {
        let o: LicenseOverride = "MPL-2.0=internal-policy-exception".parse().unwrap();
        assert_eq!(o.license, "MPL-2.0");
        assert_eq!(o.reason, "internal-policy-exception");
    }

    #[test]
    fn trims_whitespace_around_each_side() {
        let o: LicenseOverride = "  MIT  =  see TICKET-1234  ".parse().unwrap();
        assert_eq!(o.license, "MIT");
        assert_eq!(o.reason, "see TICKET-1234");
    }

    #[test]
    fn keeps_equals_inside_reason() {
        let o: LicenseOverride = "MIT=expr=value".parse().unwrap();
        assert_eq!(o.reason, "expr=value");
    }

    #[test]
    fn rejects_missing_equals() {
        let e = "MPL-2.0".parse::<LicenseOverride>().unwrap_err();
        assert!(e.contains("license"));
    }

    #[test]
    fn rejects_empty_reason() {
        let e = "MPL-2.0=".parse::<LicenseOverride>().unwrap_err();
        assert!(e.contains("reason"));
    }

    #[test]
    fn rejects_empty_license() {
        let e = "=reason".parse::<LicenseOverride>().unwrap_err();
        assert!(e.contains("license"));
    }

    #[test]
    fn rejects_whitespace_only_reason() {
        let e = "MIT=   ".parse::<LicenseOverride>().unwrap_err();
        assert!(e.contains("reason"));
    }

    #[test]
    fn find_override_matches_exact_string() {
        let overrides = vec![
            LicenseOverride {
                license: "MPL-2.0".into(),
                reason: "r1".into(),
            },
            LicenseOverride {
                license: "BSL-1.1".into(),
                reason: "r2".into(),
            },
        ];
        assert_eq!(find_override("MPL-2.0", &overrides).unwrap().reason, "r1");
        assert!(find_override("MIT", &overrides).is_none());
    }
}
