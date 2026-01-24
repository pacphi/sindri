# Backup and Restore Guide

This guide covers how to safely backup, download, and restore your Sindri V3 workspace.

## Overview

The Sindri V3 backup system is implemented in the `sindri-backup` crate, providing:

- **Three backup profiles**: UserData, Standard, and Full
- **Three restore modes**: Safe, Merge, and Full
- **Streaming compression**: tar+gzip with configurable compression levels
- **Integrity verification**: SHA256 checksums for all backups
- **System marker protection**: Critical initialization files are never restored
- **Atomic restore with rollback**: Pre-restore snapshots enable recovery from failed restores

### Architecture: Understanding What Gets Backed Up

The Sindri home directory (`/alt/home/developer`) contains multiple categories of data with different backup/restore requirements:

```text
/alt/home/developer/                    # $HOME - persistent volume
├── workspace/                          # USER DATA - Always backup
│   ├── projects/                       # Your development projects
│   ├── config/                         # User configuration files
│   ├── scripts/                        # User scripts
│   ├── bin/                            # User binaries
│   └── .system/                        # SYSTEM STATE - Never restore
│       ├── manifest/                   #   Extension manifest
│       ├── installed/                  #   Installation markers
│       ├── logs/                       #   Extension logs
│       └── bootstrap.yaml              #   Auto-install state
├── .claude/                            # USER DATA - Always backup
├── .ssh/                               # MIXED
│   ├── host_keys/                      #   Backup (fingerprint stability)
│   └── authorized_keys                 #   Skip (injected from env)
├── .gitconfig                          # CONFIG - Backup (user settings)
├── .bashrc, .profile                   # CONFIG - Backup with care
├── .config/                            # MIXED
│   └── mise/conf.d/*.toml              #   Regenerable (extension configs)
├── .local/
│   ├── share/mise/installs/            # REGENERABLE - Don't backup
│   ├── state/mise/                     # REGENERABLE - Don't backup
│   └── bin/                            # Mixed - tools installed here
├── .cache/                             # CACHE - Never backup
├── .initialized                        # SYSTEM MARKER - Never restore
└── .welcome_shown                      # SYSTEM MARKER - Never restore
```

### Data Categories

| Category           | Description                      | Backup | Restore                 |
| ------------------ | -------------------------------- | ------ | ----------------------- |
| **User Data**      | Projects, scripts, Claude data   | Always | Always                  |
| **Config**         | Shell RC, gitconfig, app configs | Yes    | Merge/Skip conflicts    |
| **Regenerable**    | Mise installs, tool caches       | No     | N/A                     |
| **System Markers** | `.initialized`, `bootstrap.yaml` | No     | **Never** (breaks init) |
| **Cache**          | `.cache/*`                       | No     | N/A                     |

### Why System Markers Must Never Be Restored

**`.initialized`**: Controls first-boot initialization

- If restored to new instance: entrypoint skips setup
- Result: Missing `.bashrc`, broken PATH, no home structure

**`bootstrap.yaml`**: Controls auto-install

- If restored: auto-install thinks extensions already installed
- Result: Missing tools, broken development environment

---

## Quick Start

### Backup Your Workspace

```bash
# Standard backup (recommended for most users)
sindri backup --output ./backups/

# User data only (smallest, safest for migration)
sindri backup --profile user-data --output ./backups/

# Full backup (disaster recovery)
sindri backup --profile full --output ./backups/
```

### Restore to New Instance

```bash
# Safe restore (default - won't overwrite existing files)
sindri restore ./backups/sindri-backup-2026-01-15.tar.gz

# Preview what will happen
sindri restore ./backups/backup.tar.gz --dry-run

# Merge with existing (backs up conflicts first)
sindri restore ./backups/backup.tar.gz --mode merge
```

---

## Backup Profiles

### `user-data` Profile (Recommended for Migration)

**Size**: 100MB - 1GB typical
**Use case**: Moving to a new Sindri instance or switching providers

**Includes**:

| Path                  | Description                           |
| --------------------- | ------------------------------------- |
| `workspace/projects/` | All development projects              |
| `workspace/config/`   | User configuration files              |
| `workspace/scripts/`  | User scripts                          |
| `workspace/bin/`      | User binaries                         |
| `.claude/`            | Claude Code settings and history      |
| `.ssh/host_keys/`     | SSH host keys (fingerprint stability) |
| `.gitconfig`          | Git configuration                     |

**Excludes**:

- All system markers (`.initialized`, `bootstrap.yaml`)
- All mise installations (regenerable)
- All caches
- Shell RC files (regenerated on new instance)
- `.config/` and `.local/` directories

