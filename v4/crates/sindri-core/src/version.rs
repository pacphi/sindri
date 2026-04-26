use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Version(pub String);

impl Version {
    pub fn new(s: impl Into<String>) -> Self {
        Version(s.into())
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum VersionSpec {
    #[default]
    Latest,
    Exact(String),
    Range(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PinnedVersion {
    pub version: Version,
    pub digest: Option<String>,
}
