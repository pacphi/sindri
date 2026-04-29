//! Git registry source (DDD-08, ADR-028 — Phase 3.1).
//!
//! `GitSource` resolves a registry index and per-component blobs from a
//! git repository: a URL plus a ref (branch / tag / sha) plus an optional
//! sub-directory inside the repo. The resolver pins the user-supplied ref
//! to a concrete commit sha at lock time and records the **sha** — never
//! the ref — in [`SourceDescriptor::Git`].
//!
//! ## Cache layout
//!
//! On disk under `~/.sindri/cache/git/<sha256(url)>/<commit-sha>/`. Two
//! resolutions of the same `(url, commit_sha)` reuse the same checkout.
//!
//! Cache eviction is best-effort and runs at the start of every
//! [`Source::fetch_index`] call — see [`super::git_cache`]. Two
//! thresholds (read from `~/.sindri/config.yaml#/cache/git`) drive the
//! policy: `max_size` (default `10GB`) and `max_age` (default `90d`).
//! Either firing triggers eviction; oldest-mtime-first within the
//! whole cache root.
//!
//! ## Sparse checkout
//!
//! When `subdir` is set, only that sub-directory is materialized on disk.
//! libgit2's checkout filter takes a path allow-list — we rely on that
//! rather than git2's higher-level sparse-checkout config so the behavior
//! is identical across platforms regardless of the user's `git` install.
//!
//! ## Signature verification (`require_signed`)
//!
//! Set on the source config: when `true`, commits must be GPG- or
//! SSH-signed and the signature must verify. We **shell out to
//! `git verify-commit`** for verification. Rationale: libgit2 returns the
//! raw signature bytes plus the signed payload but does not implement
//! GPG / SSH verification itself, and Sindri's existing crypto stack
//! (sigstore / cosign / p256) is OCI-shaped, not git-shaped — wiring it
//! to GPG keyrings would be a significant new surface. The plan's risk
//! register explicitly permitted the shell-out fallback ("`gpgme` or
//! shelled `git verify-commit` (TBD)"); this implementation chose the
//! shell-out path. When `require_signed: false` (the default) no
//! verification runs.

use crate::index::RegistryIndex;
use git2::build::CheckoutBuilder;
use git2::{FetchOptions, Repository};
use sha2::{Digest, Sha256};
use sindri_core::registry::{ComponentEntry, ComponentKind};
use sindri_core::version::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use super::{
    ComponentBlob, ComponentId, GitSource, Source, SourceContext, SourceDescriptor, SourceError,
};

/// Subdirectory inside the per-URL cache root that holds the materialized
/// checkout for a specific commit sha.
const CACHE_ROOT: &str = ".sindri/cache/git";

/// Live runtime wrapper around a [`GitSource`] config (DDD-08 §"`GitSource`").
///
/// Keeps a memoized resolved-sha so repeat `fetch_index` / `fetch_component_blob`
/// calls don't re-open the repository for every component.
pub struct GitSourceRuntime {
    /// User-supplied configuration.
    config: GitSource,
    /// Optional override for the cache root. `None` means
    /// `<home>/.sindri/cache/git`. Set explicitly by the test harness.
    cache_root_override: Option<PathBuf>,
    /// Resolved commit sha after the first successful checkout.
    resolved_sha: Mutex<Option<String>>,
}

impl std::fmt::Debug for GitSourceRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitSourceRuntime")
            .field("config", &self.config)
            .field("resolved_sha", &self.resolved_sha())
            .finish()
    }
}

impl Clone for GitSourceRuntime {
    fn clone(&self) -> Self {
        GitSourceRuntime {
            config: self.config.clone(),
            cache_root_override: self.cache_root_override.clone(),
            resolved_sha: Mutex::new(self.resolved_sha()),
        }
    }
}

