// ADR-001: User-authored sindri.yaml BOM as single source of truth
// ADR-027: Target → Component Auth Injection — `provides: Vec<AuthCapability>` on TargetConfig.
// ADR-028: Component source modes — `registry.sources: [...]` shape (Phase 4.1).
use crate::auth::AuthCapability;
use crate::component::BomEntry;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BomManifest {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub name: Option<String>,
    pub components: Vec<BomEntry>,
    /// Registry source configuration (ADR-028 §"Configuration shape", DDD-08).
    ///
    /// Declares the ordered list of registry sources consulted during
    /// resolution, shared trust policy, and global-source merge behaviour.
    /// When absent (or `sources` is empty) the resolver falls back to the
    /// OCI registry index cached at `~/.sindri/cache/registries/`
    /// (legacy pre-ADR-028 behaviour preserved for backwards compatibility).
    ///
    /// ```yaml
    /// registry:
    ///   sources:
    ///     - type: oci
    ///       url: oci://ghcr.io/sindri-dev/registry-core
    ///       tag: "2026.04"
    ///   policy:
    ///     strict_oci: true
    ///   replace_global: false
    /// ```
    #[serde(default, skip_serializing_if = "RegistrySection::is_empty")]
    pub registry: RegistrySection,
    #[serde(default)]
    pub targets: HashMap<String, TargetConfig>,
    pub preferences: Option<Preferences>,
    pub r#override: Option<Vec<OverrideEntry>>,
    /// Optional secret references (Sprint 12, Wave 4C).
    ///
    /// Map of secret-id → prefixed `AuthValue` string (`env:FOO`,
    /// `file:~/.token`, `cli:gh`, `plain:…`). Resolved on demand by
    /// `sindri secrets validate`; values are never persisted.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub secrets: HashMap<String, String>,
}

/// Top-level `registry:` section in `sindri.yaml` (ADR-028, DDD-08).
///
/// Holds the ordered list of [`RegistrySource`](sindri_registry::source::RegistrySource)
/// entries, shared trust/verification policy, and global-source merge
/// semantics. The list is consulted in declared order; the first source whose
/// scope matches a component wins (DDD-03 §"Resolution Algorithm").
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RegistrySection {
    /// Ordered list of registry sources.  The resolver uses first-match-wins
    /// per component name (DDD-08 §"Source scope").
    ///
    /// When absent or empty, the resolver falls back to the legacy OCI index
    /// cached at `~/.sindri/cache/registries/`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<RegistrySourceConfig>,

    /// Shared trust and verification policy applied across all sources.
    #[serde(default, skip_serializing_if = "RegistryPolicy::is_default")]
    pub policy: RegistryPolicy,

    /// When `true`, the project-level `sources` list entirely replaces the
    /// global sources (if any) rather than prepending to them (ADR-028 §4.1).
    ///
    /// Default: `false` (project sources prepend to global sources).
    #[serde(default, skip_serializing_if = "is_false")]
    pub replace_global: bool,
}

impl RegistrySection {
    /// `true` when the section carries no meaningful configuration and may be
    /// omitted from serialization.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty() && self.policy.is_default() && !self.replace_global
    }
}

/// Registry-wide trust and verification policy (ADR-028 §"Trust scopes").
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RegistryPolicy {
    /// When `true`, every component recorded in the lockfile MUST be served
    /// by a source that returns `true` from `Source::supports_strict_oci()`.
    ///
    /// Equivalent to passing `--strict-oci` on every `sindri lock` /
    /// `sindri resolve` invocation (ADR-028 Q3). The CLI flag overrides this
    /// config knob when both are set.
    ///
    /// Default: `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub strict_oci: bool,
}

impl RegistryPolicy {
    /// `true` when the policy is at its default values.
    pub fn is_default(&self) -> bool {
        !self.strict_oci
    }
}

fn is_false(b: &bool) -> bool {
    !b
}

