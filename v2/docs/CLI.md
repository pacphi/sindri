# Sindri CLI Reference

Complete command-line reference for Sindri deployment and extension management.

## Overview

Sindri provides two primary CLI tools:

1. **`sindri`** - Main deployment and configuration CLI
2. **`extension-manager`** - Extension installation and management

Both CLIs work on the host system (outside containers) and inside deployed containers.

---

## sindri CLI

**Location:** `cli/sindri`

Main deployment tool for managing Sindri environments across providers.

### Global Options

```bash
-h, --help               Show help message
-c, --config <file>      Use specific config file (default: sindri.yaml)
-p, --provider <name>    Deployment provider (docker, fly, devpod, e2b)
-r, --rebuild            Force rebuild of Docker image
-v, --verbose            Verbose output
```

---

## Configuration Commands

### sindri config init

Create a new `sindri.yaml` configuration file with sensible defaults.

```bash
sindri config init
```

**Behavior:**

- Creates `sindri.yaml` in current directory
- Prompts before overwriting existing file
- Generates template with common options commented

**Example Output:**

```yaml
version: 1.0
name: my-sindri-dev

deployment:
  provider: docker
  resources:
    memory: 2GB
    cpus: 2

extensions:
  profile: fullstack

secrets:
  - name: ANTHROPIC_API_KEY
    source: env
```

### sindri config validate

Validate configuration file against JSON schema.

```bash
sindri config validate [--config <file>]
```

**Options:**

- `--config <file>` - Config file to validate (default: `sindri.yaml`)

**Examples:**

```bash
# Validate default sindri.yaml
sindri config validate

# Validate specific file
sindri config validate --config examples/fly/minimal.sindri.yaml

# Validate all examples
for f in examples/**/*.sindri.yaml; do
  sindri config validate --config "$f"
done
```

**Exit Codes:**

- `0` - Validation passed
- `1` - Validation failed (schema errors, missing required fields)

---

## Deployment Commands

### sindri deploy

Deploy Sindri environment to specified provider.

```bash
sindri deploy [--provider <name>] [--rebuild] [--config <file>]
```

**Options:**

- `--provider <name>` - Override provider from config (docker, fly, devpod, e2b)
- `--rebuild` - Force rebuild Docker image (useful for Fly.io)
- `--config <file>` - Use specific config file

**Provider Behavior:**

- **docker** - Deploys via Docker Compose locally
- **fly** - Deploys to Fly.io cloud (requires flyctl)
- **devpod** - Deploys as DevContainer (requires devpod CLI)
- **e2b** - Deploys to E2B cloud sandbox (requires E2B_API_KEY)

**Examples:**

```bash
# Deploy using provider in sindri.yaml
sindri deploy

# Deploy to Docker explicitly
sindri deploy --provider docker

# Deploy to Fly.io with fresh image build
sindri deploy --provider fly --rebuild

# Deploy using example config
sindri deploy --config examples/fly/minimal.sindri.yaml

# Deploy DevPod environment
sindri deploy --provider devpod

# Deploy to E2B cloud sandbox
sindri deploy --provider e2b
```

**What Happens:**

1. Validates configuration
2. Builds/pulls Docker image (if needed)
3. Deploys using provider-specific adapter
4. Waits for environment to be ready
5. Displays connection information

### sindri plan

Show deployment plan without executing (dry-run mode).

```bash
sindri plan [--config <file>]
```

**Output:**

- Provider to be used
- Resources to be allocated
- Extensions to be installed
- Secrets configuration status
- Estimated costs (for cloud providers)

**Example:**

```bash
sindri plan --config examples/fly/production.sindri.yaml
```

### sindri destroy

Teardown deployed environment and cleanup resources.

```bash
sindri destroy [--provider <name>] [--force] [--config <file>]
```

**Options:**

- `--provider <name>` - Provider to teardown (auto-detected if omitted)
- `--force` - Skip confirmation prompt
- `--config <file>` - Config file used for deployment

**Examples:**

```bash
# Teardown with confirmation
sindri destroy

# Force teardown without confirmation
sindri destroy --force

# Teardown specific provider
sindri destroy --provider fly

# Teardown using config
sindri destroy --config examples/fly/minimal.sindri.yaml
```

**Cleanup Actions:**