impl GitSourceRuntime {
    /// Construct from a [`GitSource`] config. The cache root defaults to
    /// `<home>/.sindri/cache/git/`.
    pub fn new(config: GitSource) -> Self {
        Self {
            config,
            cache_root_override: None,
            resolved_sha: Mutex::new(None),
        }
    }

    /// Override the cache root (used by tests to keep data inside a tempdir).
    pub fn with_cache_root(mut self, root: PathBuf) -> Self {
        self.cache_root_override = Some(root);
        self
    }

    /// Borrow the typed config (URL, ref, subdir, scope, require_signed).
    pub fn config(&self) -> &GitSource {
        &self.config
    }

    /// Resolved commit sha if [`Self::ensure_checkout`] has run successfully.
    pub fn resolved_sha(&self) -> Option<String> {
        self.resolved_sha.lock().ok().and_then(|g| g.clone())
    }

    fn cache_root(&self) -> PathBuf {
        if let Some(p) = &self.cache_root_override {
            p.clone()
        } else {
            sindri_core::paths::home_dir()
                .unwrap_or_default()
                .join(CACHE_ROOT)
        }
    }

    /// `<cache_root>/<sha256(url)>/`.
    fn url_cache_dir(&self) -> PathBuf {
        let url_hash = hex::encode(Sha256::digest(self.config.url.as_bytes()));
        self.cache_root().join(url_hash)
    }

    /// `<cache_root>/<sha256(url)>/<commit_sha>/`.
    fn checkout_dir(&self, sha: &str) -> PathBuf {
        self.url_cache_dir().join(sha)
    }

    /// Clone (or reuse) the bare mirror under `<url_cache>/_bare/`. The bare
    /// repo is re-used across resolutions; checkouts are materialized into
    /// per-sha subdirectories.
    fn ensure_bare(&self) -> Result<PathBuf, SourceError> {
        let bare = self.url_cache_dir().join("_bare");
        if bare.join("HEAD").exists() {
            return Ok(bare);
        }
        if let Some(parent) = bare.parent() {
            fs::create_dir_all(parent).map_err(SourceError::from)?;
        }
        let mut fopts = FetchOptions::new();
        fopts.depth(0);
        let mut builder = git2::build::RepoBuilder::new();
        builder.bare(true).fetch_options(fopts);
        builder
            .clone(&self.config.url, &bare)
            .map_err(|e| SourceError::Io(format!("git clone {}: {}", self.config.url, e)))?;
        Ok(bare)
    }

    /// Resolve `git_ref` against the bare mirror and return the
    /// fully-qualified commit sha as a hex string.
    fn resolve_ref(&self, bare: &Path) -> Result<String, SourceError> {
        let repo = Repository::open_bare(bare)
            .map_err(|e| SourceError::Io(format!("open bare repo: {}", e)))?;
        // Try direct sha lookup first; then refs/heads/<ref>; then refs/tags/<ref>;
        // then `revparse_single` as a last resort.
        if let Ok(oid) = git2::Oid::from_str(&self.config.git_ref) {
            if repo.find_commit(oid).is_ok() {
                return Ok(oid.to_string());
            }
        }
        if let Ok(rev) = repo.revparse_single(&self.config.git_ref) {
            return Ok(rev.id().to_string());
        }
        Err(SourceError::NotFound(format!(
            "git ref '{}' not in {}",
            self.config.git_ref, self.config.url
        )))
    }

    /// Checkout the resolved `sha` into `<url_cache>/<sha>/`, optionally
    /// limited to `subdir`. Idempotent: if the target directory already
    /// holds a checkout for the same sha, reuse it.
    fn ensure_checkout(&self) -> Result<(String, PathBuf), SourceError> {
        let bare = self.ensure_bare()?;
        let sha = self.resolve_ref(&bare)?;

        let dest = self.checkout_dir(&sha);
        let marker = dest.join(".sindri-checkout-complete");

        if marker.exists() {
            // Already materialized.
            if let Ok(mut g) = self.resolved_sha.lock() {
                *g = Some(sha.clone());
            }
            return Ok((sha, dest));
        }

        // Open the bare repo for checkout.
        let repo = Repository::open_bare(&bare)
            .map_err(|e| SourceError::Io(format!("open bare for checkout: {}", e)))?;
        let oid = git2::Oid::from_str(&sha)
            .map_err(|e| SourceError::InvalidData(format!("bad sha '{}': {}", sha, e)))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| SourceError::NotFound(format!("commit {}: {}", sha, e)))?;