```bash
sindri backup --profile user-data --output ./backups/
```

### `standard` Profile (Default)

**Size**: 1GB - 5GB typical
**Use case**: Regular backups, same-instance recovery

**Includes everything in `user-data` plus**:

| Path          | Description                                |
| ------------- | ------------------------------------------ |
| `.bashrc`     | Bash configuration                         |
| `.profile`    | Profile configuration                      |
| `.config/`    | Application configs (excluding mise state) |
| `.local/bin/` | Locally installed binaries                 |

**Excludes**:

- `.config/mise/shims` - Regenerable shims
- `.local/share/mise` - Tool installations
- `.local/state` - Tool state

```bash
sindri backup --output ./backups/
# or explicitly:
sindri backup --profile standard --output ./backups/
```

### `full` Profile (Disaster Recovery)

**Size**: 5GB - 20GB typical
**Use case**: Complete disaster recovery to same instance, forensic analysis

**Includes**: Everything not in the always-excluded list

**Excludes only**:

- `.cache/` - Caches (always excluded)
- `.local/share/mise/installs/` - Can be regenerated
- `.local/state/` - Tool state
- `**/node_modules` - Node dependencies
- `**/.venv` - Python virtual environments
- `**/__pycache__` - Python cache
- `**/target/debug`, `**/target/release` - Rust build artifacts
- `**/.next`, `**/.turbo`, `**/.gradle` - Build caches

**Warning**: Restoring a full backup to a different instance may cause issues.

```bash
sindri backup --profile full --output ./backups/
```

---

## Restore Modes

### `safe` Mode (Default)

The safest option - never overwrites existing files.

**Behavior**:

| File Type      | Action                    |
| -------------- | ------------------------- |
| System markers | Never touched             |
| Existing files | Skipped (preserved)       |
| Missing files  | Restored                  |
| Conflicts      | Reported but not modified |

```bash
sindri restore ./backup.tar.gz
# or explicitly:
sindri restore ./backup.tar.gz --mode safe
```

**Best for**: Migration to new instance, adding missing files

### `merge` Mode

Smart merge with automatic backup of conflicts.

**Behavior**:

| File Type        | Action                                    |
| ---------------- | ----------------------------------------- |
| System markers   | Never touched                             |
| Existing files   | Backed up to `.bak`, then restored        |
| Shell configs    | Intelligently merged (preserves sections) |
| Creates rollback | `.bak` files enable recovery              |

```bash
sindri restore ./backup.tar.gz --mode merge
```

**Best for**: Updating existing instance, recovering from partial data loss

### `full` Mode

Complete restore - overwrites everything except system markers.

**Behavior**:

| File Type       | Action                          |
| --------------- | ------------------------------- |
| System markers  | Never touched (safety override) |
| All other files | Overwritten without backup      |

```bash
sindri restore ./backup.tar.gz --mode full
```

**Warning**: Only use for disaster recovery on the SAME instance. Using on a different instance may break initialization.

---

## CLI Reference

### Backup Command

```bash
sindri backup [options]

Options:
  -o, --output <path>     Output location (directory or file path)
                          Default: ./sindri-backup-{name}-{timestamp}.tar.gz
  -p, --profile <name>    Backup profile: user-data, standard (default), full
  --exclude <pattern>     Additional exclude pattern (can repeat)
  --dry-run               Show what would be backed up without creating archive
  --compression <level>   Compression level 1-9 (default: 6)
  -v, --verbose           Verbose output

Examples:
  sindri backup                                      # Standard backup to current dir
  sindri backup --profile user-data -o ./backups/    # User data only
  sindri backup --exclude '**/node_modules'          # Additional exclusion
  sindri backup --dry-run                            # Preview backup contents
  sindri backup --compression 9 -o ./backup.tar.gz   # Max compression
```

### Restore Command

```bash
sindri restore <source> [options]

Arguments:
  source                  Backup file path

Options:
  -m, --mode <name>       Restore mode: safe (default), merge, full
  --dry-run               Preview restore without making changes
  --force                 Skip compatibility checks
  --no-interactive        Skip confirmation prompts
  -v, --verbose           Verbose output

Examples:
  sindri restore ./backup.tar.gz                   # Safe restore
  sindri restore ./backup.tar.gz --mode merge      # Merge with backups
  sindri restore ./backup.tar.gz --dry-run         # Preview changes
  sindri restore ./backup.tar.gz --mode full       # Full overwrite
```

---

## Archive Format

### Structure

Backups are stored as gzip-compressed tar archives:

