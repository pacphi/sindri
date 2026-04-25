use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::component::{Backend, ComponentId};
use crate::version::Version;
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
}
