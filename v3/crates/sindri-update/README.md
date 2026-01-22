# sindri-update

Self-update functionality for the Sindri CLI, including version checking, binary downloads, and extension compatibility validation.

## Features

- **GitHub Release Integration**: Fetch available versions from GitHub releases
- **Self-Update Mechanism**: Download and replace CLI binary safely with atomic operations
- **Auto-Rollback**: Automatically rollback on update failure with verification
- **Backup Management**: Timestamped backups with automatic cleanup (keeps last 2)
- **Binary Verification**: Test new binary before committing with `--version` check
- **Extension Compatibility Checking**: Validate installed extensions against CLI versions
- **Manifest Integration**: Read installed extensions from `~/.sindri/manifest.yaml`
- **Progress Tracking**: Visual progress bars for downloads with retry logic
- **Checksum Verification**: SHA256 verification for downloaded binaries
- **Pretty-Printed Warnings**: Colored output for compatibility issues
- **Force Bypass Support**: Allow upgrades with `--force` flag (with warnings)

## Usage

### Self-Update with Auto-Rollback

The `SindriUpdater` provides a complete self-update mechanism with automatic rollback on failure:

```rust
use sindri_update::{SindriUpdater, UpdateResult};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create updater instance
    let updater = SindriUpdater::new()?;

    println!("Current version: {}", updater.current_version());

    // Update to a specific version
    match updater.update_to("3.1.0").await? {
        UpdateResult::AlreadyUpToDate(version) => {
            println!("Already at version {}", version);
        }
        UpdateResult::Updated { from, to, backup } => {
            println!("Successfully updated from {} to {}", from, to);
            println!("Backup saved at: {:?}", backup);
        }
    }

    Ok(())
}
```

**What happens during update:**

1. Fetches release information from GitHub
2. Downloads the binary with progress tracking
3. Verifies the downloaded binary using SHA256 checksum
4. Creates a timestamped backup of the current binary
5. Replaces the current binary atomically
6. Verifies the new binary by running `sindri --version`
7. If verification fails, automatically rolls back to the backup
8. Cleans up old backups (keeps last 2)

### Binary Verification

Before committing an update, the new binary is verified:

```rust
use sindri_update::SindriUpdater;
use std::path::Path;

let updater = SindriUpdater::new()?;

// Verify any binary by running --version
updater.verify_binary(Path::new("/path/to/sindri"))?;
```

### Rollback to Backup

If something goes wrong, you can manually rollback:

```rust
let updater = SindriUpdater::new()?;

// List available backups
let backups = updater.list_backups()?;
for backup in backups {
    println!("Backup: {:?} - {} bytes", backup.path, backup.size);
}

// Rollback to a specific backup
updater.rollback(Path::new("/path/to/backup.bak"))?;
```

### Backup Management

Backups are automatically managed:

```rust
let updater = SindriUpdater::new()?;

// Cleanup old backups (keeps last 2)
updater.cleanup_old_backups()?;

// List all backups with metadata
let backups = updater.list_backups()?;
for backup in backups {
    println!(
        "Path: {:?}\nSize: {} bytes\nCreated: {:?}\n",
        backup.path, backup.size, backup.created
    );
}
```

### Checking Extension Compatibility

```rust
use sindri_update::CompatibilityChecker;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut checker = CompatibilityChecker::new();

    // Load compatibility matrix from GitHub release
    checker.fetch_matrix_from_github("3.0.0").await?;

    // Load installed extensions from manifest
    let extensions = checker.load_installed_extensions()?;

    // Check compatibility
    let result = checker.check_compatibility("3.0.0", &extensions)?;

    // Display formatted warnings
    result.print_warnings(false);

    Ok(())
}
```

### With Force Bypass

```rust
let result = checker.check_compatibility("3.0.0", &extensions)?;

if !result.compatible {
    // Show warnings with force bypass enabled
    result.print_warnings(true);

    // User acknowledged risks, proceed anyway
    if force_flag {
        println!("Proceeding with upgrade despite incompatibilities...");
    }
}
```

### Loading from Local File

```rust
let mut checker = CompatibilityChecker::new();

// Load from local YAML file
let content = std::fs::read_to_string("compatibility-matrix.yaml")?;
checker.load_matrix_from_str(&content)?;
```

## API Reference

### `SindriUpdater`

Main struct for self-update operations with auto-rollback.

#### Methods

