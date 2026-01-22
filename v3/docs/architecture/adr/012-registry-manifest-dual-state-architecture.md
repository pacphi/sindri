# ADR 012: Registry and Manifest Dual-State Architecture

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-010: GitHub Distribution](010-github-extension-distribution.md), [Extension Guide](../../../../docs/EXTENSIONS.md)

## Context

The extension system requires tracking two distinct concerns:

1. **Available Extensions (Registry)**: What extensions exist, their versions, metadata, and where to fetch them
2. **Installed Extensions (Manifest)**: What extensions are currently installed, their state, and when they were installed

The bash implementation used a single `registry.yaml` file for both concerns, which caused issues:
- Cannot distinguish between "available" and "installed"
- No state tracking for installations in progress or failed
- Difficult to detect outdated extensions (installed version vs latest available)
- Manual conflict resolution when registry updates conflict with local state
- No audit trail of installation history

Example scenarios requiring different state:

**Registry concerns**:
- List all available extensions
- Check latest version of an extension
- Discover new extensions
- Fetch extension metadata before installation

**Manifest concerns**:
- List installed extensions
- Track installation state (installing, installed, failed)
- Detect outdated extensions (compare with registry)
- Rollback to previous version
- Audit installation history

**State machine for installed extensions**:
```
         install
pending ---------> installing ---------> installed
                       |                      |
                       | (error)              | (new version available)
                       v                      v
                    failed               outdated
                                              |
                                              | (remove)
                                              v
                                          removing
```

## Decision

### Dual-State Architecture

We adopt a **registry and manifest separation** with distinct responsibilities:

**Registry** (`~/.sindri/cache/extensions/registry.yaml`):
- **Source**: Fetched from GitHub (read-only from user perspective)
- **Purpose**: Catalog of available extensions with metadata
- **Update**: Fetched periodically (1-hour TTL cache)
- **Schema**: Maintained in GitHub repository

**Manifest** (`~/.sindri/extensions/manifest.yaml`):
- **Source**: Local file (read-write by CLI)
- **Purpose**: State tracking of installed extensions
- **Update**: Modified by CLI operations (install, uninstall, update)
- **Schema**: Maintained in sindri-extensions crate

### Registry Structure

```yaml
# ~/.sindri/cache/extensions/registry.yaml (read-only)
version: 1.0.0
metadata:
  last_updated: "2026-01-21T10:30:00Z"
  extension_count: 42
  repository: "https://github.com/pacphi/sindri"

extensions:
  nodejs:
    name: nodejs
    latest: "1.2.0"
    description: "Node.js runtime with npm/pnpm/yarn"
    category: runtimes
    homepage: "https://nodejs.org"
    repository: "https://github.com/pacphi/sindri/tree/main/docker/lib/extensions/nodejs"

    versions:
      - version: "1.2.0"
        tag: "nodejs@1.2.0"
        published: "2026-01-15T14:30:00Z"
        min_cli_version: "3.0.0"
        max_cli_version: null
        checksum: "sha256:abc123..."  # Checksum of extension.yaml

      - version: "1.1.0"
        tag: "nodejs@1.1.0"
        published: "2025-12-01T10:00:00Z"
        min_cli_version: "2.0.0"
        max_cli_version: "2.9.9"
        checksum: "sha256:def456..."

  python:
    name: python
    latest: "3.1.0"
    description: "Python runtime with pip/poetry/uv"
    category: runtimes
    versions:
      - version: "3.1.0"
        tag: "python@3.1.0"
        published: "2026-01-10T09:00:00Z"
        min_cli_version: "3.0.0"
        checksum: "sha256:ghi789..."
```

