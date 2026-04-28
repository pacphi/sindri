// ADR-002: Atomic Component replaces Extension
// ADR-004: Backend-addressed manifest syntax (`backend:name[@qualifier]`)
// ADR-024: Script-component lifecycle contract (validate/configure/remove)
// DDD-01: Component domain — full aggregate (id, manifest, options,
//         install/validate/configure/remove, per-platform overrides, capabilities)
// ADR-026: Auth-Aware Components — `auth: AuthRequirements` field on ComponentManifest.
use crate::auth::AuthRequirements;
use crate::platform::{Arch, Os, Platform};
use crate::version::VersionSpec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// ADR-002: The atomic unit of v4.
///
/// A `ComponentId` addresses a component by backend, name, and an optional
/// **qualifier** (ADR-004). The qualifier is *not* a version; it is an
/// alternative-flavour selector that disambiguates two same-named packages
/// available through the same backend (e.g. `npm:codex@openai` distinguishes
/// the OpenAI re-publish of the `codex` npm package from the upstream).
///
/// The version, when relevant, is stored separately on [`BomEntry`] (or as
/// the YAML map value in `sindri.yaml`) — never on the address itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ComponentId {
    /// Backend that installs this component.
    pub backend: Backend,
    /// Component name within the backend's namespace.
    pub name: String,
    /// Optional alternative-flavour selector (ADR-004).
    ///
    /// Distinct from version. `npm:codex@openai` has qualifier `Some("openai")`;
    /// the version (if any) is recorded separately on [`BomEntry`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualifier: Option<String>,
}

impl ComponentId {
    /// Parse a `backend:name[@qualifier]` address (ADR-004).
    ///
    /// Per ADR-004 the version is **not** part of the address — it is the YAML
    /// map value. A trailing `@token` on the address is therefore always
    /// interpreted as a qualifier.
    ///
    /// # Disambiguation note
    ///
    /// To remain compatible with informal call sites that historically wrote
    /// `backend:name@version`, this parser does not validate that the trailing
    /// token "looks like" a qualifier vs. a semver string — the address grammar
    /// makes no such distinction. Callers that want to record a version should
    /// use [`BomEntry::version`].
    ///
    /// Returns `None` if the input is missing the `backend:` prefix or the
    /// backend identifier is unknown.
    pub fn parse(s: &str) -> Option<Self> {
        let (backend_str, rest) = s.split_once(':')?;
        let backend = Backend::from_str(backend_str).ok()?;
        let (name, qualifier) = match rest.split_once('@') {
            Some((n, q)) if !n.is_empty() => {
                // If `q` itself contains another `@<version>`, keep only the
                // first segment as the qualifier.
                let qualifier = q.split('@').next().unwrap_or(q).to_string();
                let qualifier = if qualifier.is_empty() {
                    None
                } else {
                    Some(qualifier)
                };
                (n.to_string(), qualifier)
            }
            _ => (rest.to_string(), None),
        };
        if name.is_empty() {
            return None;
        }
        Some(ComponentId {
            backend,
            name,
            qualifier,
        })
    }

