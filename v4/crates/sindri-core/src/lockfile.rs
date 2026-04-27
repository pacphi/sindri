use crate::component::{Backend, ComponentId, ComponentManifest};
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
}
