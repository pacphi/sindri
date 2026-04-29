//! `sindri registry serve` — embedded OCI registry over a components dir
//! (Phase 3.2, ADR-028).
//!
//! Serves an OCI image layout (or a directory of OCI layouts, one per
//! repository) over HTTP using the OCI Distribution Spec v1.1 read endpoints.
//! This is intentionally a **developer convenience**, not a production
//! registry — the server is single-process, stores all blobs on disk, and
//! does not implement push, garbage collection, or content negotiation
//! beyond what the read path needs.
//!
//! ## Layout convention
//!
//! `--root <path>` may point at:
//!
//! - A single OCI image layout directory (`<root>/{oci-layout,index.json,blobs/sha256/}`).
//!   The whole layout is served under the repository name `sindri/registry-core`.
//!
//! - A directory of named subdirectories, each itself an OCI image layout.
//!   The subdirectory name becomes the repository name. Useful for
//!   serving multiple per-component artifacts side by side.
//!
//! ## Flags
//!
//! - `--root <path>` — required; see layout convention above.
//! - `--addr <host:port>` — optional; default `127.0.0.1:5000`.
//!
//! Note: `--sign-with` was removed in Phase 3 follow-up. The server is
//! read-only and does not re-sign manifests; it serves the pre-signed bytes
//! written by `sindri registry prefetch`. Re-signing support is deferred to
//! Phase 5.
//!
//! ## Endpoints
//!
//! Implements the read-only subset of the OCI Distribution Spec needed by
//! `oci-distribution::Client::pull_manifest` / `pull_blob`:
//!
//! - `GET /v2/`  — version probe; returns `{}` with `Docker-Distribution-API-Version: registry/2.0`.
//! - `GET /v2/<repo>/manifests/<reference>` — manifest by tag or digest.
//! - `HEAD /v2/<repo>/manifests/<reference>` — same but headers only.
//! - `GET /v2/<repo>/blobs/<digest>` — blob by digest.
//! - `HEAD /v2/<repo>/blobs/<digest>` — blob existence check.
//!
//! Tags are resolved by walking the served layout's `index.json` and
//! matching `org.opencontainers.image.ref.name` annotations.

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Default repository name when `--root` points at a single OCI layout.
const DEFAULT_REPO_NAME: &str = "sindri/registry-core";

/// Per-server shared state — just the on-disk root + a reusable file-system
/// reader. The struct is `Send + Sync` so axum can clone it for every
/// request handler.
#[derive(Clone)]
struct ServerState {
    root: PathBuf,
    /// `true` when `root` is itself an OCI layout (single-repo mode).
    /// `false` when `root` contains per-repo subdirectories.
    single_repo: bool,
}

impl ServerState {
    fn layout_dir_for(&self, repo: &str) -> PathBuf {
        if self.single_repo {
            self.root.clone()
        } else {
            // Repo names are conventionally `org/name`. Map slashes to
            // nested directories under `--root`.
            let mut p = self.root.clone();
            for segment in repo.split('/') {
                p.push(segment);
            }
            p
        }
    }

    fn current_repo_name(&self) -> Option<String> {
        if self.single_repo {
            Some(DEFAULT_REPO_NAME.to_string())
        } else {
            None
        }
    }
}