/// Typed config entry for a single source in `registry.sources:` (ADR-028
/// §"Configuration shape"). Uses `#[serde(tag = "type")]` so the YAML
/// discriminator matches the ADR-028 / DDD-08 vocabulary (`oci`,
/// `local-path`, `git`, `local-oci`).
///
/// These config DTOs live in `sindri-core` so the BOM manifest can carry
/// them without creating a circular dependency (sindri-registry already
/// depends on sindri-core). The resolver converts them to `RegistrySource`
/// trait-enum instances via `sindri_registry::source::sources_from_config`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RegistrySourceConfig {
    /// Production OCI registry — the signed distribution path.
    ///
    /// ```yaml
    /// - type: oci
    ///   url: oci://ghcr.io/sindri-dev/registry-core
    ///   tag: "2026.04"
    ///   scope: [nodejs, rust]           # optional
    /// ```
    Oci(OciSourceConfig),

    /// Local filesystem path — the inner-loop authoring source.
    ///
    /// ```yaml
    /// - type: local-path
    ///   path: ./components
    ///   scope: [my-component]           # optional
    /// ```
    LocalPath(LocalPathSourceConfig),

    /// Git repository source (Phase 3).
    ///
    /// ```yaml
    /// - type: git
    ///   url: https://github.com/acme/components.git
    ///   ref: main
    ///   subdir: components              # optional
    ///   require-signed: false           # optional
    /// ```
    Git(GitSourceConfig),

    /// On-disk OCI image layout — the air-gap / offline bundle path.
    ///
    /// ```yaml
    /// - type: local-oci
    ///   layout: ./vendor/registry-core
    ///   scope: [nodejs]                 # optional
    /// ```
    LocalOci(LocalOciSourceConfig),
}

/// Config DTO for the `oci` source variant (ADR-028).
///
/// Carries only the fields needed to express the source in `sindri.yaml`;
/// the runtime `OciSource` in `sindri-registry` adds the network client and
/// cosign verifier. The field names here match the YAML shape exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OciSourceConfig {
    /// Canonical `oci://host/path` URL (e.g. `oci://ghcr.io/sindri-dev/registry-core`).
    pub url: String,
    /// Registry tag (e.g. `2026.04`).
    pub tag: String,
    /// Optional component-name allow-list. When set, only the named
    /// components are satisfied from this source; others fall through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<String>>,
    /// Logical registry name used by the cosign trust loader. Defaults to
    /// `"sindri/core"`; third-party publishers override this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_name: Option<String>,
}

/// Config DTO for the `local-path` source variant (ADR-028).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LocalPathSourceConfig {
    /// Filesystem path to the local component directory.
    pub path: PathBuf,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<String>>,
}

/// Config DTO for the `git` source variant (ADR-028).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitSourceConfig {
    /// Repository URL (https or ssh).
    pub url: String,
    /// Branch, tag, or commit SHA. The resolver pins this to a commit sha
    /// in the lockfile at resolution time (Phase 3).
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// Optional sub-directory inside the repository where `index.yaml` lives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<PathBuf>,
    /// When `true`, unsigned or unverifiable commits are rejected (Phase 3).
    #[serde(default, rename = "require-signed", skip_serializing_if = "is_false")]
    pub require_signed: bool,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<String>>,
}

/// Config DTO for the `local-oci` source variant (ADR-028).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LocalOciSourceConfig {
    /// Path to an OCI image layout directory (v1.1 spec).
    pub layout: PathBuf,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<String>>,
    /// Logical registry name used by the cosign trust loader. Defaults to
    /// `"sindri/core"`; third-party publishers override this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_name: Option<String>,
    /// Optional manifest digest to pin a specific artifact when the layout
    /// contains more than one registry artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_ref: Option<String>,
}

/// Per-component trust scope (Wave 6A.1).
///
/// Lets a single registry publish artifacts signed by multiple teams /
/// keys / Fulcio identities — a common pattern when an organisation
/// shares one OCI registry across product groups.
///
/// Either [`Self::keys`] (key-based override) or [`Self::identity`]
/// (keyless override) should be set; setting both is allowed and means
/// the component can verify under either mode (whichever the cosign
/// signature actually used).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrustOverride {
    /// Glob pattern matched against the component's canonical address
    /// (e.g. `"mise:nodejs"`, `"team-foo/*"`, `"team-bar/specific@v1"`).
    /// `*` matches any run of characters except `/`; `**` matches any
    /// run including `/`. Most-specific match (longest pattern) wins.
    pub component_glob: String,
    /// Key-based trust override — list of paths to PEM-encoded P-256
    /// public keys. Resolved relative to the manifest file at load time.
    /// Verifier accepts if any key matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<PathBuf>>,
    /// Keyless trust override — SAN URI + OIDC issuer pair the
    /// Fulcio-issued cert must match exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<RegistryIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrustConfig {
    pub signer: String,
}

/// Mirror of `sindri_registry::keyless::KeylessIdentity` — duplicated
/// here so `sindri-core` doesn't depend on `sindri-registry` (which
/// would introduce a cycle, since the registry crate already depends on
/// core for `BomEntry` etc.). The registry crate converts via
/// `From<&RegistryIdentity>`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RegistryIdentity {
    /// The expected SAN URI extension in the Fulcio-issued certificate
    /// (e.g. a GitHub Actions workflow run URL).
    pub san_uri: String,
    /// The expected OIDC issuer URL (e.g.
    /// `https://token.actions.githubusercontent.com`).
    pub issuer: String,
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
