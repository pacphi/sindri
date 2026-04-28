use crate::component::{Backend, ComponentId, ComponentManifest};
use crate::platform::Platform;
use crate::version::Version;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Lockfile {
    pub version: u32,
    pub bom_hash: String,
    pub target: String,
    pub components: Vec<ResolvedComponent>,
}

impl Lockfile {
    pub fn new(bom_hash: String, target: String) -> Self {
        Lockfile {
            version: 1,
            bom_hash,
            target,
            components: Vec::new(),
        }
    }

    pub fn is_stale(&self, current_bom_hash: &str) -> bool {
        self.bom_hash != current_bom_hash
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResolvedComponent {
    pub id: ComponentId,
    pub version: Version,
    pub backend: Backend,
    pub oci_digest: Option<String>,
    pub checksums: HashMap<String, String>,
    pub depends_on: Vec<String>,
    /// Full component manifest, when available.
    ///
    /// The resolver does not yet fetch OCI manifests (Wave 3A), so today this
    /// is always `None` and the apply pipeline degrades gracefully — only the
    /// install + lifecycle hook steps run for a `None` manifest, and the
    /// configure / validate / remove capability executors are skipped with a
    /// `tracing::debug!`. The field is in place so that when OCI fetch lands,
    /// `sindri apply` will pick up validate / configure / per-platform overrides
    /// without further changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ComponentManifest>,
    /// OCI manifest digest of the component's `component.yaml` blob (ADR-003,
    /// ADR-014). Wave 3A.1 carries this field through the lockfile schema with
    /// `#[serde(default)]` so older lockfiles still deserialize; the resolver
    /// continues to write `None` until Wave 3A.2 hooks up the live OCI fetch
    /// path that populates the digest from the registry response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
    /// SHA-256 digest of the component's primary OCI layer (Wave 5A -- D5).
    ///
    /// Populated when the resolver fetches a signed component artifact from
    /// an OCI registry; used by `sindri apply` to verify a per-component
    /// cosign signature before the install backend runs.
    ///
    /// `#[serde(default)]` keeps the schema backward-compatible with
    /// pre-Wave-5A lockfiles, which omit the field entirely. Under
    /// `policy.require_signed_registries=true`, apply requires this field on
    /// every newly-resolved entry -- see `crates/sindri/src/commands/apply.rs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_digest: Option<String>,
    /// Platform constraints recorded from the component manifest at
    /// resolve-time (Wave 6A / ADR-002 additive extension).
    ///
    /// An empty list means the component is universal (no platform
    /// restriction). A non-empty list is the `platforms:` array from the
    /// component's `component.yaml`.
    ///
    /// Persisting this in the lockfile lets `sindri resolve --offline`
    /// run Gate 1 (ADR-008) without a network call: the offline path reads
    /// this field and builds a `CandidateRef::with_manifest` so the
    /// `AdmissionChecker` receives real platform data instead of skipping
    /// with `ADM_PLATFORM_SKIPPED`.
    ///
    /// `#[serde(default)]` keeps the schema backward-compatible with
    /// pre-Wave-6A lockfiles that omit the field entirely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<Platform>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Backend;

    fn sample(manifest_digest: Option<String>) -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: Backend::Brew,
                name: "git".into(),
                qualifier: None,
            },
            version: Version::new("2.45.0"),
            backend: Backend::Brew,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest,
            component_digest: None,
            platforms: None,
        }
    }

    #[test]
    fn lockfile_without_manifest_digest_still_deserializes() {
        // Pre-3A.1 schema: no `manifest_digest` field at all.
        let yaml = r#"
id:
  backend: brew
  name: git
version: "2.45.0"
backend: brew
oci_digest: null
checksums: {}
depends_on: []
"#;
        let parsed: ResolvedComponent = serde_yaml::from_str(yaml).unwrap();
        assert!(parsed.manifest_digest.is_none());
    }

    #[test]
    fn manifest_digest_round_trips() {
        let original = sample(Some("sha256:deadbeef".into()));
        let yaml = serde_yaml::to_string(&original).unwrap();
        let parsed: ResolvedComponent = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.manifest_digest.as_deref(), Some("sha256:deadbeef"));
        assert_eq!(parsed.id.name, original.id.name);
    }

    #[test]
    fn lockfile_without_component_digest_still_deserializes() {
        // Wave 5A backward-compat: lockfiles produced before D5 omit
        // `component_digest` entirely. They must still deserialize and
        // surface `None` for the field.
        let yaml = r#"
id:
  backend: brew
  name: git
version: "2.45.0"
backend: brew
oci_digest: null
checksums: {}
depends_on: []
manifest_digest: "sha256:abc"
"#;
        let parsed: ResolvedComponent = serde_yaml::from_str(yaml).unwrap();
        assert!(parsed.component_digest.is_none());
        assert_eq!(parsed.manifest_digest.as_deref(), Some("sha256:abc"));
    }

    #[test]
    fn component_digest_round_trips() {
        let mut comp = sample(Some("sha256:reg".into()));
        comp.component_digest = Some("sha256:comp".into());
        let yaml = serde_yaml::to_string(&comp).unwrap();
        let parsed: ResolvedComponent = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.component_digest.as_deref(), Some("sha256:comp"));
        assert_eq!(parsed.manifest_digest.as_deref(), Some("sha256:reg"));
    }

    #[test]
    fn manifest_digest_none_is_omitted_in_serialization() {
        // skip_serializing_if = "Option::is_none" keeps existing lockfiles
        // textually identical when the field isn't populated.
        let comp = sample(None);
        let yaml = serde_yaml::to_string(&comp).unwrap();
        assert!(!yaml.contains("manifest_digest"), "yaml: {}", yaml);
    }

    #[test]
    fn platforms_round_trips() {
        use crate::platform::{Arch, Os};
        let mut comp = sample(None);
        comp.platforms = Some(vec![
            Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            Platform {
                os: Os::Linux,
                arch: Arch::Aarch64,
            },
        ]);
        let yaml = serde_yaml::to_string(&comp).unwrap();
        let parsed: ResolvedComponent = serde_yaml::from_str(&yaml).unwrap();
        let platforms = parsed.platforms.expect("platforms should be present");
        assert_eq!(platforms.len(), 2);
        assert_eq!(platforms[0].os, Os::Linux);
        assert_eq!(platforms[0].arch, Arch::X86_64);
    }

    #[test]
    fn lockfile_without_platforms_still_deserializes() {
        // ADR-002 additive extension: pre-Wave-6A lockfiles omit `platforms`.
        // They must deserialize correctly with platforms=None.
        let yaml = r#"
id:
  backend: brew
  name: git
version: "2.45.0"
backend: brew
oci_digest: null
checksums: {}
depends_on: []
"#;
        let parsed: ResolvedComponent = serde_yaml::from_str(yaml).unwrap();
        assert!(
            parsed.platforms.is_none(),
            "pre-Wave-6A lockfile should have platforms=None"
        );
    }

    #[test]
    fn platforms_none_is_omitted_in_serialization() {
        let comp = sample(None);
        let yaml = serde_yaml::to_string(&comp).unwrap();
        assert!(
            !yaml.contains("platforms"),
            "platforms=None should not appear in serialized output: {}",
            yaml
        );
    }
}
