//! `sindri secrets *` — secret reference validation, listing, and S3
//! storage helpers (Sprint 12, Wave 4C).
//!
//! This module never prints raw secret values. It validates that
//! configured `AuthValue`-style references resolve, lists their
//! source-kinds, and shells out to `aws s3` for an S3-backed secrets
//! store.
//!
//! The Vault HTTP client previously inline here has been factored into
//! [`sindri_secrets::VaultBackend`] (ADR-025, Wave 6F follow-up).
//!
//! Why shell out to `aws`? Pulling in `aws-sdk-s3` would add a multi-
//! megabyte dependency for what is, today, a thin convenience over
//! `aws s3 cp/ls`. The simplification is documented in the PR body and
//! can be revisited when v4 grows a real S3 backend.

use base64::Engine;
use serde::Serialize;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::manifest::BomManifest;
use sindri_secrets::{SecretStore, VaultBackend};
use sindri_targets::auth::AuthValue;
use std::path::{Path, PathBuf};
use std::process::Command;

/// CLI args parsed in `main.rs` for `sindri secrets *`.
pub enum SecretsCmd {
    /// `sindri secrets validate <id>`.
    Validate { id: String, manifest: PathBuf },
    /// `sindri secrets list [--json]`.
    List { json: bool, manifest: PathBuf },
    /// `sindri secrets test-vault`.
    TestVault,
    /// `sindri secrets encode-file <path> [--algorithm …] [--output …]`.
    EncodeFile {
        path: PathBuf,
        algorithm: String,
        output: Option<PathBuf>,
    },
    /// `sindri secrets s3 get <key> --bucket <b>`.
    S3Get { bucket: String, key: String },
    /// `sindri secrets s3 put <key> <file> --bucket <b>`.
    S3Put {
        bucket: String,
        key: String,
        file: PathBuf,
    },
    /// `sindri secrets s3 list --bucket <b> [--prefix <p>]`.
    S3List {
        bucket: String,
        prefix: Option<String>,
    },
}

/// Source kind reported by `secrets list`. Never carries the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    Env,
    File,
    Cli,
    Plain,
}

impl SourceKind {
    fn of(value: &str) -> Self {
        if value.starts_with("env:") {
            Self::Env
        } else if value.starts_with("file:") {
            Self::File
        } else if value.starts_with("cli:") {
            Self::Cli
        } else {
            Self::Plain
        }
    }
}

#[derive(Debug, Serialize)]
struct SecretEntry<'a> {
    id: &'a str,
    kind: SourceKind,
}

/// Top-level dispatcher for `secrets *`.
pub fn run(cmd: SecretsCmd) -> i32 {
    match cmd {
        SecretsCmd::Validate { id, manifest } => run_validate(&id, &manifest),
        SecretsCmd::List { json, manifest } => run_list(json, &manifest),
        SecretsCmd::TestVault => run_test_vault(),
        SecretsCmd::EncodeFile {
            path,
            algorithm,
            output,
        } => run_encode_file(&path, &algorithm, output.as_deref()),
        SecretsCmd::S3Get { bucket, key } => exec_status(s3_get_cmd(&bucket, &key)),
        SecretsCmd::S3Put { bucket, key, file } => exec_status(s3_put_cmd(&bucket, &key, &file)),
        SecretsCmd::S3List { bucket, prefix } => {
            exec_status(s3_list_cmd(&bucket, prefix.as_deref()))
        }
    }
}

fn load_manifest(path: &Path) -> Result<BomManifest, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    serde_yaml::from_str::<BomManifest>(&text)
        .map_err(|e| format!("parse {}: {}", path.display(), e))
}

