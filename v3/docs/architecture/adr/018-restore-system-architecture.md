
# ADR 018: Restore System Architecture

**Status**: Accepted
**Date**: 2026-01-21
**Related**: [ADR-017: Backup System](017-backup-system-architecture.md), [Backup Guide](../../BACKUP_RESTORE.md)

## Context

The restore system must safely extract backups while protecting system integrity. The bash implementation provides 3 restore modes with system marker protection and atomic rollback capabilities.

### Key Requirements

1. **Backup Validation**: Integrity checks before restore
2. **Pre-Restore Snapshot**: Safety rollback capability
3. **Atomic Operations**: All-or-nothing restore
4. **System Marker Protection**: Never restore `.initialized`, `bootstrap.yaml`
5. **Conflict Resolution**: Smart handling of existing files
6. **Extension Reinstallation**: Trigger after restore
7. **Cross-Version Compatibility**: Handle v3.0 → v3.1 backups

## Decision

### Restore Pipeline

```
Stage 1: Validation
  ├─ Backup integrity (tarball validity)
  ├─ Metadata extraction
  ├─ Compatibility check
  └─ Content analysis

Stage 2: Pre-Restore Snapshot
  ├─ Create workspace snapshot
  ├─ Store in ~/.sindri/restore-snapshots/
  └─ Enable atomic rollback

Stage 3: Conflict Detection
  ├─ Scan existing files
  ├─ Categorize conflicts
  └─ Generate conflict report

Stage 4: Restore Execution (Mode-Dependent)
  ├─ Safe Mode: Skip existing, restore missing
  ├─ Merge Mode: Backup existing (.bak), restore all
  └─ Full Mode: Overwrite all (except system markers)

Stage 5: Post-Restore Actions
  ├─ Extension reinstallation
  ├─ Ownership/permission fixes
  ├─ Validation checks
  └─ Cleanup snapshot (on success)
```

### Three Restore Modes

**1. safe Mode** (Default):
- Never overwrite existing files
- System markers: Never restored
- Existing files: Skipped (preserved)
- Missing files: Restored
- Use case: Migration to new instance

**2. merge Mode**:
- System markers: Never restored
- Existing files: Backed up to `.bak`, then restored
- Shell configs: Intelligently merged
- Creates rollback capability
- Use case: Updating existing instance

**3. full Mode**:
- System markers: Optionally restored (with warning)
- All files: Overwritten
- No conflict preservation
- Use case: Disaster recovery on SAME instance only

### Type System

```rust
pub enum RestoreMode {
    Safe,    // Never overwrite
    Merge,   // Backup then overwrite
    Full,    // Overwrite (except system markers)
}

pub struct RestoreOptions {
    pub mode: RestoreMode,
    pub dry_run: bool,
    pub interactive: bool,
    pub force: bool,
    pub validate_extensions: bool,
    pub auto_upgrade_extensions: bool,
}

pub struct RestoreResult {
    pub restored: usize,
    pub skipped: usize,
    pub backed_up: usize,
    pub duration: Duration,
}

pub enum RestoreState {
    Validating,
    CreatingSnapshot,
    Extracting { progress: f32 },
    Applying { files_processed: usize, total: usize },
    ReinstallingExtensions { current: String, total: usize },
    RollingBack { reason: String },
    Complete { duration: Duration },
    Failed { error: String, snapshot_id: Option<String> },
}
```

### Atomic Restore with Rollback

```rust
pub struct RestoreTransaction {
    snapshot_id: String,
    snapshot_path: PathBuf,
    changes: Vec<RestoreChange>,
}

pub enum RestoreChange {
    FileCreated { path: PathBuf },
    FileModified { path: PathBuf, backup: PathBuf },
    FileDeleted { path: PathBuf },
    DirectoryCreated { path: PathBuf },
}

impl RestoreTransaction {
    pub async fn rollback(&mut self) -> Result<()> {
        // Revert all changes in reverse order
        for change in self.changes.iter().rev() {
            match change {
                RestoreChange::FileCreated { path } => {
                    fs::remove_file(path).await?;
                }
                RestoreChange::FileModified { path, backup } => {
                    fs::rename(backup, path).await?;
                }
                // ... handle all change types
            }
        }
        Ok(())
    }
}
```

### System Marker Protection

```rust
pub const NEVER_RESTORE: &[&str] = &[
    ".initialized",
    ".welcome_shown",
    "workspace/.system/bootstrap.yaml",
    "workspace/.system/installed",
    "workspace/.system/install-status",
];

pub fn is_system_marker(path: &Path) -> bool {
    NEVER_RESTORE.iter().any(|marker| {
        path.starts_with(marker) || path == Path::new(marker)
    })
}
```

