# ADR 026: Extension Version Lifecycle Management

**Status**: Accepted
**Date**: 2026-01-22
**Deciders**: Core Team
**Related**: [ADR-010: GitHub Extension Distribution](010-github-extension-distribution.md), [ADR-012: Registry Manifest Dual-State Architecture](012-registry-manifest-dual-state-architecture.md), [ADR-022: Phase 6 Self-Update Implementation](022-phase-6-self-update-implementation.md)

## Context

Extensions in Sindri CLI evolve over time. Users need to:

1. **Discover versions**: See what versions are available for an extension
2. **Check compatibility**: Know which versions work with their CLI version
3. **Rollback safely**: Revert to previous versions if an update causes issues
4. **Track history**: Understand the upgrade path for debugging

### Current State (Before This ADR)

ADR-010 established GitHub release-based distribution with `{extension}@{version}` tagging:

```
GitHub Releases:
├── python@1.0.0
├── python@1.1.0
├── python@1.2.0
├── nodejs@2.0.0
└── nodejs@2.1.0
```

ADR-012 established manifest tracking for installed extensions:

```yaml
# ~/.sindri/state/manifest.yaml
extensions:
  python:
    version: "1.2.0"
    installed_at: "2026-01-20T10:00:00Z"
```

**Missing capabilities**:

1. No command to enumerate available versions
2. No rollback mechanism if upgrade fails
3. No version history tracking
4. No compatibility filtering when listing versions

### Requirements

**Version Discovery**:

- List all available versions from GitHub releases
- Show compatibility status with current CLI version
- Indicate which version is currently installed
- Display release dates for context
- Support JSON output for automation

**Rollback Support**:

- Revert to the immediately previous version
- Preserve version history for multiple rollbacks
- Confirm action with user (bypass with `--yes`)
- Update manifest atomically

**Compatibility Integration**:

- Filter versions by CLI compatibility matrix
- Show warnings for incompatible versions
- Block installation of incompatible versions (configurable)

## Decision

We implement a comprehensive version lifecycle management system with three components:

### a) Version Enumeration Algorithm

**Decision**: Query GitHub releases with tag pattern matching and semver sorting.

**Implementation** (`distribution.rs`):

```rust
/// List all available versions from GitHub releases
///
/// Versions are extracted from release tags matching `{name}@{version}` pattern.
/// Returns tuples of (Version, ReleaseDate, IsCompatible) sorted newest-first.
pub async fn list_available_versions(
    &self,
    name: &str,
    compatible_range: Option<&VersionReq>,
) -> Result<Vec<(Version, DateTime<Utc>, bool)>> {
    // Fetch releases from GitHub API
    let releases = self.github_client
        .repos(owner, repo)
        .releases()
        .list()
        .per_page(100)
        .send()
        .await?;

    let prefix = format!("{}@", name);

    // Filter and parse versions from tags
    let mut versions: Vec<(Version, DateTime<Utc>, bool)> = releases.items
        .iter()
        .filter(|r| r.tag_name.starts_with(&prefix))
        .filter_map(|r| {
            let version_str = r.tag_name.trim_start_matches(&prefix);
            let version = Version::parse(version_str).ok()?;
            let published_at = r.published_at.unwrap_or_else(Utc::now);
            let is_compatible = compatible_range
                .map(|req| req.matches(&version))
                .unwrap_or(true);
            Some((version, published_at, is_compatible))
        })
        .collect();

    // Sort by version descending (newest first)
    versions.sort_by(|a, b| b.0.cmp(&a.0));

    Ok(versions)
}
```

**Tag Convention**:

- Format: `{extension_name}@{semver}`
- Examples: `python@1.2.0`, `nodejs@2.0.0-beta.1`
- Pre-release versions supported via semver spec

### b) Version History Tracking

**Decision**: Extend manifest schema to track previous versions for rollback support.

**Extended Manifest Schema**:

```yaml
# ~/.sindri/state/manifest.yaml
version: "1.0"
extensions:
  python:
    version: "1.2.0"
    installed_at: "2026-01-22T10:00:00Z"
    install_method: "mise"
    previous_versions: # NEW: Version history for rollback
      - "1.1.0"
      - "1.0.0"
    upgrade_history: # NEW: Track upgrade events
      - from: "1.1.0"
        to: "1.2.0"
        at: "2026-01-22T10:00:00Z"
```

