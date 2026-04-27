// ADR-008: Install policy as first-class subsystem
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyPreset {
    #[default]
    Default,
    Strict,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InstallPolicy {
    #[serde(default)]
    pub preset: PolicyPreset,
    #[serde(default)]
    pub allowed_licenses: Vec<String>,
    #[serde(default)]
    pub denied_licenses: Vec<String>,
    pub on_unknown_license: Option<PolicyAction>,
    pub require_signed_registries: Option<bool>,
    pub require_checksums: Option<bool>,
    pub offline: Option<bool>,
    pub audit: Option<AuditConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PolicyAction {
    Allow,
    Warn,
    Prompt,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditConfig {
    pub require_justification: bool,
}