```
backup-{name}-{timestamp}.tar.gz
├── .backup-manifest.json         # Backup metadata (first file)
├── workspace/
│   ├── projects/
│   ├── config/
│   └── scripts/
├── .claude/
└── .gitconfig
```

### Manifest Format

Every backup includes a JSON manifest with metadata:

```json
{
  "version": "1.0.0",
  "backup_type": "standard",
  "created_at": "2026-01-21T15:30:00Z",
  "created_by": "sindri-cli v3.0.0",
  "source": {
    "instance_name": "my-sindri-dev",
    "provider": "fly",
    "hostname": "my-sindri-dev.fly.dev"
  },
  "profile": "standard",
  "compression": "gzip",
  "checksum": {
    "algorithm": "sha256",
    "value": "abc123def456..."
  },
  "statistics": {
    "files_included": 1234,
    "total_size_bytes": 5242880,
    "compressed_size_bytes": 2621440,
    "compression_ratio": 0.5,
    "duration_seconds": 45.2
  },
  "extensions": {
    "installed": ["nodejs", "python", "docker"],
    "versions": {
      "nodejs": "1.2.0",
      "python": "3.1.0"
    }
  }
}
```

### Compression

| Setting             | Value                    |
| ------------------- | ------------------------ |
| Algorithm           | gzip                     |
| Default Level       | 6 (balanced speed/ratio) |
| Checksum            | SHA256                   |
| Performance Target  | 1GB workspace < 1 minute |
| Typical Compression | 50-70% size reduction    |

---

## Always Excluded Patterns

These patterns are ALWAYS excluded from backups, regardless of profile:

```
.cache/**
.local/share/mise/installs/**
.local/state/mise/**
.local/state/**
workspace/.system/logs/**
**/node_modules/**
**/.venv/**
**/__pycache__/**
**/target/debug/**
**/target/release/**
**/.next/**
**/.gradle/**
**/.turbo/**
**/dist
**/build
```

---

## System Markers (Never Restored)

These files are NEVER restored from backups to protect initialization:

| Marker                             | Purpose                        |
| ---------------------------------- | ------------------------------ |
| `.initialized`                     | First-boot initialization flag |
| `.welcome_shown`                   | Welcome message display flag   |
| `workspace/.system/bootstrap.yaml` | Extension auto-install state   |
| `workspace/.system/installed`      | Installation marker directory  |
| `workspace/.system/install-status` | Installation status tracking   |

---

## Restore Pipeline

The V3 restore system uses a 5-stage pipeline:

### Stage 1: Validation

- Verify backup file exists and is readable
- Check sufficient disk space (2x backup size recommended)
- Validate tarball integrity
- Extract and validate manifest

### Stage 2: Analysis

- Parse backup manifest
- Count files and total size
- Check version compatibility
- Identify potential conflicts

### Stage 3: Pre-Restore Snapshot

- Create snapshot of current workspace
- Store in `~/.sindri/restore-snapshots/`
- Enable atomic rollback on failure

### Stage 4: Restore Execution

- Apply mode-specific file handling
- Skip system markers automatically
- Track all changes for rollback

### Stage 5: Commit

- Finalize restore transaction
- Clean up temporary files
- Report results

### Automatic Rollback

If restore fails at any stage, the system automatically:

1. Reverts all file changes in reverse order
2. Restores backed-up files (merge mode)
3. Removes newly created files
4. Reports failure with rollback status

---

## Version Compatibility

The restore system validates version compatibility:

| Scenario                 | Behavior                         |
| ------------------------ | -------------------------------- |
| Same major version       | Compatible, restore proceeds     |
| Minor version difference | Compatible with auto-upgrade     |
| Major version mismatch   | Incompatible, requires `--force` |
| Unknown format           | Validation error                 |

```bash
# Force restore despite version mismatch
sindri restore ./backup.tar.gz --force
```

---

## Migration Workflow

### Step 1: Backup from Old Instance

```bash
# Create user-data backup (recommended for migration)
sindri backup --profile user-data --output ./migration-backup.tar.gz
```

### Step 2: Deploy New Instance

```bash
# Create and deploy new Sindri instance
sindri deploy
```

### Step 3: Restore to New Instance

```bash
# Safe restore (won't conflict with new instance setup)
sindri restore ./migration-backup.tar.gz

# Verify
sindri connect
ls ~/workspace/projects/
```

### Step 4: Extensions Re-install Automatically

The new instance will auto-install extensions based on `sindri.yaml`. Your user data is preserved, tools are fresh.

---

## Security Considerations

1. **Encrypt sensitive backups**:

   ```bash
   sindri backup --output - | gpg --symmetric > backup.tar.gz.gpg
   ```

2. **Never commit backups to git**

