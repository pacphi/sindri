// ADR-026: Auth-Aware Components — declaration side schema.
// ADR-027: Target → Component Auth Injection — target capability schema.
// DDD-07: Auth-Bindings Domain — value-object types live here in `sindri-core`.
//
// This module is **schema-only** (Phase 0 of the auth-aware implementation
// plan, 2026-04-28). No resolver, lockfile, or apply paths read these types
// yet; they ship now so Phase 1+ can populate them.
//
// All new fields are additive and `#[serde(default)]`-protected: existing
// component.yaml / sindri.yaml / target manifests deserialize unchanged.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// =============================================================================
// Component-side declaration (ADR-026)
// =============================================================================

/// What credentials a component needs to install and/or run.
///
/// Mounted on `ComponentManifest.auth` as `#[serde(default)]`. Existing
/// manifests (which omit `auth:`) deserialize as `AuthRequirements::default()`,
/// for which [`AuthRequirements::is_empty`] returns `true`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct AuthRequirements {
    /// API tokens / static bearer secrets (anything that lives as a single
    /// string).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tokens: Vec<TokenRequirement>,
    /// OAuth-flow credentials (RFC 8628 device flow today; auth-code in
    /// future).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub oauth: Vec<OAuthRequirement>,
    /// X.509 / PEM materials.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub certs: Vec<CertRequirement>,
    /// SSH key material (private + optional passphrase).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ssh: Vec<SshKeyRequirement>,
}

impl AuthRequirements {
    /// True if no requirements of any kind are declared.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
            && self.oauth.is_empty()
            && self.certs.is_empty()
            && self.ssh.is_empty()
    }
}

/// A single static bearer-style token requirement.
///
/// See ADR-026 §"Schema" for field semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TokenRequirement {
    /// Stable id, unique within the component (e.g. `github_token`).
    pub name: String,
    /// One-line human description shown by `sindri doctor` and
    /// `sindri auth show`.
    pub description: String,
    /// When the credential is needed.
    #[serde(default)]
    pub scope: AuthScope,
    /// If true, install proceeds when no source binds (degraded mode).
    #[serde(default)]
    pub optional: bool,
    /// Logical resource the token is intended for. RFC-9068 audience claim
    /// when the token is a JWT; otherwise a free-form URL or vendor URN
    /// (e.g. `https://api.github.com`, `urn:anthropic:api`).
    pub audience: String,
    /// How the component wants to *receive* the resolved value at apply time.
    #[serde(default)]
    pub redemption: Redemption,
    /// Hints the resolver uses to find a source automatically (ADR-027).
    #[serde(default)]
    pub discovery: DiscoveryHints,
}

/// OAuth-flow credential requirement (RFC 8628 device flow today).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct OAuthRequirement {
    /// Stable id (e.g. `github_oauth`).
    pub name: String,
    /// Human-friendly description.
    pub description: String,
    /// Audience the resulting access-token is intended for.
    pub audience: String,
    /// OAuth provider id (matches `OAuthProvider.id` in DDD-07).
    pub provider: String,
    /// OAuth scopes to request.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    /// When the credential is needed.
    #[serde(default)]
    pub scope: AuthScope,
    /// If true, install proceeds without a bound source.
    #[serde(default)]
    pub optional: bool,
    /// How the component wants the redeemed token delivered.
    #[serde(default)]
    pub redemption: Redemption,
}

/// X.509 / PEM certificate-material requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CertRequirement {
    /// Stable id (e.g. `client_cert`).
    pub name: String,
    /// Human-friendly description.
    pub description: String,
    /// Audience the certificate authenticates against.
    pub audience: String,
    /// When the material is needed.
    #[serde(default)]
    pub scope: AuthScope,
    /// If true, install proceeds without a bound source.
    #[serde(default)]
    pub optional: bool,
    /// Where the cert should be placed at apply time.
    #[serde(default)]
    pub redemption: Redemption,
}

/// SSH-key material requirement (private key + optional passphrase).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SshKeyRequirement {
    /// Stable id (e.g. `git_ssh_key`).
    pub name: String,
    /// Human-friendly description.
    pub description: String,
    /// Audience the key authenticates against
    /// (e.g. `ssh://git@github.com`).
    pub audience: String,
    /// When the key is needed.
    #[serde(default)]
    pub scope: AuthScope,
    /// If true, install proceeds without a bound source.
    #[serde(default)]
    pub optional: bool,
    /// Where the key file should be placed at apply time.
    #[serde(default)]
    pub redemption: Redemption,
}