        // Optional GPG/SSH signature verification.
        if self.config.require_signed {
            self.verify_commit_signature(&bare, &sha)?;
        }

        let tree = commit
            .tree()
            .map_err(|e| SourceError::Io(format!("commit tree {}: {}", sha, e)))?;

        // Set the bare repo's working tree to `dest` for this checkout, then
        // restore. `set_workdir` is the libgit2-blessed way; alternatively
        // we could open a fresh non-bare repo at `dest` — that path uses
        // more disk and isn't materially safer.
        fs::create_dir_all(&dest).map_err(SourceError::from)?;

        let mut co = CheckoutBuilder::new();
        co.force().target_dir(&dest);
        if let Some(subdir) = &self.config.subdir {
            // Sparse-style checkout: only materialize entries that begin
            // with `<subdir>/`. libgit2's `path` filter accepts UTF-8
            // path globs; the directory check below ensures the requested
            // subdir actually exists in the tree.
            if tree.get_path(subdir).is_err() {
                return Err(SourceError::NotFound(format!(
                    "subdir '{}' not in commit {}",
                    subdir.display(),
                    sha
                )));
            }
            let pattern = format!("{}/", subdir.display());
            co.path(pattern);
        }

        repo.checkout_tree(tree.as_object(), Some(&mut co))
            .map_err(|e| SourceError::Io(format!("checkout {}: {}", sha, e)))?;

        // Marker so we can short-circuit on cache hits.
        fs::write(&marker, &sha).map_err(SourceError::from)?;

        if let Ok(mut g) = self.resolved_sha.lock() {
            *g = Some(sha.clone());
        }
        Ok((sha, dest))
    }

    /// Shell out to `git verify-commit <sha>` against the bare mirror. This
    /// uses the user's `git` installation (and their GPG / SSH agent / key
    /// store). When the git binary is missing or the signature does not
    /// verify we surface [`SourceError::SignatureFailed`].
    fn verify_commit_signature(&self, bare: &Path, sha: &str) -> Result<(), SourceError> {
        let out = Command::new("git")
            .arg("--git-dir")
            .arg(bare)
            .arg("verify-commit")
            .arg(sha)
            .output()
            .map_err(|e| {
                SourceError::SignatureFailed(format!(
                    "`git verify-commit` failed to spawn ({}). \
                     `require_signed: true` needs a working git in PATH.",
                    e
                ))
            })?;
        if out.status.success() {
            tracing::info!(
                "git: verified signature on commit {} of {}",
                sha,
                self.config.url
            );
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(SourceError::SignatureFailed(format!(
                "commit {} in {} is not signed (or signature does not verify): {}",
                sha, self.config.url, stderr
            )))
        }
    }

    /// Walk `<checkout>/[subdir/]` for component.yaml files and assemble a
    /// [`RegistryIndex`].
    ///
    /// Layout convention mirrors `LocalPathSource`: `index.yaml` is read at
    /// the (subdir-relative) root if present; otherwise we synthesize an
    /// index by walking `components/<name>/component.yaml` and
    /// `collections/<name>/component.yaml`.
    fn build_index(&self, root: &Path) -> Result<RegistryIndex, SourceError> {
        let walk_root = match &self.config.subdir {
            Some(s) => root.join(s),
            None => root.to_path_buf(),
        };

        // Prefer an explicit index.yaml when one is present.
        let explicit_index = walk_root.join("index.yaml");
        if explicit_index.exists() {
            let yaml = fs::read_to_string(&explicit_index).map_err(SourceError::from)?;
            return RegistryIndex::from_yaml(&yaml)
                .map_err(|e| SourceError::InvalidData(e.to_string()));
        }

        // Synthesize one by walking the conventional sub-folders.
        let mut entries: Vec<ComponentEntry> = Vec::new();
        for (subdir, kind, default_backend) in [
            ("components", ComponentKind::Component, "mise"),
            ("collections", ComponentKind::Collection, "collection"),
        ] {
            let dir = walk_root.join(subdir);
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(&dir).map_err(SourceError::from)? {
                let entry = entry.map_err(SourceError::from)?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let yaml_path = path.join("component.yaml");
                if !yaml_path.exists() {
                    continue;
                }
                let yaml = fs::read_to_string(&yaml_path).map_err(SourceError::from)?;
                let manifest: sindri_core::component::ComponentManifest =
                    serde_yaml::from_str(&yaml)
                        .map_err(|e| SourceError::InvalidData(e.to_string()))?;
                let name = manifest.metadata.name.clone();
                entries.push(ComponentEntry {
                    name: name.clone(),
                    backend: default_backend.into(),
                    latest: manifest.metadata.version.clone(),
                    versions: vec![manifest.metadata.version.clone()],
                    description: manifest.metadata.description.clone(),
                    kind: kind.clone(),
                    oci_ref: format!("git://{}/{}", self.config.url, name),
                    license: manifest.metadata.license.clone(),
                    depends_on: vec![],
                });
            }
        }

        Ok(RegistryIndex {
            version: 1,
            registry: format!("git:{}", self.config.url),
            generated_at: None,
            components: entries,
        })
    }
}

