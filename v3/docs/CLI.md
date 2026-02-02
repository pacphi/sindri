# Sindri V3 CLI Reference

Complete reference for the Sindri V3 command-line interface.

## Overview

Sindri is a CLI tool for managing declarative cloud development environments. V3 is a complete rewrite in Rust, providing improved performance, enhanced security features, and native container image management.

```bash
sindri [OPTIONS] <COMMAND>
```

## Installation

### From GitHub Releases

```bash
# Download the latest release
curl -fsSL https://github.com/pacphi/sindri/releases/latest/download/sindri-$(uname -s)-$(uname -m).tar.gz | tar -xz
sudo mv sindri /usr/local/bin/
```

### From Source

```bash
cd v3
cargo build --release
sudo cp target/release/sindri /usr/local/bin/
```

## Global Options

These options can be used with any command:

| Option            | Short | Description                        |
| ----------------- | ----- | ---------------------------------- |
| `--verbose`       | `-v`  | Increase verbosity (-v, -vv, -vvv) |
| `--quiet`         | `-q`  | Suppress output                    |
| `--config <PATH>` | `-c`  | Path to sindri.yaml config file    |
| `--help`          | `-h`  | Print help information             |
| `--version`       | `-V`  | Print version information          |

## Commands

### version

Show version information.

**Synopsis:**

```bash
sindri version [OPTIONS]
```

**Options:**

| Option   | Description    |
| -------- | -------------- |
| `--json` | Output as JSON |

**Examples:**

```bash
# Show version
sindri version

# Output as JSON
sindri version --json
```

---

### config

Configuration management.

#### config init

Initialize a new sindri.yaml configuration file.

**Synopsis:**

```bash
sindri config init [OPTIONS]
```

**Options:**

| Option                  | Short | Default     | Description             |
| ----------------------- | ----- | ----------- | ----------------------- |
| `--name <NAME>`         | `-n`  | -           | Project name            |
| `--provider <PROVIDER>` | `-p`  | docker      | Provider to use         |
| `--profile <PROFILE>`   | -     | -           | Extension profile       |
| `--output <PATH>`       | `-o`  | sindri.yaml | Output file path        |
| `--force`               | `-f`  | -           | Overwrite existing file |

**Examples:**

```bash
# Initialize with defaults
sindri config init

# Initialize for Kubernetes
sindri config init --provider k8s --profile kubernetes

# Force overwrite existing config
sindri config init --force --name my-project
```

#### config validate

Validate the configuration file.

**Synopsis:**

```bash
sindri config validate [OPTIONS]
```

**Options:**

| Option               | Short | Description                                     |
| -------------------- | ----- | ----------------------------------------------- |
| `--file <PATH>`      | `-f`  | Path to config file (default: find sindri.yaml) |
| `--check-extensions` | -     | Verify that configured extensions exist         |

**Examples:**

```bash
# Validate default config
sindri config validate

# Validate specific file with extension check
sindri config validate --file ./configs/dev.yaml --check-extensions
```

#### config show

Show the resolved configuration.

**Synopsis:**

```bash
sindri config show [OPTIONS]
```

**Options:**

| Option   | Description    |
| -------- | -------------- |
| `--json` | Output as JSON |

**Examples:**

```bash
# Show human-readable config
sindri config show

# Show as JSON
sindri config show --json
```

---

### deploy

Deploy the environment.

**Synopsis:**

```bash
sindri deploy [OPTIONS]
```

**Options:**

| Option                      | Short  | Default | Description                                         |
| --------------------------- | ------ | ------- | --------------------------------------------------- |
| `--force`                   | `-f`   | -       | Force recreation of environment                     |
| `--dry-run`                 | -      | -       | Show what would happen without deploying            |
| `--wait`                    | `-w`   | true    | Wait for deployment to complete                     |
| `--timeout <SECONDS>`       | `-t`   | 600     | Deployment timeout in seconds                       |
| `--skip-validation`         | -      | -       | Skip configuration validation                       |
| `--skip-image-verification` | -      | -       | Skip image signature and provenance verification    |
| `--env-file <PATH>`         | -      | -       | Path to custom .env file for secrets                |
| `--from-source`             | `--fs` | -       | Build from Sindri GitHub repository (requires push) |

**Examples:**

```bash
# Deploy with defaults (uses pre-built image)
sindri deploy

# Dry run to preview changes
sindri deploy --dry-run

# Force recreation with longer timeout
sindri deploy --force --timeout 900

# Skip image verification for local development
sindri deploy --skip-image-verification

# Use custom .env file (relative to sindri.yaml location)
sindri deploy --env-file config/production.env

# Use custom .env file (absolute path)
sindri deploy --env-file /secrets/.env

# Combine custom config and custom env file
sindri deploy --config /path/to/sindri.yaml --env-file /path/to/.env

# Build from source (requires push to GitHub first)
sindri deploy --from-source
```

**Building from Source vs Local Development:**

The `--from-source` flag clones the Sindri repository from GitHub and builds from that clone.
This is useful for CI/CD and verifying pushed code, but **requires your changes to be pushed first**.

| Goal                           | Method                          | Push Required |
| ------------------------------ | ------------------------------- | ------------- |
| Test local/uncommitted changes | `make v3-cycle-fast CONFIG=...` | No            |
| Verify pushed code works       | `sindri deploy --from-source`   | Yes           |
| CI/CD builds                   | `--from-source` with `gitRef`   | Yes           |

For local development without pushing, use the Makefile workflow instead:

```bash
make v3-cycle-fast CONFIG=sindri.yaml
```

