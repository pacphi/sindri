use sindri_core::component::ComponentManifest;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_registry::signing::TrustedKey;

pub enum RegistryCmd {
    Refresh { name: String, url: String },
    Lint { path: String, json: bool },
    Trust { name: String, signer: String },
    Verify { name: String },
    FetchChecksums { path: String },
}

pub fn run(cmd: RegistryCmd) -> i32 {
    match cmd {
        RegistryCmd::Refresh { name, url } => refresh(&name, &url),
        RegistryCmd::Lint { path, json } => lint(&path, json),
        RegistryCmd::Trust { name, signer } => trust(&name, &signer),
        RegistryCmd::Verify { name } => verify(&name),
        RegistryCmd::FetchChecksums { path } => fetch_checksums(&path),
    }
}

fn refresh(name: &str, url: &str) -> i32 {
    // local registry protocol
    if let Some(path) = url.strip_prefix("registry:local:") {
        let index_path = std::path::Path::new(path).join("index.yaml");
        if !index_path.exists() {
            eprintln!("Local registry index not found: {}", index_path.display());
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
        let content = match std::fs::read_to_string(&index_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Cannot read local registry: {}", e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        };
        return write_to_cache(name, &content);
    }

    // HTTP fetch via curl (matches Sprint 2 risk mitigation strategy)
    let index_url = format!("{}/index.yaml", url.trim_end_matches('/'));
    eprintln!("Fetching registry index from {}...", index_url);

    match std::process::Command::new("curl")
        .args(["-sSfL", &index_url, "--max-time", "30"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let content = String::from_utf8_lossy(&out.stdout);
            write_to_cache(name, &content)
        }
        Ok(out) => {
            eprintln!("curl failed: {}", String::from_utf8_lossy(&out.stderr));
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
        Err(e) => {
            eprintln!(
                "curl not available: {}. Install curl to fetch registries.",
                e
            );
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn write_to_cache(name: &str, content: &str) -> i32 {
    let cache_dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("cache")
        .join("registries")
        .join(name);

    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        eprintln!("Cannot create cache dir: {}", e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let index_path = cache_dir.join("index.yaml");
    let tmp_path = cache_dir.join("index.yaml.tmp");

    if let Err(e) = std::fs::write(&tmp_path, content) {
        eprintln!("Cannot write cache: {}", e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    if let Err(e) = std::fs::rename(&tmp_path, &index_path) {
        eprintln!("Cannot finalize cache: {}", e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    println!("Registry '{}' refreshed ({} bytes)", name, content.len());
    EXIT_SUCCESS
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

/// Verify the cosign signature on a registry's manifest (ADR-014).
///
/// **Wave 3A.1 placeholder.** Verification — fetching the cosign signature
/// manifest, decoding the simple-signing payload, and verifying the
/// signature bytes against trusted keys — lands in Wave 3A.2. Today this
/// command exits non-zero with a clear message so callers can wire it into
/// CI without it silently passing.
fn verify(name: &str) -> i32 {
    eprintln!(
        "registry verify '{}': not yet implemented (deferred to Wave 3A.2). \
         Trust-key loading is in place; signature verification will be wired \
         once the live oci-client fetch path lands.",
        name
    );
    EXIT_SCHEMA_OR_RESOLVE_ERROR
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
