// ADR-001: User-authored sindri.yaml BOM as single source of truth
// ADR-027: Target → Component Auth Injection — `provides: Vec<AuthCapability>` on TargetConfig.
use crate::auth::AuthCapability;
use crate::component::BomEntry;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BomManifest {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub name: Option<String>,
    pub components: Vec<BomEntry>,
    #[serde(default)]
    pub registries: Vec<RegistryConfig>,
    #[serde(default)]
    pub targets: HashMap<String, TargetConfig>,
    pub preferences: Option<Preferences>,
    pub r#override: Option<Vec<OverrideEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
    pub trust: Option<TrustConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrustConfig {
    pub signer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TargetConfig {
    pub kind: String,
    pub infra: Option<serde_json::Value>,
    pub auth: Option<HashMap<String, String>>,
    /// User-visible overrides of (or additions to) the target's intrinsic
    /// auth capabilities (ADR-027 §"Per-target manifest extension"). Empty by
    /// default; existing target configs deserialize unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<AuthCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Preferences {
    pub backend_order: Option<HashMap<String, Vec<String>>>,
    pub default_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OverrideEntry {
    pub address: String,
    pub reason: String,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthSource;

    #[test]
    fn target_config_without_provides_defaults_empty() {
        let yaml = r#"
kind: fly
"#;
        let t: TargetConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(t.provides.is_empty());
    }

    #[test]
    fn target_config_with_provides_round_trips() {
        let yaml = r#"
kind: fly
auth: { token: "secret:vault/fly/team-prod" }
provides:
  - id: github_token
    audience: "https://api.github.com"
    source: { kind: from-secrets-store, backend: vault, path: "secrets/github/team" }
    priority: 100
"#;
        let t: TargetConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(t.kind, "fly");
        assert_eq!(t.provides.len(), 1);
        let cap = &t.provides[0];
        assert_eq!(cap.id, "github_token");
        assert_eq!(cap.audience, "https://api.github.com");
        assert_eq!(cap.priority, 100);
        match &cap.source {
            AuthSource::FromSecretsStore { backend, path } => {
                assert_eq!(backend, "vault");
                assert_eq!(path, "secrets/github/team");
            }
            other => panic!("expected FromSecretsStore, got {:?}", other),
        }

        // Round-trip
        let s = serde_yaml::to_string(&t).unwrap();
        let t2: TargetConfig = serde_yaml::from_str(&s).unwrap();
        assert_eq!(t.provides, t2.provides);
    }

    #[test]
    fn target_config_empty_provides_omitted_on_serialise() {
        let t = TargetConfig {
            kind: "local".to_string(),
            infra: None,
            auth: None,
            provides: vec![],
        };
        let s = serde_yaml::to_string(&t).unwrap();
        assert!(
            !s.contains("provides"),
            "expected serialised TargetConfig to omit empty provides, got:\n{}",
            s
        );
    }
}
