//! `sindri secrets *` — secret reference validation, listing, S3 storage
//! helpers, and native HashiCorp Vault HTTP API client (Sprint 12, Wave 6F / D8).
//!
//! # D8 — Vault native HTTP implementation
//!
//! The previous `secrets test-vault` sub-command shelled out to the `vault`
//! CLI binary. Sprint 12.x (D8) replaces that shell-out with direct HTTP calls
//! against the Vault KV v2 API using `reqwest`. Architecture choice: **the CLI
//! shell-out path has been removed entirely** rather than kept behind a
//! `cli-fallback` Cargo feature. Rationale:
//!
//! - A feature flag adds compile-time complexity without runtime benefit on any
//!   supported deployment (all real Vault instances expose the HTTP API).
//! - The native path is strictly better: no `vault` binary dependency on the
//!   host, no PATH sensitivity, richer error classification (403 vs 404 vs 503).
//! - The removed code was roughly 15 lines — below the "worth keeping" bar.
//!
//! The new [`VaultClient`] implements:
//! - `GET /v1/{mount}/data/{path}` — read secret (KV v2)
//! - `POST /v1/{mount}/data/{path}` — write secret (KV v2)
//! - `DELETE /v1/{mount}/data/{path}` — soft-delete latest version
//! - `GET /v1/sys/health` — used by `secrets test-vault` to probe liveness
//!
//! Auth: `VAULT_ADDR` (default `http://127.0.0.1:8200`) + `VAULT_TOKEN` env.
//! TLS skip-verify: gated behind the `tls_skip_verify` flag on [`VaultClient`].
//!
//! # S3 shell-out rationale (unchanged)
//!
//! Why shell out to `aws`? Pulling in `aws-sdk-s3` would add a multi-megabyte
//! dependency for what is, today, a thin convenience over `aws s3 cp/ls`.

use base64::Engine;
use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::manifest::BomManifest;
use sindri_targets::auth::AuthValue;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

// ---- Vault HTTP client (D8) ---------------------------------------------

/// Error conditions returned by the Vault HTTP API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultError {
    /// 403 — token lacks the required capability on the path.
    MissingCapability { path: String },
    /// 404 — path not found (secret does not exist or mount is absent).
    PathNotFound { path: String },
    /// 503 — Vault is sealed or unavailable.
    Sealed { detail: String },
    /// Any other HTTP error.
    Http { status: u16, body: String },
    /// Transport-level error (connection refused, TLS failure, etc.).
    Transport(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::MissingCapability { path } => {
                write!(f, "Vault 403: token lacks capability for `{}`", path)
            }
            VaultError::PathNotFound { path } => {
                write!(f, "Vault 404: path `{}` not found", path)
            }
            VaultError::Sealed { detail } => write!(f, "Vault sealed / unavailable: {}", detail),
            VaultError::Http { status, body } => {
                write!(f, "Vault HTTP {}: {}", status, body.trim())
            }
            VaultError::Transport(e) => write!(f, "Vault transport error: {}", e),
        }
    }
}

/// The KV v2 data envelope returned by `GET /v1/{mount}/data/{path}`.
#[derive(Debug, Deserialize)]
pub struct VaultKvData {
    pub data: serde_json::Value,
}

/// Thin native HTTP wrapper around the Vault KV v2 API.
///
/// Construction:
/// ```rust,ignore
/// let client = VaultClient::from_env();
/// ```
///
/// For tests, construct directly with [`VaultClient::new`].
pub struct VaultClient {
    /// Base URL, e.g. `http://127.0.0.1:8200`.
    base_url: String,
    /// Vault token (`X-Vault-Token` header).
    token: String,
    /// When `true`, TLS certificate errors are ignored. **Use only in
    /// development/test environments.**
    pub tls_skip_verify: bool,
    /// Optional URL override for the HTTP client (used in tests to point at
    /// a wiremock server). When `None` the client builds its own.
    client_override: Option<HttpClient>,
}

