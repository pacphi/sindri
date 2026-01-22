# ADR 017: Backup System Architecture

**Status**: Accepted
**Date**: 2026-01-21
**Related**: [ADR-001: Workspace Architecture](001-rust-migration-workspace-architecture.md)

## Context

Sindri workspaces contain critical user data requiring reliable backup: development projects, configuration, extension state, secrets, and tool installations. The bash implementation (`cli/backup-restore`, 1,101 lines) provides three backup profiles (user-data, standard, full), three restore modes (safe, merge, full), smart exclusion of regenerable files, system marker protection, and S3 support.

### Key Challenges

- **Data categorization**: User data vs config vs regenerable state vs system markers
- **Large workspace sizes**: 1-50GB depending on profile
- **Performance**: Target <5 minutes for typical workspace
- **Security**: Should backups include secrets? Encryption options?
- **Cross-provider compatibility**: Backups portable between providers

## Decision

### Backup Architecture: Profile-Based tar.gz Streaming

Three profiles with distinct inclusion/exclusion patterns:

**1. user-data Profile** (Migration-Focused, 100MB-1GB):
- Includes: `workspace/projects/`, `workspace/config/`, `.claude/`, `.gitconfig`, SSH host keys
- Excludes: Shell configs, `.config/`, `.local/`, system markers, caches
- Use case: Migrating to new provider, switching Sindri versions

**2. standard Profile** (Default, 1-5GB):
- Includes: All from user-data + `.bashrc`, `.profile`, `.config/`, `.local/bin/`
- Excludes: Mise shims/installs, tool state
- Use case: Regular backups, disaster recovery on same provider

**3. full Profile** (Disaster Recovery, 5-20GB):
- Includes: Everything except caches and mise installs
- Excludes: `.cache/`, `.local/share/mise/installs/`, `.local/state/`
- Use case: Complete disaster recovery, forensic analysis

### Archive Structure

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

```json
{
  "version": "1.0.0",
  "backup_type": "full",
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
    "value": "abc123..."
  },
  "statistics": {
    "files_included": 1234,
    "total_size_bytes": 5242880,
    "compressed_size_bytes": 2621440,
    "compression_ratio": 0.50
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

### Always Excluded Patterns

```rust
pub const ALWAYS_EXCLUDE: &[&str] = &[
    ".cache",
    ".local/share/mise/installs",
    ".local/state/mise",
    "**/node_modules",
    "**/.venv",
    "**/__pycache__",
    "**/target/debug",
    "**/target/release",
    "**/.next",
    "**/.gradle",
];
```

### Never Restore System Markers

```rust
pub const NEVER_RESTORE: &[&str] = &[
    ".initialized",
    ".welcome_shown",
    "workspace/.system/bootstrap.yaml",
    "workspace/.system/installed",
    "workspace/.system/install-status",
];
```

**Rationale**: Restoring these breaks entrypoint initialization flow.

### Compression Strategy

- **Algorithm**: gzip (level 6 - balanced speed/ratio)
- **Rationale**: Widely supported, 50-70% compression, streaming-capable
- **Performance**: 1GB workspace < 1 minute, 5GB workspace < 5 minutes

### Type Definitions

```rust
pub enum BackupProfile {
    UserData,   // Smallest backup for migration
    Standard,   // Default balanced backup
    Full,       // Complete disaster recovery
}

pub struct BackupResult {
    pub archive_path: PathBuf,
    pub size_bytes: u64,
    pub file_count: usize,
    pub profile: BackupProfile,
    pub created_at: DateTime<Utc>,
}
```

### CLI Interface

```bash
# Basic backup
sindri backup --profile standard --output ./backup.tar.gz

# Dry-run to preview
sindri backup --dry-run

# With exclusions
sindri backup --exclude "**/node_modules" --exclude ".venv"

# Exclude secrets
sindri backup --exclude-secrets

# Encrypt with GPG
sindri backup --encrypt --gpg-recipient user@example.com
```

## Consequences

### Positive

1. **Profile-based flexibility**: Three profiles cover migration, regular backup, and disaster recovery
2. **System marker protection**: Prevents critical initialization bugs
3. **Streaming architecture**: Handles large workspaces without memory issues
4. **Integrity verification**: Checksums ensure backup reliability
5. **Cloud storage support**: S3 integration for remote backups
6. **Portable backups**: Works across providers (with caveats for full mode)

### Negative

1. **Complexity**: More complex than bash implementation
2. **Dependencies**: AWS SDK adds ~50 dependencies
3. **No incremental**: Phase 5 doesn't implement incremental backups
4. **Full mode risk**: Full restore to different provider can break environment

## Implementation

**Crate**: `sindri-backup` (new library crate)

**Modules**:
- `profile.rs`: Backup profile definitions
- `archive.rs`: tar + gzip operations
- `backup/`: Backup orchestration (local, docker, fly)
- `compression.rs`: Streaming compression
- `progress.rs`: Progress reporting

**Dependencies**:
```toml
tar = "0.4"
flate2 = "1.0"
aws-sdk-s3 = "1.10"
sha2 = "0.10"
walkdir = "2.4"
```

## Related Decisions

- [ADR-001: Workspace Architecture](001-rust-migration-workspace-architecture.md)
- [ADR-012: Registry and Manifest](012-registry-manifest-dual-state-architecture.md)
