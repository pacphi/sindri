# Extension Sourcing Modes: Bundled vs Downloaded

This document describes the two extension sourcing modes in Sindri v3 and how they are implemented.

## Overview

Sindri v3 supports two modes for sourcing extensions:

1. **Bundled Mode (From Source)** - Extensions pre-packaged in Docker image
2. **Downloaded Mode (From GitHub)** - Extensions fetched from GitHub at runtime

The same codebase transparently handles both modes with automatic detection and fallback logic.

---

## Mode 1: Bundled Mode (From Source) 🏗️

Extensions are pre-packaged into the Docker image during build time, enabling offline/air-gapped deployments.

### Dockerfile Configuration

**File:** `Dockerfile.dev` (lines 194-205)

```dockerfile
# Bundle extensions into /opt/sindri
RUN mkdir -p /opt/sindri

COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} v3/extensions/ /opt/sindri/extensions/
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} v3/registry.yaml /opt/sindri/registry.yaml
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} v3/profiles.yaml /opt/sindri/profiles.yaml
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} v3/compatibility-matrix.yaml /opt/sindri/compatibility-matrix.yaml
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} v3/extension-source.yaml /opt/sindri/extension-source.yaml
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} v3/common.sh /opt/sindri/common.sh

ENV SINDRI_EXT_HOME=/opt/sindri/extensions
```

### Directory Structure

```
/opt/sindri/
├── extensions/          ← Bundled extensions
│   ├── nodejs/
│   │   ├── extension.yaml
│   │   └── mise.toml
│   ├── python/
│   │   └── extension.yaml
│   ├── docker/
│   ├── kubernetes/
│   └── ... (40+ extensions)
├── registry.yaml        ← Extension catalog
├── profiles.yaml        ← Profile definitions
├── compatibility-matrix.yaml
├── extension-source.yaml
└── common.sh
```

### Code Path Detection

**File:** `crates/sindri-extensions/src/source.rs` (lines 42-101)

```rust
pub struct BundledSource {
    pub base_path: PathBuf,  // /opt/sindri/extensions
}

impl BundledSource {
    /// Create from SINDRI_EXT_HOME environment variable
    pub fn from_env() -> Option<Self> {
        std::env::var("SINDRI_EXT_HOME").ok().and_then(|path| {
            let path = PathBuf::from(&path);

            // CRITICAL: Must start with /opt/sindri (not user home)
            if path.starts_with("/opt/sindri") && path.exists() {
                Some(Self::new(path))
            } else {
                None  // Not bundled mode
            }
        })
    }
}
```

**Detection Logic:**

1. Check `SINDRI_EXT_HOME` environment variable
2. Validate path starts with `/opt/sindri`
3. Verify path exists
4. If all pass → **Bundled mode active**

### Installation Flow (Bundled)

**File:** `crates/sindri-extensions/src/distribution.rs` (lines 432-461)

```rust
async fn get_bundled_extension_dir(&self, name: &str) -> Result<Option<PathBuf>> {
    if let Ok(ext_home) = std::env::var("SINDRI_EXT_HOME") {
        let ext_home_path = PathBuf::from(&ext_home);

        // Only treat as bundled if under /opt/sindri
        if !ext_home_path.starts_with("/opt/sindri") {
            return Ok(None);  // Not bundled
        }

        let bundled_ext_dir = ext_home_path.join(name);

        if bundled_ext_dir.exists() &&
           bundled_ext_dir.join("extension.yaml").exists() {
            return Ok(Some(bundled_ext_dir));  // Use bundled
        }
    }
    Ok(None)  // Fall back to GitHub
}
```

### Execution Path Resolution

**File:** `crates/sindri-extensions/src/executor.rs` (lines 54-105)

```rust
fn resolve_extension_dir(&self, name: &str) -> PathBuf {
    // Case 1: Direct path (bundled - already at extension directory)
    if self.extension_dir.join("extension.yaml").exists() {
        return self.extension_dir.clone();
    }

    // Case 2: Flat structure (bundled - extension_dir/{name}/)
    let flat_path = self.extension_dir.join(name);
    if flat_path.join("extension.yaml").exists() {
        return flat_path;  // /opt/sindri/extensions/nodejs
    }

    // Case 3: Versioned structure (downloaded - fallback)
    // ... see Downloaded Mode section
}
```