impl VaultClient {
    /// Construct from explicit parameters.
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token: token.into(),
            tls_skip_verify: false,
            client_override: None,
        }
    }

    /// Construct from `VAULT_ADDR` / `VAULT_TOKEN` environment variables.
    ///
    /// Defaults to `http://127.0.0.1:8200` when `VAULT_ADDR` is absent.
    /// Returns `None` when `VAULT_TOKEN` is not set.
    pub fn from_env() -> Option<Self> {
        let addr =
            std::env::var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".to_string());
        let token = std::env::var("VAULT_TOKEN").ok()?;
        Some(Self::new(addr, token))
    }

    fn http_client(&self) -> Result<HttpClient, VaultError> {
        if let Some(ref c) = self.client_override {
            return Ok(c.clone());
        }
        reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(self.tls_skip_verify)
            .build()
            .map_err(|e| VaultError::Transport(e.to_string()))
    }

    fn map_status(status: u16, path: &str, body: String) -> VaultError {
        match status {
            403 => VaultError::MissingCapability {
                path: path.to_string(),
            },
            404 => VaultError::PathNotFound {
                path: path.to_string(),
            },
            503 => VaultError::Sealed { detail: body },
            other => VaultError::Http {
                status: other,
                body,
            },
        }
    }

    /// `GET /v1/{mount}/data/{path}` — read a KV v2 secret.
    pub fn read_kv2(&self, mount: &str, path: &str) -> Result<VaultKvData, VaultError> {
        let url = format!("{}/v1/{}/data/{}", self.base_url, mount, path);
        let client = self.http_client()?;
        let resp = client
            .get(&url)
            .header("X-Vault-Token", &self.token)
            .send()
            .map_err(|e| VaultError::Transport(e.to_string()))?;
        let status = resp.status().as_u16();
        let body = resp.text().unwrap_or_default();
        if status == 200 {
            serde_json::from_str::<VaultKvData>(&body)
                .map_err(|e| VaultError::Transport(format!("JSON decode: {}", e)))
        } else {
            Err(Self::map_status(status, path, body))
        }
    }

    /// `POST /v1/{mount}/data/{path}` — write a KV v2 secret.
    ///
    /// `data` should be a JSON object of key-value string pairs, e.g.
    /// `{"key": "value"}`.
    pub fn write_kv2(
        &self,
        mount: &str,
        path: &str,
        data: serde_json::Value,
    ) -> Result<(), VaultError> {
        let url = format!("{}/v1/{}/data/{}", self.base_url, mount, path);
        let client = self.http_client()?;
        let body = serde_json::json!({ "data": data });
        let resp = client
            .post(&url)
            .header("X-Vault-Token", &self.token)
            .json(&body)
            .send()
            .map_err(|e| VaultError::Transport(e.to_string()))?;
        let status = resp.status().as_u16();
        if status == 200 || status == 204 {
            Ok(())
        } else {
            let body = resp.text().unwrap_or_default();
            Err(Self::map_status(status, path, body))
        }
    }

    /// `DELETE /v1/{mount}/data/{path}` — soft-delete the latest version.
    pub fn delete_kv2(&self, mount: &str, path: &str) -> Result<(), VaultError> {
        let url = format!("{}/v1/{}/data/{}", self.base_url, mount, path);
        let client = self.http_client()?;
        let resp = client
            .delete(&url)
            .header("X-Vault-Token", &self.token)
            .send()
            .map_err(|e| VaultError::Transport(e.to_string()))?;
        let status = resp.status().as_u16();
        if status == 200 || status == 204 {
            Ok(())
        } else {
            let body = resp.text().unwrap_or_default();
            Err(Self::map_status(status, path, body))
        }
    }

    /// `GET /v1/sys/health` — probe Vault liveness.
    ///
    /// Returns `Ok(())` when Vault is healthy and unsealed.
    /// Returns `Err(VaultError::Sealed)` on 503 (sealed / standby).
    pub fn health(&self) -> Result<(), VaultError> {
        let url = format!("{}/v1/sys/health", self.base_url);
        let client = self.http_client()?;
        let resp = client
            .get(&url)
            .header("X-Vault-Token", &self.token)
            .send()
            .map_err(|e| VaultError::Transport(e.to_string()))?;
        let status = resp.status().as_u16();
        match status {
            200 => Ok(()),
            503 => {
                let body = resp.text().unwrap_or_default();
                Err(VaultError::Sealed { detail: body })
            }
            _ => {
                let body = resp.text().unwrap_or_default();
                Err(VaultError::Http { status, body })
            }
        }
    }
}

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
    // D8: native HTTP probe via VaultClient; no `vault` CLI binary required.
    // We look for VAULT_TOKEN; if absent we can still probe the health endpoint
    // (which does not require auth) to check if Vault is alive and unsealed.
    let addr = std::env::var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".to_string());
    let token = std::env::var("VAULT_TOKEN").unwrap_or_default();
    let client = VaultClient::new(&addr, &token);
    match client.health() {
        Ok(()) => {
            println!("OK: Vault at {} is healthy and unsealed", addr);
            EXIT_SUCCESS
        }
        Err(VaultError::Sealed { detail }) => {
            eprintln!(
                "error: Vault at {} is sealed or unavailable: {}",
                addr, detail
            );
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
        Err(e) => {
            eprintln!("error: Vault health probe failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
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

    // ---- Vault HTTP API tests (D8, wiremock) --------------------------------
    //
    // The VaultClient uses reqwest::blocking, which requires that no tokio
    // runtime exists on the current thread (blocking clients create their own
    // single-threaded runtime internally). We therefore use plain `#[test]`
    // plus `tokio::runtime::Runtime::new()` to drive the wiremock MockServer
    // setup, then call the blocking VaultClient from the same thread after
    // dropping the tokio handle.
    //
    // Alternatively we use `tokio::task::spawn_blocking` from inside a
    // `#[tokio::test]` to run the blocking call off the async thread.

    fn vault_client_for(base_url: &str) -> VaultClient {
        VaultClient::new(base_url, "test-token")
    }

    /// Helper: spin up a wiremock server, register `mock_fn` against it, then
    /// run `test_fn` with the server URI in a `spawn_blocking` context so the
    /// reqwest blocking client does not conflict with the tokio runtime.
    async fn with_mock_server<F, T>(
        mock_fn: impl Fn(&str) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
        test_fn: F,
    ) where
        F: FnOnce(String) -> T + Send + 'static,
        T: Send + 'static,
    {
        use wiremock::MockServer;
        let server = MockServer::start().await;
        let uri = server.uri();
        // Register mocks via the closure.
        mock_fn(&uri).await;
        let uri_clone = uri.clone();
        tokio::task::spawn_blocking(move || test_fn(uri_clone))
            .await
            .expect("spawn_blocking panicked");
        // Keep server alive until here.
        drop(server);
    }

    #[tokio::test]
    async fn vault_read_kv2_success() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/secret/data/myapp/config"))
            .and(header("X-Vault-Token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "key": "value" }
            })))
            .mount(&server)
            .await;

        let uri = server.uri();
        tokio::task::spawn_blocking(move || {
            let client = vault_client_for(&uri);
            let kv = client.read_kv2("secret", "myapp/config").unwrap();
            assert_eq!(kv.data["key"], "value");
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn vault_write_kv2_success() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/secret/data/myapp/config"))
            .and(header("X-Vault-Token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {}
            })))
            .mount(&server)
            .await;

        let uri = server.uri();
        tokio::task::spawn_blocking(move || {
            let client = vault_client_for(&uri);
            client
                .write_kv2(
                    "secret",
                    "myapp/config",
                    serde_json::json!({ "key": "updated" }),
                )
                .unwrap();
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn vault_read_403_missing_capability() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/secret/data/restricted"))
            .respond_with(
                ResponseTemplate::new(403).set_body_string(r#"{"errors":["permission denied"]}"#),
            )
            .mount(&server)
            .await;

        let uri = server.uri();
        tokio::task::spawn_blocking(move || {
            let client = vault_client_for(&uri);
            let err = client.read_kv2("secret", "restricted").unwrap_err();
            assert!(
                matches!(err, VaultError::MissingCapability { .. }),
                "expected MissingCapability, got: {}",
                err
            );
            assert!(err.to_string().contains("403"));
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn vault_read_404_path_not_found() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/secret/data/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_string(r#"{"errors":[]}"#))
            .mount(&server)
            .await;

        let uri = server.uri();
        tokio::task::spawn_blocking(move || {
            let client = vault_client_for(&uri);
            let err = client.read_kv2("secret", "missing").unwrap_err();
            assert!(
                matches!(err, VaultError::PathNotFound { .. }),
                "expected PathNotFound, got: {}",
                err
            );
            assert!(err.to_string().contains("404"));
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn vault_health_503_sealed() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/sys/health"))
            .respond_with(
                ResponseTemplate::new(503).set_body_string(r#"{"initialized":true,"sealed":true}"#),
            )
            .mount(&server)
            .await;

        let uri = server.uri();
        tokio::task::spawn_blocking(move || {
            let client = vault_client_for(&uri);
            let err = client.health().unwrap_err();
            assert!(
                matches!(err, VaultError::Sealed { .. }),
                "expected Sealed, got: {}",
                err
            );
            assert!(err.to_string().to_lowercase().contains("sealed"));
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn vault_health_200_ok() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/sys/health"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"initialized":true,"sealed":false})),
            )
            .mount(&server)
            .await;

        let uri = server.uri();
        tokio::task::spawn_blocking(move || {
            let client = vault_client_for(&uri);
            client.health().unwrap();
        })
        .await
        .unwrap();
    }
}