- `new() -> Result<Self>` - Create updater for current binary
- `current_version() -> &Version` - Get current version
- `binary_path() -> &Path` - Get path to current binary
- `update_to(&self, target_version: &str) -> Result<UpdateResult>` - Update to specific version
- `verify_binary(&self, path: &Path) -> Result<()>` - Verify a binary by running --version
- `rollback(&self, backup_path: &Path) -> Result<()>` - Rollback to a backup
- `cleanup_old_backups(&self) -> Result<()>` - Remove old backups (keeps last 2)
- `list_backups(&self) -> Result<Vec<BackupInfo>>` - List all available backups
- `with_compatibility_checker(checker: CompatibilityChecker) -> Self` - Set compatibility checker
- `release_manager() -> &ReleaseManager` - Get release manager
- `compatibility_checker() -> &CompatibilityChecker` - Get compatibility checker

### `UpdateResult`

Result of an update operation.

#### Variants

- `AlreadyUpToDate(String)` - Already at target version
- `Updated { from: String, to: String, backup: PathBuf }` - Successfully updated

### `BackupInfo`

Information about a backup file.

#### Fields

- `path: PathBuf` - Path to backup file
- `size: u64` - Size in bytes
- `created: SystemTime` - Creation timestamp

### `BinaryDownloader`

Binary downloader with retry logic and progress tracking.

#### Methods

- `new() -> Result<Self>` - Create new downloader
- `with_max_retries(retries: u32) -> Self` - Set maximum retry attempts
- `with_progress(show: bool) -> Self` - Enable/disable progress bars
- `download_release(&self, release: &Release, platform: Option<&str>) -> Result<DownloadResult>` - Download release binary
- `verify_checksum(&self, path: &Path, expected: &str) -> Result<bool>` - Verify SHA256 checksum
- `get_platform_asset(&self, release: &Release) -> Option<&ReleaseAsset>` - Get asset for current platform
- `list_available_platforms(&self, release: &Release) -> Vec<String>` - List all platforms

### `DownloadResult`

Result of a download operation.

#### Fields

- `file_path: PathBuf` - Path to downloaded file
- `file_size: u64` - Size in bytes
- `checksum: String` - SHA256 checksum
- `resumed: bool` - Whether download was resumed

### `ReleaseManager`

Manager for fetching GitHub releases.

#### Methods

- `new() -> Self` - Create new release manager
- `with_prerelease() -> Self` - Include prerelease versions
- `get_latest() -> Result<Release>` - Get latest release
- `list_releases(limit: usize) -> Result<Vec<Release>>` - List recent releases
- `get_release(tag: &str) -> Result<Release>` - Get specific release by tag
- `check_update(current: &str) -> Result<Option<Release>>` - Check if update available
- `get_platform_asset(release: &Release) -> Option<&ReleaseAsset>` - Get asset for platform

### `CompatibilityChecker`

Main struct for checking extension compatibility.

#### Methods

- `new() -> Self` - Create checker with default manifest path (`~/.sindri/manifest.yaml`)
- `with_manifest_path(path: PathBuf) -> Self` - Create with custom manifest path
- `load_matrix(&mut self, url: &str) -> Result<()>` - Load matrix from URL
- `load_matrix_from_str(&mut self, content: &str) -> Result<()>` - Load from string
- `fetch_matrix_from_github(&mut self, version: &str) -> Result<()>` - Fetch from GitHub releases
- `load_installed_extensions(&self) -> Result<HashMap<String, String>>` - Load from manifest
- `check_compatibility(&self, target_version: &str, extensions: &HashMap<String, String>) -> Result<CompatResult>` - Check compatibility

### `CompatResult`

Result of a compatibility check.

#### Fields

- `compatible: bool` - Whether all extensions are compatible
- `incompatible_extensions: Vec<IncompatibleExtension>` - List of incompatible extensions
- `warnings: Vec<String>` - Warning messages
- `breaking_changes: Vec<String>` - Breaking changes in target version

#### Methods

- `print_warnings(&self, force_enabled: bool)` - Display formatted warnings with colors
- `print_summary(&self)` - Display one-line summary

### `IncompatibleExtension`

Information about an incompatible extension.

#### Fields

- `name: String` - Extension name
- `current_version: String` - Currently installed version
- `required_range: String` - Required version range
- `reason: String` - Why it's incompatible

## Compatibility Matrix Format

The `compatibility-matrix.yaml` file defines CLI-extension version compatibility:

```yaml
schema_version: "1.0"

cli_versions:
  "3.0.x":
    extension_schema: "1.0"

    compatible_extensions:
      python: ">=1.0.0,<2.0.0"
      nodejs: ">=2.0.0,<3.0.0"
      rust: ">=1.0.0,<2.0.0"

    breaking_changes:
      - "CLI rewritten in Rust"
      - "Extension schema updated to v1.0"
```

### Wildcard Versions

CLI versions support wildcards:
- `3.0.x` - Matches 3.0.0, 3.0.1, 3.0.2, etc.
- `3.1.x` - Matches 3.1.0, 3.1.1, etc.

### Version Range Syntax