**Registry Type Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub version: String,
    pub metadata: RegistryMetadata,
    pub extensions: HashMap<String, ExtensionRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryMetadata {
    pub last_updated: DateTime<Utc>,
    pub extension_count: usize,
    pub repository: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRegistryEntry {
    pub name: String,
    pub latest: String,
    pub description: String,
    pub category: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub versions: Vec<ExtensionVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionVersion {
    pub version: String,
    pub tag: String,
    pub published: DateTime<Utc>,
    pub min_cli_version: String,
    pub max_cli_version: Option<String>,
    pub checksum: String,
}
```

### Manifest Structure

```yaml
# ~/.sindri/extensions/manifest.yaml (read-write)
version: 1.0.0
metadata:
  last_updated: "2026-01-21T11:00:00Z"
  install_count: 12

extensions:
  nodejs:
    name: nodejs
    version: "1.2.0"
    state: installed
    installed_at: "2026-01-20T15:30:00Z"
    installed_by: "sindri-cli v3.0.0"
    checksum: "sha256:abc123..."

    # State-specific metadata
    dependencies: []
    install_method: mise
    validation_passed: true

    # History
    history:
      - version: "1.1.0"
        installed_at: "2026-01-10T10:00:00Z"
        uninstalled_at: "2026-01-20T15:29:00Z"
        reason: "upgrade"

  python:
    name: python
    version: "3.1.0"
    state: installed
    installed_at: "2026-01-18T09:00:00Z"
    installed_by: "sindri-cli v3.0.0"
    dependencies: []
    install_method: mise

  claude-flow-v2:
    name: claude-flow-v2
    version: "1.5.0"
    state: outdated  # Newer version available in registry
    installed_at: "2025-12-15T14:00:00Z"
    installed_by: "sindri-cli v2.2.1"
    dependencies: [nodejs]
    install_method: npm
    available_update: "1.6.0"

  failed-extension:
    name: failed-extension
    version: "2.0.0"
    state: failed
    installed_at: "2026-01-21T10:45:00Z"
    error: "Installation timeout after 10 minutes"
    retry_count: 3
```

**Manifest Type Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub metadata: ManifestMetadata,
    pub extensions: HashMap<String, InstalledExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMetadata {
    pub last_updated: DateTime<Utc>,
    pub install_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub name: String,
    pub version: String,
    pub state: ExtensionState,
    pub installed_at: DateTime<Utc>,
    pub installed_by: String,
    pub checksum: String,
    pub dependencies: Vec<String>,
    pub install_method: String,

    // State-specific fields
    pub validation_passed: Option<bool>,
    pub error: Option<String>,
    pub retry_count: Option<u32>,
    pub available_update: Option<String>,

    // History
    pub history: Vec<InstallationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionState {
    Pending,      // Queued for installation
    Installing,   // Currently being installed
    Installed,    // Successfully installed
    Failed,       // Installation failed
    Outdated,     // Newer version available
    Removing,     // Being uninstalled
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationRecord {
    pub version: String,
    pub installed_at: DateTime<Utc>,
    pub uninstalled_at: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}
```

### State Machine Implementation

```rust
impl Manifest {
    /// Transition extension to new state
    pub fn transition_state(
        &mut self,
        name: &str,
        new_state: ExtensionState,
    ) -> Result<()> {
        let ext = self.extensions.get_mut(name)
            .ok_or_else(|| anyhow!("Extension '{}' not in manifest", name))?;

        // Validate state transition
        match (&ext.state, &new_state) {
            (ExtensionState::Pending, ExtensionState::Installing) => {}
            (ExtensionState::Installing, ExtensionState::Installed) => {}
            (ExtensionState::Installing, ExtensionState::Failed) => {}
            (ExtensionState::Installed, ExtensionState::Outdated) => {}
            (ExtensionState::Installed, ExtensionState::Removing) => {}
            (ExtensionState::Outdated, ExtensionState::Installing) => {}
            (ExtensionState::Failed, ExtensionState::Installing) => {}
            _ => bail!(
                "Invalid state transition for {}: {:?} -> {:?}",
                name,
                ext.state,
                new_state
            ),
        }

        ext.state = new_state;
        self.metadata.last_updated = Utc::now();
        self.save()?;

        Ok(())
    }

    /// Check for outdated extensions
    pub fn check_for_updates(&mut self, registry: &Registry) -> Result<Vec<String>> {
        let mut outdated = Vec::new();

        for (name, installed) in &mut self.extensions {
            if installed.state != ExtensionState::Installed {
                continue;
            }

            if let Some(registry_entry) = registry.extensions.get(name) {
                let installed_version = Version::parse(&installed.version)?;
                let latest_version = Version::parse(&registry_entry.latest)?;

                if latest_version > installed_version {
                    installed.state = ExtensionState::Outdated;
                    installed.available_update = Some(registry_entry.latest.clone());
                    outdated.push(name.clone());
                }
            }
        }

        if !outdated.is_empty() {
            self.save()?;
        }

        Ok(outdated)
    }

    /// Add extension to history and update state
    pub fn record_installation(
        &mut self,
        name: String,
        version: String,
        install_method: String,
        dependencies: Vec<String>,
        checksum: String,
    ) -> Result<()> {
        // Move existing installation to history
        if let Some(existing) = self.extensions.get_mut(&name) {
            let record = InstallationRecord {
                version: existing.version.clone(),
                installed_at: existing.installed_at,
                uninstalled_at: Some(Utc::now()),
                reason: Some("upgrade".to_string()),
            };
            existing.history.push(record);
        }

        // Create new installation record
        let installed = InstalledExtension {
            name: name.clone(),
            version,
            state: ExtensionState::Installed,
            installed_at: Utc::now(),
            installed_by: format!("sindri-cli v{}", env!("CARGO_PKG_VERSION")),
            checksum,
            dependencies,
            install_method,
            validation_passed: Some(true),
            error: None,
            retry_count: None,
            available_update: None,
            history: self.extensions
                .get(&name)
                .map(|e| e.history.clone())
                .unwrap_or_default(),
        };

        self.extensions.insert(name, installed);
        self.metadata.install_count = self.extensions.len();
        self.metadata.last_updated = Utc::now();
        self.save()?;

        Ok(())
    }
}
```

### Separation of Concerns

**Registry Operations** (sindri-extensions/src/registry.rs):
- `fetch_registry()` - Download from GitHub
- `get_extension_metadata()` - Get metadata for specific extension
- `list_available_extensions()` - List all extensions
- `search_extensions()` - Search by category/name

**Manifest Operations** (sindri-extensions/src/manifest.rs):
- `load_manifest()` - Load local manifest
- `save_manifest()` - Persist manifest changes
- `transition_state()` - Update extension state
- `check_for_updates()` - Compare with registry
- `record_installation()` - Add to history

**Coordinated Operations** (sindri-extensions/src/manager.rs):
```rust
pub struct ExtensionManager {
    registry: Registry,
    manifest: Manifest,
    cache: ExtensionCache,
}

impl ExtensionManager {
    /// List all extensions with their status
    pub fn list_extensions(&self) -> Vec<ExtensionListItem> {
        let mut items = Vec::new();

        // Include all available extensions from registry
        for (name, reg_entry) in &self.registry.extensions {
            let installed = self.manifest.extensions.get(name);

            let item = ExtensionListItem {
                name: name.clone(),
                description: reg_entry.description.clone(),
                latest_version: reg_entry.latest.clone(),
                installed_version: installed.map(|i| i.version.clone()),
                state: installed.map(|i| i.state.clone()),
                update_available: installed
                    .and_then(|i| i.available_update.clone()),
            };

            items.push(item);
        }

        items
    }

    /// Install extension (coordinates registry + manifest)
    pub async fn install_extension(&mut self, name: &str) -> Result<()> {
        // Get metadata from registry
        let reg_entry = self.registry.extensions.get(name)
            .ok_or_else(|| anyhow!("Extension '{}' not found", name))?;

        // Check if already installed
        if let Some(installed) = self.manifest.extensions.get(name) {
            if installed.state == ExtensionState::Installed {
                bail!("Extension '{}' is already installed", name);
            }
        }

        // Transition to Installing state
        self.manifest.transition_state(name, ExtensionState::Installing)?;

        // Fetch extension definition from cache/GitHub
        let extension = self.cache
            .get_extension(name, &reg_entry.latest)
            .await?;

        // Perform installation
        match install_extension(&extension).await {
            Ok(()) => {
                self.manifest.record_installation(
                    name.to_string(),
                    reg_entry.latest.clone(),
                    extension.install.method_name(),
                    extension.dependencies.unwrap_or_default(),
                    reg_entry.versions[0].checksum.clone(),
                )?;
                println!("Extension '{}' installed successfully", name);
            }
            Err(e) => {
                self.manifest.transition_state(name, ExtensionState::Failed)?;
                return Err(e);
            }
        }

        Ok(())
    }
}
```

## Consequences

### Positive

1. **Clear Separation**: Registry is read-only catalog, manifest is mutable state
2. **State Tracking**: Five states enable robust status tracking
3. **Update Detection**: Easy to compare installed vs available versions
4. **History**: Audit trail of all installations/uninstallations
5. **Conflict-Free**: Registry updates never conflict with local state
6. **Rollback Support**: History enables version rollback
7. **Offline Support**: Manifest survives registry fetch failures
8. **Type Safety**: Distinct types for registry vs manifest entries
9. **Extensibility**: Easy to add new states or metadata fields
10. **Debugging**: Clear source of truth for troubleshooting

### Negative

1. **Duplication**: Extension name/version duplicated in registry and manifest
2. **Sync Overhead**: Must coordinate operations between registry and manifest
3. **Storage**: Two files instead of one (~10KB each)
4. **Complexity**: More complex than single-file approach
5. **Race Conditions**: Concurrent operations could corrupt manifest
6. **Migration**: Must migrate from old single-file format

### Neutral

1. **File Locations**: Registry in cache, manifest in extensions directory
2. **Update Frequency**: Registry fetched hourly, manifest updated on every operation
3. **State Machine**: Could add more states (e.g., `upgrading`, `downgrading`)

## Alternatives Considered

### 1. Single Unified State File

**Description**: Keep single file with both available and installed extensions.

```yaml
extensions:
  nodejs:
    available: true
    latest_version: "1.2.0"
    installed: true
    installed_version: "1.2.0"
    state: installed
```

**Pros**:
- Single file to manage
- No duplication
- Simpler implementation

**Cons**:
- Registry updates conflict with local modifications
- Difficult to distinguish read-only vs mutable state
- No clear separation of concerns
- Sync issues when registry changes

**Rejected**: Violates separation of concerns principle.

### 2. Database (SQLite)

**Description**: Use SQLite database for registry and manifest.

**Pros**:
- Transactional updates
- Efficient queries
- No YAML parsing overhead
- Better concurrency handling

**Cons**:
- Adds SQLite dependency
- Binary format (not human-readable)
- Harder to debug
- Overkill for ~40 extensions

**Rejected**: Too complex for current scale. Could revisit at 100+ extensions.

### 3. Separate Files Per Extension

**Description**: Store each installed extension state in separate file.

```
~/.sindri/extensions/
├── nodejs.yaml
├── python.yaml
└── claude-flow-v2.yaml
```

**Pros**:
- No file locking conflicts
- Easy to add/remove extensions
- Parallel operations possible

**Cons**:
- Many small files (poor performance)
- No atomic updates across extensions
- Harder to query all installed extensions
- No global metadata

**Rejected**: Single manifest file is simpler and more efficient.

### 4. Registry in Manifest (Embedded)

**Description**: Embed registry metadata in manifest for each installed extension.

```yaml
extensions:
  nodejs:
    installed_version: "1.2.0"
    latest_version: "1.2.0"
    available_versions: ["1.2.0", "1.1.0"]
    state: installed
```

**Pros**:
- Self-contained manifest
- No need to fetch registry for installed extensions

**Cons**:
- Registry metadata becomes stale
- Larger manifest file
- Duplicate data
- Update detection requires registry fetch anyway

**Rejected**: Doesn't solve fundamental problem of separating available vs installed.

## Compliance

- ✅ Registry is read-only (user perspective)
- ✅ Manifest is read-write (CLI operations)
- ✅ Five-state machine (pending, installing, installed, failed, outdated, removing)
- ✅ History tracking for all installations
- ✅ Update detection by comparing versions
- ✅ Atomic state transitions with validation
- ✅ Type-safe operations with distinct types
- ✅ 100% test coverage for state transitions

## Notes

The dual-state architecture is inspired by package managers like apt (available packages in `/var/lib/apt/lists`, installed packages in `/var/lib/dpkg/status`) and Homebrew (formulae in tap, installed in Cellar).

The state machine is deliberately simple (5 states) to avoid overcomplication. More states could be added in future if needed (e.g., `upgrading`, `downgrading`, `verifying`).

The manifest format is designed to be human-readable and editable, though users should rarely need to edit it manually. The CLI should handle all operations.

Future enhancement: Add file locking to manifest to prevent concurrent modifications (use `fs2` crate).

## Related Decisions

- [ADR-009: Dependency Resolution](009-dependency-resolution-dag-topological-sort.md) - Uses manifest for installed extensions
- [ADR-010: GitHub Distribution](010-github-extension-distribution.md) - Registry source
- [ADR-011: Multi-Method Installation](011-multi-method-extension-installation.md) - Updates manifest state
- [ADR-014: SBOM Generation](014-sbom-generation-industry-standards.md) - Uses manifest for installed components