See [MAINTAINER_GUIDE.md](MAINTAINER_GUIDE.md#two-development-paths) for the complete guide.

**Secrets Resolution:**

When you deploy, Sindri performs a preflight check to detect `.env` files:

- **Default behavior**: Looks for `.env` and `.env.local` in the same directory as `sindri.yaml`
- **Custom path**: Use `--env-file` to specify a different location
- **Priority**: shell env > `.env.local` > `.env` > `fromFile` > S3 > Vault

See [SECRETS_MANAGEMENT.md](SECRETS_MANAGEMENT.md) for complete secrets documentation.

**Exit Codes:**

| Code | Description                    |
| ---- | ------------------------------ |
| 0    | Deployment successful          |
| 1    | Configuration error            |
| 2    | Provider error                 |
| 3    | Timeout waiting for deployment |

---

### connect

Connect to a deployed environment.

**Synopsis:**

```bash
sindri connect [OPTIONS]
```

**Options:**

| Option            | Short | Description                                 |
| ----------------- | ----- | ------------------------------------------- |
| `--command <CMD>` | `-c`  | Command to run instead of interactive shell |

**Examples:**

```bash
# Connect with interactive shell
sindri connect

# Run a specific command
sindri connect -c "ls -la"

# Execute a script
sindri connect --command "python3 /app/script.py"
```

---

### status

Show deployment status.

**Synopsis:**

```bash
sindri status [OPTIONS]
```

**Options:**

| Option              | Short | Description                    |
| ------------------- | ----- | ------------------------------ |
| `--json`            | -     | Output as JSON                 |
| `--watch <SECONDS>` | `-w`  | Refresh status every N seconds |

**Examples:**

```bash
# Show status
sindri status

# Output as JSON
sindri status --json

# Watch with 5 second refresh
sindri status --watch 5
```

---

### destroy

Destroy the deployment.

**Synopsis:**

```bash
sindri destroy [OPTIONS]
```

**Options:**

| Option      | Short | Description                    |
| ----------- | ----- | ------------------------------ |
| `--force`   | `-f`  | Skip confirmation prompt       |
| `--volumes` | -     | Also remove associated volumes |

**Examples:**

```bash
# Destroy with confirmation
sindri destroy

# Force destroy without prompting
sindri destroy --force

# Destroy including volumes
sindri destroy --volumes --force
```

---

### extension

Extension management commands.

#### extension install

Install an extension.

**Synopsis:**

```bash
sindri extension install [OPTIONS] [NAME]
```

**Options:**

| Option                 | Short | Description                             |
| ---------------------- | ----- | --------------------------------------- |
| `<NAME>`               | -     | Extension name (with optional @version) |
| `--version <VERSION>`  | `-V`  | Specific version to install             |
| `--from-config <PATH>` | -     | Install extensions from sindri.yaml     |
| `--profile <NAME>`     | -     | Install all extensions from a profile   |
| `--force`              | `-f`  | Force reinstall if already installed    |
| `--no-deps`            | -     | Skip dependency installation            |
| `--yes`                | `-y`  | Skip confirmation prompt                |

**Examples:**

```bash
# Install latest version
sindri extension install mise

# Install specific version
sindri extension install mise@2024.1.0

# Install from profile
sindri extension install --profile python-data-science

# Install from config file
sindri extension install --from-config sindri.yaml
```

#### extension list

List available extensions.

**Synopsis:**

```bash
sindri extension list [OPTIONS]
```

**Options:**

| Option                  | Short | Description                    |
| ----------------------- | ----- | ------------------------------ |
| `--category <CATEGORY>` | `-c`  | Filter by category             |
| `--installed`           | -     | Show only installed extensions |
| `--json`                | -     | Output as JSON                 |

**Examples:**

```bash
# List all extensions
sindri extension list

# List installed only
sindri extension list --installed

# List by category
sindri extension list --category languages
```

#### extension validate

Validate an extension definition.

**Synopsis:**

```bash
sindri extension validate <NAME> [OPTIONS]
```

**Options:**

| Option          | Short | Description                 |
| --------------- | ----- | --------------------------- |
| `<NAME>`        | -     | Extension name or path      |
| `--file <PATH>` | `-f`  | Path to extension.yaml file |

**Examples:**

```bash
# Validate installed extension
sindri extension validate mise

# Validate local extension file
sindri extension validate my-ext --file ./extension.yaml
```

#### extension status

Show extension status.

**Synopsis:**

```bash
sindri extension status [NAME] [OPTIONS]
```

**Options:**

| Option   | Description                                 |
| -------- | ------------------------------------------- |
| `[NAME]` | Extension name (shows all if not specified) |
| `--json` | Output as JSON                              |

**Examples:**

```bash
# Show all extension status
sindri extension status

# Show specific extension
sindri extension status mise --json
```

#### extension info

Show detailed extension information.

**Synopsis:**

```bash
sindri extension info <NAME> [OPTIONS]
```

**Options:**

| Option   | Description    |
| -------- | -------------- |
| `<NAME>` | Extension name |
| `--json` | Output as JSON |

**Examples:**

```bash
# Show extension info
sindri extension info claude-code

# Output as JSON
sindri extension info mise --json
```

#### extension upgrade

Upgrade an installed extension.

**Synopsis:**

```bash
sindri extension upgrade <NAME> [OPTIONS]
```

**Options:**

| Option                | Short | Description              |
| --------------------- | ----- | ------------------------ |
| `<NAME>`              | -     | Extension name           |
| `--version <VERSION>` | `-v`  | Target version           |
| `--yes`               | `-y`  | Skip confirmation prompt |

**Examples:**

```bash
# Upgrade to latest
sindri extension upgrade mise

# Upgrade to specific version
sindri extension upgrade mise --version 2024.2.0 -y
```

#### extension remove

Remove an installed extension.

**Synopsis:**

```bash
sindri extension remove <NAME> [OPTIONS]
```

**Options:**

| Option    | Short | Description                                         |
| --------- | ----- | --------------------------------------------------- |
| `<NAME>`  | -     | Extension name                                      |
| `--yes`   | `-y`  | Skip confirmation prompt                            |
| `--force` | `-f`  | Force removal even if other extensions depend on it |

**Examples:**

```bash
# Remove with confirmation
sindri extension remove my-extension

# Force remove without prompts
sindri extension remove my-extension --force -y
```

#### extension versions

Show available versions for an extension.

**Synopsis:**

```bash
sindri extension versions <NAME> [OPTIONS]
```

**Options:**

| Option   | Description    |
| -------- | -------------- |
| `<NAME>` | Extension name |
| `--json` | Output as JSON |

**Examples:**

```bash
# List versions
sindri extension versions mise

# Output as JSON
sindri extension versions mise --json
```

#### extension check

Check for extension updates.

**Synopsis:**

```bash
sindri extension check [EXTENSIONS...] [OPTIONS]
```

**Options:**

| Option            | Description                                         |
| ----------------- | --------------------------------------------------- |
| `[EXTENSIONS...]` | Specific extensions to check (all if not specified) |
| `--json`          | Output as JSON                                      |

**Examples:**

```bash
# Check all extensions
sindri extension check

# Check specific extensions
sindri extension check mise claude-code --json
```

#### extension rollback

Rollback to previous extension version.

**Synopsis:**

```bash
sindri extension rollback <NAME> [OPTIONS]
```

**Options:**

| Option   | Short | Description              |
| -------- | ----- | ------------------------ |
| `<NAME>` | -     | Extension name           |
| `--yes`  | `-y`  | Skip confirmation prompt |

**Examples:**

```bash
# Rollback to previous version
sindri extension rollback mise

# Rollback without prompting
sindri extension rollback mise -y
```

---

### profile

Extension profile management.

#### profile list

List available profiles.

**Synopsis:**

```bash
sindri profile list [OPTIONS]
```

**Options:**

| Option   | Description    |
| -------- | -------------- |
| `--json` | Output as JSON |

**Examples:**

```bash
sindri profile list
sindri profile list --json
```

#### profile install

Install all extensions in a profile.

**Synopsis:**

```bash
sindri profile install <PROFILE> [OPTIONS]
```

**Options:**

| Option                | Short | Default | Description                                       |
| --------------------- | ----- | ------- | ------------------------------------------------- |
| `<PROFILE>`           | -     | -       | Profile name                                      |
| `--yes`               | `-y`  | -       | Skip confirmation prompt                          |
| `--continue-on-error` | -     | true    | Continue installing other extensions if one fails |

**Examples:**

```bash
# Install python-data-science profile
sindri profile install python-data-science

# Install without prompts
sindri profile install kubernetes -y
```

#### profile reinstall

Reinstall all extensions in a profile.

**Synopsis:**

```bash
sindri profile reinstall <PROFILE> [OPTIONS]
```

**Options:**

| Option      | Short | Description              |
| ----------- | ----- | ------------------------ |
| `<PROFILE>` | -     | Profile name             |
| `--yes`     | `-y`  | Skip confirmation prompt |

**Examples:**

```bash
sindri profile reinstall python-data-science -y
```

#### profile info

Show profile information.

**Synopsis:**

```bash
sindri profile info <PROFILE> [OPTIONS]
```

**Options:**

| Option      | Description    |
| ----------- | -------------- |
| `<PROFILE>` | Profile name   |
| `--json`    | Output as JSON |

**Examples:**

```bash
sindri profile info kubernetes
sindri profile info python-data-science --json
```

#### profile status

Check profile installation status.

**Synopsis:**

```bash
sindri profile status <PROFILE> [OPTIONS]
```

**Options:**

| Option      | Description    |
| ----------- | -------------- |
| `<PROFILE>` | Profile name   |
| `--json`    | Output as JSON |

**Examples:**

```bash
sindri profile status kubernetes
```

---

### secrets

Secrets management commands.

#### secrets validate

Validate all secrets are accessible.

**Synopsis:**

```bash
sindri secrets validate [OPTIONS]
```

**Options:**

| Option          | Description                          |
| --------------- | ------------------------------------ |
| `--show-values` | Show actual secret values (CAUTION!) |

**Examples:**

```bash
# Validate secrets
sindri secrets validate

# Show values (use with caution)
sindri secrets validate --show-values
```

#### secrets list

List all configured secrets.

**Synopsis:**

```bash
sindri secrets list [OPTIONS]
```

**Options:**

| Option              | Description                             |
| ------------------- | --------------------------------------- |
| `--json`            | Output as JSON                          |
| `--source <SOURCE>` | Filter by source (env, file, vault, s3) |

**Examples:**

```bash
# List all secrets
sindri secrets list

# List only S3 secrets
sindri secrets list --source s3 --json
```

#### secrets test-vault

Test Vault connectivity.

**Synopsis:**

```bash
sindri secrets test-vault [OPTIONS]
```

**Options:**

| Option            | Description          |
| ----------------- | -------------------- |
| `--address <URL>` | Vault server address |
| `--token <TOKEN>` | Vault token          |
| `--json`          | Output as JSON       |

**Examples:**

```bash
sindri secrets test-vault --address https://vault.example.com
```

#### secrets encode-file

Encode a file as base64 for secret storage.

**Synopsis:**

```bash
sindri secrets encode-file <FILE> [OPTIONS]
```

**Options:**

| Option            | Short | Description                           |
| ----------------- | ----- | ------------------------------------- |
| `<FILE>`          | -     | File to encode                        |
| `--output <PATH>` | `-o`  | Output file (stdout if not specified) |
| `--newline`       | -     | Add trailing newline                  |

**Examples:**

```bash
# Encode to stdout
sindri secrets encode-file ./credentials.json

# Encode to file
sindri secrets encode-file ./key.pem --output encoded-key.txt
```

#### secrets s3

S3 encrypted storage commands.

##### secrets s3 init

Initialize S3 backend for encrypted secret storage.

**Synopsis:**

```bash
sindri secrets s3 init [OPTIONS]
```

**Options:**

| Option              | Description                                 |
| ------------------- | ------------------------------------------- |
| `--bucket <NAME>`   | S3 bucket name (required)                   |
| `--region <REGION>` | AWS region (required)                       |
| `--endpoint <URL>`  | Custom S3-compatible endpoint (e.g., MinIO) |
| `--key-file <PATH>` | Master key path                             |
| `--create-bucket`   | Create bucket if it doesn't exist           |
| `--output <PATH>`   | Output configuration to file                |

**Examples:**

```bash
# Initialize S3 backend
sindri secrets s3 init --bucket my-secrets --region us-east-1 --create-bucket

# With custom MinIO endpoint
sindri secrets s3 init --bucket secrets --region us-east-1 \
  --endpoint https://minio.example.com --create-bucket
```

##### secrets s3 push

Push a secret to S3.

**Synopsis:**

```bash
sindri secrets s3 push <NAME> [OPTIONS]
```

**Options:**

| Option               | Short | Description                |
| -------------------- | ----- | -------------------------- |
| `<NAME>`             | -     | Secret name                |
| `--value <VALUE>`    | -     | Secret value               |
| `--from-file <PATH>` | -     | Read value from file       |
| `--stdin`            | -     | Read value from stdin      |
| `--s3-path <PATH>`   | -     | Custom S3 path             |
| `--bucket <NAME>`    | -     | Override configured bucket |
| `--region <REGION>`  | -     | Override configured region |
| `--key-file <PATH>`  | -     | Master key file path       |
| `--force`            | `-f`  | Overwrite existing secret  |

**Examples:**

```bash
# Push with value
sindri secrets s3 push DATABASE_URL --value "postgres://..."

# Push from file
sindri secrets s3 push SSH_KEY --from-file ~/.ssh/id_rsa

# Push from stdin
echo "secret-value" | sindri secrets s3 push MY_SECRET --stdin
```

##### secrets s3 pull

Pull a secret from S3.

**Synopsis:**

```bash
sindri secrets s3 pull <NAME> [OPTIONS]
```

**Options:**

| Option              | Short | Description                           |
| ------------------- | ----- | ------------------------------------- |
| `<NAME>`            | -     | Secret name or S3 path                |
| `--output <PATH>`   | `-o`  | Write to file                         |
| `--export`          | -     | Output as environment variable format |
| `--bucket <NAME>`   | -     | Override configured bucket            |
| `--region <REGION>` | -     | Override configured region            |
| `--key-file <PATH>` | -     | Master key file path                  |
| `--show`            | -     | Display secret value                  |

**Examples:**

```bash
# Pull and show value
sindri secrets s3 pull DATABASE_URL --show

# Pull to file
sindri secrets s3 pull SSH_KEY --output ./key.pem

# Export format
sindri secrets s3 pull API_KEY --export
# Output: export API_KEY='value'
```

##### secrets s3 sync

Synchronize secrets with S3.

**Synopsis:**

```bash
sindri secrets s3 sync [OPTIONS]
```

**Options:**

| Option              | Default | Description                               |
| ------------------- | ------- | ----------------------------------------- |
| `--dry-run`         | -       | Show what would be synced                 |
| `--direction <DIR>` | both    | Sync direction: push, pull, both          |
| `--delete-remote`   | -       | Delete remote secrets not in local config |
| `--bucket <NAME>`   | -       | Override configured bucket                |
| `--region <REGION>` | -       | Override configured region                |
| `--key-file <PATH>` | -       | Master key file path                      |

**Examples:**

```bash
# Dry run sync
sindri secrets s3 sync --dry-run

# Push only
sindri secrets s3 sync --direction push

# Bidirectional sync
sindri secrets s3 sync --direction both
```

##### secrets s3 keygen

Generate a new master encryption key.

**Synopsis:**

```bash
sindri secrets s3 keygen [OPTIONS]
```

**Options:**

| Option            | Short | Default            | Description                 |
| ----------------- | ----- | ------------------ | --------------------------- |
| `--output <PATH>` | `-o`  | .sindri-master.key | Output key file path        |
| `--force`         | `-f`  | -                  | Overwrite existing key file |

**Examples:**

```bash
# Generate key with default path
sindri secrets s3 keygen

# Generate to custom path
sindri secrets s3 keygen --output ./keys/master.key
```

**Security Notes:**

- Uses age X25519 encryption
- Key file should be added to .gitignore
- Keep key backed up securely

##### secrets s3 rotate

Rotate master encryption key.

**Synopsis:**

```bash
sindri secrets s3 rotate [OPTIONS]
```

**Options:**

| Option              | Short | Description                              |
| ------------------- | ----- | ---------------------------------------- |
| `--new-key <PATH>`  | -     | New master key path (required)           |
| `--old-key <PATH>`  | -     | Old master key path (defaults to config) |
| `--add-only`        | -     | Only add new key, don't remove old       |
| `--bucket <NAME>`   | -     | Override configured bucket               |
| `--region <REGION>` | -     | Override configured region               |
| `--yes`             | `-y`  | Skip confirmation prompt                 |

**Examples:**

```bash
# Rotate key
sindri secrets s3 rotate --new-key ./new-master.key

# Add new key without removing old (dual-key support)
sindri secrets s3 rotate --new-key ./new.key --add-only
```

---

### backup

Backup workspace to a tar.gz archive.

**Synopsis:**

```bash
sindri backup [OPTIONS]
```

**Options:**

| Option                  | Short | Default  | Description                                           |
| ----------------------- | ----- | -------- | ----------------------------------------------------- |
| `--output <PATH>`       | `-o`  | -        | Output file (default: sindri-backup-TIMESTAMP.tar.gz) |
| `--profile <PROFILE>`   | `-p`  | standard | Backup profile: user-data, standard, full             |
| `--exclude <PATTERN>`   | `-x`  | -        | Additional exclude patterns (can repeat)              |
| `--exclude-secrets`     | -     | -        | Exclude all secret files                              |
| `--encrypt`             | `-e`  | -        | Encrypt backup with age                               |
| `--key-file <PATH>`     | `-k`  | -        | Encryption key file (age identity)                    |
| `--dry-run`             | `-d`  | -        | Show what would be backed up                          |
| `--compression <LEVEL>` | -     | 6        | Compression level (0-9)                               |
| `--verbose`             | `-v`  | -        | Show all files being backed up                        |
| `--yes`                 | `-y`  | -        | Skip confirmation prompt                              |

**Backup Profiles:**

| Profile     | Description                          |
| ----------- | ------------------------------------ |
| `user-data` | Only user data and config (smallest) |
| `standard`  | User data + extensions (default)     |
| `full`      | Everything including caches          |

**Examples:**

```bash
# Standard backup
sindri backup

# Encrypted backup
sindri backup --encrypt --key-file ~/.sindri-backup.key

# Full backup with custom output
sindri backup --profile full --output /backups/sindri-$(date +%Y%m%d).tar.gz

# Dry run to preview
sindri backup --dry-run
```

---

### restore

Restore workspace from a backup archive.

**Synopsis:**

```bash
sindri restore <SOURCE> [OPTIONS]
```

**Options:**

| Option                      | Short | Default | Description                                           |
| --------------------------- | ----- | ------- | ----------------------------------------------------- |
| `<SOURCE>`                  | -     | -       | Backup source (file path, s3://, or https://)         |
| `--mode <MODE>`             | `-m`  | safe    | Restore mode: safe, merge, full                       |
| `--target <PATH>`           | `-d`  | $HOME   | Target directory                                      |
| `--dry-run`                 | -     | -       | Show what would be restored                           |
| `--no-interactive`          | -     | -       | Skip confirmation prompts                             |
| `--auto-upgrade-extensions` | -     | -       | Auto-upgrade extensions to latest compatible versions |
| `--decrypt`                 | -     | -       | Decrypt with age key                                  |
| `--key-file <PATH>`         | -     | -       | Decryption key file (age identity)                    |
| `--verbose`                 | `-v`  | -       | Show all files being restored                         |
| `--skip-validation`         | -     | -       | Skip validation of restored files                     |

**Restore Modes:**

| Mode    | Description                                           |
| ------- | ----------------------------------------------------- |
| `safe`  | Only restore if no conflicts, preserve system markers |
| `merge` | Merge with existing files, newer wins                 |
| `full`  | Complete restore, overwrite everything (DANGEROUS)    |

**Examples:**

```bash
# Safe restore from file
sindri restore ./sindri-backup-20260115.tar.gz

# Merge mode restore
sindri restore ./backup.tar.gz --mode merge

# Restore from S3
sindri restore s3://my-bucket/backups/latest.tar.gz --decrypt

# Dry run to preview
sindri restore ./backup.tar.gz --dry-run --verbose
```

---

### project

Project management commands.

#### project new

Create a new project from template.

**Synopsis:**

```bash
sindri project new <NAME> [OPTIONS]
```

**Options:**

| Option                  | Short | Description                                   |
| ----------------------- | ----- | --------------------------------------------- |
| `<NAME>`                | -     | Project name                                  |
| `--project-type <TYPE>` | `-t`  | Project type (auto-detected if not specified) |
| `--interactive`         | `-i`  | Force interactive type selection              |
| `--git-name <NAME>`     | -     | Git user name                                 |
| `--git-email <EMAIL>`   | -     | Git user email                                |
| `--skip-tools`          | -     | Skip agentic tools installation               |

**Available Project Types:**

- `rust`, `rust-lib`, `rust-cli`, `rust-workspace`
- `python`, `python-package`, `python-api`, `python-ml`
- `typescript`, `typescript-lib`, `typescript-api`
- `go`, `go-cli`, `go-api`
- `java`, `java-api`
- `elixir`, `elixir-api`
- `zig`, `zig-lib`
- `generic`

**Examples:**

```bash
# Auto-detect from name (my-api creates API project)
sindri project new my-api

# Specify type
sindri project new my-lib --project-type rust-lib

# Interactive mode
sindri project new my-project -i

# With git configuration
sindri project new my-project --git-name "John Doe" --git-email john@example.com
```

#### project clone

Clone a project repository with enhancements.

**Synopsis:**

```bash
sindri project clone <REPOSITORY> [OPTIONS]
```

**Options:**

| Option                | Short | Description                     |
| --------------------- | ----- | ------------------------------- |
| `<REPOSITORY>`        | -     | Repository URL                  |
| `--fork`              | `-f`  | Fork before cloning             |
| `--branch <BRANCH>`   | `-b`  | Branch to checkout              |
| `--depth <DEPTH>`     | `-d`  | Clone depth (shallow clone)     |
| `--git-name <NAME>`   | -     | Git user name                   |
| `--git-email <EMAIL>` | -     | Git user email                  |
| `--feature <NAME>`    | -     | Feature branch to create        |
| `--no-deps`           | -     | Skip dependency installation    |
| `--skip-tools`        | -     | Skip agentic tools installation |
| `--no-enhance`        | -     | Skip all enhancements           |

**Examples:**

```bash
# Clone repository
sindri project clone https://github.com/user/repo

# Fork and clone
sindri project clone https://github.com/user/repo --fork

# Clone with feature branch
sindri project clone https://github.com/user/repo --feature add-auth

# Shallow clone
sindri project clone https://github.com/user/repo --depth 1
```

---

### doctor

Check system for required tools and dependencies.

**Synopsis:**

```bash
sindri doctor [OPTIONS]
```

**Options:**

| Option                  | Short | Default | Description                                                            |
| ----------------------- | ----- | ------- | ---------------------------------------------------------------------- |
| `--provider <PROVIDER>` | `-p`  | -       | Check tools for specific provider (docker, fly, devpod, e2b, k8s)      |
| `--command <COMMAND>`   | -     | -       | Check tools for specific command (project, extension, secrets, deploy) |
| `--all`                 | `-a`  | -       | Check all tools regardless of current usage                            |
| `--ci`                  | -     | -       | Exit with non-zero code if required tools are missing                  |
| `--format <FORMAT>`     | -     | human   | Output format: human, json, yaml                                       |
| `--verbose-output`      | -     | -       | Show detailed information including timing                             |
| `--check-auth`          | -     | -       | Check authentication status for tools that require it                  |
| `--fix`                 | -     | -       | Attempt to install missing tools                                       |
| `--yes`                 | `-y`  | -       | Skip confirmation prompts when installing                              |
| `--dry-run`             | -     | -       | Show what would be installed without actually installing               |
| `--check-extensions`    | -     | -       | Check tools required by installed extensions                           |
| `--extension <NAME>`    | -     | -       | Check a specific extension's tool requirements                         |

**Examples:**

```bash
# Basic system check
sindri doctor

# Check for Kubernetes provider
sindri doctor --provider k8s

# CI mode with JSON output
sindri doctor --ci --format json

# Fix missing tools
sindri doctor --fix

# Check extension requirements
sindri doctor --check-extensions --extension mise
```

**Exit Codes (CI mode):**

| Code | Description                  |
| ---- | ---------------------------- |
| 0    | All required tools available |
| 1    | Missing required tools       |
| 2    | Missing optional tools       |

---

### upgrade

Upgrade the CLI to a newer version.

**Synopsis:**

```bash
sindri upgrade [OPTIONS]
```

**Options:**

| Option                | Short | Description                                       |
| --------------------- | ----- | ------------------------------------------------- |
| `--check`             | -     | Check for updates only                            |
| `--list`              | -     | List available versions                           |
| `--version <VERSION>` | -     | Install specific version                          |
| `--compat <VERSION>`  | -     | Check extension compatibility for a version       |
| `--prerelease`        | -     | Include prereleases                               |
| `--allow-downgrade`   | -     | Allow downgrade to older version                  |
| `--yes`               | `-y`  | Skip confirmation prompts                         |
| `--force`             | `-f`  | Force upgrade even if extensions are incompatible |

**Examples:**

```bash
# Check for updates
sindri upgrade --check

# List available versions
sindri upgrade --list

# Check compatibility before upgrading
sindri upgrade --compat 3.1.0

# Upgrade to latest
sindri upgrade

# Upgrade to specific version
sindri upgrade --version 3.1.0 -y

# Downgrade
sindri upgrade --version 3.0.0 --allow-downgrade
```

---

### k8s

Local Kubernetes cluster management using kind or k3d.

#### k8s create

Create a local Kubernetes cluster.

**Synopsis:**

```bash
sindri k8s create [OPTIONS]
```

**Options:**

| Option                    | Short | Default      | Description                      |
| ------------------------- | ----- | ------------ | -------------------------------- |
| `--provider <PROVIDER>`   | `-p`  | kind         | Cluster provider (kind, k3d)     |
| `--name <NAME>`           | `-n`  | sindri-local | Cluster name                     |
| `--nodes <N>`             | `-N`  | 1            | Number of nodes                  |
| `--k8s-version <VERSION>` | -     | v1.35.0      | Kubernetes version               |
| `--registry`              | -     | -            | Enable local registry (k3d only) |
| `--registry-port <PORT>`  | -     | 5000         | Registry port (k3d only)         |
| `--json`                  | -     | -            | Output as JSON                   |

**Examples:**

```bash
# Create kind cluster
sindri k8s create

# Create k3d cluster with registry
sindri k8s create --provider k3d --registry

# Multi-node cluster
sindri k8s create --nodes 3 --name dev-cluster

# Specific Kubernetes version
sindri k8s create --k8s-version v1.34.0
```

#### k8s destroy

Destroy a local Kubernetes cluster.

**Synopsis:**

```bash
sindri k8s destroy [OPTIONS]
```

**Options:**

| Option          | Short | Default      | Description       |
| --------------- | ----- | ------------ | ----------------- |
| `--name <NAME>` | `-n`  | sindri-local | Cluster name      |
| `--force`       | `-f`  | -            | Skip confirmation |

**Examples:**

```bash
# Destroy with confirmation
sindri k8s destroy --name my-cluster

# Force destroy
sindri k8s destroy --name my-cluster --force
```

#### k8s list

List local Kubernetes clusters.

**Synopsis:**

```bash
sindri k8s list [OPTIONS]
```

**Options:**

| Option                  | Short | Description                    |
| ----------------------- | ----- | ------------------------------ |
| `--provider <PROVIDER>` | `-p`  | Filter by provider (kind, k3d) |
| `--json`                | -     | Output as JSON                 |

**Examples:**

```bash
# List all clusters
sindri k8s list

# List only kind clusters
sindri k8s list --provider kind --json
```

#### k8s status

Show cluster status.

**Synopsis:**

```bash
sindri k8s status [OPTIONS]
```

**Options:**

| Option                  | Short | Default      | Description                               |
| ----------------------- | ----- | ------------ | ----------------------------------------- |
| `--name <NAME>`         | `-n`  | sindri-local | Cluster name                              |
| `--provider <PROVIDER>` | `-p`  | -            | Provider (auto-detected if not specified) |
| `--json`                | -     | -            | Output as JSON                            |

**Examples:**

```bash
sindri k8s status
sindri k8s status --name dev-cluster --json
```

#### k8s config

Show kubeconfig for a cluster.

**Synopsis:**

```bash
sindri k8s config [OPTIONS]
```

**Options:**

| Option                  | Short | Default      | Description                               |
| ----------------------- | ----- | ------------ | ----------------------------------------- |
| `--name <NAME>`         | `-n`  | sindri-local | Cluster name                              |
| `--provider <PROVIDER>` | `-p`  | -            | Provider (auto-detected if not specified) |

**Examples:**

```bash
# Show kubeconfig
sindri k8s config

# Use with kubectl
sindri k8s config --name my-cluster > ~/.kube/my-cluster.yaml
export KUBECONFIG=~/.kube/my-cluster.yaml
```

#### k8s install

Install cluster management tools (kind/k3d).

**Synopsis:**

```bash
sindri k8s install <TOOL> [OPTIONS]
```

**Options:**

| Option   | Short | Description                 |
| -------- | ----- | --------------------------- |
| `<TOOL>` | -     | Tool to install (kind, k3d) |
| `--yes`  | `-y`  | Skip confirmation           |

**Examples:**

```bash
# Install kind
sindri k8s install kind

# Install k3d without confirmation
sindri k8s install k3d -y
```

---

### image

Container image management.

#### image list

List available images from registry.

**Synopsis:**

```bash
sindri image list [OPTIONS]
```

**Options:**

| Option                 | Default       | Description                  |
| ---------------------- | ------------- | ---------------------------- |
| `--registry <URL>`     | ghcr.io       | Registry URL                 |
| `--repository <NAME>`  | pacphi/sindri | Repository name              |
| `--filter <PATTERN>`   | -             | Filter tags by regex pattern |
| `--include-prerelease` | -             | Include prerelease versions  |
| `--json`               | -             | Output as JSON               |

**Examples:**

```bash
# List all images
sindri image list

# List with filter
sindri image list --filter "^v3\\."

# Include prereleases
sindri image list --include-prerelease --json
```

#### image inspect

Inspect image details.

**Synopsis:**

```bash
sindri image inspect <TAG> [OPTIONS]
```

**Options:**

| Option     | Description                              |
| ---------- | ---------------------------------------- |
| `<TAG>`    | Image tag to inspect                     |
| `--digest` | Show image digest                        |
| `--sbom`   | Download and show SBOM (requires cosign) |
| `--json`   | Output as JSON                           |

**Examples:**

```bash
# Inspect image
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0

# With SBOM
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --sbom
```

#### image verify

Verify image signature and provenance.

**Synopsis:**

```bash
sindri image verify <TAG> [OPTIONS]
```

**Options:**

| Option            | Description                  |
| ----------------- | ---------------------------- |
| `<TAG>`           | Image tag to verify          |
| `--no-signature`  | Skip signature verification  |
| `--no-provenance` | Skip provenance verification |

**Requires:** cosign (install from https://docs.sigstore.dev/cosign/installation/)

**Examples:**

```bash
# Full verification
sindri image verify ghcr.io/pacphi/sindri:v3.0.0

# Signature only
sindri image verify ghcr.io/pacphi/sindri:v3.0.0 --no-provenance
```

#### image versions

Show version compatibility matrix.

**Synopsis:**

```bash
sindri image versions [OPTIONS]
```

**Options:**

| Option                    | Default | Description                            |
| ------------------------- | ------- | -------------------------------------- |
| `--cli-version <VERSION>` | current | CLI version to check compatibility for |
| `--format <FORMAT>`       | table   | Output format (table, json)            |

**Examples:**

```bash
# Show compatible images for current CLI
sindri image versions

# Check for specific CLI version
sindri image versions --cli-version 3.0.0 --format json
```

#### image current

Show currently deployed image.

**Synopsis:**

```bash
sindri image current [OPTIONS]
```

**Options:**

| Option   | Description    |
| -------- | -------------- |
| `--json` | Output as JSON |

**Examples:**

```bash
sindri image current
sindri image current --json
```

---

### packer

Build and manage VM images with HashiCorp Packer. Supports multiple cloud providers including AWS, Azure, GCP, OCI, and Alibaba Cloud.

#### packer build

Build a VM image for a specified cloud provider.

**Synopsis:**

```bash
sindri vm build --cloud <PROVIDER> [OPTIONS]
```

**Options:**

| Option                   | Short | Default  | Description                                  |
| ------------------------ | ----- | -------- | -------------------------------------------- |
| `--cloud <PROVIDER>`     | `-c`  | required | Target cloud (aws, azure, gcp, oci, alibaba) |
| `--name <NAME>`          | `-n`  | -        | Image name prefix                            |
| `--sindri-version <VER>` | -     | latest   | Sindri version to install in image           |
| `--profile <PROFILE>`    | -     | -        | Extension profile to install                 |
| `--extensions <LIST>`    | -     | -        | Additional extensions to install (comma-sep) |
| `--region <REGION>`      | `-r`  | -        | Cloud region (defaults vary by provider)     |
| `--instance-type <TYPE>` | -     | -        | Instance type / VM size for build            |
| `--disk-size <GB>`       | -     | 60       | Disk size in GB                              |
| `--cis-hardening`        | -     | -        | Enable CIS security hardening                |
| `--force`                | `-f`  | -        | Force rebuild even if cached image exists    |
| `--dry-run`              | -     | -        | Generate template without building           |
| `--debug`                | -     | -        | Enable debug output                          |
| `--var-file <PATH>`      | -     | -        | Path to variable file                        |
| `--json`                 | -     | -        | Output as JSON                               |

**Default Instance Types by Provider:**

| Provider | Default Instance Type | Default Region |
| -------- | --------------------- | -------------- |
| AWS      | t3.large              | us-west-2      |
| Azure    | Standard_D2s_v3       | eastus         |
| GCP      | e2-standard-2         | us-central1-a  |
| OCI      | VM.Standard.E4.Flex   | -              |
| Alibaba  | ecs.g6.xlarge         | cn-hangzhou    |

**Examples:**

```bash
# Build AWS AMI with defaults
sindri vm build --cloud aws

# Build AWS AMI with specific version and region
sindri vm build --cloud aws --sindri-version v3.0.0 --region us-east-1

# Build Azure image with custom name
sindri vm build --cloud azure --name my-dev-env --instance-type Standard_D4s_v3

# Build GCP image with profile
sindri vm build --cloud gcp --profile python-data-science

# Build with CIS hardening enabled
sindri vm build --cloud aws --cis-hardening --disk-size 100

# Dry run to preview template
sindri vm build --cloud aws --dry-run

# Force rebuild, output JSON
sindri vm build --cloud aws --force --json

# Build with custom variable file
sindri vm build --cloud azure --var-file ./custom-vars.pkrvars.hcl
```

**Environment Variables (by provider):**

| Provider | Required Environment Variables                                |
| -------- | ------------------------------------------------------------- |
| AWS      | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` (or AWS profile) |
| Azure    | `AZURE_SUBSCRIPTION_ID`, `AZURE_RESOURCE_GROUP`               |
| GCP      | `GCP_PROJECT_ID`, `GOOGLE_APPLICATION_CREDENTIALS`            |
| OCI      | `OCI_COMPARTMENT_OCID`, `OCI_SUBNET_OCID`                     |
| Alibaba  | `ALICLOUD_ACCESS_KEY`, `ALICLOUD_SECRET_KEY`                  |

---

#### packer validate

Validate a Packer template without building.

**Synopsis:**

```bash
sindri vm validate --cloud <PROVIDER> [OPTIONS]
```

**Options:**

| Option                   | Short | Default  | Description                                  |
| ------------------------ | ----- | -------- | -------------------------------------------- |
| `--cloud <PROVIDER>`     | `-c`  | required | Target cloud (aws, azure, gcp, oci, alibaba) |
| `--name <NAME>`          | `-n`  | -        | Image name prefix                            |
| `--sindri-version <VER>` | -     | latest   | Sindri version                               |
| `--syntax-only`          | -     | -        | Syntax check only (no provider validation)   |
| `--json`                 | -     | -        | Output as JSON                               |

**Examples:**

```bash
# Validate AWS template
sindri vm validate --cloud aws

# Validate with syntax check only
sindri vm validate --cloud azure --syntax-only

# Validate and output JSON
sindri vm validate --cloud gcp --json
```

---

#### packer list

List VM images in the cloud provider.

**Synopsis:**

```bash
sindri vm list --cloud <PROVIDER> [OPTIONS]
```

**Options:**

| Option               | Short | Default  | Description                                  |
| -------------------- | ----- | -------- | -------------------------------------------- |
| `--cloud <PROVIDER>` | `-c`  | required | Target cloud (aws, azure, gcp, oci, alibaba) |
| `--name <NAME>`      | `-n`  | -        | Filter by name prefix                        |
| `--region <REGION>`  | `-r`  | -        | Cloud region                                 |
| `--json`             | -     | -        | Output as JSON                               |

**Examples:**

```bash
# List all AWS images
sindri vm list --cloud aws

# List AWS images in specific region
sindri vm list --cloud aws --region us-east-1

# List Azure images with name filter
sindri vm list --cloud azure --name sindri-dev

# List GCP images as JSON
sindri vm list --cloud gcp --json
```

---

#### packer delete

Delete a VM image from the cloud provider.

**Synopsis:**

```bash
sindri vm delete --cloud <PROVIDER> <IMAGE_ID> [OPTIONS]
```

**Options:**

| Option               | Short | Default  | Description                                  |
| -------------------- | ----- | -------- | -------------------------------------------- |
| `--cloud <PROVIDER>` | `-c`  | required | Target cloud (aws, azure, gcp, oci, alibaba) |
| `<IMAGE_ID>`         | -     | required | Image ID to delete                           |
| `--region <REGION>`  | `-r`  | -        | Cloud region                                 |
| `--force`            | `-f`  | -        | Skip confirmation prompt                     |

**Examples:**

```bash
# Delete AWS AMI with confirmation
sindri vm delete --cloud aws ami-0123456789abcdef0

# Delete Azure image without confirmation
sindri vm delete --cloud azure /subscriptions/.../my-image --force

# Delete GCP image in specific region
sindri vm delete --cloud gcp sindri-dev-20260101 --region us-central1 --force
```

---

#### packer doctor

Check Packer prerequisites and cloud provider authentication status.

**Synopsis:**

```bash
sindri vm doctor [OPTIONS]
```

**Options:**

| Option               | Short | Default | Description                                       |
| -------------------- | ----- | ------- | ------------------------------------------------- |
| `--cloud <PROVIDER>` | `-c`  | all     | Target cloud (aws, azure, gcp, oci, alibaba, all) |
| `--json`             | -     | -       | Output as JSON                                    |

**Output includes:**

- Packer installation status and version
- Cloud CLI installation status and version
- Credentials/authentication configuration status
- Hints for resolving missing prerequisites

**Examples:**

```bash
# Check all prerequisites
sindri vm doctor

# Check AWS prerequisites only
sindri vm doctor --cloud aws

# Check Azure prerequisites with JSON output
sindri vm doctor --cloud azure --json

# Check all providers
sindri vm doctor --cloud all
```

---

#### packer init

Generate a Packer HCL template file for customization.

**Synopsis:**

```bash
sindri vm init --cloud <PROVIDER> [OPTIONS]
```

**Options:**

| Option               | Short | Default  | Description                                  |
| -------------------- | ----- | -------- | -------------------------------------------- |
| `--cloud <PROVIDER>` | `-c`  | required | Target cloud (aws, azure, gcp, oci, alibaba) |
| `--output <PATH>`    | `-o`  | .        | Output directory for generated files         |
| `--force`            | `-f`  | -        | Force overwrite existing files               |

**Generated Files:**

- `<cloud>.pkr.hcl` - Main Packer template

**Examples:**

```bash
# Generate AWS template in current directory
sindri vm init --cloud aws

# Generate Azure template in specific directory
sindri vm init --cloud azure --output ./packer/

# Overwrite existing template
sindri vm init --cloud gcp --force

# Generate OCI template
sindri vm init --cloud oci --output ./infra/packer/
```

**Next steps after init:**

1. Edit the generated `.pkr.hcl` file as needed
2. Run `packer init <file>.pkr.hcl` to download plugins
3. Run `packer build <file>.pkr.hcl` to build the image

---

#### packer deploy

Deploy a VM instance from a previously built image.

**Synopsis:**

```bash
sindri vm deploy --cloud <PROVIDER> <IMAGE_ID> [OPTIONS]
```

**Options:**

| Option                   | Short | Default  | Description                                  |
| ------------------------ | ----- | -------- | -------------------------------------------- |
| `--cloud <PROVIDER>`     | `-c`  | required | Target cloud (aws, azure, gcp, oci, alibaba) |
| `<IMAGE_ID>`             | -     | required | Image ID to deploy                           |
| `--region <REGION>`      | `-r`  | -        | Cloud region                                 |
| `--instance-type <TYPE>` | -     | -        | Instance type / VM size                      |
| `--json`                 | -     | -        | Output as JSON                               |

**Output:**

- Instance ID
- Public IP address (if assigned)
- Private IP address
- SSH command to connect

**Examples:**

```bash
# Deploy AWS instance from AMI
sindri vm deploy --cloud aws ami-0123456789abcdef0

# Deploy with specific region and instance type
sindri vm deploy --cloud aws ami-0123456789abcdef0 \
    --region us-east-1 --instance-type t3.xlarge

# Deploy Azure VM
sindri vm deploy --cloud azure /subscriptions/.../my-image \
    --region eastus --instance-type Standard_D4s_v3

# Deploy GCP instance with JSON output
sindri vm deploy --cloud gcp sindri-dev-20260101 --json
```

---

## Environment Variables

| Variable                | Description                                 |
| ----------------------- | ------------------------------------------- |
| `SINDRI_CONFIG`         | Path to sindri.yaml config file             |
| `SINDRI_LOG_LEVEL`      | Log level (trace, debug, info, warn, error) |
| `SINDRI_HOME`           | Sindri home directory (default: ~/.sindri)  |
| `SINDRI_PROVIDER`       | Default provider to use                     |
| `GITHUB_TOKEN`          | GitHub token for registry authentication    |
| `AWS_PROFILE`           | AWS profile for S3 secrets backend          |
| `AWS_REGION`            | Default AWS region                          |
| `AWS_ACCESS_KEY_ID`     | AWS access key                              |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key                              |
| `VAULT_ADDR`            | HashiCorp Vault address                     |
| `VAULT_TOKEN`           | HashiCorp Vault token                       |

## Exit Codes

| Code | Description          |
| ---- | -------------------- |
| 0    | Success              |
| 1    | General error        |
| 2    | Configuration error  |
| 3    | Provider error       |
| 4    | Network error        |
| 5    | Authentication error |

## Troubleshooting

### Docker not running

```
Error: Docker is not running. Please start Docker and try again.
```

Start Docker Desktop or the Docker daemon:

```bash
# macOS/Windows: Start Docker Desktop

# Linux
sudo systemctl start docker
```

### Missing kubectl

```
Error: kubectl is not installed
```

Install kubectl:

```bash
# macOS
brew install kubectl

# Linux
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
sudo install kubectl /usr/local/bin/
```

### S3 Authentication Failed

```
Error: Failed to create S3 client: credential error
```

Configure AWS credentials:

```bash
# Set environment variables
export AWS_ACCESS_KEY_ID=your-key-id
export AWS_SECRET_ACCESS_KEY=your-secret-key

# Or use AWS CLI
aws configure
```

### Cosign not installed

```
Warning: cosign not installed - SBOM verification requires cosign
```

Install cosign:

```bash
# macOS
brew install cosign

# Linux
curl -O -L https://github.com/sigstore/cosign/releases/latest/download/cosign-linux-amd64
sudo mv cosign-linux-amd64 /usr/local/bin/cosign
sudo chmod +x /usr/local/bin/cosign
```

### Extension Validation Failed

```
Error: Extension 'my-extension' validation failed
```

Check extension definition:

```bash
sindri extension validate my-extension --file ./extension.yaml
```

### Cluster Provider Not Found

```
Error: No cluster provider installed
```

Install kind or k3d:

```bash
sindri k8s install kind
# or
sindri k8s install k3d
```

## See Also

- [Configuration Reference](./CONFIGURATION.md) - Detailed sindri.yaml documentation
- [Secrets Management](./SECRETS_MANAGEMENT.md) - Secrets backend configuration
- [Backup & Restore](./BACKUP_RESTORE.md) - Backup strategies and procedures
- [Projects](./PROJECTS.md) - Project creation and templates
- [Doctor](./DOCTOR.md) - System diagnostics guide
- [Image Management](./IMAGE_MANAGEMENT.md) - Container image security