    /// Render the address back to its canonical `backend:name[@qualifier]` form.
    ///
    /// The version is never included — see the type docs.
    pub fn to_address(&self) -> String {
        match &self.qualifier {
            Some(q) => format!("{}:{}@{}", self.backend.as_str(), self.name, q),
            None => format!("{}:{}", self.backend.as_str(), self.name),
        }
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

/// The v4 component manifest shape (component.yaml).
///
/// Fully aligned with DDD-01: in addition to install metadata, a manifest may
/// declare typed user-facing [`Options`], post-install [`ValidateConfig`],
/// post-validate [`ConfigureConfig`] (env + template files), and removal
/// instructions ([`RemoveConfig`]). Per-platform overrides are supported via
/// the [`overrides`](Self::overrides) map.
///
/// All new fields are additive and `#[serde(default)]`-protected: existing
/// component.yaml files in `registry-core/components/` deserialize unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComponentManifest {
    pub metadata: ComponentMetadata,
    pub platforms: Vec<Platform>,
    pub install: InstallConfig,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub capabilities: ComponentCapabilities,

    // ----- DDD-01 additions (all backward-compatible) -----
    /// Typed user-facing options schema (DDD-01 §Options). Empty by default.
    #[serde(default)]
    pub options: Options,
    /// Post-install health checks (ADR-024).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validate: Option<ValidateConfig>,
    /// Post-install configuration: environment vars + rendered template files
    /// (DDD-01 §Configure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configure: Option<ConfigureConfig>,
    /// Removal instructions executed by `sindri remove` (ADR-024).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<RemoveConfig>,
    /// Per-platform overrides for any of: install, configure, validate, remove.
    /// Keyed by `{os}-{arch}` string (e.g. `"linux-x86_64"`, `"macos-aarch64"`).
    /// See [`platform_key`].
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub overrides: HashMap<String, PlatformOverride>,

    // ----- ADR-026 addition (additive; default-empty) -----
    /// Credentials this component declares it needs to install and/or run
    /// (ADR-026). Phase 0 ships the schema only; the resolver, lockfile, and
    /// apply paths do not read this field yet (Phases 1+ will).
    #[serde(default, skip_serializing_if = "AuthRequirements::is_empty")]
    pub auth: AuthRequirements,
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

/// Lifecycle hook commands declared by a component.
///
/// Each field is an optional shell command string executed via the active
/// [`sindri_targets::Target`] at the corresponding lifecycle stage.
///
/// Per ADR-024 (script-component lifecycle contract), hooks are declarative:
/// they run on the same target as the install, observe the same environment,
/// and a non-zero exit code aborts the lifecycle phase.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct HooksConfig {
    /// Runs immediately before the install backend executes.
    pub pre_install: Option<String>,
    /// Runs immediately after a successful install.
    pub post_install: Option<String>,
    /// Runs before any [`ProjectInitStep`] executes for this component.
    pub pre_project_init: Option<String>,
    /// Runs after the final [`ProjectInitStep`] for this component succeeds.
    pub post_project_init: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectInitStep {
    pub command: String,
    pub priority: u32,
}

// =============================================================================
// DDD-01: Options schema
// =============================================================================

/// Typed schema of user-configurable options exposed by the component
/// (DDD-01 §Options).
///
/// Each field name maps to an [`OptionSpec`] that declares the option's type,
/// default value, and validation hints. Empty by default; deserializes from
/// either an absent `options:` key or an empty map.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Options {
    #[serde(flatten)]
    pub fields: HashMap<String, OptionSpec>,
}

/// One typed option entry in [`Options`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum OptionSpec {
    /// Boolean flag.
    Bool {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// Free-form string, optionally constrained by an `enum_values` whitelist.
    String {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Optional allowed-values list (rendered as `enum:` in YAML).
        #[serde(
            default,
            rename = "enum",
            alias = "enum_values",
            skip_serializing_if = "Option::is_none"
        )]
        enum_values: Option<Vec<String>>,
    },
    /// Numeric option, optionally bounded.
    Number {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    },
    /// Filesystem path (executor expands `~`).
    Path {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

// =============================================================================
// DDD-01: Validate / Configure / Remove configs (ADR-024)
// =============================================================================

/// Post-install health-check commands (ADR-024).
///
/// Each [`ValidateCommand`] is run by the active target; all must exit 0 (and
/// match optional `expected_output`/`version_match` assertions) for the
/// component to be considered installed successfully.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ValidateConfig {
    /// Validation commands run after install. All must succeed.
    #[serde(default)]
    pub commands: Vec<ValidateCommand>,
}

/// One health-check command in [`ValidateConfig`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ValidateCommand {
    /// Shell command line to execute.
    pub command: String,
    /// Optional regex/literal expected as a substring of stdout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<String>,
    /// Optional version assertion (e.g. `">=22.0.0"`) parsed against stdout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_match: Option<String>,
}

/// Post-install configuration: environment variables + rendered template
/// files (DDD-01 §Configure, ADR-024).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigureConfig {
    #[serde(default)]
    pub environment: Vec<EnvSetting>,
    #[serde(default)]
    pub files: Vec<FileTemplate>,
}

/// A single environment variable to apply at the configured [`EnvScope`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct EnvSetting {
    pub name: String,
    pub value: String,
    /// Where to apply: see [`EnvScope`]. Defaults to [`EnvScope::ShellRc`].
    #[serde(default = "default_env_scope")]
    pub scope: EnvScope,
}

/// Where an [`EnvSetting`] is applied (per implementation-plan §5.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EnvScope {
    /// Append to user's interactive shell rc file (e.g. `~/.bashrc`).
    ShellRc,
    /// Append to login profile (e.g. `~/.profile`, `~/.zprofile`).
    Login,
    /// Set only for the current sindri session/process.
    Session,
    /// Persist as a user-level env var (Windows: registry; Unix: shell-rc fallback).
    UserEnvVar,
}

fn default_env_scope() -> EnvScope {
    EnvScope::ShellRc
}

