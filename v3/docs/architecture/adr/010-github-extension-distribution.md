# ADR 010: GitHub-based Extension Distribution

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-009: Dependency Resolution](009-dependency-resolution-dag-topological-sort.md), [Extension Guide](../../EXTENSIONS.md)

## Context

The Sindri extension system requires a distribution mechanism that supports:

1. **Version Management**: Extensions have semantic versions (e.g., `nodejs@1.2.0`)
2. **Immutability**: Once published, a version should not change
3. **Discoverability**: Users can browse available extensions
4. **Update Notifications**: CLI can check for newer versions
5. **Offline Support**: Extensions cached locally for offline installation
6. **Rollback**: Users can downgrade to previous versions
7. **Compatibility**: Extensions declare compatible CLI versions
8. **Low Maintenance**: Minimal infrastructure required

The bash implementation used a single monorepo with manual version tagging, but lacked:

- Formal version compatibility tracking
- Automatic update checks
- Local caching strategy
- Rollback support

Distribution patterns considered:

- **Package registries**: npm, crates.io, PyPI (requires account, publishing process)
- **Container registries**: Docker Hub, GHCR (heavy for simple YAML files)
- **Git repositories**: GitHub releases, tags (lightweight, version-native)
- **Object storage**: S3, CloudFlare R2 (requires infrastructure)

## Decision

### Monorepo with Tagged Releases

We adopt a **GitHub-based monorepo distribution model** with extension-specific version tags:

**Repository Structure**:

```
github.com/pacphi/sindri/
├── v3/extensions/
│   ├── nodejs/
│   │   ├── extension.yaml           # Current version
│   │   └── README.md
│   ├── python/
│   │   ├── extension.yaml
│   │   └── README.md
│   └── claude-flow-v2/
│       ├── extension.yaml
│       ├── scripts/
│       └── README.md
└── v3/registry.yaml          # Master registry
```

**Version Tagging Convention**:

```
git tag nodejs@1.2.0        # Tag specific extension version
git tag python@3.1.0
git tag claude-flow-v2@1.5.0
git tag v2.2.1              # CLI version (existing pattern)
```

**Registry File** (`v3/registry.yaml`):

```yaml
version: 1.0.0
extensions:
  nodejs:
    name: nodejs
    latest: 1.2.0
    description: Node.js runtime with npm/pnpm/yarn
    category: runtimes
    versions:
      - version: 1.2.0
        tag: nodejs@1.2.0
        min_cli_version: 3.0.0
        max_cli_version: null
        published: 2026-01-15
      - version: 1.1.0
        tag: nodejs@1.1.0
        min_cli_version: 2.0.0
        max_cli_version: 2.9.9
        published: 2025-12-01

  python:
    name: python
    latest: 3.1.0
    description: Python runtime with pip/poetry/uv
    category: runtimes
    versions:
      - version: 3.1.0
        tag: python@3.1.0
        min_cli_version: 3.0.0
        published: 2026-01-10

metadata:
  last_updated: 2026-01-21
  extension_count: 42
```

### Compatibility Matrix

Each extension version declares CLI version compatibility:

```yaml
# In extension.yaml
metadata:
  name: nodejs
  version: 1.2.0
  min_cli_version: 3.0.0 # Requires at least CLI v3.0.0
  max_cli_version: null # No upper bound (forward-compatible)
```

**CLI Version Check**:

```rust
pub fn check_compatibility(extension: &Extension, cli_version: &Version) -> Result<()> {
    let min_version = extension.metadata.min_cli_version
        .as_ref()
        .and_then(|v| Version::parse(v).ok());

    let max_version = extension.metadata.max_cli_version
        .as_ref()
        .and_then(|v| Version::parse(v).ok());

    if let Some(min) = min_version {
        if cli_version < &min {
            bail!(
                "Extension {} v{} requires CLI v{} or higher (you have v{})",
                extension.metadata.name,
                extension.metadata.version,
                min,
                cli_version
            );
        }
    }

    if let Some(max) = max_version {
        if cli_version > &max {
            bail!(
                "Extension {} v{} is not compatible with CLI v{} (max: v{})",
                extension.metadata.name,
                extension.metadata.version,
                cli_version,
                max
            );
        }
    }

    Ok(())
}
```

### Local Caching Strategy

Extensions are cached locally to support offline installation and reduce GitHub API calls:

**Cache Directory Structure**:

```
$HOME/.sindri/cache/extensions/
├── registry.yaml               # Cached registry (1-hour TTL)
├── nodejs/
│   ├── 1.2.0/
│   │   ├── extension.yaml
│   │   └── .metadata          # Timestamp, checksum
│   └── 1.1.0/
│       ├── extension.yaml
│       └── .metadata
└── python/
    └── 3.1.0/
        ├── extension.yaml
        └── .metadata
```

**Cache Refresh Logic**:

```rust
pub struct ExtensionCache {
    cache_dir: PathBuf,
    ttl: Duration,  // Default: 1 hour
}

impl ExtensionCache {
    pub async fn get_registry(&self) -> Result<Registry> {
        let cache_file = self.cache_dir.join("registry.yaml");

        // Check if cache exists and is fresh
        if cache_file.exists() {
            let metadata = fs::metadata(&cache_file)?;
            let age = metadata.modified()?.elapsed()?;

            if age < self.ttl {
                // Cache hit
                return self.load_registry_from_cache(&cache_file);
            }
        }

        // Cache miss or stale - fetch from GitHub
        let registry = self.fetch_registry_from_github().await?;
        self.save_registry_to_cache(&cache_file, &registry)?;

        Ok(registry)
    }

    pub async fn get_extension(
        &self,
        name: &str,
        version: &str,
    ) -> Result<Extension> {
        let cache_path = self.cache_dir
            .join(name)
            .join(version)
            .join("extension.yaml");

        // Try cache first
        if cache_path.exists() {
            if let Ok(extension) = self.load_extension_from_cache(&cache_path) {
                return Ok(extension);
            }
        }

        // Fetch from GitHub
        let extension = self.fetch_extension_from_github(name, version).await?;
        self.save_extension_to_cache(&cache_path, &extension)?;

        Ok(extension)
    }

    async fn fetch_registry_from_github(&self) -> Result<Registry> {
        let url = format!(
            "https://raw.githubusercontent.com/pacphi/sindri/main/v3/registry.yaml"
        );

        let response = reqwest::get(&url).await?;
        let content = response.text().await?;
        let registry: Registry = serde_yaml::from_str(&content)?;

        Ok(registry)
    }

    async fn fetch_extension_from_github(
        &self,
        name: &str,
        version: &str,
    ) -> Result<Extension> {
        // Find tag for this version
        let registry = self.get_registry().await?;
        let ext_meta = registry.extensions.get(name)
            .ok_or_else(|| anyhow!("Extension '{}' not found", name))?;

        let version_info = ext_meta.versions.iter()
            .find(|v| v.version == version)
            .ok_or_else(|| anyhow!("Version '{}' not found", version))?;

        // Fetch from tagged release
        let url = format!(
            "https://raw.githubusercontent.com/pacphi/sindri/{}/v3/extensions/{}/extension.yaml",
            version_info.tag,
            name
        );

        let response = reqwest::get(&url).await?;
        let content = response.text().await?;
        let extension: Extension = serde_yaml::from_str(&content)?;

        Ok(extension)
    }
}
```

### Rollback Support

Users can install specific versions or rollback to previous versions:

```bash
# Install latest version
sindri extension install nodejs

# Install specific version
sindri extension install nodejs@1.1.0

# Rollback to previous version
sindri extension rollback nodejs

# List available versions
sindri extension versions nodejs
```

**Rollback Implementation**:

```rust
pub async fn rollback_extension(&self, name: &str) -> Result<()> {
    // Get current installed version from manifest
    let manifest = self.load_manifest()?;
    let current = manifest.get_installed_version(name)?;

    // Get previous version from registry
    let registry = self.cache.get_registry().await?;
    let ext_meta = registry.extensions.get(name)
        .ok_or_else(|| anyhow!("Extension '{}' not found", name))?;

    let previous = ext_meta.versions.iter()
        .filter(|v| Version::parse(&v.version).unwrap() < Version::parse(current).unwrap())
        .max_by_key(|v| Version::parse(&v.version).unwrap())
        .ok_or_else(|| anyhow!("No previous version available"))?;

    println!("Rolling back {} from {} to {}", name, current, previous.version);

    // Uninstall current, install previous
    self.uninstall_extension(name).await?;
    self.install_extension_version(name, &previous.version).await?;

    Ok(())
}
```

## Consequences

### Positive