### Version Compatibility

```rust
pub struct VersionCompatibility {
    pub backup_version: semver::Version,
    pub current_version: semver::Version,
    pub compatible: bool,
    pub issues: Vec<CompatibilityIssue>,
}

pub enum CompatibilityIssue {
    MajorVersionMismatch { backup: u64, current: u64 },
    ExtensionFormatChanged { old_format: String, new_format: String },
    MissingExtension { name: String, required_version: String },
    IncompatibleProvider { backup_provider: String },
}

impl VersionCompatibility {
    pub fn can_auto_upgrade(&self) -> bool {
        // Allow minor/patch version differences
        // Reject major version mismatches
        self.backup_version.major == self.current_version.major
    }
}
```

### Safety Mechanisms

**Pre-Flight Checks**:
```rust
pub fn validate_restore_preconditions(backup: &Path, options: &RestoreOptions) -> Result<()> {
    // Check backup file exists and is readable
    ensure!(backup.exists(), "Backup file not found");

    // Check sufficient disk space
    let backup_size = fs::metadata(backup)?.len();
    let available = get_available_space()?;
    ensure!(available > backup_size * 2, "Insufficient disk space");

    // Validate tarball integrity
    validate_tarball(backup)?;

    // Parse and validate metadata
    let metadata = extract_metadata(backup)?;
    validate_compatibility(&metadata)?;

    Ok(())
}
```

**Rollback Capability**:
```rust
pub async fn restore_with_rollback(
    backup: &Path,
    options: &RestoreOptions,
) -> Result<RestoreResult> {
    let mut transaction = RestoreTransaction::begin().await?;

    match perform_restore(backup, options, &mut transaction).await {
        Ok(result) => {
            transaction.commit().await?;
            Ok(result)
        }
        Err(e) => {
            eprintln!("Restore failed: {}", e);
            eprintln!("Rolling back changes...");
            transaction.rollback().await?;
            Err(e)
        }
    }
}
```

### CLI Interface

```bash
# Basic restore (safe mode)
sindri restore backup.tar.gz

# Dry-run to preview
sindri restore backup.tar.gz --dry-run

# Merge mode with existing
sindri restore backup.tar.gz --mode merge

# Full restore (requires confirmation)
sindri restore backup.tar.gz --mode full

# Restore from S3
sindri restore s3://bucket/backup.tar.gz

# Force without confirmation (CI use)
sindri restore backup.tar.gz --mode full --force --no-interactive

# Auto-upgrade extensions if versions differ
sindri restore backup.tar.gz --auto-upgrade-extensions
```

### Edge Cases

**1. Backup from Different Provider**:
- Solution: Provider-agnostic backup format, only user files
- Note: Provider-specific configs excluded by default

**2. Incompatible Extensions**:
- Solution: Warn user, skip extension reinstallation, restore user data anyway

**3. Partial Restore Failure**:
- Solution: Automatic rollback to pre-restore snapshot

**4. Encrypted Backup**:
- Solution: Detect encryption, prompt for decryption, decrypt to temp

**5. Concurrent Restore**:
- Solution: Lock file prevents multiple restores simultaneously

**6. Corrupted Backup**:
- Solution: Validate tarball integrity, check checksum, clear error message

## Consequences

### Positive

1. **Safe by default**: Safe mode prevents data loss
2. **Atomic operations**: All-or-nothing with rollback
3. **System protection**: System markers never restored
4. **Version compatibility**: Handles cross-version restores
5. **Comprehensive validation**: Multiple pre-flight checks
6. **Clear feedback**: Dry-run and progress indicators

### Negative

1. **Complexity**: More complex than bash implementation
2. **Snapshot overhead**: Pre-restore snapshot uses disk space
3. **Performance**: Validation adds latency before restore

## Implementation

**Crate**: `sindri-backup` (same as backup system)

**Modules**:
- `restore/mod.rs`: Restore orchestration
- `restore/modes.rs`: Safe/merge/full modes
- `restore/analysis.rs`: Backup analysis
- `restore/markers.rs`: System marker protection

**Testing**:
- Unit tests for each mode
- Integration tests with real archives
- Rollback scenario tests
- Cross-version compatibility tests

## Related Decisions

- [ADR-017: Backup System](017-backup-system-architecture.md)
- [ADR-001: Workspace Architecture](001-rust-migration-workspace-architecture.md)
- [Backup Guide](../../BACKUP_RESTORE.md)