3. **Verify backup integrity before restore**:

   ```bash
   tar tzf backup.tar.gz > /dev/null && echo "OK"
   ```

4. **Exclude secrets when sharing backups**:

   ```bash
   sindri backup --exclude '.ssh' --exclude '.env*'
   ```

---

## Troubleshooting

### Restore Fails: Permission Denied

```bash
# Fix ownership after restore
sudo chown -R developer:developer /alt/home/developer
```

### Extensions Not Working After Restore

This happens if `bootstrap.yaml` was accidentally restored (should not happen in V3).

```bash
# Remove bootstrap marker to trigger re-install
rm ~/workspace/.system/bootstrap.yaml
rm -rf ~/workspace/.system/installed/

# Restart container or re-deploy
```

### Shell Environment Broken

This happens if `.initialized` was restored or `.bashrc` is corrupt.

```bash
# Reset initialization (will recreate shell configs)
rm ~/.initialized
# Then restart container
```

### Backup Too Large

```bash
# Use user-data profile
sindri backup --profile user-data --output ./backup.tar.gz

# Or add exclusions
sindri backup --exclude '**/node_modules' --exclude '**/.venv'
```

### Dry Run Shows Unexpected Files

```bash
# Check what's in the backup
tar tzf backup.tar.gz | head -50

# Check with verbose dry-run
sindri restore backup.tar.gz --dry-run --verbose
```

### Rollback After Failed Restore

The system automatically rolls back on failure. If manual intervention is needed:

```bash
# Find the snapshot
ls ~/.sindri/restore-snapshots/

# Manually restore from snapshot
cd /alt/home/developer
tar xzf ~/.sindri/restore-snapshots/snapshot-{id}.tar.gz
```

---

## Current Limitations

The following features are planned but not yet implemented in V3:

| Feature                    | Status  | Notes                                |
| -------------------------- | ------- | ------------------------------------ |
| S3 Backup Destination      | WIP     | Local backups fully supported        |
| HTTPS Download Source      | WIP     | Local restore fully supported        |
| GPG Encryption Integration | Planned | Use external gpg for now             |
| Incremental Backups        | Planned | Full backups only                    |
| Extension Reinstallation   | Planned | Manual reinstall after restore       |
| Shell Config Merging       | Partial | Basic overwrite, smart merge planned |

---

## Related Documentation

- [ADR-017: Backup System Architecture](architecture/adr/017-backup-system-architecture.md)
- [ADR-018: Restore System Architecture](architecture/adr/018-restore-system-architecture.md)
- [Getting Started](getting-started.md)

---

## API Reference (Rust)

For programmatic access, the `sindri-backup` crate exports:

```rust
use sindri_backup::{
    // Backup
    ArchiveBuilder,
    ArchiveConfig,
    BackupProfile,
    BackupResult,

    // Restore
    RestoreManager,
    RestoreMode,
    RestoreOptions,
    RestoreResult,

    // Utilities
    BackupManifest,
    SourceInfo,
    ExclusionConfig,
    RestoreFilter,

    // Constants
    ALWAYS_EXCLUDE,
    NEVER_RESTORE,
    MANIFEST_FILENAME,
    MANIFEST_VERSION,
};
```

### Example: Programmatic Backup

```rust
use sindri_backup::{ArchiveBuilder, ArchiveConfig, BackupProfile, SourceInfo};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let source_info = SourceInfo {
        instance_name: "my-sindri".to_string(),
        provider: "docker".to_string(),
        hostname: "localhost".to_string(),
    };

    let config = ArchiveConfig::new(BackupProfile::Standard, source_info)?
        .with_compression_level(6)
        .with_progress(true);

    let builder = ArchiveBuilder::new(config);
    let result = builder.create(
        Path::new("/alt/home/developer"),
        Path::new("backup.tar.gz"),
    ).await?;

    println!("Backup created: {} files, {} bytes",
             result.file_count, result.size_bytes);
    Ok(())
}
```

### Example: Programmatic Restore

```rust
use sindri_backup::{RestoreManager, RestoreMode, RestoreOptions};
use camino::Utf8Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manager = RestoreManager::new(RestoreMode::Safe);

    let options = RestoreOptions {
        mode: RestoreMode::Safe,
        dry_run: false,
        interactive: true,
        force: false,
        validate_extensions: true,
        auto_upgrade_extensions: false,
    };

    let result = manager.restore(
        Utf8Path::new("backup.tar.gz"),
        Utf8Path::new("/alt/home/developer"),
        options,
    ).await?;

    println!("Restored: {} files, Skipped: {} files",
             result.restored, result.skipped);
    Ok(())
}
```
