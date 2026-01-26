# Backup and Restore Guide

This guide covers how to safely backup, download, and restore your Sindri workspace.

## Overview

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
./v2/cli/sindri backup --output ./backups/

# User data only (smallest, safest for migration)
./v2/cli/sindri backup --profile user-data --output ./backups/

# Full backup (disaster recovery)
./v2/cli/sindri backup --profile full --output ./backups/
```

### Restore to New Instance

```bash
# Safe restore (default - won't overwrite existing files)
./v2/cli/sindri restore ./backups/sindri-backup-2026-01-15.tar.gz

# Preview what will happen
./v2/cli/sindri restore ./backups/backup.tar.gz --dry-run

# Merge with existing (backs up conflicts first)
./v2/cli/sindri restore ./backups/backup.tar.gz --mode merge
```

---

## Backup Profiles

### `user-data` Profile (Recommended for Migration)

**Size**: Small (100MB - 1GB typical)
**Use case**: Moving to a new Sindri instance

Includes:

- `workspace/projects/` - All projects
- `workspace/config/` - User configs
- `workspace/scripts/` - User scripts
- `workspace/bin/` - User binaries
- `.claude/` - Claude Code settings and history
- `.ssh/host_keys/` - SSH host keys (fingerprint stability)
- `.gitconfig` - Git configuration

Excludes:

- All system markers
- All mise installations (regenerable)
- All caches
- Shell RC files (regenerated on new instance)

```bash
./v2/cli/sindri backup --profile user-data --output ./backups/
```

### `standard` Profile (Default)

**Size**: Medium (1GB - 5GB typical)
**Use case**: Regular backups, same-instance recovery

Includes everything in `user-data` plus:

- `.bashrc`, `.profile` - Shell configuration
- `.config/` - Application configs (excluding mise state)
- `.local/bin/` - Locally installed binaries

```bash
./v2/cli/sindri backup --output ./backups/
# or explicitly:
./v2/cli/sindri backup --profile standard --output ./backups/
```

### `full` Profile (Disaster Recovery)

**Size**: Large (5GB - 20GB typical)
**Use case**: Complete disaster recovery to same instance

Includes everything except:

- `.cache/` - Caches (always excluded)
- `.local/share/mise/installs/` - Can be regenerated
- `.local/state/` - Tool state

**Warning**: Restoring a full backup to a different instance may cause issues.

```bash
./v2/cli/sindri backup --profile full --output ./backups/
```

---

## Restore Modes

### `safe` Mode (Default)

The safest option - never overwrites existing files.

**Behavior**:

- System markers: Never touched
- Existing files: Skipped (preserved)
- Missing files: Restored
- Conflicts: Reported but not modified

```bash
./v2/cli/sindri restore ./backup.tar.gz
# or explicitly:
./v2/cli/sindri restore ./backup.tar.gz --mode safe
```

**Best for**: Migration to new instance, adding missing files

### `merge` Mode

Smart merge with automatic backup of conflicts.

**Behavior**:

- System markers: Never touched
- Existing files: Backed up to `.bak`, then restored
- Shell configs: Intelligently merged (preserves user sections)
- Creates rollback capability

```bash
./v2/cli/sindri restore ./backup.tar.gz --mode merge
```

**Best for**: Updating existing instance, recovering from partial data loss

### `full` Mode

Complete restore - overwrites everything.

**Behavior**:

- System markers: Optionally restored (with warning)
- All files: Overwritten
- No conflict preservation

```bash
./v2/cli/sindri restore ./backup.tar.gz --mode full
```

**Warning**: Only use for disaster recovery on the SAME instance. Using on a different instance may break initialization.

---

## CLI Reference

### Backup Command

```bash
./v2/cli/sindri backup [options]

Options:
  -o, --output <path>     Output location (directory or file path)
                          Default: ./sindri-backup-{name}-{timestamp}.tar.gz
  -p, --profile <name>    Backup profile: user-data, standard (default), full
  -c, --config <file>     Sindri config file (default: sindri.yaml)
  --exclude <pattern>     Additional exclude pattern (can repeat)
  --dry-run               Show what would be backed up without creating archive
  --list                  List available backups on the instance
  -v, --verbose           Verbose output