/// When in the lifecycle a credential is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AuthScope {
    /// Needed only while install/configure scripts run.
    Install,
    /// Needed when the installed tool is invoked by the user.
    Runtime,
    /// Both phases.
    #[default]
    Both,
}

/// How the component wants to *receive* a resolved credential at apply time.
///
/// Internally-tagged on `kind` for serde_yaml compatibility (matches the
/// [`AuthSource`] convention). Field names are kebab-cased on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Redemption {
    /// Inject as `<env_name>=<value>` into `Target::exec` env.
    EnvVar {
        #[serde(rename = "env-name")]
        env_name: String,
    },
    /// Write to `<path>` (mode 0600 by default; deleted post-apply unless
    /// `persist: true`).
    File {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<u32>,
        #[serde(default)]
        persist: bool,
    },
    /// Both: env-var pointing at file (e.g.
    /// `GOOGLE_APPLICATION_CREDENTIALS`).
    EnvFile {
        #[serde(rename = "env-name")]
        env_name: String,
        path: String,
    },
}

impl Default for Redemption {
    fn default() -> Self {
        // An empty env-var name is the "unspecified" sentinel; the real value
        // is supplied per-requirement when the manifest declares one. The
        // default exists so `#[serde(default)]` on a wrapping requirement
        // can still produce a valid value during partial-decode scenarios.
        Redemption::EnvVar {
            env_name: String::new(),
        }
    }
}

/// Component-side aliases that help the resolver auto-bind without explicit
/// `targets.<n>.provides` configuration (ADR-026 §"Schema",
/// ADR-027 §"Binding algorithm").
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DiscoveryHints {
    /// Env-var names to probe (e.g. `["ANTHROPIC_API_KEY","CLAUDE_API_KEY"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_aliases: Vec<String>,
    /// `cli:` invocations that produce the token
    /// (e.g. `["gh auth token"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cli_aliases: Vec<String>,
    /// OAuth provider id this requirement maps to (matches
    /// `OAuthProvider.id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_provider: Option<String>,
}

// =============================================================================
// Target-side capability declaration (ADR-027)
// =============================================================================

/// Audience the resulting credential is valid for.
///
/// Currently a thin newtype around `String`; matched as canonicalised,
/// lower-cased strings (no globs). See ADR-026 §"Audience binding".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Audience(pub String);

impl Audience {
    /// Wrap an audience string in the newtype.
    pub fn new(s: impl Into<String>) -> Self {
        Audience(s.into())
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Audience {
    fn from(s: String) -> Self {
        Audience(s)
    }
}

impl From<&str> for Audience {
    fn from(s: &str) -> Self {
        Audience(s.to_string())
    }
}

/// What a target advertises it can fulfill (ADR-027 §1).
///
/// Returned by `Target::auth_capabilities()` (added in Phase 1) and
/// declarable per-target via `TargetConfig.provides`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct AuthCapability {
    /// Capability id (e.g. `github_token`, `anthropic_api_key`, `aws_sso`).
    pub id: String,
    /// Audience the produced credential is valid for. Must match a
    /// requirement's audience (ADR-026 §"Audience binding") to bind.
    pub audience: String,
    /// Where this credential physically comes from when redeemed.
    pub source: AuthSource,
    /// Priority for resolver tie-breaking (higher = preferred). Default `0`.
    #[serde(default)]
    pub priority: i32,
}

/// Where a credential value physically comes from when redeemed (ADR-027 §1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AuthSource {
    /// Resolve via `sindri-secrets` (Vault, S3, KV).
    FromSecretsStore { backend: String, path: String },
    /// Resolve from environment variable on the target.
    FromEnv { var: String },
    /// Resolve from a file readable on the target.
    FromFile {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<u32>,
    },
    /// Delegate to an installed CLI (mirrors `cli:` of ADR-020).
    FromCli { command: String },
    /// Reuse the target's own upstream auth (e.g. the target's session token
    /// doubles as a child-workload credential).
    FromUpstreamCredentials,
    /// Run an OAuth device flow
    /// (ADR-026 → `DiscoveryHints.oauth_provider`).
    #[serde(rename = "from-oauth")]
    FromOAuth { provider: String },
    /// Interactive prompt (TTY only; rejected in `--ci` mode by Gate 5).
    Prompt,
}