**Rust Types**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub version: String,
    pub installed_at: DateTime<Utc>,
    pub install_method: InstallMethod,

    /// Previous versions for rollback (most recent first)
    #[serde(default)]
    pub previous_versions: Vec<String>,

    /// Upgrade history for auditing
    #[serde(default)]
    pub upgrade_history: Vec<UpgradeEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeEvent {
    pub from: String,
    pub to: String,
    pub at: DateTime<Utc>,
}
```

**History Management**:

- `previous_versions` is a LIFO stack (most recent previous version first)
- Maximum history depth: 5 versions (configurable)
- On upgrade: push current version to history
- On rollback: pop from history, push current to front

### c) Rollback State Machine

**Decision**: Implement atomic rollback with confirmation and manifest updates.

**State Transitions**:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Extension Lifecycle States                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌──────────┐     upgrade      ┌──────────┐                   │
│   │ v1.0.0   │ ───────────────> │ v1.1.0   │                   │
│   │ current  │                  │ current  │                   │
│   └──────────┘                  └──────────┘                   │
│        ▲                              │                        │
│        │         rollback             │                        │
│        └──────────────────────────────┘                        │
│                                                                 │
│   Manifest State During Rollback:                               │
│   ┌────────────────────────────────────────────────────────┐   │
│   │ Before:                    After:                       │   │
│   │   version: "1.1.0"          version: "1.0.0"           │   │
│   │   previous_versions:         previous_versions:         │   │
│   │     - "1.0.0"                 - "1.1.0"                 │   │
│   │                               - "1.0.0"                 │   │
│   └────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Rollback Implementation** (`extension.rs`):

```rust
/// Rollback an extension to its previous version
///
/// Process:
/// 1. Load manifest to get current version and history
/// 2. Validate previous version exists in history
/// 3. Confirm with user (unless --yes)
/// 4. Install previous version via distributor
/// 5. Update manifest atomically
async fn rollback(args: ExtensionRollbackArgs) -> Result<()> {
    // 1. Load manifest
    let manifest = load_manifest()?;
    let entry = manifest.extensions.get(&args.name)
        .ok_or_else(|| anyhow!("Extension '{}' not installed", args.name))?;

    let current = Version::parse(&entry.version)?;

    // 2. Get previous version from history
    let previous_str = entry.previous_versions.first()
        .ok_or_else(|| anyhow!("No previous version available for rollback"))?;
    let previous = Version::parse(previous_str)?;

    // 3. Confirm with user
    if !args.yes {
        let confirmed = Confirm::new()
            .with_prompt(format!("Rollback {} from {} to {}?",
                args.name, current, previous))
            .interact()?;

        if !confirmed {
            return Ok(());
        }
    }

    // 4. Perform rollback via distributor
    distributor.rollback(&args.name).await?;

    // 5. Manifest is updated by distributor.rollback()
    Ok(())
}
```

### d) Versions Command

**Decision**: Implement `sindri extension versions <name>` with rich output.

**Command Interface**:

```bash
# List versions with compatibility info
sindri extension versions python

# JSON output for automation
sindri extension versions python --json
```

**Output Format**:

```
Available Versions: python

┌─────────┬────────────┬───────────────────┬────────────┐
│ VERSION │ COMPATIBLE │ STATUS            │ RELEASED   │
├─────────┼────────────┼───────────────────┼────────────┤
│ 1.3.0   │ yes        │ latest            │ 2026-01-22 │
│ 1.2.0   │ yes        │ installed         │ 2026-01-15 │
│ 1.1.0   │ yes        │ -                 │ 2026-01-08 │
│ 1.0.0   │ no         │ -                 │ 2026-01-01 │
└─────────┴────────────┴───────────────────┴────────────┘

Compatible range: >=1.1.0
Current CLI version: 3.0.0

Upgrade available: 1.2.0 -> 1.3.0
Run 'sindri extension upgrade python' to upgrade
```

**JSON Output**:

```json
{
  "extension": "python",
  "cli_version": "3.0.0",
  "compatible_range": ">=1.1.0",
  "installed_version": "1.2.0",
  "latest_version": "1.3.0",
  "versions": [
    {
      "version": "1.3.0",
      "compatible": true,
      "status": "latest",
      "released": "2026-01-22"
    },
    {
      "version": "1.2.0",
      "compatible": true,
      "status": "installed",
      "released": "2026-01-15"
    }
  ]
}
```

### e) Integration with Compatibility Matrix

**Decision**: Use compatibility matrix from ADR-022 to filter and annotate versions.

**Compatibility Check Flow**:

```rust
// 1. Get compatible version range for current CLI
let cli_pattern = format!("{}.{}.x", cli_version.major, cli_version.minor);
let compatible_range: Option<VersionReq> = matrix
    .cli_versions
    .get(&cli_pattern)
    .and_then(|compat| compat.compatible_extensions.get(&extension_name))
    .and_then(|range_str| VersionReq::parse(range_str).ok());