### Characteristics

| Characteristic        | Value                                            |
| --------------------- | ------------------------------------------------ |
| **Image Size**        | ~1.2GB                                           |
| **Internet Required** | ❌ No (offline capable)                          |
| **Extension Count**   | 40+ pre-installed                                |
| **Update Method**     | Rebuild Docker image                             |
| **Use Case**          | Development, air-gapped, consistent environments |
| **Disk Space**        | Higher (all extensions included)                 |

---

## Mode 2: Downloaded Mode (From GitHub) 🌐

Extensions are fetched from GitHub at runtime, enabling smaller images and independent extension updates.

### Dockerfile Configuration

**File:** `Dockerfile` (production - lines 260-280)

```dockerfile
# Extension Configuration (Production Mode)
# Extensions are NOT bundled with the image.
# They are installed at runtime to ${HOME}/.sindri/extensions

# Set extension home to volume-mounted path
ENV SINDRI_EXT_HOME=/alt/home/developer/.sindri/extensions

# Bundle compatibility matrix and extension source config only
RUN mkdir -p /alt/home/developer/.sindri/extensions
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} \
    v3/compatibility-matrix.yaml \
    /alt/home/developer/.sindri/compatibility-matrix.yaml
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} \
    v3/extension-source.yaml \
    /alt/home/developer/.sindri/extension-source.yaml
```

**Key Differences:**

- ❌ **NO** extensions copied
- ✅ **Only** compatibility matrix and source config
- ✅ Empty extensions directory created

### Directory Structure (Runtime)

```
/alt/home/developer/.sindri/
├── extensions/          ← Empty initially, populated at runtime
│   ├── nodejs/
│   │   └── 1.2.0/      ← Versioned structure
│   │       ├── extension.yaml
│   │       └── mise.toml
│   └── python/
│       └── 3.1.0/
│           └── extension.yaml
├── cache/              ← Download cache
│   ├── compatibility-matrix.yaml
│   └── ...
├── compatibility-matrix.yaml
└── extension-source.yaml
```

### Configuration File

**File:** `extension-source.yaml` (lines 1-21)

```yaml
# Extension source configuration
# Defines where to fetch extensions from GitHub

github:
  # Repository owner (GitHub username or organization)
  owner: "pacphi"

  # Repository name
  repo: "sindri"

  # Base path within the repository where extensions are located
  # Extensions are at: {base_path}/{extension_name}/extension.yaml
  base_path: "v3/extensions"
```

**Purpose:** Externalizes GitHub repository configuration (can be customized)

### Code Path Detection

**File:** `crates/sindri-extensions/src/source.rs` (lines 104-217)

```rust
pub struct DownloadedSource {
    pub extensions_dir: PathBuf,  // ~/.sindri/extensions
    pub cache_dir: PathBuf,       // ~/.sindri/cache
    pub cli_version: Version,
}

impl DownloadedSource {
    pub fn from_env() -> Result<Self> {
        let home = std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
            .ok_or_else(|| anyhow!("Could not determine home directory"))?;

        let extensions_dir = std::env::var("SINDRI_EXT_HOME")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".sindri/extensions"));

        let cache_dir = home.join(".sindri/cache");

        let cli_version = Version::parse(env!("CARGO_PKG_VERSION"))
            .unwrap_or_else(|_| Version::new(3, 0, 0));

        Ok(Self::new(extensions_dir, cache_dir, cli_version))
    }

    /// Download extension from GitHub and then load it
    pub async fn download_and_get(&self, name: &str) -> Result<Extension> {
        let distributor = ExtensionDistributor::new(
            self.cache_dir.clone(),
            self.extensions_dir.clone(),
            self.cli_version.clone(),
        )?;

        distributor.install(name, None).await?;  // Download from GitHub

        self.get_extension(name)  // Load from downloaded location
    }
}
```

### URL Construction

All GitHub URLs are built through `ExtensionSourceConfig`, which loads repository coordinates from `extension-source.yaml` (or falls back to defaults). No module constructs GitHub URLs from hardcoded string literals.