1. **Zero Infrastructure**: Uses GitHub as free CDN and version control
2. **Immutability**: Git tags are immutable (once pushed, cannot change)
3. **Version History**: Full history of all extension versions in git log
4. **Rollback Support**: Easy to fetch any previous version by tag
5. **Discoverability**: Browse extensions on GitHub web interface
6. **Offline Support**: Local cache enables offline installation
7. **Update Checks**: Compare local manifest with remote registry
8. **Compatibility Tracking**: Registry enforces CLI version compatibility
9. **Low Latency**: 1-hour TTL reduces GitHub API calls by ~95%
10. **Developer Friendly**: Standard git workflow for publishing extensions

### Negative

1. **GitHub Dependency**: Relies on GitHub availability (can be mitigated with mirror)
2. **Rate Limits**: GitHub API has rate limits (5000/hour authenticated, 60/hour unauthenticated)
3. **Monorepo Size**: Large number of extensions could make repo heavy (mitigated: only YAML files)
4. **Tag Namespace**: Extension tags must not conflict with CLI version tags
5. **Manual Registry Updates**: Publishing new version requires updating both git tag and registry.yaml
6. **No Automatic Deprecation**: Can't automatically deprecate old versions (must be documented)
7. **Cache Invalidation**: Aggressive caching could delay critical security updates (mitigated: 1-hour TTL)

### Neutral

1. **Centralized vs Distributed**: Monorepo is centralized, but could support third-party registries in future
2. **Tag Convention**: `name@version` follows npm convention, but differs from git's `v` prefix
3. **Registry Format**: YAML registry is human-readable but larger than binary format

## Alternatives Considered

### 1. Separate Repository per Extension

**Description**: Each extension in its own repository (e.g., `sindri-extension-nodejs`, `sindri-extension-python`).

**Pros**:

- Independent versioning
- Clear ownership per extension
- Standard git tagging (v1.2.0)
- Easier to accept community contributions

**Cons**:

- 40+ repositories to manage
- No single source of truth for all extensions
- Harder to discover all available extensions
- More complex dependency management
- CI/CD must run across multiple repos

**Rejected**: Too complex for current scale. Can migrate to this model if needed in future.

### 2. npm Package Registry

**Description**: Publish extensions as npm packages (e.g., `@sindri/extension-nodejs`).

**Pros**:

- Battle-tested infrastructure
- Automatic versioning and immutability
- Built-in update checks (npm outdated)
- Dependency resolution (package.json)

**Cons**:

- Requires npm account and publishing process
- Extensions are YAML, not JavaScript (awkward fit)
- Adds npm as CLI dependency
- Can't easily browse extensions without npm
- Overkill for simple YAML files

**Rejected**: npm is designed for code packages, not configuration files.

### 3. CloudFlare R2 or S3

**Description**: Store extensions in object storage (S3, CloudFlare R2).

**Pros**:

- High availability and low latency
- No rate limits
- Versioning built-in
- CDN-backed

**Cons**:

- Requires paid infrastructure
- Needs custom upload/publishing tooling
- No version control integration
- More complex to manage than git

**Rejected**: Adds infrastructure cost and complexity. GitHub is sufficient.

### 4. Git Submodules for Extensions

**Description**: Each extension is a git submodule in the main repo.

**Pros**:

- Independent versioning per extension
- Standard git workflow
- Can update extensions independently

**Cons**:

- Submodules are notoriously complex
- Requires recursive clone
- Poor developer experience
- Harder to browse extensions

**Rejected**: Submodules add more complexity than they solve.

### 5. Container Registry (GHCR, Docker Hub)

**Description**: Package extensions as OCI container images.

**Pros**:

- Immutable layers
- Built-in versioning
- Content-addressable storage
- Standard tooling (docker pull)

**Cons**:

- Heavy for YAML files (~5KB → 5MB container)
- Requires container runtime
- Complex publishing process
- Overkill for simple files

**Rejected**: Containers are designed for executables, not configuration.

## Compliance

- ✅ Semantic versioning for all extensions
- ✅ Immutable versions (git tags)
- ✅ Local cache with 1-hour TTL
- ✅ CLI version compatibility matrix
- ✅ Rollback support
- ✅ Offline installation support
- ✅ Update notification system
- ✅ Zero infrastructure cost

## Notes

The monorepo approach is pragmatic for Sindri's current scale (~40 extensions). If the ecosystem grows significantly (100+ extensions, multiple maintainers), we can migrate to separate repositories or a dedicated package registry.

The tag naming convention (`name@version`) follows npm/yarn conventions and avoids conflicts with CLI version tags (`v2.2.1`).

