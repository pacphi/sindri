use sindri_core::component::ComponentManifest;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};

pub enum RegistryCmd {
    Refresh { name: String, url: String },
    Lint { path: String, json: bool },
    Trust { name: String, signer: String },
    FetchChecksums { path: String },
}

pub fn run(cmd: RegistryCmd) -> i32 {
    match cmd {
        RegistryCmd::Refresh { name, url } => refresh(&name, &url),
        RegistryCmd::Lint { path, json } => lint(&path, json),
        RegistryCmd::Trust { name, signer } => trust(&name, &signer),
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

fn trust(name: &str, signer: &str) -> i32 {
    let trust_dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("trust");

    if let Err(e) = std::fs::create_dir_all(&trust_dir) {
        eprintln!("Cannot create trust dir: {}", e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let entry = format!(
        r#"{{"registry":"{}","signer":"{}","stored_at":"{}"}}"#,
        name,
        signer,
        chrono_now()
    );

    match std::fs::write(trust_dir.join(format!("{}.json", name)), entry) {
        Ok(_) => {
            println!("Stored trust config for '{}' (signer: {})", name, signer);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Cannot store trust config: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
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

fn chrono_now() -> String {
    // Simple timestamp without chrono dep
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}