/// Run the embedded registry. Blocks until SIGINT (Ctrl-C) is received,
/// then shuts down cleanly.
///
/// `--sign-with` was stripped in Phase 3 follow-up because the flag
/// parsed but never performed any signing — shipping a no-op flag is
/// misleading. Re-signing support is tracked in Phase 5 (`registry serve
/// --sign-with <key>`); for now the server serves pre-signed bytes verbatim.
pub fn run(addr: &str, root: &str) -> i32 {
    let root = PathBuf::from(root);
    if !root.exists() {
        eprintln!("registry serve: --root {} does not exist", root.display());
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    let single_repo = root.join("oci-layout").exists() || root.join("index.json").exists();

    let state = Arc::new(ServerState { root, single_repo });

    let socket: SocketAddr = match addr.parse() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("registry serve: bad --addr '{}': {}", addr, e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("registry serve: failed to start tokio runtime: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let result = runtime.block_on(async move { serve_until_signal(socket, state).await });
    match result {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            eprintln!("registry serve: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

/// Identical to [`run`] except it returns the bound socket address through
/// a oneshot. Used by tests to start the server on an ephemeral port and
/// then send requests through `reqwest`.
pub async fn run_for_test(
    addr: &str,
    root: &Path,
    ready_tx: tokio::sync::oneshot::Sender<SocketAddr>,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> std::io::Result<()> {
    let single_repo = root.join("oci-layout").exists() || root.join("index.json").exists();
    let state = Arc::new(ServerState {
        root: root.to_path_buf(),
        single_repo,
    });
    let socket: SocketAddr = addr.parse().map_err(std::io::Error::other)?;

    let listener = tokio::net::TcpListener::bind(socket).await?;
    let bound = listener.local_addr()?;
    let _ = ready_tx.send(bound);

    let app = router(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .await?;
    Ok(())
}

async fn serve_until_signal(addr: SocketAddr, state: Arc<ServerState>) -> std::io::Result<()> {
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    println!("sindri registry serve listening on http://{}", bound);
    println!("(press Ctrl-C to stop)");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            println!("\nsindri registry serve: shutting down");
        })
        .await
}

fn router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/v2/", get(api_root))
        .route("/v2", get(api_root))
        .route(
            "/v2/{*rest}",
            any(distribution_dispatch).with_state(state.clone()),
        )
        .with_state(state)
}

async fn api_root() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Docker-Distribution-API-Version",
        HeaderValue::from_static("registry/2.0"),
    );
    (StatusCode::OK, headers, "{}")
}

/// Single dispatcher for everything under `/v2/<rest>`. We can't use axum's
/// path matchers directly because OCI repo names contain slashes
/// (`sindri/registry-core/manifests/...`) and axum's path-segment matcher
/// is single-segment by default.
async fn distribution_dispatch(
    State(state): State<Arc<ServerState>>,
    AxumPath(rest): AxumPath<String>,
    method: axum::http::Method,
) -> Response {
    // Split `rest` into `(repo, kind, reference)`. `kind` is `manifests` or `blobs`.
    let parts: Vec<&str> = rest.splitn(2, "/manifests/").collect();
    let (repo, kind, reference) = if parts.len() == 2 {
        (parts[0].to_string(), "manifests", parts[1].to_string())
    } else {
        let bparts: Vec<&str> = rest.splitn(2, "/blobs/").collect();
        if bparts.len() == 2 {
            (bparts[0].to_string(), "blobs", bparts[1].to_string())
        } else {
            return (StatusCode::NOT_FOUND, "unknown endpoint").into_response();
        }
    };

    if state.single_repo {
        // Single-repo mode rewrites every incoming repo to the synthetic
        // repo name. Any caller path is accepted as long as the
        // tag/digest exists in the served layout.
        let _ = repo;
    }

    println!("[serve] {} /v2/{}", method, rest);

    match kind {
        "manifests" => serve_manifest(&state, &repo, &reference, method).await,
        "blobs" => serve_blob(&state, &repo, &reference, method).await,
        _ => (StatusCode::NOT_FOUND, "unknown endpoint").into_response(),
    }
}

async fn serve_manifest(
    state: &ServerState,
    repo: &str,
    reference: &str,
    method: axum::http::Method,
) -> Response {
    let layout = state.layout_dir_for(repo);
    let result = if reference.starts_with("sha256:") {
        read_blob_with_media_type(&layout, reference)
    } else {
        // Treat `reference` as a tag — walk index.json for a matching
        // `org.opencontainers.image.ref.name` annotation.
        match resolve_tag_to_digest(&layout, reference) {
            Ok(Some(digest)) => read_blob_with_media_type(&layout, &digest),
            Ok(None) => return (StatusCode::NOT_FOUND, "tag not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        }
    };

    match result {
        Ok((bytes, digest, media_type)) => manifest_response(bytes, digest, media_type, method),
        Err(e) if e.contains("not found") => (StatusCode::NOT_FOUND, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn serve_blob(
    state: &ServerState,
    repo: &str,
    digest: &str,
    method: axum::http::Method,
) -> Response {
    let layout = state.layout_dir_for(repo);
    match read_blob_with_media_type(&layout, digest) {
        Ok((bytes, computed_digest, _media_type)) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            headers.insert(
                "Docker-Content-Digest",
                HeaderValue::from_str(&computed_digest)
                    .unwrap_or_else(|_| HeaderValue::from_static("")),
            );
            headers.insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&bytes.len().to_string())
                    .unwrap_or_else(|_| HeaderValue::from_static("0")),
            );
            if method == axum::http::Method::HEAD {
                (StatusCode::OK, headers, Body::empty()).into_response()
            } else {
                (StatusCode::OK, headers, bytes).into_response()
            }
        }
        Err(e) if e.contains("not found") => (StatusCode::NOT_FOUND, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

fn manifest_response(
    bytes: Vec<u8>,
    digest: String,
    media_type: String,
    method: axum::http::Method,
) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&media_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        "Docker-Content-Digest",
        HeaderValue::from_str(&digest).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&bytes.len().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    if method == axum::http::Method::HEAD {
        (StatusCode::OK, headers, Body::empty()).into_response()
    } else {
        (StatusCode::OK, headers, bytes).into_response()
    }
}

fn read_blob_with_media_type(
    layout: &Path,
    digest: &str,
) -> Result<(Vec<u8>, String, String), String> {
    let path = blob_path(layout, digest);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("blob {} not found", digest));
        }
        Err(e) => return Err(format!("read {}: {}", path.display(), e)),
    };
    // Pick a sensible media type. If the bytes parse as JSON with
    // `mediaType`, surface that — that's how OCI manifests / image-indexes
    // self-describe; otherwise fall back to a generic layer media type.
    let media_type = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|v| {
            v.get("mediaType")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "application/octet-stream".to_string());
    Ok((bytes, digest.to_string(), media_type))
}

fn blob_path(layout: &Path, digest: &str) -> PathBuf {
    let (alg, hex) = match digest.split_once(':') {
        Some(p) => p,
        None => return layout.join("blobs").join("sha256").join(digest),
    };
    layout.join("blobs").join(alg).join(hex)
}

fn resolve_tag_to_digest(layout: &Path, tag: &str) -> Result<Option<String>, String> {
    let raw = match std::fs::read(layout.join("index.json")) {
        Ok(r) => r,
        Err(e) => return Err(format!("read index.json: {}", e)),
    };
    let v: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|e| format!("index.json parse: {}", e))?;
    let manifests = v
        .get("manifests")
        .and_then(|m| m.as_array())
        .ok_or("index.json missing 'manifests'")?;
    for desc in manifests {
        let annotations = desc.get("annotations").and_then(|a| a.as_object());
        let ref_name = annotations
            .and_then(|a| a.get("org.opencontainers.image.ref.name"))
            .and_then(|s| s.as_str());
        if ref_name == Some(tag) {
            if let Some(d) = desc.get("digest").and_then(|d| d.as_str()) {
                return Ok(Some(d.to_string()));
            }
        }
    }
    // Fallback: if there's only one manifest and the caller asked for any
    // tag, return that. This keeps the "single layout, single artifact"
    // dev path painless.
    if manifests.len() == 1 {
        if let Some(d) = manifests[0].get("digest").and_then(|d| d.as_str()) {
            return Ok(Some(d.to_string()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_registry::source::local_oci::build_test_fixture;
    use tempfile::TempDir;

    /// Smoke test: start the server on an ephemeral port against a
    /// `build_test_fixture`-built OCI layout, fetch /v2/, and confirm the
    /// API-Version header is set correctly.
    #[tokio::test]
    async fn serves_v2_root() {
        let tmp = TempDir::new().unwrap();
        let _layout = build_test_fixture(tmp.path(), false).unwrap();

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let layout_path = tmp.path().to_path_buf();
        let server = tokio::spawn(async move {
            run_for_test("127.0.0.1:0", &layout_path, ready_tx, shutdown_rx).await
        });
        let addr = ready_rx.await.unwrap();

        let client = reqwest::Client::new();
        let url = format!("http://{}/v2/", addr);
        let resp = client.get(&url).send().await.unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("Docker-Distribution-API-Version")
                .unwrap(),
            "registry/2.0"
        );

        let _ = shutdown_tx.send(());
        let _ = server.await;
    }

    /// Pull a known manifest by digest and verify the `Docker-Content-Digest`
    /// matches what the fixture wrote.
    #[tokio::test]
    async fn serves_manifest_by_digest() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let layout_path = tmp.path().to_path_buf();
        let server = tokio::spawn(async move {
            run_for_test("127.0.0.1:0", &layout_path, ready_tx, shutdown_rx).await
        });
        let addr = ready_rx.await.unwrap();

        let client = reqwest::Client::new();
        let url = format!(
            "http://{}/v2/sindri/registry-core/manifests/{}",
            addr, layout.manifest_digest
        );
        let resp = client.get(&url).send().await.unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        assert_eq!(
            resp.headers().get("Docker-Content-Digest").unwrap(),
            layout.manifest_digest.as_str()
        );

        let _ = shutdown_tx.send(());
        let _ = server.await;
    }

    /// Pull a layer blob by digest and assert the bytes match.
    #[tokio::test]
    async fn serves_blob_by_digest() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();
        let nodejs = layout
            .components
            .iter()
            .find(|c| c.name == "nodejs")
            .unwrap();

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let layout_path = tmp.path().to_path_buf();
        let server = tokio::spawn(async move {
            run_for_test("127.0.0.1:0", &layout_path, ready_tx, shutdown_rx).await
        });
        let addr = ready_rx.await.unwrap();

        let client = reqwest::Client::new();
        let url = format!(
            "http://{}/v2/sindri/registry-core/blobs/{}",
            addr, nodejs.layer_digest
        );
        let resp = client.get(&url).send().await.unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let bytes = resp.bytes().await.unwrap().to_vec();
        assert_eq!(bytes, nodejs.layer_bytes);

        let _ = shutdown_tx.send(());
        let _ = server.await;
    }
}