**File:** `crates/sindri-extensions/src/distribution.rs`

```rust
impl ExtensionSourceConfig {
    /// Build URL for an extension file
    pub fn build_url(&self, tag: &str, name: &str, file: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}/{}/{}",
            self.github.owner,    // from extension-source.yaml or "pacphi"
            self.github.repo,     // from extension-source.yaml or "sindri"
            tag,                  // "v3.0.0-alpha.11"
            self.github.base_path,// "v3/extensions"
            name,                 // "nodejs"
            file                  // "extension.yaml"
        )
    }

    /// Build URL for a repo-level file (registry, profiles, compat matrix)
    pub fn build_repo_url(&self, tag: &str, path: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            self.github.owner, self.github.repo, tag, path
        )
    }
}
```

**Example URLs (with default config):**

```
# Extension file
https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.11/v3/extensions/nodejs/extension.yaml

# Registry (repo-level file)
https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.11/v3/registry.yaml
```

**Custom source example** (`~/.sindri/extension-source.yaml`):

```yaml
github:
  owner: "my-org"
  repo: "my-sindri-fork"
  base_path: "v3/extensions"
```

```
# Same extension, different repo
https://raw.githubusercontent.com/my-org/my-sindri-fork/v3.0.0-alpha.11/v3/extensions/nodejs/extension.yaml
```

### Download Flow

**File:** `crates/sindri-extensions/src/distribution.rs` (lines 787-836)

```rust
async fn download_extension_files(&self, name: &str, version: &Version) -> Result<PathBuf> {
    let dest = self.extensions_dir
        .join(name)
        .join(version.to_string());  // Versioned structure

    // Skip if already downloaded
    if dest.join("extension.yaml").exists() {
        return Ok(dest);
    }

    fs::create_dir_all(&dest).await?;

    let tag = self.get_cli_tag();  // e.g., "v3.0.0-alpha.11"
    let client = reqwest::Client::new();

    // Download extension.yaml
    let ext_yaml_url = self.source_config.build_url(&tag, name, "extension.yaml");
    let content = self.fetch_file_with_fallback(
        &client, &ext_yaml_url, &tag, name, "extension.yaml"
    ).await?;

    // Parse to discover additional files
    let extension: Extension = serde_yaml_ng::from_str(&content)?;

    // Save extension.yaml
    fs::write(dest.join("extension.yaml"), &content).await?;

    // Download additional files (scripts, templates, etc.)
    self.download_additional_files(&client, &tag, name, &dest, &extension).await?;

    Ok(dest)
}
```

**Tag-First Fallback Logic (consistent across all modules):**

1. Load `ExtensionSourceConfig` from `extension-source.yaml` (or defaults)
2. Build URL using CLI version tag (e.g., `v3.0.0-alpha.18`)
3. If tag not found (404) → Retry with `main` branch
4. Cache downloaded files locally

This pattern is used consistently by:

- `ExtensionDistributor` (extension files)
- `ExtensionRegistry` (registry.yaml, profiles.yaml)
- `SupportFileManager` (common.sh, compatibility-matrix.yaml)
- `fetch_sindri_build_context()` (git clone for Docker builds)

**Exception:** `upgrade.rs` intentionally uses `main` for the compatibility matrix because the upgrade command needs the latest matrix to check forward compatibility.

### Execution Path Resolution

**File:** `crates/sindri-extensions/src/executor.rs` (lines 77-97)

```rust
fn resolve_extension_dir(&self, name: &str) -> PathBuf {
    // ... (skipping bundled checks)

    // Case 3: Versioned structure (downloaded)
    let flat_path = self.extension_dir.join(name);
    if flat_path.exists() {
        if let Ok(entries) = std::fs::read_dir(&flat_path) {
            let versions: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter(|e| e.path().join("extension.yaml").exists())
                .collect();

            // Return newest version
            if let Some(version_entry) = versions.into_iter().last() {
                return version_entry.path();  // ~/.sindri/extensions/nodejs/1.2.0
            }
        }
    }

    flat_path  // Fallback
}
```

### Characteristics

