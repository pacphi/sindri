//! Lock-step doc/impl test: extracts the canonical example YAML from
//! `v4/docs/POLICY.md` and asserts that it deserialises against the live
//! `InstallPolicy` schema. Guards against the doc-vs-impl drift that the
//! 2026-04-30 reconciliation plan resolved (F-POL-01).

use sindri_core::policy::{InstallPolicy, PolicyAction, PolicyPreset, TrustList};

const POLICY_MD_PATH: &str = "../../docs/POLICY.md";

/// Pull the first ```yaml fenced block that contains `apiVersion: sindri.dev/v4`
/// from POLICY.md. Returns the raw YAML string.
fn extract_canonical_example() -> String {
    let md = std::fs::read_to_string(POLICY_MD_PATH).unwrap_or_else(|e| {
        panic!(
            "could not read {} from {}: {}",
            POLICY_MD_PATH,
            std::env::current_dir().unwrap().display(),
            e
        )
    });

    let mut in_block = false;
    let mut buf = String::new();
    for line in md.lines() {
        if line.trim_start().starts_with("```yaml") {
            in_block = true;
            buf.clear();
            continue;
        }
        if in_block && line.trim_start() == "```" {
            if buf.contains("apiVersion: sindri.dev/v4") {
                return buf;
            }
            in_block = false;
            buf.clear();
            continue;
        }
        if in_block {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    panic!(
        "no ```yaml fenced block in POLICY.md contained `apiVersion: sindri.dev/v4`. \
         Update either the doc or this test."
    );
}

#[test]
fn policy_md_canonical_example_parses() {
    let yaml = extract_canonical_example();
    let parsed: InstallPolicy = serde_yaml::from_str(&yaml).unwrap_or_else(|e| {
        panic!(
            "POLICY.md canonical example failed to deserialise.\n\
             ---YAML---\n{}\n---ERROR---\n{}",
            yaml, e
        )
    });

    // Spot-check the salient fields from the canonical example.
    assert_eq!(parsed.preset, PolicyPreset::Strict);
    assert!(parsed.licenses.allow.contains(&"MIT".to_string()));
    assert!(parsed.licenses.deny.contains(&"GPL-3.0-only".to_string()));
    assert_eq!(parsed.licenses.on_unknown, Some(PolicyAction::Warn));
    assert_eq!(parsed.registries.require_signed, Some(true));
    assert_eq!(parsed.registries.trust.len(), 2);
    assert!(parsed.requires_pinned_versions());
    assert_eq!(parsed.script_backend_action(), PolicyAction::Prompt);
    assert_eq!(parsed.privileged_action(), PolicyAction::Prompt);
    assert!(matches!(
        parsed.capabilities.trust_sources.mcp_registration,
        TrustList::Wildcard(_)
    ));
    assert_eq!(
        parsed.auth.on_unresolved_required,
        sindri_core::policy::PolicyAction::Deny
    );
}