Examples:
  ./cli/sindri backup                                    # Standard backup to current dir
  ./cli/sindri backup --profile user-data -o ./backups/  # User data only
  ./cli/sindri backup --exclude 'node_modules'           # Exclude pattern
  ./cli/sindri backup --dry-run                          # Preview backup contents
```

### Restore Command

```bash
./v2/cli/sindri restore <source> [options]

Arguments:
  source                  Backup file: local path, s3://bucket/path, or https://url

Options:
  -m, --mode <name>       Restore mode: safe (default), merge, full
  -c, --config <file>     Sindri config file (default: sindri.yaml)
  --dry-run               Preview restore without making changes
  --no-interactive        Skip confirmation prompts
  -v, --verbose           Verbose output

Examples:
  ./cli/sindri restore ./backup.tar.gz                   # Safe restore
  ./cli/sindri restore ./backup.tar.gz --mode merge      # Merge with backups
  ./cli/sindri restore ./backup.tar.gz --dry-run         # Preview changes
  ./cli/sindri restore s3://bucket/backup.tar.gz         # Restore from S3
```

### List Backups Command

```bash
./v2/cli/sindri backup list [options]

Options:
  -c, --config <file>     Sindri config file (default: sindri.yaml)

Lists backups stored on the Sindri instance in ~/workspace/backups/
```

---

## Provider-Specific Instructions

### Docker Provider

#### Backup from Docker

```bash
# Via CLI (recommended)
./v2/cli/sindri backup --output ./backups/

# Manual method
docker exec sindri-docker tar czf - \
  --exclude='.cache' \
  --exclude='.local/share/mise/installs' \
  --exclude='.local/state' \
  --exclude='.initialized' \
  --exclude='workspace/.system/bootstrap.yaml' \
  --exclude='workspace/.system/installed' \
  -C /alt/home/developer . > backup.tar.gz
```

#### Restore to Docker

```bash
# Via CLI (recommended)
./v2/cli/sindri restore ./backup.tar.gz

# Manual method - stop container first
docker compose stop
docker run --rm -v sindri_dev_home:/data -v $(pwd):/backup \
  alpine sh -c "cd /data && tar xzf /backup/backup.tar.gz"
docker compose start
```

### Fly.io Provider

#### Backup from Fly.io

```bash
# Via CLI (recommended)
./v2/cli/sindri backup --output ./backups/

# Using Fly.io native snapshots (for disaster recovery)
flyctl volumes list -a my-sindri-dev
flyctl volumes snapshots create vol_abc123
```

#### Restore to Fly.io

```bash
# Via CLI (recommended)
./v2/cli/sindri restore ./backup.tar.gz

# From Fly.io snapshot (creates new volume)
flyctl volumes create workspace \
  --snapshot-id snap_abc123 \
  --region sjc --size 10 \
  -a my-sindri-dev
```

#### Manual SFTP Access

For direct SFTP access to manage backups:

```bash
# Create backup on instance
flyctl ssh console -a my-sindri-dev -C \
  "cd ~ && tar czf workspace/backups/backup-\$(date +%Y%m%d).tar.gz \
   --exclude='.cache' --exclude='.local/share/mise/installs' \
   --exclude='.initialized' --exclude='workspace/.system/bootstrap.yaml' \
   ."

# List backups
flyctl ssh console -a my-sindri-dev -C "ls -lh ~/workspace/backups/"

# Download via SFTP
flyctl ssh sftp shell -a my-sindri-dev
sftp> cd workspace/backups
sftp> get backup-20250115.tar.gz
sftp> exit

# Upload for restore
flyctl ssh sftp shell -a my-sindri-dev
sftp> put backup.tar.gz /tmp/
sftp> exit

# Extract (after upload)
flyctl ssh console -a my-sindri-dev -C \
  "cd /alt/home/developer && tar xzf /tmp/backup.tar.gz && rm /tmp/backup.tar.gz"