- Stops and removes containers (Docker)
- Destroys Fly.io machines and volumes
- Removes DevPod workspace
- Cleans up provider-specific resources

### sindri connect

Connect to deployed Sindri environment via SSH.

```bash
sindri connect [--config <file>]
```

**Behavior:**

- Auto-detects connection method based on provider
- Opens SSH session as `developer` user
- Lands in `/workspace` directory

**Examples:**

```bash
# Connect to running environment
sindri connect

# Connect using specific config
sindri connect --config my-custom.sindri.yaml
```

**Provider Connection Methods:**

- **docker** - `docker exec` into container
- **fly** - `flyctl ssh console` via proxy
- **devpod** - `devpod ssh` into workspace
- **e2b** - WebSocket connection to sandbox terminal

### sindri status

Show deployment status for the current environment.

```bash
sindri status [--config <file>]
```

**Behavior:**

- Auto-detects provider from configuration
- Queries provider-specific status information
- Shows resource usage and connection details

**Examples:**

```bash
# Check status of current deployment
sindri status

# Check status for specific config
sindri status --config production.sindri.yaml
```

**Provider Status Information:**

- **docker** - Container state, resource usage (CPU/memory)
- **fly** - Machine status, volumes, app state
- **devpod** - Workspace status, Kubernetes pod state (if k8s backend)
- **e2b** - Sandbox state, remaining timeout, template info

---

## Testing Commands

### sindri test

Run test suite on deployed environment.

```bash
sindri test [--config <file>] [--suite <name>]
```

**Options:**

- `--config <file>` - Config file to test
- `--suite <name>` - Test suite to run (smoke, integration, full)

**Test Suites:**

- `smoke` - Quick health checks (extension presence, basic functionality)
- `integration` - Full integration tests (extension interactions)
- `full` - Comprehensive tests (performance, security, compliance)

**Examples:**

```bash
# Run smoke tests on current deployment
sindri test --suite smoke

# Test specific configuration
sindri test --config examples/fly/ai-dev.sindri.yaml --suite integration

# Run full test suite
sindri test --suite full
```

---

## E2B-Specific Commands

These commands are only available when using the E2B provider.

### sindri pause

Pause sandbox to preserve state. E2B sandboxes auto-terminate after timeout; pause saves state for later resume.

```bash
sindri pause [--config <file>]
```

**Options:**

- `--config <file>` - Config file (default: `sindri.yaml`)

**Examples:**

```bash
# Pause current E2B sandbox
sindri pause

# Pause with specific config
sindri pause --config my-e2b.sindri.yaml
```

**Notes:**

- Only available for E2B provider
- Paused sandboxes can be resumed with `sindri deploy`
- Preserves filesystem state and running processes

### sindri template

Manage E2B templates for faster sandbox startup.

```bash
sindri template <subcommand> [options]
```

**Subcommands:**

- `list` - List available templates
- `create` - Create template from current sandbox state
- `delete` - Delete a template
- `info` - Show template details

**Examples:**

```bash
# List available templates
sindri template list

# Create template from running sandbox
sindri template create --name my-dev-env

# Delete a template
sindri template delete my-dev-env

# Show template info
sindri template info my-dev-env
```

---

## Backup and Restore Commands

### sindri backup

Create a backup of the workspace with configurable profiles.

```bash
sindri backup [--profile <name>] [--output <path>] [--config <file>]
```

**Options:**

- `--profile <name>` - Backup profile: `user-data`, `standard` (default), `full`
- `--output <path>` - Output path (local or `s3://bucket/path`)
- `--config <file>` - Sindri config file
- `--exclude <pattern>` - Additional exclude pattern (repeatable)
- `--dry-run` - Preview what would be backed up
- `--list` - List backups on instance

**Backup Profiles:**

| Profile     | Contents                                            | Size   | Use Case          |
| ----------- | --------------------------------------------------- | ------ | ----------------- |
| `user-data` | Projects, scripts, Claude data, SSH keys, gitconfig | Small  | Migration         |
| `standard`  | user-data + shell configs + app configs             | Medium | Regular backups   |
| `full`      | Everything except caches                            | Large  | Disaster recovery |

**Examples:**

```bash
# Standard backup
sindri backup

# User data only (smallest, for migration)
sindri backup --profile user-data --output ./backups/

# Backup to S3
sindri backup --output s3://my-bucket/sindri-backups/

# Preview backup
sindri backup --dry-run

# List backups on instance
sindri backup list
```

