// ADR-002: Atomic Component replaces Extension
// ADR-004: Backend-addressed manifest syntax
use crate::platform::Platform;
use crate::version::VersionSpec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// ADR-002: The atomic unit of v4.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComponentId {
    pub backend: Backend,
    pub name: String,
}

impl ComponentId {
    /// Parse `backend:name[@version]` syntax (ADR-004)
    pub fn parse(s: &str) -> Option<Self> {
        let (backend_str, rest) = s.split_once(':')?;
        let name = rest.split('@').next()?.to_string();
        let backend = Backend::from_str(backend_str).ok()?;
        Some(ComponentId { backend, name })
    }

    pub fn to_address(&self) -> String {
        format!("{}:{}", self.backend.as_str(), self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    // Version managers
    Mise,
    // System package managers
    Apt,
    Dnf,
    Zypper,
    Pacman,
    Apk,
    Brew,
    Winget,
    Scoop,
    // Universal
    Npm,
    Pipx,
    Cargo,
    GoInstall,
    // Download
    Binary,
    // Script
    Script,
    // SDK managers
    Sdkman,
    // Meta
    Collection,
}

impl Backend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Backend::Mise => "mise",
            Backend::Apt => "apt",
            Backend::Dnf => "dnf",
            Backend::Zypper => "zypper",
            Backend::Pacman => "pacman",
            Backend::Apk => "apk",
            Backend::Brew => "brew",
            Backend::Winget => "winget",
            Backend::Scoop => "scoop",
            Backend::Npm => "npm",
            Backend::Pipx => "pipx",
            Backend::Cargo => "cargo",
            Backend::GoInstall => "go-install",
            Backend::Binary => "binary",
            Backend::Script => "script",
            Backend::Sdkman => "sdkman",
            Backend::Collection => "collection",
        }
    }
}

impl FromStr for Backend {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mise" => Ok(Backend::Mise),
            "apt" => Ok(Backend::Apt),
            "dnf" => Ok(Backend::Dnf),
            "zypper" => Ok(Backend::Zypper),
            "pacman" => Ok(Backend::Pacman),
            "apk" => Ok(Backend::Apk),
            "brew" => Ok(Backend::Brew),
            "winget" => Ok(Backend::Winget),
            "scoop" => Ok(Backend::Scoop),
            "npm" => Ok(Backend::Npm),
            "pipx" => Ok(Backend::Pipx),
            "cargo" => Ok(Backend::Cargo),
            "go-install" => Ok(Backend::GoInstall),
            "binary" => Ok(Backend::Binary),
            "script" => Ok(Backend::Script),
            "sdkman" => Ok(Backend::Sdkman),
            "collection" => Ok(Backend::Collection),
            _ => Err(()),
        }
    }
}

/// The v4 component manifest shape (component.yaml)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComponentManifest {
    pub metadata: ComponentMetadata,
    pub platforms: Vec<Platform>,
    pub install: InstallConfig,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub capabilities: ComponentCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComponentMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct InstallConfig {
    pub mise: Option<MiseInstallConfig>,
    pub apt: Option<PackageInstallConfig>,
    pub dnf: Option<PackageInstallConfig>,
    pub zypper: Option<PackageInstallConfig>,
    pub pacman: Option<PackageInstallConfig>,
    pub apk: Option<PackageInstallConfig>,
    pub brew: Option<BrewInstallConfig>,
    pub winget: Option<WingetInstallConfig>,
    pub scoop: Option<ScoopInstallConfig>,
    pub npm: Option<NpmInstallConfig>,
    pub cargo: Option<CargoInstallConfig>,
    pub pipx: Option<PipxInstallConfig>,
    #[serde(rename = "go-install")]
    pub go_install: Option<GoInstallConfig>,
    pub binary: Option<BinaryInstallConfig>,
    pub script: Option<ScriptInstallConfig>,
    pub sdkman: Option<SdkmanInstallConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MiseInstallConfig {
    pub tools: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PackageInstallConfig {
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrewInstallConfig {
    pub package: String,
    pub tap: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WingetInstallConfig {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScoopInstallConfig {
    pub package: String,
    pub bucket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NpmInstallConfig {
    pub package: String,
    pub global: bool,
}

/// Install a crate via `cargo install`.
///
/// Maps to: `cargo install <crate> [--version <v>] [--features ...] [--locked] [--git <url>]`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CargoInstallConfig {
    /// The crate name (serialized as `crate` since `crate` is a Rust keyword).
    #[serde(rename = "crate")]
    pub crate_name: String,
    /// Optional pinned version (`--version`).
    pub version: Option<String>,
    /// Optional Cargo feature flags (`--features ...`).
    #[serde(default)]
    pub features: Vec<String>,
    /// Optional git URL to install from (`--git <url>`).
    pub git: Option<String>,
    /// Whether to pass `--locked` (defaults to true for reproducibility).
    #[serde(default = "default_true")]
    pub locked: bool,
}

fn default_true() -> bool {
    true
}

/// Install a Python application via `pipx install`.
///
/// Maps to: `pipx install <package>[==<version>] [--python <python>]`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct PipxInstallConfig {
    /// The pipx package name (PyPI distribution name).
    pub package: String,
    /// Optional pinned version (rendered as `package==version`).
    pub version: Option<String>,
    /// Optional Python interpreter to use (`--python <python>`).
    pub python: Option<String>,
}

/// Install a Go module via `go install`.
///
/// Maps to: `go install <module>@<version>`. `go install` requires an explicit version
/// (`@latest` or `@vX.Y.Z`), so `version` is non-optional here.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct GoInstallConfig {
    /// Fully-qualified module path (e.g. `github.com/foo/bar/cmd/baz`).
    pub module: String,
    /// Required version (`latest` or a semver tag like `v1.2.3`).
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BinaryInstallConfig {
    pub url_template: String,
    pub checksums: HashMap<String, String>,
    pub install_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScriptInstallConfig {
    pub sh: Option<String>,
    pub ps1: Option<String>,
}

/// Install a candidate via SDKMAN: `sdk install <candidate> <version>`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SdkmanInstallConfig {
    /// SDKMAN candidate name (e.g. "java", "kotlin", "gradle")
    pub candidate: String,
    /// Specific version identifier (e.g. "21.0.5-tem", "2.1.0", "8.11")
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ComponentCapabilities {
    pub collision_handling: Option<CollisionHandlingConfig>,
    pub hooks: Option<HooksConfig>,
    pub project_init: Option<Vec<ProjectInitStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CollisionHandlingConfig {
    pub path_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HooksConfig {
    pub pre_install: Option<String>,
    pub post_install: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectInitStep {
    pub command: String,
    pub priority: u32,
}

/// An entry in the BOM manifest referencing a component
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BomEntry {
    /// `backend:name[@version]` address
    pub address: String,
    pub version: Option<VersionSpec>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}