impl Source for GitSourceRuntime {
    fn fetch_index(&self, _ctx: &SourceContext) -> Result<RegistryIndex, SourceError> {
        // Phase 4.5: best-effort cache eviction. Runs on every call but
        // costs only one stat per cached commit — bounded by the number
        // of cached commits which is small in practice. Failure is
        // logged and ignored; eviction must never block resolution.
        let cache_root = self.cache_root();
        let cache_cfg = sindri_core::cache_config::load_user_cache_config().git;
        if let Err(e) = super::git_cache::run_eviction(&cache_root, &cache_cfg) {
            tracing::debug!(
                "git-cache eviction failed at {}: {}",
                cache_root.display(),
                e
            );
        }

        let (_sha, dest) = self.ensure_checkout()?;
        let mut index = self.build_index(&dest)?;
        if let Some(scope) = &self.config.scope {
            let allow: std::collections::HashSet<&str> = scope.iter().map(|n| n.as_str()).collect();
            index.components.retain(|c| allow.contains(c.name.as_str()));
        }
        Ok(index)
    }

    fn fetch_component_blob(
        &self,
        id: &ComponentId,
        _version: &Version,
        _ctx: &SourceContext,
    ) -> Result<ComponentBlob, SourceError> {
        if !self
            .config
            .scope
            .as_ref()
            .map(|s| s.iter().any(|n| n == &id.name))
            .unwrap_or(true)
        {
            return Err(SourceError::NotFound(id.name.as_str().to_string()));
        }

        let (_sha, dest) = self.ensure_checkout()?;
        let walk_root = match &self.config.subdir {
            Some(s) => dest.join(s),
            None => dest.clone(),
        };
        let folder = if id.backend == "collection" {
            "collections"
        } else {
            "components"
        };
        let yaml_path = walk_root
            .join(folder)
            .join(id.name.as_str())
            .join("component.yaml");
        let bytes = fs::read(&yaml_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => SourceError::NotFound(id.name.as_str().to_string()),
            _ => SourceError::Io(format!("{}: {}", yaml_path.display(), e)),
        })?;
        Ok(ComponentBlob {
            bytes,
            digest: None,
        })
    }

    /// Return the descriptor that the resolver writes into the lockfile.
    ///
    /// The descriptor records `(url, commit_sha, subdir)` only — never an
    /// absolute cache path. The on-disk cache lives at
    /// `~/.sindri/cache/git/<sha256(url)>/<commit_sha>/`, deterministically
    /// derived from the descriptor on every host. Recording an absolute path
    /// would break portability the moment two operators share a lockfile;
    /// derivation is identical at resolve-time and apply-time, so cache hits
    /// behave correctly without explicit path tracking.
    ///
    /// The `commit_sha` is the *resolved* sha when [`Self::ensure_checkout`]
    /// has already run, or the raw user-supplied ref as a placeholder when
    /// the descriptor is requested before a fetch — matching the best-effort
    /// behaviour of the static-config dispatch helper.
    fn lockfile_descriptor(&self) -> SourceDescriptor {
        let commit_sha = self
            .resolved_sha()
            .unwrap_or_else(|| self.config.git_ref.clone());
        SourceDescriptor::Git {
            url: self.config.url.clone(),
            commit_sha,
            subdir: self.config.subdir.clone(),
        }
    }

    fn supports_strict_oci(&self) -> bool {
        // GitSource is reference-resolved, not OCI content-addressed: even
        // with `require_signed: true` the bytes-on-disk are not OCI
        // descriptors and the `--strict-oci` admission gate forbids it
        // (DDD-08 §Invariant 4 + ADR-028 §"Strict-OCI semantics").
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::ComponentName;
    use git2::{IndexAddOption, Signature};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper: build a fixture git repository with three components under
    /// `components/<name>/component.yaml` and an `index.yaml` at the root.
    /// Returns the on-disk path of the bare-equivalent repo (a non-bare
    /// repo we can still clone from over the `file://` protocol).
    fn make_fixture_repo(dir: &Path, branch: &str) -> Repository {
        let repo = Repository::init(dir).unwrap();

        // Author signature.
        let sig = Signature::now("Test", "test@example.com").unwrap();

        // Components.
        for (name, version, license) in [
            ("nodejs", "20.10.0", "MIT"),
            ("rust", "1.75.0", "Apache-2.0"),
            ("ripgrep", "14.1.0", "MIT"),
        ] {
            let comp_dir = dir.join("components").join(name);
            fs::create_dir_all(&comp_dir).unwrap();
            let yaml = format!(
                "metadata:\n  name: {name}\n  version: \"{version}\"\n  description: test\n  license: {license}\n  tags: []\nplatforms: []\ninstall: {{}}\ndepends_on: []\n",
                name = name,
                version = version,
                license = license,
            );
            fs::write(comp_dir.join("component.yaml"), yaml).unwrap();
        }

        // index.yaml at root.
        let mut idx_yaml = String::from("version: 1\nregistry: fixture\ncomponents:\n");
        for (name, version, license) in [
            ("nodejs", "20.10.0", "MIT"),
            ("rust", "1.75.0", "Apache-2.0"),
            ("ripgrep", "14.1.0", "MIT"),
        ] {
            idx_yaml.push_str(&format!(
                "  - name: {name}\n    backend: mise\n    latest: \"{version}\"\n    versions: [\"{version}\"]\n    description: test\n    kind: component\n    oci_ref: \"git://fixture/{name}\"\n    license: {license}\n    depends_on: []\n",
            ));
        }
        fs::write(dir.join("index.yaml"), idx_yaml).unwrap();

        // Stage everything and commit on `branch`.
        let mut index = repo.index().unwrap();
        index
            .add_all(["."].iter(), IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "fixture commit", &tree, &[])
            .unwrap();
        drop(tree);

        // Create / update the requested branch ref to point at the commit.
        if branch != "main" && branch != "master" {
            let commit = repo.find_commit(oid).unwrap();
            repo.branch(branch, &commit, true).unwrap();
            drop(commit);
        }

        repo
    }

    fn fixture_url(repo_dir: &Path) -> String {
        // RFC 8089 file URL. On POSIX the path is `/tmp/...` (already absolute,
        // begins with `/`), giving `file:///tmp/...`. On Windows the path is
        // `C:\Users\...`; libgit2 will reject `file://C:\...` because the colon
        // is read as a port separator and backslashes are not URL-safe — so we
        // forward-slash the path and add the extra `/` before the drive letter,
        // yielding `file:///C:/Users/...`.
        let path = repo_dir.display().to_string().replace('\\', "/");
        if cfg!(windows) {
            format!("file:///{}", path)
        } else {
            format!("file://{}", path)
        }
    }

    #[test]
    fn fetch_index_resolves_branch_to_commit_sha() {
        let tmp = TempDir::new().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).unwrap();
        make_fixture_repo(&repo_dir, "main");

        // Get current HEAD sha for assertion.
        let repo = Repository::open(&repo_dir).unwrap();
        let head = repo.head().unwrap().target().unwrap().to_string();

        let cache = TempDir::new().unwrap();
        let src = GitSourceRuntime::new(GitSource {
            url: fixture_url(&repo_dir),
            git_ref: "HEAD".into(),
            subdir: None,
            scope: None,
            require_signed: false,
        })
        .with_cache_root(cache.path().to_path_buf());

        let idx = src.fetch_index(&SourceContext::default()).unwrap();
        assert_eq!(idx.components.len(), 3);
        assert_eq!(src.resolved_sha().unwrap(), head);

        // Lockfile descriptor records the *sha*, not the ref.
        match src.lockfile_descriptor() {
            SourceDescriptor::Git {
                url,
                commit_sha,
                subdir,
            } => {
                assert!(url.starts_with("file://"));
                assert_eq!(commit_sha, head);
                assert!(subdir.is_none());
            }
            other => panic!("expected Git descriptor, got {:?}", other),
        }
    }

    #[test]
    fn re_resolution_reuses_cache() {
        let tmp = TempDir::new().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).unwrap();
        make_fixture_repo(&repo_dir, "main");

        let cache = TempDir::new().unwrap();
        let src = GitSourceRuntime::new(GitSource {
            url: fixture_url(&repo_dir),
            git_ref: "HEAD".into(),
            subdir: None,
            scope: None,
            require_signed: false,
        })
        .with_cache_root(cache.path().to_path_buf());

        let _ = src.fetch_index(&SourceContext::default()).unwrap();
        let sha = src.resolved_sha().unwrap();
        // Touch the marker timestamp to make sure the second call is a cache hit.
        let marker = cache
            .path()
            .join(hex::encode(Sha256::digest(src.config.url.as_bytes())))
            .join(&sha)
            .join(".sindri-checkout-complete");
        assert!(marker.exists(), "checkout marker must be present");
        let _ = src.fetch_index(&SourceContext::default()).unwrap();
    }

    #[test]
    fn subdir_limits_index_walk() {
        let tmp = TempDir::new().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).unwrap();
        let repo = Repository::init(&repo_dir).unwrap();
        let sig = Signature::now("Test", "t@e.com").unwrap();

        // Two parallel registries inside the repo: `main-reg/` and `extra/`.
        for (folder, names) in [
            ("main-reg", vec!["alpha", "beta"]),
            ("extra", vec!["gamma"]),
        ] {
            let mut idx_yaml = String::from("version: 1\nregistry: fixture\ncomponents:\n");
            for name in &names {
                let comp_dir = repo_dir.join(folder).join("components").join(name);
                fs::create_dir_all(&comp_dir).unwrap();
                fs::write(
                    comp_dir.join("component.yaml"),
                    format!(
                        "metadata:\n  name: {name}\n  version: \"1.0.0\"\n  description: test\n  license: MIT\n  tags: []\nplatforms: []\ninstall: {{}}\ndepends_on: []\n",
                    ),
                )
                .unwrap();
                idx_yaml.push_str(&format!(
                    "  - name: {name}\n    backend: mise\n    latest: \"1.0.0\"\n    versions: [\"1.0.0\"]\n    description: test\n    kind: component\n    oci_ref: \"git://fixture/{name}\"\n    license: MIT\n    depends_on: []\n",
                ));
            }
            fs::write(repo_dir.join(folder).join("index.yaml"), idx_yaml).unwrap();
        }

        let mut idx = repo.index().unwrap();
        idx.add_all(["."].iter(), IndexAddOption::DEFAULT, None)
            .unwrap();
        idx.write().unwrap();
        let tree_id = idx.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let cache = TempDir::new().unwrap();
        let src = GitSourceRuntime::new(GitSource {
            url: fixture_url(&repo_dir),
            git_ref: "HEAD".into(),
            subdir: Some(PathBuf::from("main-reg")),
            scope: None,
            require_signed: false,
        })
        .with_cache_root(cache.path().to_path_buf());

        let index = src.fetch_index(&SourceContext::default()).unwrap();
        let names: Vec<&str> = index.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
        assert!(!names.contains(&"gamma"), "extra/ should be filtered out");
    }

    #[test]
    fn scope_filters_components() {
        let tmp = TempDir::new().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).unwrap();
        make_fixture_repo(&repo_dir, "main");

        let cache = TempDir::new().unwrap();
        let src = GitSourceRuntime::new(GitSource {
            url: fixture_url(&repo_dir),
            git_ref: "HEAD".into(),
            subdir: None,
            scope: Some(vec![ComponentName::from("nodejs")]),
            require_signed: false,
        })
        .with_cache_root(cache.path().to_path_buf());

        let idx = src.fetch_index(&SourceContext::default()).unwrap();
        assert_eq!(idx.components.len(), 1);
        assert_eq!(idx.components[0].name, "nodejs");
    }

    #[test]
    fn require_signed_rejects_unsigned_commit() {
        let tmp = TempDir::new().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).unwrap();
        make_fixture_repo(&repo_dir, "main");

        let cache = TempDir::new().unwrap();
        let src = GitSourceRuntime::new(GitSource {
            url: fixture_url(&repo_dir),
            git_ref: "HEAD".into(),
            subdir: None,
            scope: None,
            require_signed: true,
        })
        .with_cache_root(cache.path().to_path_buf());

        let err = src.fetch_index(&SourceContext::default()).unwrap_err();
        match err {
            SourceError::SignatureFailed(_) => {}
            other => panic!("expected SignatureFailed, got {:?}", other),
        }
    }

    #[test]
    fn does_not_support_strict_oci() {
        let src = GitSourceRuntime::new(GitSource {
            url: "https://x".into(),
            git_ref: "main".into(),
            subdir: None,
            scope: None,
            require_signed: false,
        });
        assert!(!src.supports_strict_oci());
    }

    #[test]
    fn fetch_component_blob_returns_yaml_bytes() {
        let tmp = TempDir::new().unwrap();
        let repo_dir = tmp.path().join("repo");
        fs::create_dir_all(&repo_dir).unwrap();
        make_fixture_repo(&repo_dir, "main");

        let cache = TempDir::new().unwrap();
        let src = GitSourceRuntime::new(GitSource {
            url: fixture_url(&repo_dir),
            git_ref: "HEAD".into(),
            subdir: None,
            scope: None,
            require_signed: false,
        })
        .with_cache_root(cache.path().to_path_buf());

        let id = ComponentId {
            backend: "mise".into(),
            name: ComponentName::from("nodejs"),
        };
        let blob = src
            .fetch_component_blob(&id, &Version::new("20.10.0"), &SourceContext::default())
            .unwrap();
        let text = std::str::from_utf8(&blob.bytes).unwrap();
        assert!(text.contains("name: nodejs"));
        assert!(blob.digest.is_none());
    }
}