### sindri restore

Restore workspace from backup with collision handling.

```bash
sindri restore <source> [--mode <name>] [--config <file>]
```

**Arguments:**

- `source` - Backup file: local path, `s3://bucket/path`, or `https://url`

**Options:**

- `--mode <name>` - Restore mode: `safe` (default), `merge`, `full`
- `--config <file>` - Sindri config file
- `--dry-run` - Preview restore without making changes
- `--no-interactive` - Skip confirmation prompts

**Restore Modes:**

| Mode    | Behavior         | System Markers | Existing Files   |
| ------- | ---------------- | -------------- | ---------------- |
| `safe`  | Never overwrite  | Skipped        | Preserved        |
| `merge` | Smart merge      | Skipped        | Backed up (.bak) |
| `full`  | Complete restore | Optional       | Overwritten      |

**Examples:**

```bash
# Safe restore (default)
sindri restore ./backup.tar.gz

# Preview restore
sindri restore ./backup.tar.gz --dry-run

# Merge with automatic backup of conflicts
sindri restore ./backup.tar.gz --mode merge

# Restore from S3
sindri restore s3://my-bucket/backups/backup.tar.gz
```

**Important Notes:**

- System markers (`.initialized`, `bootstrap.yaml`) are **never** restored to prevent breaking initialization
- Use `--dry-run` to preview changes before restoring
- See [Backup & Restore Guide](BACKUP_RESTORE.md) for detailed documentation

---

## Profile Commands

### sindri profiles list

List all available extension profiles.

```bash
sindri profiles list
```

**Output:**

```text
Available profiles:

Standard:
  minimal       - Basic Node.js + Python setup (2 extensions)
  fullstack     - Full-stack development (4 extensions)
  ai-dev        - AI/ML development (9 extensions)
  anthropic-dev - Complete Anthropic toolset (20 extensions)
  systems       - Systems programming (4 extensions)
  enterprise    - Multi-language enterprise stack (10 extensions)
  devops        - DevOps and infrastructure (4 extensions)
  mobile        - Mobile development backend (3 extensions)

VisionFlow:
  visionflow-core           - Document processing (9 extensions)
  visionflow-data-scientist - AI research and ML (9 extensions)
  visionflow-creative       - 3D modeling and creative (5 extensions)
  visionflow-full           - All VisionFlow tools (34 extensions)
```

### sindri profiles show

Show details about a specific profile.

```bash
sindri profiles show <profile-name>
```

**Example:**

```bash
sindri profiles show ai-dev
```

**Output:**

```text
Profile: ai-dev
Description: AI/ML development with Claude Code and monitoring
Extensions:
  - nodejs (language)
  - python (language)
  - ai-toolkit (ai)
  - openskills (ai)
  - monitoring (monitoring)
```

---

## extension-manager CLI

**Location:** `cli/extension-manager`

Manage extensions in Sindri environments. Can be used on host or inside container.

### Extension Manager Options

```bash
-h, --help               Show help message
-v, --verbose            Verbose output
--category <name>        Filter by category (for list command)
```

---

## Extension Listing Commands

### extension-manager list

List all available extensions.

```bash
extension-manager list [--category <name>]
```

**Options:**

- `--category <name>` - Filter by category

**Examples:**

```bash
# List all extensions
extension-manager list

# List only language extensions
extension-manager list --category language

# List AI tools
extension-manager list --category ai
```

**Output Format:**

```text
Available extensions:
  nodejs           - Node.js runtime and npm (language)
  python           - Python runtime and pip (language)
  golang           - Go programming language (language)
  rust             - Rust toolchain (language)
  docker           - Docker client and CLI (dev-tools)
  ai-toolkit       - AI development toolkit (ai)
  ...
```

### extension-manager list-profiles

List all available extension profiles.

```bash
extension-manager list-profiles
```

**Output:**

```text
Available profiles:
  minimal       - nodejs, python
  fullstack     - nodejs, python, docker, nodejs-devtools
  ai-dev        - nodejs, python, ai-toolkit, openskills, monitoring
  ...
```

### extension-manager list-categories

List all extension categories.

```bash
extension-manager list-categories
```

**Output:**