| Characteristic        | Value                             |
| --------------------- | --------------------------------- |
| **Image Size**        | ~800MB                            |
| **Internet Required** | ✅ Yes (for first install)        |
| **Extension Count**   | 0 initially, installed on-demand  |
| **Update Method**     | `sindri extension install <name>` |
| **Use Case**          | Production, flexible deployments  |
| **Disk Space**        | Lower (only installed extensions) |

---

## Priority Resolution Logic

The system supports three sources with the following priority order:

**File:** `crates/sindri-extensions/src/source.rs` (lines 289-470)

```rust
pub struct ExtensionSourceResolver {
    bundled: Option<BundledSource>,      // Priority 1
    local_dev: Option<LocalDevSource>,   // Priority 2
    downloaded: DownloadedSource,        // Priority 3 (always available)
}

impl ExtensionSourceResolver {
    /// Get extension from any available source
    pub async fn get_extension(&self, name: &str) -> Result<Extension> {
        // Priority 1: Bundled source
        if let Some(ref bundled) = self.bundled {
            if bundled.is_available(name) {
                return bundled.get_extension(name);  // Load from /opt/sindri
            }
        }

        // Priority 2: Local dev source (cargo run from source tree)
        if let Some(ref local_dev) = self.local_dev {
            if local_dev.is_available(name) {
                return local_dev.get_extension(name);  // Load from v3/extensions
            }
        }

        // Priority 3: Downloaded source (check if already downloaded)
        if self.downloaded.is_available(name) {
            return self.downloaded.get_extension(name);  // Load from ~/.sindri
        }

        // Fallback: Download from GitHub
        self.downloaded.download_and_get(name).await  // Fetch from GitHub
    }
}
```

### Source Priority

1. **Bundled** (`/opt/sindri/extensions/*`) - Fastest, no network
2. **LocalDev** (`v3/extensions/*`) - For development with `cargo run`
3. **Downloaded** (`~/.sindri/extensions/*/*`) - On-demand from GitHub

---

## Comparison Matrix

| Aspect                   | **Bundled Mode**                         | **Downloaded Mode**                                      |
| ------------------------ | ---------------------------------------- | -------------------------------------------------------- |
| **Dockerfile**           | `Dockerfile.dev`                         | `Dockerfile` (production)                                |
| **Extensions Included**  | ✅ Yes (40+ extensions)                  | ❌ No (empty directory)                                  |
| **Environment Variable** | `SINDRI_EXT_HOME=/opt/sindri/extensions` | `SINDRI_EXT_HOME=/alt/home/developer/.sindri/extensions` |
| **Detection Path**       | `/opt/sindri/extensions/*`               | `~/.sindri/extensions/*/*`                               |
| **Structure**            | Flat (`nodejs/extension.yaml`)           | Versioned (`nodejs/1.2.0/extension.yaml`)                |
| **Configuration Files**  | All bundled                              | Only compatibility matrix + source config                |
| **Internet Required**    | ❌ No (offline)                          | ✅ Yes (first install)                                   |
| **Image Size**           | ~1.2GB                                   | ~800MB                                                   |
| **Build Time**           | ~8 minutes                               | ~3-5 minutes                                             |
| **Update Method**        | Rebuild image                            | CLI command                                              |
| **Use Case**             | Development, air-gapped                  | Production, CI/CD                                        |
| **Code Detection**       | `path.starts_with("/opt/sindri")`        | `!path.starts_with("/opt/sindri")`                       |
| **Version Tracking**     | Single version (image snapshot)          | Multiple versions (rollback support)                     |

---

## Flow Diagrams