Rate limit mitigation strategies:

1. Use authenticated GitHub API (5000 requests/hour)
2. Local cache with 1-hour TTL (reduces requests by ~95%)
3. Batch registry fetches (single request for all extensions)
4. Fall back to cached data if rate limit exceeded

Security consideration: Extensions fetched over HTTPS from GitHub are trusted. For paranoid users, we could add checksum verification against registry.yaml.

## Related Decisions

- [ADR-008: Extension Type System](008-extension-type-system-yaml-deserialization.md) - Extension structure
- [ADR-009: Dependency Resolution](009-dependency-resolution-dag-topological-sort.md) - Uses registry for dependency graph
- [ADR-012: Registry and Manifest Architecture](012-registry-manifest-dual-state-architecture.md) - Registry format
- [ADR-013: Schema Validation](013-schema-validation-strategy.md) - Validates fetched extensions

---

## Amendment: Migration to raw.githubusercontent.com (2026-02)

### Context for Amendment

The original design proposed using GitHub Releases with per-extension version tags (e.g., `nodejs@1.1.0`). In practice, this approach had several issues:

1. **Non-existent releases**: The system expected releases like `nodejs@1.1.0` that were never published
2. **Maintenance burden**: Publishing new extension versions required creating GitHub releases manually
3. **Version drift**: Extensions could evolve independently of CLI versions, causing compatibility issues
4. **Octocrab dependency**: Required the heavy octocrab GitHub API client for release queries

### Decision Amendment

We migrate from GitHub Releases API to **raw.githubusercontent.com** with CLI version tags:

**Key changes:**

1. **CLI version tag = extension snapshot**: Instead of per-extension releases, each CLI release (e.g., `v3.0.0-alpha.5`) serves as a snapshot of all extensions at that point in time.

2. **Version in extension.yaml only**: Extension versions are defined solely in their `extension.yaml` metadata. No separate release tags needed.

3. **Direct file fetch**: Extensions are fetched directly from `raw.githubusercontent.com`:

   ```
   https://raw.githubusercontent.com/{owner}/{repo}/{cli-tag}/{base_path}/{name}/extension.yaml
   ```

4. **Externalized configuration**: Repository configuration moved to `extension-source.yaml`:

   ```yaml
   github:
     owner: "pacphi"
     repo: "sindri"
     base_path: "v3/extensions"
   ```

5. **Removed octocrab dependency**: No longer need GitHub API client; simple HTTP requests to raw URLs.

### URL Derivation

For CLI version `v3.0.0-alpha.5` requesting extension `nodejs`:

```
https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.5/v3/extensions/nodejs/extension.yaml
```

With fallback to `main` branch if tag doesn't exist (for development builds).

### Benefits

1. **Simpler publishing**: Tagging a CLI release automatically includes all extension changes
2. **Guaranteed compatibility**: Extensions at a CLI tag are known to work with that CLI version
3. **Reduced dependencies**: Removed octocrab (~5MB compile time savings)
4. **Faster fetches**: Direct file download vs. API queries
5. **No rate limits**: raw.githubusercontent.com doesn't have GitHub API rate limits
6. **Single source of truth**: Extension version defined in one place only

### Trade-offs

1. **Less flexible versioning**: Cannot independently version extensions without CLI release
2. **No version history browsing**: Can't easily list all historical extension versions
3. **Requires CLI tag to exist**: Falls back to main branch during development

### Configuration File

New `extension-source.yaml` allows customization of the extension source:

```yaml
github:
  owner: "pacphi"
  repo: "sindri"
  base_path: "v3/extensions"
```

Schema available at `v3/schemas/extension-source.schema.json`.

### Migration Path

1. Existing installations continue to work (bundled extensions unchanged)
2. New downloads use raw.githubusercontent.com URLs
3. octocrab dependency removed from sindri-extensions crate
4. `extension-source.yaml` bundled in Docker images

### Files Changed

| File                                              | Change                         |
| ------------------------------------------------- | ------------------------------ |
| `v3/extension-source.yaml`                        | NEW - externalized repo config |
| `v3/schemas/extension-source.schema.json`         | NEW - schema for config        |
| `v3/crates/sindri-extensions/src/distribution.rs` | Major refactor                 |
| `v3/Dockerfile`                                   | Copy extension-source.yaml     |
| `v3/Dockerfile.dev`                               | Copy extension-source.yaml     |
| `v3/crates/sindri-extensions/Cargo.toml`          | Remove octocrab                |