```

---

## S3 Backup Storage

### Configuration

```bash
# AWS credentials
export AWS_ACCESS_KEY_ID=your-key
export AWS_SECRET_ACCESS_KEY=your-secret
export AWS_DEFAULT_REGION=us-west-2

# Or use AWS CLI profile
export AWS_PROFILE=sindri-backup
```

### Backup to S3

```bash
./v2/cli/sindri backup --output s3://my-bucket/sindri-backups/
```

### Restore from S3

```bash
./v2/cli/sindri restore s3://my-bucket/sindri-backups/backup-2026-01-15.tar.gz
```

### S3-Compatible Storage

For MinIO, Wasabi, DigitalOcean Spaces:

```bash
export AWS_ENDPOINT_URL=https://s3.wasabisys.com
./v2/cli/sindri backup --output s3://my-bucket/backups/
```

---

## Migration Workflow

### Step 1: Backup from Old Instance

```bash
# Create user-data backup (recommended for migration)
./v2/cli/sindri backup --profile user-data --output ./migration-backup.tar.gz
```

### Step 2: Deploy New Instance

```bash
# Create new sindri.yaml or modify existing
./v2/cli/sindri config init
vim sindri.yaml

# Deploy
./v2/cli/sindri deploy
```

### Step 3: Restore to New Instance

```bash
# Safe restore (won't conflict with new instance setup)
./v2/cli/sindri restore ./migration-backup.tar.gz

# Verify
./v2/cli/sindri connect
ls ~/workspace/projects/
```

### Step 4: Extensions Re-install Automatically

The new instance will auto-install extensions based on `sindri.yaml`. Your user data is preserved, tools are fresh.

---

## Automated Backups

### Local Cron Job

```bash
# Add to crontab: crontab -e
# Daily backup at 2 AM
0 2 * * * cd /path/to/sindri && ./cli/sindri backup --output ~/sindri-backups/
```

### Backup Rotation Script

```bash
#!/bin/bash
# backup-rotate.sh
BACKUP_DIR="$HOME/sindri-backups"
KEEP_DAYS=7

./v2/cli/sindri backup --output "$BACKUP_DIR/"
find "$BACKUP_DIR" -name "sindri-backup-*.tar.gz" -mtime +$KEEP_DAYS -delete
```

### S3 with Lifecycle Policy

Configure S3 lifecycle rules for automatic expiration:

```json
{
  "Rules": [
    {
      "ID": "ExpireOldBackups",
      "Status": "Enabled",
      "Filter": { "Prefix": "sindri-backups/" },
      "Expiration": { "Days": 30 }
    }
  ]
}
```

---

## Troubleshooting

### Restore Fails: Permission Denied

```bash
# Fix ownership after restore
flyctl ssh console -a my-app -C \
  "sudo chown -R developer:developer /alt/home/developer"
```

### Extensions Not Working After Restore

This happens if `bootstrap.yaml` was accidentally restored.

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
./v2/cli/sindri backup --profile user-data --output ./backup.tar.gz

# Or add exclusions
./v2/cli/sindri backup --exclude 'node_modules' --exclude '.venv'
```

### Dry Run Shows Unexpected Files

```bash
# Check what's in the backup
tar tzf backup.tar.gz | head -50

# Check categories
./v2/cli/sindri restore backup.tar.gz --dry-run --verbose
```

---

## Security Considerations

1. **Encrypt sensitive backups**:

   ```bash
   ./cli/sindri backup --output - | gpg --symmetric > backup.tar.gz.gpg
   ```

2. **Never commit backups to git**

3. **Use IAM roles** for S3 instead of long-lived credentials

4. **Verify backup integrity**:

   ```bash
   tar tzf backup.tar.gz > /dev/null && echo "OK"
   ```

5. **Exclude secrets** when sharing backups:
   ```bash
   ./cli/sindri backup --exclude '.ssh' --exclude '.env*'
   ```

---

## Related Documentation

- [Architecture](ARCHITECTURE.md) - Volume structure details
- [Docker Provider](providers/DOCKER.md) - Docker-specific operations
- [Fly.io Provider](providers/FLY.md) - Fly.io-specific operations
- [Configuration](CONFIGURATION.md) - sindri.yaml reference