/// A rendered template file to write during configure.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct FileTemplate {
    /// Destination path (executor expands a leading `~`).
    pub path: String,
    /// Inline template body (Mustache-style `{{var}}` substitution).
    pub template: String,
    /// Whether to overwrite an existing file at `path`.
    #[serde(default)]
    pub overwrite: bool,
}

/// Removal instructions executed by `sindri remove <component>` (ADR-024).
///
/// The backend's own uninstall runs first; these commands and file deletions
/// are extra cleanup (e.g. wiping a config directory).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveConfig {
    /// Extra shell commands to run during remove.
    #[serde(default)]
    pub commands: Vec<String>,
    /// Files or directories to delete (executor expands a leading `~`).
    #[serde(default)]
    pub files: Vec<String>,
}

/// Per-platform override of any subset of install/configure/validate/remove
/// (DDD-01 §Per-platform overrides). Each field is optional; when present,
/// it fully replaces the top-level value for the matching platform.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct PlatformOverride {
    pub install: Option<InstallConfig>,
    pub configure: Option<ConfigureConfig>,
    pub validate: Option<ValidateConfig>,
    pub remove: Option<RemoveConfig>,
}

/// Canonical platform key string used for [`ComponentManifest::overrides`]:
/// `"{os}-{arch}"`, e.g. `"linux-x86_64"`, `"macos-aarch64"`.
pub fn platform_key(platform: &Platform) -> String {
    let os = match platform.os {
        Os::Linux => "linux",
        Os::Macos => "macos",
        Os::Windows => "windows",
    };
    let arch = match platform.arch {
        Arch::X86_64 => "x86_64",
        Arch::Aarch64 => "aarch64",
    };
    format!("{}-{}", os, arch)
}

impl ComponentManifest {
    /// Returns the effective install config for `platform`: the per-platform
    /// override if present, else the top-level [`Self::install`].
    pub fn effective_install(&self, platform: &Platform) -> &InstallConfig {
        let key = platform_key(platform);
        self.overrides
            .get(&key)
            .and_then(|o| o.install.as_ref())
            .unwrap_or(&self.install)
    }

    /// Returns the effective validate config for `platform`, or `None` if
    /// neither the override nor the top-level manifest declares one.
    pub fn effective_validate(&self, platform: &Platform) -> Option<&ValidateConfig> {
        let key = platform_key(platform);
        self.overrides
            .get(&key)
            .and_then(|o| o.validate.as_ref())
            .or(self.validate.as_ref())
    }

    /// Returns the effective configure config for `platform`, or `None`.
    pub fn effective_configure(&self, platform: &Platform) -> Option<&ConfigureConfig> {
        let key = platform_key(platform);
        self.overrides
            .get(&key)
            .and_then(|o| o.configure.as_ref())
            .or(self.configure.as_ref())
    }

    /// Returns the effective remove config for `platform`, or `None`.
    pub fn effective_remove(&self, platform: &Platform) -> Option<&RemoveConfig> {
        let key = platform_key(platform);
        self.overrides
            .get(&key)
            .and_then(|o| o.remove.as_ref())
            .or(self.remove.as_ref())
    }
}

