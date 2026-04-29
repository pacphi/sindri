use sindri_core::component::ComponentManifest;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_registry::signing::TrustedKey;
use sindri_registry::{CosignVerifier, OciRef, RegistryClient};

pub mod serve;

pub enum RegistryCmd {
    Refresh {
        name: String,
        url: String,
        insecure: bool,
    },
    Lint {
        path: String,
        json: bool,
        /// Enable the auth-aware lint rule (ADR-026 Phase 3): warn on
        /// components in known-credentialed categories that lack an `auth:`
        /// block. Warning-only — never fails the build.
        auth: bool,
    },
    Trust {
        name: String,
        signer: String,
    },
    Verify {
        name: String,
        url: String,
    },
    FetchChecksums {
        path: String,
    },
    /// Embedded OCI registry over a components directory (Phase 3.2,
    /// ADR-028).
    Serve {
        addr: String,
        root: String,
        sign_with: Option<String>,
    },
}

pub fn run(cmd: RegistryCmd) -> i32 {
    match cmd {
        RegistryCmd::Refresh {
            name,
            url,
            insecure,
        } => refresh(&name, &url, insecure),
        RegistryCmd::Lint { path, json, auth } => lint(&path, json, auth),
        RegistryCmd::Trust { name, signer } => trust(&name, &signer),
        RegistryCmd::Verify { name, url } => verify(&name, &url),
        RegistryCmd::FetchChecksums { path } => fetch_checksums(&path),
        RegistryCmd::Serve {
            addr,
            root,
            sign_with,
        } => serve::run(&addr, &root, sign_with),
    }
}