// =============================================================================
// Auth binding (DDD-07 — aggregate root of the Auth-Bindings domain)
// =============================================================================

/// Status of an [`AuthBinding`] once the resolver has walked the candidate
/// chain (DDD-07 §"Lifecycle states", excluding the transient `Redeemed`
/// state which lives only at apply time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AuthBindingStatus {
    /// A candidate source matched and was selected.
    Bound,
    /// No source matched, but the requirement is `optional: true` —
    /// install proceeds with a warning.
    Deferred,
    /// No source matched and the requirement is non-optional — Gate 5
    /// (Phase 2) will deny apply.
    Failed,
}

/// A candidate that was considered but rejected by the binding algorithm
/// (ADR-027 §3 "considered-but-rejected list").
///
/// Persisted into the lockfile so `sindri auth show` can explain *why* a
/// particular source did not win.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RejectedCandidate {
    /// The capability id that was considered.
    pub capability_id: String,
    /// The source kind (the discriminant of [`AuthSource`]).
    pub source_kind: String,
    /// Reason for rejection (e.g. `"audience-mismatch"`,
    /// `"scope-mismatch"`, `"duplicate"`).
    pub reason: String,
}

/// The aggregate root of the Auth-Bindings domain (DDD-07 §"Core
/// Aggregate"). Computed at resolve time; persisted in the per-target
/// lockfile.
///
/// The binding records *references only* — no resolved credential value
/// can ever live here (DDD-07 invariant 3 "no value capture").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct AuthBinding {
    /// Deterministic id: `sha256(component_address || requirement.name ||
    /// target_id)` truncated to 16 hex chars. Stable across hosts so
    /// lockfile diffs reflect intent changes only (DDD-07 invariant 4).
    pub id: String,
    /// Component address (e.g. `npm:claude-code`).
    pub component: String,
    /// Requirement name within the component manifest.
    pub requirement: String,
    /// Audience canonicalised to lower-case. Equal to
    /// `req.audience == source.audience` (DDD-07 invariant 1).
    pub audience: String,
    /// Target name (key in `BomManifest.targets`).
    pub target: String,
    /// The bound source (None if status is `Deferred` or `Failed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<AuthSource>,
    /// Capability priority that won (0 if none).
    #[serde(default)]
    pub priority: i32,
    /// Lifecycle state of the binding.
    pub status: AuthBindingStatus,
    /// Reason string when `status` is `Deferred` or `Failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Other candidates that were considered but rejected (ordered as
    /// walked).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub considered: Vec<RejectedCandidate>,
}

/// Discriminant string for an [`AuthSource`] — used by the binding
/// algorithm's tie-breaker and by [`RejectedCandidate::source_kind`].
///
/// The order also defines the deterministic tie-breaker when capability
/// priorities are equal (ADR-027 §3 "Stable order"):
/// `FromSecretsStore` > `FromEnv` > `FromFile` > `FromCli` >
/// `FromUpstreamCredentials` > `FromOAuth` > `Prompt`.
pub fn auth_source_kind(s: &AuthSource) -> &'static str {
    match s {
        AuthSource::FromSecretsStore { .. } => "from-secrets-store",
        AuthSource::FromEnv { .. } => "from-env",
        AuthSource::FromFile { .. } => "from-file",
        AuthSource::FromCli { .. } => "from-cli",
        AuthSource::FromUpstreamCredentials => "from-upstream-credentials",
        AuthSource::FromOAuth { .. } => "from-oauth",
        AuthSource::Prompt => "prompt",
    }
}

/// Sort rank for [`auth_source_kind`] — lower is preferred.
pub fn auth_source_rank(s: &AuthSource) -> u8 {
    match s {
        AuthSource::FromSecretsStore { .. } => 0,
        AuthSource::FromEnv { .. } => 1,
        AuthSource::FromFile { .. } => 2,
        AuthSource::FromCli { .. } => 3,
        AuthSource::FromUpstreamCredentials => 4,
        AuthSource::FromOAuth { .. } => 5,
        AuthSource::Prompt => 6,
    }
}