### Bundled Mode Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Docker Build (Dockerfile.dev)                            │
│    COPY v3/extensions/ → /opt/sindri/extensions/            │
│    ENV SINDRI_EXT_HOME=/opt/sindri/extensions               │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Runtime Detection (source.rs:from_env)                   │
│    Check: SINDRI_EXT_HOME starts with "/opt/sindri"?        │
│    ✅ Yes → BundledSource::new("/opt/sindri/extensions")    │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Extension Installation (distribution.rs)                 │
│    get_bundled_extension_dir("nodejs")                      │
│    → Returns: /opt/sindri/extensions/nodejs                 │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. Execution (executor.rs:resolve_extension_dir)            │
│    Check: /opt/sindri/extensions/nodejs/extension.yaml?     │
│    ✅ Exists → Use bundled path directly                    │
└─────────────────────────────────────────────────────────────┘
```

### Downloaded Mode Flow

```
┌──────────────────────────────────────────────────────────────┐
│ 1. Docker Build (Dockerfile)                                 │
│    COPY compatibility-matrix.yaml + extension-source.yaml    │
│    ENV SINDRI_EXT_HOME=/alt/home/developer/.sindri/extensions│
│    (NO extensions copied)                                    │
└──────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Runtime Detection (source.rs:from_env)                   │
│    Check: SINDRI_EXT_HOME starts with "/opt/sindri"?        │
│    ❌ No → DownloadedSource::from_env()                     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Load Source Config (distribution.rs)                     │
│    ExtensionSourceConfig::load()                            │
│    → ~/.sindri/extension-source.yaml or defaults            │
│    → {owner}/{repo} + base_path                             │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. Extension Request (distribution.rs:install)              │
│    Check bundled? ❌ No                                      │
│    Check downloaded? ❌ Not yet                              │
│    → Trigger download from GitHub                           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌──────────────────────────────────────────────────────────────┐
│ 5. GitHub Download (distribution.rs:download_extension_files)│
│    Build URL via source_config.build_url():                  │
│      raw.githubusercontent.com/{owner}/{repo}/               │
│      v3.0.0-alpha.11/v3/extensions/nodejs/extension.yaml     │
│    Tag not found? → Fallback to main branch                  │
│    Download to: ~/.sindri/extensions/nodejs/1.2.0/           │
└──────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 6. Execution (executor.rs:resolve_extension_dir)            │
│    Scan: ~/.sindri/extensions/nodejs/*/extension.yaml       │
│    Find newest version: 1.2.0                               │
│    → Use: ~/.sindri/extensions/nodejs/1.2.0/                │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Code Entry Points

| Functionality                    | File               | Function                                   | Lines   |
| -------------------------------- | ------------------ | ------------------------------------------ | ------- |
| **Source config loading**        | `distribution.rs`  | `ExtensionSourceConfig::load()`            | 60-108  |
| **Extension URL construction**   | `distribution.rs`  | `ExtensionSourceConfig::build_url()`       | 116-121 |
| **Repo-level URL construction**  | `distribution.rs`  | `ExtensionSourceConfig::build_repo_url()`  | 128-133 |
| **Bundled detection**            | `source.rs`        | `BundledSource::from_env()`                | 56-66   |
| **Downloaded detection**         | `source.rs`        | `DownloadedSource::from_env()`             | 126-144 |
| **Priority resolution**          | `source.rs`        | `ExtensionSourceResolver::get_extension()` | 400-426 |
| **Bundled check during install** | `distribution.rs`  | `get_bundled_extension_dir()`              | 432-461 |
| **GitHub download**              | `distribution.rs`  | `download_extension_files()`               | 787-836 |
| **Registry fetch (tag-first)**   | `registry.rs`      | `fetch_from_github()`                      | 197-256 |
| **Support file fetch**           | `support_files.rs` | `SupportFileManager::build_github_url()`   | 188-190 |
| **Execution path resolution**    | `executor.rs`      | `resolve_extension_dir()`                  | 54-105  |

---

## Environment Variables

| Variable            | Bundled Mode             | Downloaded Mode                          |
| ------------------- | ------------------------ | ---------------------------------------- |
| `SINDRI_EXT_HOME`   | `/opt/sindri/extensions` | `/alt/home/developer/.sindri/extensions` |
| `HOME`              | `/alt/home/developer`    | `/alt/home/developer`                    |
| `CARGO_PKG_VERSION` | (embedded in binary)     | (embedded in binary)                     |

---

## Security Considerations

### Path Traversal Prevention

**File:** `crates/sindri-extensions/src/source.rs` (lines 56-66)

```rust
// Only accept /opt/sindri as bundled path to prevent spoofing
if path.starts_with("/opt/sindri") && path.exists() {
    Some(Self::new(path))
} else {
    None  // Reject user home paths as bundled
}
```

This prevents users from:

- Setting `SINDRI_EXT_HOME=~/.sindri/extensions` and having it treated as bundled
- Path traversal attacks by setting malicious paths
- Spoofing bundled mode in production containers

### HTTPS-Only Downloads

**File:** `crates/sindri-extensions/src/distribution.rs`

All downloads use `https://raw.githubusercontent.com/` (HTTPS enforced)

### Checksum Verification

Future enhancement: ADR-010 mentions optional checksum verification against `registry.yaml`

---

## Use Cases

### When to Use Bundled Mode

✅ **Development environments**

- Fast local iteration
- No network dependencies
- Consistent tooling across team

✅ **Air-gapped deployments**

- Offline environments
- Restricted network access
- Compliance requirements

✅ **CI/CD pipelines**

- Reproducible builds
- Fixed tool versions
- No external dependencies

### When to Use Downloaded Mode

✅ **Production deployments**

- Smaller container images
- Independent extension updates
- Pay-per-use extensions

✅ **Cloud environments**

- Network access available
- Storage optimization
- Dynamic scaling

✅ **Multi-tenant systems**

- Per-user extension selection
- Reduced base image size
- Flexible updates

---

## Critical Fix: common.sh in Downloaded Mode

### Issue (Pre-Alpha.11)

Extensions failed in downloaded mode because `common.sh` was not copied to the production Dockerfile.

**Error:** `./install.sh: line 8: ./common.sh: No such file or directory`

### Fix (Alpha.11+)

**File:** `v3/Dockerfile` (line 280)

```dockerfile
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} v3/common.sh /alt/home/developer/.sindri/extensions/common.sh
```

**Why this location:**

- Extension scripts use: `source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"`
- From `~/.sindri/extensions/{name}/{version}/install.sh`, 3 `dirname` calls resolve to `~/.sindri/extensions/`
- common.sh must be at `~/.sindri/extensions/common.sh` to be found

**Impact:**

- ✅ All 40+ script-based extensions work in downloaded mode
- ✅ 4KB overhead (negligible)
- ✅ No extension script changes needed

---

## Troubleshooting

### Extension Not Found (Bundled Mode)

```bash
# Check SINDRI_EXT_HOME
echo $SINDRI_EXT_HOME
# Expected: /opt/sindri/extensions

# Verify extension exists
ls -la /opt/sindri/extensions/nodejs/extension.yaml

# Check if path validation passes
# Path must start with /opt/sindri
```

### Extension Not Found (Downloaded Mode)

```bash
# Check SINDRI_EXT_HOME
echo $SINDRI_EXT_HOME
# Expected: /alt/home/developer/.sindri/extensions

# Check extension-source.yaml config (if customized)
cat ~/.sindri/extension-source.yaml

# Verify internet connectivity (uses your configured source or defaults)
# Default: raw.githubusercontent.com/pacphi/sindri
curl -I https://raw.githubusercontent.com/pacphi/sindri/main/v3/extensions/nodejs/extension.yaml

# Check cache
ls -la ~/.sindri/cache/

# Manually trigger download
sindri extension install nodejs
```

### Wrong Mode Detected

```bash
# Check which mode is active
# Bundled mode: /opt/sindri/extensions exists
# Downloaded mode: /opt/sindri/extensions does NOT exist

# Force downloaded mode by unsetting or changing SINDRI_EXT_HOME
export SINDRI_EXT_HOME=$HOME/.sindri/extensions
```

---

## Related Documentation

- **ADR-010:** GitHub-based Extension Distribution
- **ADR-012:** Registry and Manifest Dual-State Architecture
- **Dockerfile:** Production image configuration
- **Dockerfile.dev:** Development image configuration with bundled extensions
- **extension-source.yaml:** GitHub repository configuration

---

## Architecture Benefits

1. ✅ **Flexibility** - Same codebase supports both modes
2. ✅ **Offline capability** - Bundled mode works air-gapped
3. ✅ **Updatability** - Downloaded mode enables independent extension updates
4. ✅ **Transparency** - Priority system automatically selects best source
5. ✅ **Security** - Path validation prevents spoofing
6. ✅ **Efficiency** - Smaller production images, faster builds
7. ✅ **Compatibility** - Seamless transition between modes