fn run_validate(id: &str, manifest_path: &Path) -> i32 {
    let manifest = match load_manifest(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let raw = match manifest.secrets.get(id) {
        Some(v) => v,
        None => {
            eprintln!(
                "error: secret `{}` not configured in {}",
                id,
                manifest_path.display()
            );
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let av = match AuthValue::parse(raw) {
        Some(v) => v,
        None => {
            eprintln!("error: secret `{}` has unparseable value", id);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    match av.resolve() {
        Ok(_value) => {
            // NEVER print the value.
            println!("OK: secret `{}` resolves ({:?})", id, SourceKind::of(raw));
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("error: secret `{}` failed to resolve: {}", id, e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn run_list(json: bool, manifest_path: &Path) -> i32 {
    let manifest = match load_manifest(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let mut entries: Vec<SecretEntry<'_>> = manifest
        .secrets
        .iter()
        .map(|(id, v)| SecretEntry {
            id: id.as_str(),
            kind: SourceKind::of(v.as_str()),
        })
        .collect();
    entries.sort_by(|a, b| a.id.cmp(b.id));
    if json {
        match serde_json::to_string_pretty(&entries) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("error: serialise: {}", e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        }
    } else if entries.is_empty() {
        println!("(no secrets configured)");
    } else {
        println!("{:<32} SOURCE", "ID");
        for e in &entries {
            println!("{:<32} {:?}", e.id, e.kind);
        }
    }
    EXIT_SUCCESS
}

fn run_test_vault() -> i32 {
    // 1. Try the `sindri-secrets` VaultBackend (HTTP) if VAULT_TOKEN is set.
    if let Some(backend) = VaultBackend::from_env() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        match rt.block_on(backend.list()) {
            Ok(_) => {
                println!("OK: vault HTTP API reachable (VaultBackend)");
                return EXIT_SUCCESS;
            }
            Err(e) => {
                eprintln!("error: VaultBackend list failed: {}", e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        }
    }
    // 2. Fall back to `vault status` CLI; then `aws secretsmanager`.
    if let Some(vault) = which("vault") {
        let status = Command::new(vault).arg("status").status();
        return match status {
            Ok(s) if s.success() => {
                println!("OK: vault status responded successfully");
                EXIT_SUCCESS
            }
            _ => {
                eprintln!("error: `vault status` did not return success");
                EXIT_SCHEMA_OR_RESOLVE_ERROR
            }
        };
    }
    if let Some(aws) = which("aws") {
        let status = Command::new(aws)
            .args(["secretsmanager", "list-secrets", "--max-results", "1"])
            .status();
        return match status {
            Ok(s) if s.success() => {
                println!("OK: aws secretsmanager reachable");
                EXIT_SUCCESS
            }
            _ => {
                eprintln!("error: aws secretsmanager list-secrets failed");
                EXIT_SCHEMA_OR_RESOLVE_ERROR
            }
        };
    }
    eprintln!("error: neither `vault` nor `aws` CLI found on PATH");
    EXIT_SCHEMA_OR_RESOLVE_ERROR
}

fn run_encode_file(path: &Path, algorithm: &str, output: Option<&Path>) -> i32 {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: read {}: {}", path.display(), e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let encoded = match algorithm {
        "base64" => base64::engine::general_purpose::STANDARD.encode(&bytes),
        "sha256" => {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&bytes);
            hex::encode(h.finalize())
        }
        other => {
            eprintln!(
                "error: unsupported algorithm `{}` (use base64 or sha256)",
                other
            );
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    match output {
        Some(out) => {
            if let Err(e) = std::fs::write(out, &encoded) {
                eprintln!("error: write {}: {}", out.display(), e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
            println!("Wrote {} bytes to {}", encoded.len(), out.display());
        }
        None => println!("{}", encoded),
    }
    EXIT_SUCCESS
}

// ---- aws s3 shell-out helpers -------------------------------------------

/// Build the argv for `aws s3 ls s3://<bucket>/<prefix>`. Pure function so
/// tests can assert the shape without invoking aws.
pub fn s3_list_argv(bucket: &str, prefix: Option<&str>) -> Vec<String> {
    let url = match prefix {
        Some(p) if !p.is_empty() => format!("s3://{}/{}", bucket, p),
        _ => format!("s3://{}/", bucket),
    };
    vec!["s3".into(), "ls".into(), url]
}

/// Build the argv for `aws s3 cp s3://<bucket>/<key> -`.
pub fn s3_get_argv(bucket: &str, key: &str) -> Vec<String> {
    vec![
        "s3".into(),
        "cp".into(),
        format!("s3://{}/{}", bucket, key),
        "-".into(),
    ]
}

/// Build the argv for `aws s3 cp <file> s3://<bucket>/<key>`.
pub fn s3_put_argv(bucket: &str, key: &str, file: &Path) -> Vec<String> {
    vec![
        "s3".into(),
        "cp".into(),
        file.display().to_string(),
        format!("s3://{}/{}", bucket, key),
    ]
}

fn s3_list_cmd(bucket: &str, prefix: Option<&str>) -> Command {
    let mut c = Command::new("aws");
    c.args(s3_list_argv(bucket, prefix));
    c
}
fn s3_get_cmd(bucket: &str, key: &str) -> Command {
    let mut c = Command::new("aws");
    c.args(s3_get_argv(bucket, key));
    c
}
fn s3_put_cmd(bucket: &str, key: &str, file: &Path) -> Command {
    let mut c = Command::new("aws");
    c.args(s3_put_argv(bucket, key, file));
    c
}

fn exec_status(mut c: Command) -> i32 {
    match c.status() {
        Ok(s) if s.success() => EXIT_SUCCESS,
        Ok(s) => s.code().unwrap_or(EXIT_SCHEMA_OR_RESOLVE_ERROR),
        Err(e) => {
            eprintln!("error: spawn aws: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|d| {
            let c = d.join(name);
            if c.is_file() {
                Some(c)
            } else {
                None
            }
        })
    })
}

// ---- tests --------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_manifest(dir: &TempDir, body: &str) -> PathBuf {
        let p = dir.path().join("sindri.yaml");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        p
    }

    #[test]
    fn validate_env_value_present() {
        let dir = TempDir::new().unwrap();
        let key = "SINDRI_TEST_SECRET_PRESENT";
        // Ensure unique key to avoid cross-test interference.
        std::env::set_var(key, "shhh");
        let manifest = write_manifest(
            &dir,
            &format!("components: []\nsecrets:\n  api: env:{}\n", key),
        );
        let code = run_validate("api", &manifest);
        std::env::remove_var(key);
        assert_eq!(code, EXIT_SUCCESS);
    }

    #[test]
    fn validate_env_value_missing_errors() {
        let dir = TempDir::new().unwrap();
        let key = "SINDRI_TEST_SECRET_DEFINITELY_NOT_SET_X9";
        std::env::remove_var(key);
        let manifest = write_manifest(
            &dir,
            &format!("components: []\nsecrets:\n  api: env:{}\n", key),
        );
        let code = run_validate("api", &manifest);
        assert_ne!(code, EXIT_SUCCESS);
    }

    #[test]
    fn validate_file_value_present() {
        let dir = TempDir::new().unwrap();
        let secret = dir.path().join("token");
        std::fs::write(&secret, "topsecretvalue").unwrap();
        let manifest = write_manifest(
            &dir,
            &format!(
                "components: []\nsecrets:\n  tok: file:{}\n",
                secret.display()
            ),
        );
        let code = run_validate("tok", &manifest);
        assert_eq!(code, EXIT_SUCCESS);
    }

    #[test]
    fn list_never_prints_values() {
        let dir = TempDir::new().unwrap();
        let secret_value = "DO_NOT_LEAK_ABCDEF";
        std::env::set_var("SINDRI_LIST_TEST", secret_value);
        let manifest = write_manifest(
            &dir,
            "components: []\nsecrets:\n  api: env:SINDRI_LIST_TEST\n",
        );

        // Invoke list and ensure the secret value never makes it into a
        // serialised JSON dump (we test the data shape because capturing
        // stdout from run_list is awkward in unit tests).
        let parsed = load_manifest(&manifest).unwrap();
        let entries: Vec<SecretEntry<'_>> = parsed
            .secrets
            .iter()
            .map(|(id, v)| SecretEntry {
                id,
                kind: SourceKind::of(v),
            })
            .collect();
        let json = serde_json::to_string(&entries).unwrap();
        assert!(
            !json.contains(secret_value),
            "list output leaked secret value"
        );
        std::env::remove_var("SINDRI_LIST_TEST");
    }

    #[test]
    fn encode_file_base64_round_trip() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("payload.bin");
        let payload = b"hello sindri";
        std::fs::write(&src, payload).unwrap();
        let out = dir.path().join("encoded.txt");
        let code = run_encode_file(&src, "base64", Some(&out));
        assert_eq!(code, EXIT_SUCCESS);
        let encoded = std::fs::read_to_string(&out).unwrap();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn s3_list_command_built_correctly() {
        assert_eq!(
            s3_list_argv("my-bucket", Some("path/")),
            vec!["s3", "ls", "s3://my-bucket/path/"]
        );
        assert_eq!(
            s3_list_argv("my-bucket", None),
            vec!["s3", "ls", "s3://my-bucket/"]
        );
        assert_eq!(s3_get_argv("b", "k"), vec!["s3", "cp", "s3://b/k", "-"]);
        let f = std::path::PathBuf::from("/tmp/payload");
        assert_eq!(
            s3_put_argv("b", "k", &f),
            vec!["s3", "cp", "/tmp/payload", "s3://b/k"]
        );
    }
}
