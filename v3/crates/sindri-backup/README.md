# sindri-backup

Backup and restore system for Sindri workspaces. Supports three backup profiles with streaming tar+gzip compression and SHA256 integrity checksums.

## Features

- Three backup profiles: user-data (migration), standard (default), full (disaster recovery)
- Streaming tar+gzip compression for efficient handling of large workspaces
- SHA256 checksum verification for backup integrity
- Smart exclusion filters (caches, build artifacts, node_modules)
- System marker protection (never backs up or restores initialization markers)
- JSON manifest format with metadata and statistics
- Progress reporting for long-running operations

## Modules

- `archive` - Archive creation with `ArchiveBuilder` and `ArchiveConfig`
- `compression` - Gzip compression and SHA256 checksum calculation
- `filters` - Exclusion rules (`ALWAYS_EXCLUDE`) and restore guards (`NEVER_RESTORE`)
- `manifest` - JSON backup manifest with source info and statistics
- `profile` - Backup profile definitions (user-data, standard, full)
- `progress` - Progress bars and spinners for backup/restore operations
- `restore` - Archive extraction and restore logic

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-backup = { path = "../sindri-backup" }
```

## Part of [Sindri](../../)