// =============================================================================
// Secret reference (minimal, until `sindri-secrets` lands)
// =============================================================================

/// Typed reference to a secret in a backend store.
///
/// Used by [`crate::auth::AuthSource::FromSecretsStore`] and by the
/// `secret:<backend>/<path>` form of `AuthValue` (ADR-020).
///
/// This is a deliberately minimal placeholder for the eventual
/// `sindri-secrets` crate (ADR-025). When that crate lands, the canonical
/// definition will move there and this module will re-export it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SecretRef {
    /// Backend id (e.g. `vault`, `aws-sm`, `gcp-sm`, `kv`).
    pub backend: String,
    /// Backend-specific path / key reference.
    pub path: String,
}

impl SecretRef {
    /// Construct a new [`SecretRef`].
    pub fn new(backend: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            path: path.into(),
        }
    }

    /// Parse the `<backend>/<path>` portion of a `secret:<backend>/<path>`
    /// reference. The leading `secret:` prefix MUST already have been
    /// stripped by the caller.
    ///
    /// Returns `None` if the input is missing the `/` separator or if either
    /// side is empty.
    pub fn parse(rest: &str) -> Option<Self> {
        let (backend, path) = rest.split_once('/')?;
        if backend.is_empty() || path.is_empty() {
            return None;
        }
        Some(SecretRef {
            backend: backend.to_string(),
            path: path.to_string(),
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_requirements_default_is_empty() {
        let a = AuthRequirements::default();
        assert!(a.is_empty());
    }

    #[test]
    fn auth_requirements_round_trip_token() {
        let yaml = r#"
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
      cli-aliases: ["sindri-anthropic-cli token"]
"#;
        let a: AuthRequirements = serde_yaml::from_str(yaml).unwrap();
        assert!(!a.is_empty());
        assert_eq!(a.tokens.len(), 1);
        let t = &a.tokens[0];
        assert_eq!(t.name, "anthropic_api_key");
        assert_eq!(t.scope, AuthScope::Runtime);
        assert!(!t.optional);
        assert_eq!(t.audience, "urn:anthropic:api");
        match &t.redemption {
            Redemption::EnvVar { env_name } => assert_eq!(env_name, "ANTHROPIC_API_KEY"),
            other => panic!("expected EnvVar, got {:?}", other),
        }
        assert_eq!(
            t.discovery.env_aliases,
            vec![
                "ANTHROPIC_API_KEY".to_string(),
                "CLAUDE_API_KEY".to_string(),
            ]
        );
        assert_eq!(
            t.discovery.cli_aliases,
            vec!["sindri-anthropic-cli token".to_string()]
        );

        // Round-trip
        let s = serde_yaml::to_string(&a).unwrap();
        let a2: AuthRequirements = serde_yaml::from_str(&s).unwrap();
        assert_eq!(a, a2);
    }

    #[test]
    fn auth_scope_default_is_both() {
        assert_eq!(AuthScope::default(), AuthScope::Both);
    }

    #[test]
    fn redemption_file_round_trips() {
        let yaml = r#"
tokens:
  - name: gcp_creds
    description: "GCP service account JSON."
    audience: "https://iam.googleapis.com"
    redemption:
      kind: env-file
      env-name: GOOGLE_APPLICATION_CREDENTIALS
      path: "/run/secrets/gcp.json"
"#;
        let a: AuthRequirements = serde_yaml::from_str(yaml).unwrap();
        let t = &a.tokens[0];
        match &t.redemption {
            Redemption::EnvFile { env_name, path } => {
                assert_eq!(env_name, "GOOGLE_APPLICATION_CREDENTIALS");
                assert_eq!(path, "/run/secrets/gcp.json");
            }
            other => panic!("expected EnvFile, got {:?}", other),
        }
    }

    #[test]
    fn redemption_file_persist_default_false() {
        let yaml = r#"
tokens:
  - name: client_cert
    description: "Client cert."
    audience: "https://example.com"
    redemption:
      kind: file
      path: "/etc/sindri/cert.pem"
      mode: 0o600
"#;
        let a: AuthRequirements = serde_yaml::from_str(yaml).unwrap();
        match &a.tokens[0].redemption {
            Redemption::File {
                path,
                mode,
                persist,
            } => {
                assert_eq!(path, "/etc/sindri/cert.pem");
                assert_eq!(*mode, Some(0o600));
                assert!(!*persist);
            }
            other => panic!("expected File, got {:?}", other),
        }
    }

    #[test]
    fn auth_source_round_trips_all_variants() {
        let cases: &[(&str, AuthSource)] = &[
            (
                r#"{ kind: from-secrets-store, backend: vault, path: "secrets/x" }"#,
                AuthSource::FromSecretsStore {
                    backend: "vault".to_string(),
                    path: "secrets/x".to_string(),
                },
            ),
            (
                r#"{ kind: from-env, var: GITHUB_TOKEN }"#,
                AuthSource::FromEnv {
                    var: "GITHUB_TOKEN".to_string(),
                },
            ),
            (
                r#"{ kind: from-cli, command: "gh auth token" }"#,
                AuthSource::FromCli {
                    command: "gh auth token".to_string(),
                },
            ),
            (
                r#"{ kind: from-upstream-credentials }"#,
                AuthSource::FromUpstreamCredentials,
            ),
            (
                r#"{ kind: from-oauth, provider: github }"#,
                AuthSource::FromOAuth {
                    provider: "github".to_string(),
                },
            ),
            (r#"{ kind: prompt }"#, AuthSource::Prompt),
        ];
        for (yaml, expected) in cases {
            let parsed: AuthSource = serde_yaml::from_str(yaml).unwrap();
            assert_eq!(&parsed, expected, "yaml={}", yaml);
            let s = serde_yaml::to_string(&parsed).unwrap();
            let again: AuthSource = serde_yaml::from_str(&s).unwrap();
            assert_eq!(parsed, again);
        }
    }

    #[test]
    fn auth_capability_round_trips() {
        let yaml = r#"
id: github_token
audience: "https://api.github.com"
source: { kind: from-secrets-store, backend: vault, path: "secrets/github/team" }
priority: 100
"#;
        let c: AuthCapability = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.id, "github_token");
        assert_eq!(c.audience, "https://api.github.com");
        assert_eq!(c.priority, 100);
        match &c.source {
            AuthSource::FromSecretsStore { backend, path } => {
                assert_eq!(backend, "vault");
                assert_eq!(path, "secrets/github/team");
            }
            other => panic!("expected FromSecretsStore, got {:?}", other),
        }
    }

    #[test]
    fn auth_capability_priority_defaults_to_zero() {
        let yaml = r#"
id: local_env
audience: "https://api.github.com"
source: { kind: from-env, var: GITHUB_TOKEN }
"#;
        let c: AuthCapability = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.priority, 0);
    }

    #[test]
    fn secret_ref_parses_canonical_form() {
        let r = SecretRef::parse("vault/secrets/anthropic/prod").unwrap();
        assert_eq!(r.backend, "vault");
        assert_eq!(r.path, "secrets/anthropic/prod");
    }

    #[test]
    fn secret_ref_rejects_malformed() {
        assert!(SecretRef::parse("no-slash").is_none());
        assert!(SecretRef::parse("/missing-backend").is_none());
        assert!(SecretRef::parse("missing-path/").is_none());
    }

    #[test]
    fn audience_newtype_round_trips() {
        let a = Audience::new("urn:anthropic:api");
        let s = serde_json::to_string(&a).unwrap();
        // transparent → serialises as a bare string
        assert_eq!(s, "\"urn:anthropic:api\"");
        let back: Audience = serde_json::from_str(&s).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn oauth_requirement_round_trips() {
        let yaml = r#"
name: github_oauth
description: "GitHub OAuth for repo access."
audience: "https://api.github.com"
provider: github
scopes: [repo, read:org]
scope: install
optional: true
"#;
        let o: OAuthRequirement = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(o.name, "github_oauth");
        assert_eq!(o.provider, "github");
        assert_eq!(o.scopes, vec!["repo".to_string(), "read:org".to_string()]);
        assert_eq!(o.scope, AuthScope::Install);
        assert!(o.optional);
    }
}