/// Refresh a registry index via the live OCI Distribution Spec pipeline
/// (ADR-003) and verify its cosign signature (ADR-014) before caching.
///
/// `--insecure` bypasses cosign verification with a loud warning. It is
/// rejected when the active install policy sets `require_signed_registries`.
fn refresh(name: &str, url: &str, insecure: bool) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Cannot start async runtime: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    runtime.block_on(async move {
        let mut client = match RegistryClient::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Cannot construct registry client: {}", e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        };

        // Load policy + trust keys. Policy may not exist yet for a fresh
        // install; default to permissive in that case so `--insecure`
        // semantics are usable out of the box.
        let policy = sindri_policy::loader::load_effective_policy().policy;
        client = client.with_policy(policy);

        let trust_dir = sindri_core::paths::home_dir()
            .unwrap_or_default()
            .join(".sindri")
            .join("trust");
        match CosignVerifier::load_from_trust_dir(&trust_dir) {
            Ok(v) => client = client.with_verifier(v),
            Err(e) => {
                eprintln!("Cannot load trust keys from {}: {}", trust_dir.display(), e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        }

        if insecure {
            client = client.with_insecure(true);
            tracing::warn!(
                "INSECURE: cosign verification will be skipped for registry '{}'",
                name
            );
        }

        match client.refresh_index(name, url).await {
            Ok((index, digest)) => {
                let bytes_hint = match serde_yaml::to_string(&index) {
                    Ok(s) => s.len(),
                    Err(_) => 0,
                };
                match digest {
                    Some(d) => println!(
                        "Registry '{}' refreshed (digest {}, {} components)",
                        name,
                        d,
                        index.components.len()
                    ),
                    None => println!(
                        "Registry '{}' refreshed from local protocol ({} components, {} bytes)",
                        name,
                        index.components.len(),
                        bytes_hint
                    ),
                }
                EXIT_SUCCESS
            }
            Err(e) => {
                eprintln!("Registry refresh failed: {}", e);
                EXIT_SCHEMA_OR_RESOLVE_ERROR
            }
        }
    })
}

fn lint(path: &str, json: bool, auth: bool) -> i32 {
    let p = std::path::Path::new(path);
    if !p.exists() {
        let msg = format!("Path not found: {}", path);
        if json {
            eprintln!(r#"{{"error":"FILE_NOT_FOUND","path":"{}"}}"#, path);
        } else {
            eprintln!("{}", msg);
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    if p.is_dir() {
        return lint_dir(p, json, auth);
    }

    lint_file(p, json, auth)
}

/// Categories that historically require credentials. Components whose `tags`
/// intersect this set should declare an `auth:` block (ADR-026 Phase 3).
///
/// Detection is tag-based — the v4 component schema has no dedicated
/// `category` field; the survey
/// (`v4/docs/research/auth-aware-survey-2026-04-28.md`) groups components by
/// these well-known tags.
pub(crate) const AUTH_CREDENTIALED_TAGS: &[&str] = &[
    // Cloud-CLI bucket: aws-cli/azure-cli/gcloud/ibmcloud/aliyun/doctl/flyctl.
    "cloud",
    // AI-dev bucket: claude-code/codex/gemini-cli/grok/goose/droid/opencode/
    // claudish/compahook/ruflo/claude-marketplace.
    "ai", "ai-dev",
    // MCP servers: linear-mcp/jira-mcp/pal-mcp-server/notebooklm-mcp-cli.
    "mcp",
];

/// Marker comment that opts a component out of the `--auth` lint rule.
/// Must appear within the first few lines of `component.yaml`.
pub(crate) const AUTH_LINT_OPTOUT: &str = "# sindri-lint: auth-not-required";

/// Result of the `--auth` lint check on a single component.
pub(crate) struct AuthLintFinding {
    /// Tag(s) that triggered the credentialed-category match.
    pub matched_tags: Vec<String>,
}

/// Apply the auth-aware lint rule to a parsed manifest + raw YAML body.
///
/// Returns:
/// - `Ok(None)`  — clean (either the component declares `auth:`, opts out via
///   the marker comment, or doesn't fall into a credentialed category).
/// - `Ok(Some(finding))` — component falls into a credentialed category but
///   lacks an `auth:` block. The caller should emit a **warning**, not an
///   error: this rule never fails a build.
pub(crate) fn auth_lint_check(
    manifest: &ComponentManifest,
    yaml_body: &str,
) -> Option<AuthLintFinding> {
    // Opt-out via leading comment annotation.
    let head: String = yaml_body.lines().take(8).collect::<Vec<_>>().join("\n");
    if head.contains(AUTH_LINT_OPTOUT) {
        return None;
    }

    // Already declares auth — fine.
    if !manifest.auth.is_empty() {
        return None;
    }

    // Otherwise, check tags for credentialed-category membership.
    let matched: Vec<String> = manifest
        .metadata
        .tags
        .iter()
        .filter(|t| AUTH_CREDENTIALED_TAGS.contains(&t.as_str()))
        .cloned()
        .collect();

    if matched.is_empty() {
        None
    } else {
        Some(AuthLintFinding {
            matched_tags: matched,
        })
    }
}

fn lint_file(p: &std::path::Path, json: bool, auth: bool) -> i32 {
    let content = match std::fs::read_to_string(p) {
        Ok(c) => c,
        Err(e) => {
            if json {
                eprintln!(r#"{{"error":"READ_ERROR","detail":"{}"}}"#, e);
            } else {
                eprintln!("Cannot read {}: {}", p.display(), e);
            }
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let manifest: ComponentManifest = match serde_yaml::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            if json {
                eprintln!(
                    r#"{{"error":"PARSE_ERROR","path":"{}","detail":"{}"}}"#,
                    p.display(),
                    e
                );
            } else {
                eprintln!("{}: FAILED (parse error: {})", p.display(), e);
            }
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let mut errors: Vec<String> = Vec::new();

    if manifest.platforms.is_empty() {
        errors.push("LINT_EMPTY_PLATFORMS: `platforms` must not be empty".into());
    }
    if manifest.metadata.license.trim().is_empty() {
        errors.push(
            "LINT_MISSING_LICENSE: `metadata.license` must be a valid SPDX identifier".into(),
        );
    }
    if manifest
        .install
        .binary
        .as_ref()
        .map(|b| b.checksums.is_empty())
        .unwrap_or(false)
    {
        errors.push("LINT_MISSING_CHECKSUMS: `binary` components must have `checksums`".into());
    }
    if let Some(cap) = &manifest.capabilities.collision_handling {
        let expected = format!("{}/", manifest.metadata.name);
        if !cap.path_prefix.starts_with(&expected) && cap.path_prefix != ":shared" {
            errors.push(format!(
                "LINT_COLLISION_PREFIX: path_prefix must start with `{}/`",
                manifest.metadata.name
            ));
        }
    }

    // Auth-aware lint rule (ADR-026 Phase 3). Warning-only — does not affect
    // exit code. Only runs when the caller passed `--auth`.
    let auth_warning: Option<String> = if auth {
        auth_lint_check(&manifest, &content).map(|finding| {
            format!(
                "LINT_AUTH_MISSING: component is in a credentialed category (tags: {}) but \
                 has no `auth:` block. Either declare credentials per ADR-026 or add the \
                 opt-out comment `{}` at the top of component.yaml.",
                finding.matched_tags.join(", "),
                AUTH_LINT_OPTOUT
            )
        })
    } else {
        None
    };

    if errors.is_empty() {
        if json {
            // Embed the warning in the JSON object (warnings field) when
            // present so machine-readable consumers see it too.
            match &auth_warning {
                Some(w) => println!(
                    r#"{{"valid":true,"path":"{}","warnings":[{{"warning":"{}"}}]}}"#,
                    p.display(),
                    w.replace('"', "\\\"")
                ),
                None => println!(r#"{{"valid":true,"path":"{}"}}"#, p.display()),
            }
        } else {
            println!("{}: OK", p.display());
            if let Some(w) = &auth_warning {
                eprintln!("  ⚠ {}", w);
            }
        }
        EXIT_SUCCESS
    } else {
        if json {
            let errs: Vec<serde_json::Value> = errors
                .iter()
                .map(|e| serde_json::json!({"error": e}))
                .collect();
            println!(
                "{}",
                serde_json::to_string(
                    &serde_json::json!({"valid":false,"path":p.display().to_string(),"errors":errs})
                )
                .unwrap_or_default()
            );
        } else {
            eprintln!("{}: FAILED", p.display());
            for e in &errors {
                eprintln!("  - {}", e);
            }
        }
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    }
}

fn lint_dir(dir: &std::path::Path, json: bool, auth: bool) -> i32 {
    let mut any_failed = false;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Cannot read directory: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "yaml").unwrap_or(false)
            && lint_file(&path, json, auth) != EXIT_SUCCESS
        {
            any_failed = true;
        }
    }

    if any_failed {
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    } else {
        EXIT_SUCCESS
    }
}

/// Trust a cosign public key for a registry (ADR-014).
///
/// `signer` accepts the form `cosign:key=<path>` (the canonical syntax in
/// ADR-014) or a bare path. The key is read, parsed as a P-256 SPKI PEM, and
/// copied to `~/.sindri/trust/<name>/cosign-<short-key-id>.pub`.
///
/// Wave 3A.1 lands the actual copy + parse step (the audit flagged the
/// previous behaviour — writing a JSON sidecar with the raw path — as
/// security theatre). Wave 3A.2 wires the loaded keys into
/// `RegistryClient::verify`.
fn trust(name: &str, signer: &str) -> i32 {
    // Accept either `cosign:key=<path>` or a bare path.
    let path_str = signer.strip_prefix("cosign:key=").unwrap_or(signer).trim();
    if path_str.is_empty() {
        eprintln!("Empty signer; expected `cosign:key=<path>` or a path to a P-256 PEM public key");
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let src = std::path::Path::new(path_str);
    let pem = match std::fs::read_to_string(src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot read signer key '{}': {}", path_str, e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let key = match TrustedKey::from_pem(&pem) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Invalid cosign public key at '{}': {}", path_str, e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let trust_dir = sindri_core::paths::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("trust")
        .join(name);

    if let Err(e) = std::fs::create_dir_all(&trust_dir) {
        eprintln!("Cannot create trust dir '{}': {}", trust_dir.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let target = trust_dir.join(format!("cosign-{}.pub", key.key_id));
    let tmp = target.with_extension("tmp");
    if let Err(e) = std::fs::write(&tmp, &pem) {
        eprintln!("Cannot write trust key '{}': {}", target.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    if let Err(e) = std::fs::rename(&tmp, &target) {
        eprintln!("Cannot finalize trust key '{}': {}", target.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    println!(
        "Trusted cosign key {} for registry '{}' (stored at {})",
        key.key_id,
        name,
        target.display()
    );
    EXIT_SUCCESS
}

/// Verify the cosign signature on a registry's artifact (ADR-014).
///
/// Wave 3A.2: runs the full cosign verification flow against the trust
/// keys in `~/.sindri/trust/<name>/`. The OCI ref must be supplied because
/// the CLI does not yet maintain a registry-name → URL map.
fn verify(name: &str, url: &str) -> i32 {
    let oci_ref = match OciRef::parse(url) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Invalid OCI reference '{}': {}", url, e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Cannot start async runtime: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    runtime.block_on(async move {
        let mut client = match RegistryClient::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Cannot construct registry client: {}", e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        };
        client = client.with_policy(sindri_policy::loader::load_effective_policy().policy);
        let trust_dir = sindri_core::paths::home_dir()
            .unwrap_or_default()
            .join(".sindri")
            .join("trust");
        match CosignVerifier::load_from_trust_dir(&trust_dir) {
            Ok(v) => client = client.with_verifier(v),
            Err(e) => {
                eprintln!("Cannot load trust keys: {}", e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        }

        match client.verify(name, &oci_ref).await {
            Ok(key_id) if key_id == "<unsigned>" => {
                println!(
                    "Registry '{}': no trust keys configured; verification skipped (permissive policy)",
                    name
                );
                EXIT_SUCCESS
            }
            Ok(key_id) => {
                println!(
                    "Verified registry '{}': signed by trusted key {}",
                    name, key_id
                );
                EXIT_SUCCESS
            }
            Err(e) => {
                eprintln!("Verification failed for registry '{}': {}", name, e);
                EXIT_SCHEMA_OR_RESOLVE_ERROR
            }
        }
    })
}

fn fetch_checksums(path: &str) -> i32 {
    // Sprint 2: stub — downloads assets and computes sha256
    // Full implementation uses sha2 crate + reqwest
    println!(
        "fetch-checksums for {}: stub (Sprint 2 — full download in Sprint 6)",
        path
    );
    EXIT_SUCCESS
}

#[cfg(test)]
mod auth_lint_tests {
    use super::*;

    /// `auth_lint_check` should flag a `cloud`-tagged component that has no
    /// `auth:` block. The caller treats this as a warning — not an error —
    /// so `lint_file` returns success and `--auth` never breaks the build.
    #[test]
    fn warns_on_cloud_component_without_auth() {
        let yaml = r#"
metadata:
  name: testcloud
  version: "1.0.0"
  description: "Test cloud component fixture."
  license: MIT
  tags:
    - cloud
platforms:
  - os: linux
    arch: x86_64
install:
  script:
    install: "install.sh"
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let f = auth_lint_check(&m, yaml);
        assert!(
            f.is_some(),
            "expected a warning for cloud component without auth"
        );
        let f = f.unwrap();
        assert!(f.matched_tags.iter().any(|t| t == "cloud"));
    }

    /// `mcp`-tagged components without `auth:` should warn.
    #[test]
    fn warns_on_mcp_component_without_auth() {
        let yaml = r#"
metadata:
  name: testmcp
  version: "1.0.0"
  description: "Test mcp component fixture."
  license: MIT
  tags:
    - mcp
    - linear
platforms:
  - os: linux
    arch: x86_64
install:
  script:
    install: "install.sh"
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        assert!(auth_lint_check(&m, yaml).is_some());
    }

    /// Components with an `auth:` block — even a minimal one — pass clean.
    #[test]
    fn clean_when_auth_block_present() {
        let yaml = r#"
metadata:
  name: testcloud
  version: "1.0.0"
  description: "Test cloud component fixture."
  license: MIT
  tags:
    - cloud
platforms:
  - os: linux
    arch: x86_64
install:
  script:
    install: "install.sh"
auth:
  tokens:
    - name: provider_token
      description: "Some provider token."
      audience: "https://api.example.com"
      redemption:
        kind: env-var
        env-name: PROVIDER_TOKEN
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        assert!(auth_lint_check(&m, yaml).is_none());
    }

    /// Opt-out comment at the top of the file suppresses the warning.
    #[test]
    fn clean_with_optout_comment() {
        let yaml = r#"# sindri-lint: auth-not-required
metadata:
  name: testcloud
  version: "1.0.0"
  description: "Test cloud component fixture."
  license: MIT
  tags:
    - cloud
platforms:
  - os: linux
    arch: x86_64
install:
  script:
    install: "install.sh"
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        assert!(
            auth_lint_check(&m, yaml).is_none(),
            "opt-out comment must suppress the warning"
        );
    }

    /// Components outside the credentialed-tag set never warn.
    #[test]
    fn clean_when_not_credentialed_category() {
        let yaml = r#"
metadata:
  name: testlang
  version: "1.0.0"
  description: "Test language component fixture."
  license: MIT
  tags:
    - language
    - rust
platforms:
  - os: linux
    arch: x86_64
install:
  mise:
    tools:
      rust: "1.83.0"
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        assert!(auth_lint_check(&m, yaml).is_none());
    }

    /// End-to-end: `lint_file --auth` on a credentialed-tag fixture without
    /// `auth:` must return SUCCESS (warning-only contract).
    #[test]
    fn lint_file_warning_only_does_not_fail_build() {
        let yaml = r#"
metadata:
  name: testcloud
  version: "1.0.0"
  description: "Test cloud component fixture."
  license: MIT
  tags:
    - cloud
platforms:
  - os: linux
    arch: x86_64
install:
  script:
    install: "install.sh"
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("component.yaml");
        std::fs::write(&path, yaml).unwrap();

        // With --auth: we still expect EXIT_SUCCESS because the rule is
        // warning-only.
        let rc = lint_file(&path, true, /* auth = */ true);
        assert_eq!(rc, EXIT_SUCCESS);

        // Without --auth: also success (rule is gated on the flag).
        let rc = lint_file(&path, false, /* auth = */ false);
        assert_eq!(rc, EXIT_SUCCESS);
    }
}