Extension version ranges use semver syntax:
- `>=1.0.0,<2.0.0` - Version 1.x.x
- `>=2.0.0,<3.0.0` - Version 2.x.x
- `^1.2.0` - Compatible with 1.2.0 (>=1.2.0,<2.0.0)
- `~1.2.0` - Compatible with 1.2.x (>=1.2.0,<1.3.0)

## Manifest File Format

The manifest file at `~/.sindri/manifest.yaml` tracks installed extensions:

```yaml
schema_version: "1.0"
cli_version: "3.0.0"
last_updated: "2026-01-22T10:00:00Z"

extensions:
  python:
    version: "1.2.0"
    installed_at: "2026-01-20T15:30:00Z"
    source: "github:sindri/sindri-extensions"

  nodejs:
    version: "2.0.0"
    installed_at: "2026-01-19T10:00:00Z"
    source: "github:sindri/sindri-extensions"
```

## Update Flow and Error Handling

The self-update mechanism follows a safe, multi-step process:

```
1. Fetch Release
   ├─ Get release info from GitHub
   ├─ Find platform-specific binary
   └─ Check if already at version
          ↓
2. Download Binary
   ├─ Download with progress tracking
   ├─ Resume partial downloads
   ├─ Retry on failure (up to 3 attempts)
   └─ Calculate SHA256 checksum
          ↓
3. Extract & Verify
   ├─ Extract from tarball if needed
   ├─ Make binary executable
   └─ Run --version to verify
          ↓
4. Create Backup
   ├─ Generate timestamped filename
   └─ Copy current binary to backup
          ↓
5. Replace Binary
   ├─ Atomic replacement (Unix: rename)
   └─ Handle locked files (Windows)
          ↓
6. Verify Installation
   ├─ Run --version on new binary
   ├─ Check output contains "sindri"
   └─ If fails → AUTO-ROLLBACK
          ↓
7. Cleanup
   └─ Remove old backups (keep last 2)
          ↓
8. Success
   └─ Return UpdateResult::Updated
```

### Error Handling

Every step has detailed error handling:

```rust
match updater.update_to("3.1.0").await {
    Ok(UpdateResult::Updated { from, to, backup }) => {
        println!("Updated from {} to {}", from, to);
        println!("Backup: {:?}", backup);
    }
    Ok(UpdateResult::AlreadyUpToDate(version)) => {
        println!("Already at {}", version);
    }
    Err(e) => {
        // Errors include context about what failed:
        // - "Failed to fetch release"
        // - "Failed to download binary"
        // - "Downloaded binary verification failed"
        // - "Failed to create backup"
        // - "Failed to replace binary"
        // - "Update failed: verification error. Rolled back."
        eprintln!("Update failed: {}", e);
    }
}
```

### Auto-Rollback

If verification fails after binary replacement, rollback is automatic:

```
New binary verification failed!
  ↓
Automatic Rollback Triggered
  ├─ Verify backup exists
  ├─ Test backup with --version
  ├─ Restore backup to original location
  └─ Return error with rollback notice
```

The user will see:
```
Error: Update failed: verification error: <reason>. Rolled back to previous version.
```

## Integration with Upgrade Flow

The compatibility system integrates with the upgrade command:

```rust
// In the upgrade command
let mut checker = CompatibilityChecker::new();

// Fetch matrix for target version
checker.fetch_matrix_from_github(&target_version).await?;

// Load installed extensions
let extensions = checker.load_installed_extensions()?;

// Check compatibility
let compat_result = checker.check_compatibility(&target_version, &extensions)?;

if !compat_result.compatible {
    // Show warnings
    compat_result.print_warnings(args.force);

    if !args.force {
        // Prompt user
        if !dialoguer::Confirm::new()
            .with_prompt("Continue with upgrade?")
            .default(false)
            .interact()? {
            return Ok(());
        }
    }
}

// Proceed with upgrade...
```

## Output Examples

### Compatible Extensions

```
✓ All extensions are compatible!
```

### Incompatible Extensions

```
⚠ Extension Compatibility Issues Detected

Incompatible Extensions:

  Extension                 Current         Required
  ────────────────────────────────────────────────────────────
  rust                      0.9.0           >=1.0.0,<2.0.0
    → Version 0.9.0 does not satisfy >=1.0.0,<2.0.0

Breaking Changes:

  • CLI rewritten in Rust - bash-based extensions from v2.x are not compatible
  • Extension schema updated to v1.0 with new validation requirements

ℹ Use --force to bypass these checks (not recommended)
```

### With --force Flag

```
⚠ Bypassing compatibility checks with --force flag
  ⚠ This may result in broken functionality!
```

## Testing

Run the tests:

```bash
cargo test --package sindri-update
```

## License

MIT