```text
Available categories:
  base          - Core system components
  language      - Programming languages and runtimes
  dev-tools     - Development utilities and tools
  infrastructure - Cloud, containers, orchestration
  ai            - AI and machine learning tools
  utilities     - General purpose tools
  desktop       - Desktop environments (GUI)
  monitoring    - Monitoring and observability
```

---

## Extension Installation Commands

### extension-manager install

Install a single extension with its dependencies.

```bash
extension-manager install <extension-name>
```

**Behavior:**

1. Resolves dependencies recursively
2. Installs in topological order
3. Validates installation
4. Updates manifest

**Examples:**

```bash
# Install Node.js (includes mise-config dependency)
extension-manager install nodejs

# Install AI toolkit (includes nodejs, python, golang, github-cli dependencies)
extension-manager install ai-toolkit

# Install Docker
extension-manager install docker
```

### extension-manager install-profile

Install all extensions from a profile.

```bash
extension-manager install-profile <profile-name>
```

**Examples:**

```bash
# Install minimal profile (nodejs + python)
extension-manager install-profile minimal

# Install full AI development stack
extension-manager install-profile ai-dev

# Install enterprise profile (all languages)
extension-manager install-profile enterprise
```

### extension-manager install-all

Install all extensions listed in the manifest.

```bash
extension-manager install-all
```

**Use Case:**

- Restoring extensions after container restart
- Bulk installation during environment setup

---

## Extension Reinstall Commands

### extension-manager reinstall

Remove and reinstall an extension. Useful when an extension definition has changed
or when troubleshooting installation issues.

```bash
extension-manager reinstall <extension-name>
```

**Behavior:**

1. Removes the existing installation (forced, no confirmation)
2. Clears manifest entry and installation marker
3. Resolves dependencies
4. Reinstalls extension with fresh configuration

**Examples:**

```bash
# Reinstall supabase-cli after extension definition update
extension-manager reinstall supabase-cli

# Reinstall nodejs to fix broken installation
extension-manager reinstall nodejs
```

### extension-manager reinstall-profile

Remove and reinstall all extensions in a profile. Useful after upgrading
Sindri or when extension definitions have been updated.

```bash
extension-manager reinstall-profile <profile-name>
```

**Behavior:**

1. Resolves all dependencies for profile extensions
2. Removes extensions in reverse dependency order
3. Reinstalls base/protected extensions first
4. Reinstalls all profile extensions in dependency order
5. Reports success/failure counts

**Examples:**

```bash
# Reinstall all extensions in the base profile
extension-manager reinstall-profile base

# Reinstall the full development stack
extension-manager reinstall-profile fullstack
```

**Use Case:**

- After deploying updated Sindri image with new extension definitions
- Recovering from broken extension installations
- Applying updated extension configurations across a profile

---

## Extension Removal Commands

### extension-manager remove

Uninstall an extension.

```bash
extension-manager remove <extension-name>
```

**Behavior:**

1. Runs extension's `remove` script/configuration
2. Removes from manifest
3. Cleans up installation artifacts

**Examples:**

```bash
# Remove Node.js
extension-manager remove nodejs

# Remove Docker
extension-manager remove docker
```

**Warning:** Does not automatically remove dependent extensions.

---

## Extension Validation Commands

### extension-manager validate

Validate a single extension's installation.

```bash
extension-manager validate <extension-name>
```

**Checks:**

- Extension is installed
- Validation commands pass
- Expected patterns match output
- Dependencies are satisfied

**Examples:**

```bash
# Validate Node.js installation
extension-manager validate nodejs

# Validate Python
extension-manager validate python

# Validate AI toolkit
extension-manager validate ai-toolkit
```

**Exit Codes:**

- `0` - Validation passed
- `1` - Validation failed

### extension-manager validate-all

Validate all installed extensions.

```bash
extension-manager validate-all
```

**Output:**

```text
Validating extensions...
✓ nodejs - Node.js v20.11.0 (validated)
✓ python - Python 3.11.8 (validated)
✓ docker - Docker 25.0.3 (validated)
✗ rust - rustc not found (failed)

Summary: 3/4 extensions valid
```

---

## Extension Status Commands

### extension-manager status

Show installation status of a specific extension.

```bash
extension-manager status <extension-name>
```

**Output:**