/// An entry in the BOM manifest referencing a component
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BomEntry {
    /// `backend:name[@qualifier]` address (ADR-004).
    pub address: String,
    /// Pinned version or version range (resolved to exact in `sindri.lock`).
    pub version: Option<VersionSpec>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn registry_root() -> PathBuf {
        // crates/sindri-core/src/component.rs -> ../../../registry-core/components
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("registry-core")
            .join("components")
    }

    // ----- ComponentId parsing -----

    #[test]
    fn component_id_parse_qualifier_only() {
        let id = ComponentId::parse("npm:codex@openai").expect("valid address");
        assert_eq!(id.backend, Backend::Npm);
        assert_eq!(id.name, "codex");
        assert_eq!(id.qualifier.as_deref(), Some("openai"));
    }

    #[test]
    fn component_id_parse_no_qualifier() {
        let id = ComponentId::parse("mise:nodejs").expect("valid address");
        assert_eq!(id.backend, Backend::Mise);
        assert_eq!(id.name, "nodejs");
        assert!(id.qualifier.is_none());
    }

    #[test]
    fn component_id_round_trip_with_qualifier() {
        let id = ComponentId::parse("npm:codex@openai").unwrap();
        let addr = id.to_address();
        assert_eq!(addr, "npm:codex@openai");
        let id2 = ComponentId::parse(&addr).unwrap();
        assert_eq!(id, id2);
    }

    #[test]
    fn component_id_round_trip_no_qualifier() {
        let id = ComponentId::parse("mise:nodejs").unwrap();
        assert_eq!(id.to_address(), "mise:nodejs");
    }

    #[test]
    fn component_id_parse_unknown_backend_returns_none() {
        assert!(ComponentId::parse("nope:thing").is_none());
        assert!(ComponentId::parse("no-colon").is_none());
        assert!(ComponentId::parse("mise:").is_none());
    }

    #[test]
    fn component_id_parse_strips_secondary_at_segment() {
        // `npm:codex@openai@1.2.3` — qualifier is `openai`; trailing version segment is dropped
        // (versions live on BomEntry, not on the address).
        let id = ComponentId::parse("npm:codex@openai@1.2.3").unwrap();
        assert_eq!(id.name, "codex");
        assert_eq!(id.qualifier.as_deref(), Some("openai"));
    }

    // ----- DDD-01 manifest deserialization -----

    #[test]
    fn manifest_with_validate_round_trip() {
        let yaml = r#"
metadata:
  name: t
  version: "1.0.0"
  description: x
  license: MIT
platforms:
  - { os: linux, arch: x86_64 }
install: {}
validate:
  commands:
    - command: "node --version"
      version-match: ">=22.0.0"
    - command: "echo ok"
      expected-output: "ok"
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let v = m.validate.as_ref().expect("validate present");
        assert_eq!(v.commands.len(), 2);
        assert_eq!(v.commands[0].version_match.as_deref(), Some(">=22.0.0"));
        assert_eq!(v.commands[1].expected_output.as_deref(), Some("ok"));

        // Round-trip
        let s = serde_yaml::to_string(&m).unwrap();
        let m2: ComponentManifest = serde_yaml::from_str(&s).unwrap();
        assert_eq!(
            m2.validate.unwrap().commands[0].command,
            "node --version".to_string()
        );
    }

    #[test]
    fn manifest_with_configure_environment() {
        let yaml = r#"
metadata: { name: t, version: "1.0.0", description: x, license: MIT }
platforms: [{ os: linux, arch: x86_64 }]
install: {}
configure:
  environment:
    - { name: FOO, value: "1" }
    - { name: BAR, value: "2", scope: login }
    - { name: BAZ, value: "3", scope: session }
    - { name: QUX, value: "4", scope: user-env-var }
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let c = m.configure.unwrap();
        assert_eq!(c.environment.len(), 4);
        assert_eq!(c.environment[0].scope, EnvScope::ShellRc); // default applied
        assert_eq!(c.environment[1].scope, EnvScope::Login);
        assert_eq!(c.environment[2].scope, EnvScope::Session);
        assert_eq!(c.environment[3].scope, EnvScope::UserEnvVar);
    }

    #[test]
    fn manifest_with_remove_files() {
        let yaml = r#"
metadata: { name: t, version: "1.0.0", description: x, license: MIT }
platforms: [{ os: linux, arch: x86_64 }]
install: {}
remove:
  commands:
    - "rm -rf ~/.cache/t"
  files:
    - "~/.config/t"
    - "/opt/t"
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let r = m.remove.unwrap();
        assert_eq!(r.commands, vec!["rm -rf ~/.cache/t".to_string()]);
        assert_eq!(r.files.len(), 2);
    }

    #[test]
    fn manifest_with_platform_override_install() {
        let yaml = r#"
metadata: { name: t, version: "1.0.0", description: x, license: MIT }
platforms:
  - { os: linux, arch: x86_64 }
  - { os: macos, arch: aarch64 }
install:
  mise:
    tools:
      node: "22.0.0"
overrides:
  linux-x86_64:
    install:
      apt:
        packages: ["nodejs"]
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let linux = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };
        let mac = Platform {
            os: Os::Macos,
            arch: Arch::Aarch64,
        };
        let inst_linux = m.effective_install(&linux);
        assert!(inst_linux.apt.is_some(), "linux override picks apt");
        assert!(inst_linux.mise.is_none());

        let inst_mac = m.effective_install(&mac);
        assert!(
            inst_mac.mise.is_some(),
            "macos falls back to top-level mise"
        );
    }

    #[test]
    fn effective_install_falls_back_when_no_override() {
        let yaml = r#"
metadata: { name: t, version: "1.0.0", description: x, license: MIT }
platforms: [{ os: linux, arch: x86_64 }]
install:
  mise:
    tools: { node: "22.0.0" }
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let p = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };
        let inst = m.effective_install(&p);
        assert!(inst.mise.is_some());
        assert!(m.effective_validate(&p).is_none());
        assert!(m.effective_configure(&p).is_none());
        assert!(m.effective_remove(&p).is_none());
    }

    #[test]
    fn default_env_scope_is_shell_rc() {
        assert_eq!(default_env_scope(), EnvScope::ShellRc);
    }

    #[test]
    fn options_string_with_enum_round_trips() {
        let yaml = r#"
metadata: { name: t, version: "1.0.0", description: x, license: MIT }
platforms: [{ os: linux, arch: x86_64 }]
install: {}
options:
  log_level:
    type: string
    default: info
    enum: [debug, info, warn, error]
    description: "Verbosity"
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let spec = m
            .options
            .fields
            .get("log_level")
            .expect("log_level present");
        match spec {
            OptionSpec::String {
                default,
                enum_values,
                ..
            } => {
                assert_eq!(default.as_deref(), Some("info"));
                assert_eq!(
                    enum_values.as_ref().unwrap(),
                    &vec![
                        "debug".to_string(),
                        "info".to_string(),
                        "warn".to_string(),
                        "error".to_string(),
                    ]
                );
            }
            other => panic!("expected String spec, got {:?}", other),
        }

        let s = serde_yaml::to_string(&m).unwrap();
        let m2: ComponentManifest = serde_yaml::from_str(&s).unwrap();
        assert!(m2.options.fields.contains_key("log_level"));
    }

    #[test]
    fn platform_key_format() {
        assert_eq!(
            platform_key(&Platform {
                os: Os::Linux,
                arch: Arch::X86_64
            }),
            "linux-x86_64"
        );
        assert_eq!(
            platform_key(&Platform {
                os: Os::Macos,
                arch: Arch::Aarch64
            }),
            "macos-aarch64"
        );
    }

    #[test]
    fn existing_registry_components_still_deserialize() {
        // Hard requirement: the 97 component.yaml files in registry-core/components
        // must all deserialize unchanged. We sample 5 representative components
        // covering different install backends.
        let names = ["nodejs", "gh", "claude-code", "clarity", "guacamole"];
        let root = registry_root();
        for name in names {
            let path = root.join(name).join("component.yaml");
            let yaml = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let m: ComponentManifest = serde_yaml::from_str(&yaml)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
            assert_eq!(m.metadata.name, name, "metadata.name mismatch for {name}");
            // New fields default cleanly:
            assert!(m.options.fields.is_empty());
            assert!(m.overrides.is_empty());
            // Auth may be empty (most components) or populated (Phase 3 migrations);
            // we only assert successful deserialization here.
        }
    }

    // ----- ADR-026: auth block round-trips through ComponentManifest -----

    #[test]
    fn manifest_with_auth_block_round_trips() {
        use crate::auth::{AuthScope, Redemption};

        let yaml = r#"
metadata: { name: claude-code, version: "1.0.0", description: x, license: MIT }
platforms: [{ os: linux, arch: x86_64 }]
install:
  npm:
    package: "@anthropic-ai/claude-code"
    global: true
auth:
  tokens:
    - name: anthropic_api_key
      description: "Anthropic API key used by the Claude Code CLI."
      scope: runtime
      optional: false
      audience: "urn:anthropic:api"
      redemption:
        kind: env-var
        env-name: ANTHROPIC_API_KEY
      discovery:
        env-aliases: [ANTHROPIC_API_KEY, CLAUDE_API_KEY]
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        assert!(!m.auth.is_empty());
        assert_eq!(m.auth.tokens.len(), 1);
        let t = &m.auth.tokens[0];
        assert_eq!(t.name, "anthropic_api_key");
        assert_eq!(t.scope, AuthScope::Runtime);
        assert_eq!(t.audience, "urn:anthropic:api");
        match &t.redemption {
            Redemption::EnvVar { env_name } => assert_eq!(env_name, "ANTHROPIC_API_KEY"),
            other => panic!("expected EnvVar, got {:?}", other),
        }

        // Round-trip: serialise then deserialise, the `auth` block must survive.
        let s = serde_yaml::to_string(&m).unwrap();
        let m2: ComponentManifest = serde_yaml::from_str(&s).unwrap();
        assert_eq!(m.auth, m2.auth);
    }

    #[test]
    fn manifest_without_auth_block_has_empty_default() {
        let yaml = r#"
metadata: { name: t, version: "1.0.0", description: x, license: MIT }
platforms: [{ os: linux, arch: x86_64 }]
install: {}
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        assert!(m.auth.is_empty());

        // And serialising back must NOT emit an empty `auth:` key
        // (the field is `skip_serializing_if = "AuthRequirements::is_empty"`).
        let s = serde_yaml::to_string(&m).unwrap();
        assert!(
            !s.contains("auth:"),
            "expected serialised manifest to omit empty auth block, got:\n{}",
            s
        );
    }
}