// 2. Pass to version listing
let versions = distributor
    .list_available_versions(name, compatible_range.as_ref())
    .await?;

// 3. Each version is annotated with compatibility status
for (version, date, is_compatible) in versions {
    // is_compatible = compatible_range.matches(&version)
}
```

## Consequences

### Positive

1. **Safe Rollbacks**: Users can revert problematic upgrades without manual intervention
2. **Version Visibility**: Clear view of available versions and compatibility
3. **Audit Trail**: Version history enables debugging upgrade issues
4. **Automation Support**: JSON output enables CI/CD integration
5. **Compatibility Awareness**: Users see which versions work with their CLI
6. **Consistent UX**: Follows patterns from `sindri upgrade` (self-update)

### Negative

1. **Manifest Growth**: Version history increases manifest size (~50 bytes per version)
2. **GitHub API Dependency**: Version listing requires API access (rate limits apply)
3. **Complexity**: More state to manage in manifest
4. **History Depth Limit**: Only 5 previous versions tracked (may be insufficient for some use cases)

### Neutral

1. **History Depth**: 5 versions is a reasonable default, can be made configurable
2. **Atomic Operations**: Rollback is not truly atomic at filesystem level
3. **No Cross-Extension Rollback**: Each extension rolls back independently

## Alternatives Considered

### 1. Git-Based Version Control for Extensions

**Description**: Use git branches/tags locally to manage extension versions.

**Pros**:

- Full version history
- Atomic operations via git
- Works offline

**Cons**:

- Requires git in extension directory
- More disk space
- Complex merge conflicts possible
- Overkill for simple use case

**Rejected**: Manifest-based history is simpler and sufficient.

### 2. Keep All Versions Installed

**Description**: Install each version in separate directory, switch symlinks.

**Pros**:

- Instant rollback
- No re-download needed
- Multiple versions available

**Cons**:

- Disk space explosion
- Confusing directory structure
- Complex cleanup logic
- Permission issues with symlinks

**Rejected**: Disk space cost too high for typical use cases.

### 3. No History, Re-Download on Rollback

**Description**: Don't track history, let user specify version to install.

**Pros**:

- Simpler manifest
- No history depth limit
- Explicit version selection

**Cons**:

- User must know previous version
- No "undo" capability
- Requires network for rollback
- Worse UX

**Rejected**: Version history significantly improves user experience.

### 4. Infinite History Depth

**Description**: Keep all previous versions in history, no limit.

**Pros**:

- Complete history
- No data loss

**Cons**:

- Manifest grows unbounded
- Most history rarely used
- Performance impact on manifest parsing

**Rejected**: 5-version limit covers 99% of use cases with bounded growth.

## Compliance

- ✅ Version enumeration from GitHub releases
- ✅ Semver parsing and sorting
- ✅ Compatibility filtering via matrix
- ✅ Version history tracking in manifest
- ✅ Atomic rollback with confirmation
- ✅ Rich CLI output with table formatting
- ✅ JSON output for automation
- ✅ Integration with existing distribution infrastructure

## Notes

### Version History Rotation

When history exceeds 5 versions, oldest versions are dropped:

```rust
fn add_to_history(entry: &mut ManifestEntry, version: &str) {
    entry.previous_versions.insert(0, version.to_string());
    entry.previous_versions.truncate(MAX_HISTORY_DEPTH);
}
```

### Rollback vs Downgrade

- **Rollback**: Revert to immediately previous version (uses history)
- **Downgrade**: Install specific older version (uses `install --version`)

Both are supported, rollback is the common "undo" operation.

### Network Failure During Rollback

If network fails during rollback:

1. Original version remains installed
2. Manifest unchanged
3. User can retry
4. No partial state

### Pre-release Versions

Pre-release versions (e.g., `1.2.0-beta.1`) are:

- Listed in versions output
- Filtered by compatibility range
- Not auto-upgraded to (require explicit install)

## Related Decisions

- [ADR-010: GitHub Extension Distribution](010-github-extension-distribution.md) - Release tagging convention
- [ADR-012: Registry Manifest Dual-State Architecture](012-registry-manifest-dual-state-architecture.md) - Manifest schema
- [ADR-022: Phase 6 Self-Update Implementation](022-phase-6-self-update-implementation.md) - Compatibility matrix