```text
Extension: nodejs
Status: installed
Version: 1.0.0
Category: language
Dependencies: mise-config
Installed: 2024-01-15T10:30:00Z
```

### extension-manager status-all

Show status of all extensions in manifest.

```bash
extension-manager status-all
```

---

## Extension Information Commands

### extension-manager info

Show detailed information about an extension.

```bash
extension-manager info <extension-name>
```

**Output:**

```text
Name: nodejs
Version: 1.0.0
Category: language
Description: Node.js runtime and npm package manager

Requirements:
  - Disk Space: 500MB
  - Domains: nodejs.org, npmjs.com

Dependencies:
  - mise-config

Install Method: mise
Configuration: mise.toml

Validation:
  - node --version (expected: v\d+\.\d+\.\d+)
  - npm --version
```

### extension-manager bom

Show Bill of Materials for installed extensions.

```bash
extension-manager bom [extension-name]
```

**Examples:**

```bash
# Show BOM for all extensions
extension-manager bom

# Show BOM for specific extension
extension-manager bom nodejs
```

### extension-manager bom-regenerate

Regenerate Bill of Materials for all extensions.

```bash
extension-manager bom-regenerate
```

---

## Environment Variables

### Global Variables

```bash
SINDRI_CONFIG       # Override config file path
SINDRI_PROVIDER     # Override deployment provider
SINDRI_REBUILD      # Force rebuild (true/false)
SINDRI_VERBOSE      # Enable verbose output (true/false)
```

### Extension Manager Variables

```bash
DOCKER_LIB          # Path to docker/lib (auto-detected)
WORKSPACE_ROOT      # Workspace root path (default: /workspace)
MANIFEST_DIR        # Manifest directory (default: /workspace/.system/manifest)
```

---

## Exit Codes

All CLI commands follow standard exit code conventions:

- `0` - Success
- `1` - General error (invalid arguments, file not found)
- `2` - Configuration error (invalid YAML, schema violation)
- `3` - Deployment error (provider failure, resource exhaustion)
- `4` - Validation error (extension validation failed)

---

## Scripting and Automation

### Batch Operations

```bash
# Deploy multiple configurations
for config in examples/fly/*.sindri.yaml; do
  sindri deploy --config "$config"
done

# Validate all examples
find examples -name "*.sindri.yaml" -exec sindri config validate --config {} \;

# Install multiple extensions
for ext in nodejs python docker; do
  extension-manager install "$ext"
done
```

### CI/CD Integration

```bash
# Validate and deploy in CI
#!/bin/bash
set -e

sindri config validate --config production.sindri.yaml
sindri deploy --config production.sindri.yaml --provider fly

# Wait for health check
sleep 30
sindri test --config production.sindri.yaml --suite smoke
```

### Error Handling

```bash
# Check if deployment succeeded
if sindri deploy --provider docker; then
  echo "Deployment successful"
  sindri connect
else
  echo "Deployment failed"
  sindri destroy --force
  exit 1
fi
```

---

## Troubleshooting

### Common Issues

#### Config validation fails

```bash
# Check YAML syntax
yamllint sindri.yaml

# Validate against schema
sindri config validate --verbose
```

#### Extension installation fails

```bash
# Check extension status
extension-manager status <extension>

# Validate extension
extension-manager validate <extension>

# Check logs
cat /workspace/.system/logs/<extension>.log
```

#### Deployment hangs

```bash
# Use verbose mode
sindri deploy --verbose

# Check provider status
# Docker: docker ps -a
# Fly: flyctl status
# DevPod: devpod list
```

#### Can't connect to environment

```bash
# Check deployment status
sindri plan

# Verify provider connection
# Docker: docker exec -it <container> /docker/scripts/entrypoint.sh /bin/bash
# Fly: flyctl ssh console
# DevPod: devpod ssh <workspace>
```

---

## See Also

- [Backup & Restore Guide](BACKUP_RESTORE.md) - Workspace backup and restore procedures
- [Configuration Reference](CONFIGURATION.md) - Complete configuration guide
- [Schema Reference](SCHEMA.md) - JSON schema documentation
- [Extension Authoring](EXTENSION_AUTHORING.md) - Creating custom extensions
- [Deployment Guide](DEPLOYMENT.md) - Provider-specific deployment details
- [Troubleshooting](TROUBLESHOOTING.md) - Common issues and solutions
