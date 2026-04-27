use sindri_core::component::ComponentManifest;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_registry::signing::TrustedKey;
use sindri_registry::{CosignVerifier, OciRef, RegistryClient};

pub enum RegistryCmd {
    Refresh {
        name: String,
        url: String,
        insecure: bool,
    },
    Lint {
        path: String,
        json: bool,
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
}

pub fn run(cmd: RegistryCmd) -> i32 {
    match cmd {
        RegistryCmd::Refresh {
            name,
            url,
            insecure,
        } => refresh(&name, &url, insecure),
        RegistryCmd::Lint { path, json } => lint(&path, json),
        RegistryCmd::Trust { name, signer } => trust(&name, &signer),
        RegistryCmd::Verify { name, url } => verify(&name, &url),
        RegistryCmd::FetchChecksums { path } => fetch_checksums(&path),
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

        let trust_dir = dirs_next::home_dir()
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

fn lint(path: &str, json: bool) -> i32 {
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
        return lint_dir(p, json);
    }

    lint_file(p, json)
}

fn lint_file(p: &std::path::Path, json: bool) -> i32 {
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

    if errors.is_empty() {
        if json {
            println!(r#"{{"valid":true,"path":"{}"}}"#, p.display());
        } else {
            println!("{}: OK", p.display());
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

fn lint_dir(dir: &std::path::Path, json: bool) -> i32 {
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
            && lint_file(&path, json) != EXIT_SUCCESS
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

    let trust_dir = dirs_next::home_dir()
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
        let trust_dir = dirs_next::home_dir()
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
