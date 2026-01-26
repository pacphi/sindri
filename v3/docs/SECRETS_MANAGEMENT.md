# Secrets Management

Comprehensive guide to managing secrets in Sindri V3 across all deployment providers.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Secret Sources](#secret-sources)
  - [Environment Variables](#environment-variables-source-env)
  - [Files](#files-source-file)
  - [HashiCorp Vault](#hashicorp-vault-source-vault)
  - [S3 Encrypted Storage](#s3-encrypted-storage-source-s3)
- [Configuration Reference](#configuration-reference)
- [CLI Commands](#cli-commands)
- [Provider Integration](#provider-integration)
- [Security Best Practices](#security-best-practices)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Overview

Sindri V3 provides a unified, type-safe secrets management system built in Rust with async resolution, automatic memory zeroing, and support for four secret sources.

### What's New in V3

| Feature              | V2 (Bash)               | V3 (Rust)               |
| -------------------- | ----------------------- | ----------------------- |
| S3 Encrypted Storage | Not supported           | Full support            |
| Memory Safety        | Plaintext in temp files | `zeroize` auto-clearing |
| Vault Integration    | CLI subprocess          | Native async HTTP API   |
| Resolution           | Sequential              | Parallel async          |
| Type Safety          | Runtime validation      | Compile-time guarantees |
| Error Handling       | Global counters         | Structured `Result<T>`  |

### Design Principles

1. **Zero Configuration Default** - Works out of the box with `.env` files
2. **Progressive Enhancement** - Supports advanced sources (Vault, S3) when needed
3. **Provider Awareness** - Leverages each provider's native secret management
4. **Security by Default** - Never logs or exposes secrets, automatic memory zeroing
5. **Developer Friendly** - Clear feedback, helpful errors, validation

### Supported Secret Sources

| Source  | Use Case                             | Example                          |
| ------- | ------------------------------------ | -------------------------------- |
| `env`   | API keys, tokens, passwords          | `.env` files, shell exports      |
| `file`  | Certificates, SSH keys, config files | TLS certs, private keys          |
| `vault` | Production secrets, rotation         | HashiCorp Vault KV store         |
| `s3`    | Team collaboration, CI/CD, backup    | S3 encrypted storage (new in V3) |

### Resolution Priority

Secrets are resolved in this order (highest priority first):

```
1. Shell environment variables
2. .env.local
3. .env
4. fromFile (if specified)
5. S3 (if configured)
6. Vault (if configured)
```

## Quick Start

### Simple .env File (Recommended for Development)

**1. Create `.env` file in project root:**

```bash
# .env (committed to git - safe defaults only)
NODE_ENV=development
LOG_LEVEL=info

# .env.local (gitignored - personal secrets)
ANTHROPIC_API_KEY=sk-ant-api03-...
GITHUB_TOKEN=ghp_...
```

**2. Configure in `sindri.yaml`:**

```yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: env
  - name: GITHUB_TOKEN
    source: env
```

**3. Validate and Deploy:**

```bash
# Validate secrets are resolvable
sindri secrets validate

# Deploy
sindri deploy
```

## Secret Sources

### Environment Variables (`source: env`)

Resolve secrets from environment variables, `.env` files, shell exports, or local files.

#### Resolution Priority

1. **Shell environment variables** - `export ANTHROPIC_API_KEY=...`
2. **`.env.local`** - Gitignored, personal secrets
3. **`.env`** - Committed, shared defaults
4. **`fromFile`** - Read content from a local file (if specified)

#### Configuration

```yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: env

  - name: DATABASE_PASSWORD
    source: env
    required: true # Fail deployment if missing
```

#### The `fromFile` Property

Use `fromFile` to read secret content directly from a local file:

```yaml
secrets:
  # SSH public key - reads from file automatically
  - name: AUTHORIZED_KEYS
    source: env
    fromFile: ~/.ssh/id_ed25519.pub # Supports ~ expansion

  # Git signing key
  - name: GPG_PUBLIC_KEY
    source: env
    fromFile: ~/.gnupg/public.asc
```

**Key differences: `fromFile` vs `source: file`**

| Feature            | `fromFile`                         | `source: file`             |
| ------------------ | ---------------------------------- | -------------------------- |
| **Purpose**        | Read file content to env var       | Mount file into container  |
| **Result**         | Sets environment variable          | File exists at `mountPath` |
| **Use case**       | SSH public keys, API keys in files | Certificates, config files |
| **Container path** | N/A                                | Required via `mountPath`   |

### Files (`source: file`)

Use files for certificates, SSH keys, and configuration files that need to be mounted in the container.

#### Configuration

```yaml
secrets:
  - name: TLS_CERT
    source: file
    path: ./certs/tls.crt
    mountPath: /etc/ssl/certs/app.crt

  - name: TLS_KEY
    source: file
    path: ./certs/tls.key
    mountPath: /etc/ssl/private/app.key
    permissions: "0600" # Restrictive permissions for private key
```

#### Configuration Fields

| Field         | Required | Description                              | Default  |
| ------------- | -------- | ---------------------------------------- | -------- |
| `path`        | Yes      | Local file path (supports `~` expansion) | -        |
| `mountPath`   | Yes      | Destination path in container            | -        |
| `permissions` | No       | Unix file permissions (octal)            | `"0644"` |

#### Security Validation

V3 includes automatic security validation:

- **Path traversal prevention**: Rejects paths with `..` components
- **Permission validation**: Ensures octal format is valid
- **File existence check**: Validates file exists before deployment

### HashiCorp Vault (`source: vault`)

Integrate with HashiCorp Vault for production secret management and rotation.

#### Prerequisites

1. **Vault CLI or API access:**

   ```bash
   # Set environment variables
   export VAULT_ADDR='https://vault.company.com'
   export VAULT_TOKEN='hvs.xxxxx'

   # Verify connection
   vault status
   ```

2. **Or use ~/.vault-token file:**

   ```bash
   echo 'hvs.xxxxx' > ~/.vault-token
   chmod 600 ~/.vault-token
   ```

#### Configuration

```yaml
secrets:
  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: password
    vaultMount: secret # Optional, default: secret
    required: true

  - name: API_SECRET_KEY
    source: vault
    vaultPath: secret/data/sindri/${ENV}/${SERVICE}/api
    vaultKey: secret_key
    required: true
```

#### Configuration Fields

| Field        | Required | Description                                 | Default    |
| ------------ | -------- | ------------------------------------------- | ---------- |
| `vaultPath`  | Yes      | Full KV path (e.g., `secret/data/app/prod`) | -          |
| `vaultKey`   | Yes      | Key within the secret                       | -          |
| `vaultMount` | No       | KV mount point                              | `"secret"` |

#### V3 Vault Features

- **Native async HTTP API**: Uses `vaultrs` crate, no CLI dependency
- **Connection pooling**: Efficient for multiple secrets
- **Token renewal**: Automatic renewal when TTL < 1 hour
- **Multiple auth methods**: Token, AppRole, Kubernetes service accounts
- **Path templating**: Environment variable substitution in paths

#### Authentication Methods

```yaml
# Token-based (VAULT_TOKEN)
# Default, uses environment variable

# AppRole (for CI/CD)
# Set VAULT_ROLE_ID and VAULT_SECRET_ID

# Kubernetes (for K8s deployments)
# Set VAULT_K8S_ROLE and uses service account JWT
```

### S3 Encrypted Storage (`source: s3`)

**New in V3**: Store secrets in S3 with client-side encryption for team collaboration and CI/CD integration.

#### Use Cases

- **Team Secret Sharing**: Pull encrypted secrets from shared S3 bucket
- **CI/CD Integration**: GitHub Actions pulls from S3 instead of platform secrets
- **Multi-Environment**: Single bucket with environment-specific prefixes
- **Secret Rotation**: Centralized updates propagate to all deployments
- **Backup**: S3 as authoritative backup for secrets

#### Encryption Architecture

S3 secrets use **envelope encryption** with defense in depth:

```
Layer 1: S3 Server-Side Encryption (SSE-S3)
Layer 2: Client-Side Encryption (ChaCha20-Poly1305)
Layer 3: Master Key Encryption (age)
Layer 4: IAM/Bucket Policies
Layer 5: TLS in Transit
```

**How it works:**

1. Generate random 256-bit Data Encryption Key (DEK) per secret
2. Encrypt secret value with DEK using ChaCha20-Poly1305
3. Encrypt DEK with Master Key using age encryption
4. Store encrypted DEK + encrypted value in S3
5. Add authenticated metadata (version, timestamp, key ID)

#### Quick Setup

```bash
# Initialize S3 backend
sindri secrets s3 init --bucket my-secrets --region us-east-1

# Output:
# Generating master key...
# Master key saved to .sindri-master.key
# Public key: age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p
# Bucket exists and is accessible
# S3 backend initialized
```

#### Configuration

```yaml
secrets:
  backend:
    type: s3
    bucket: my-sindri-secrets
    region: us-east-1
    endpoint: https://s3.amazonaws.com # Optional, for S3-compatible
    prefix: secrets/prod/
    encryption:
      algorithm: chacha20poly1305
      key_source: file
      key_file: ~/.sindri/master.key
    cache:
      enabled: true
      ttl_seconds: 3600
      path: ~/.sindri/cache/secrets/

  secrets:
    - name: ANTHROPIC_API_KEY
      source: s3
      s3_path: anthropic/api-key
      required: true

    - name: DATABASE_PASSWORD
      source: s3
      s3_path: database/password
      fallback: env # Fall back to env if S3 unavailable
      required: true
```

#### Master Key Management

**Tier 1: Environment Variable** (Development)

```yaml
encryption:
  key_source: env
  key_env: SINDRI_MASTER_KEY
```

**Tier 2: File-Based** (Production)

```yaml
encryption:
  key_source: file
  key_file: ~/.sindri/master.key
```

**Tier 3: AWS KMS** (Enterprise - Future)

```yaml
encryption:
  key_source: kms
  kms_key_id: arn:aws:kms:us-east-1:123456789012:key/abc-def
```

#### S3-Compatible Storage

Works with any S3-compatible storage:

```yaml
# AWS S3 (default)
backend:
  type: s3
  bucket: my-secrets
  region: us-east-1

# MinIO
backend:
  type: s3
  bucket: my-secrets
  region: us-east-1
  endpoint: http://minio.local:9000

# DigitalOcean Spaces
backend:
  type: s3
  bucket: my-secrets
  region: nyc3
  endpoint: https://nyc3.digitaloceanspaces.com

# Wasabi
backend:
  type: s3
  bucket: my-secrets
  region: us-east-1
  endpoint: https://s3.us-east-1.wasabisys.com
```

## Configuration Reference

### Secret Object Schema

```yaml
secrets:
  - name: string # Required: Environment variable name
    source: env|file|vault|s3 # Required: Secret source type
    required: boolean # Optional: Fail if missing (default: false)

    # For source: env (optional)
    fromFile: string # Read value from file content (supports ~ expansion)

    # For source: file
    path: string # Required: Local file path
    mountPath: string # Required: Container destination path
    permissions: string # Optional: Unix permissions (default: "0644")

    # For source: vault
    vaultPath: string # Required: Vault KV path
    vaultKey: string # Required: Key within secret
    vaultMount: string # Optional: Mount point (default: "secret")

    # For source: s3
    s3_path: string # Required: Path within S3 prefix
    fallback: env|vault # Optional: Fallback source if S3 unavailable
```

### Complete Example

```yaml
version: "3"
name: my-production-app

deployment:
  provider: fly
  resources:
    memory: 4GB
    cpus: 2

secrets:
  # S3 backend configuration (optional)
  backend:
    type: s3
    bucket: my-team-secrets
    region: us-east-1
    prefix: secrets/prod/
    encryption:
      algorithm: chacha20poly1305
      key_source: file
      key_file: .sindri-master.key
    cache:
      enabled: true
      ttl_seconds: 3600

  secrets:
    # API keys from S3 (with env fallback)
    - name: ANTHROPIC_API_KEY
      source: s3
      s3_path: anthropic/api-key
      fallback: env
      required: true

    # Production secrets from Vault
    - name: DATABASE_PASSWORD
      source: vault
      vaultPath: secret/data/sindri/prod/database
      vaultKey: password
      required: true

    # TLS certificates from files
    - name: TLS_CERT
      source: file
      path: ./certs/production-tls.crt
      mountPath: /etc/ssl/certs/app.crt

    - name: TLS_KEY
      source: file
      path: ./certs/production-tls.key
      mountPath: /etc/ssl/private/app.key
      permissions: "0600"

    # Non-sensitive config from env
    - name: LOG_LEVEL
      source: env
      required: false

    # SSH key content from file
    - name: AUTHORIZED_KEYS
      source: env
      fromFile: ~/.ssh/id_ed25519.pub
```

## CLI Commands

### General Commands

#### Validate Secrets

```bash
sindri secrets validate

# Output:
# Validating secrets from sindri.yaml...
# [OK] ANTHROPIC_API_KEY (env): Found in .env.local
# [OK] GITHUB_TOKEN (env): Found in shell environment
# [OK] TLS_CERT (file): Found at ./certs/tls.crt (1.2 KB)
# [OK] DATABASE_PASSWORD (vault): Retrieved successfully
# [WARN] OPTIONAL_KEY (env): Not found (optional, will not be set)
```

#### List Configured Secrets

```bash
sindri secrets list

# Output:
# Configured secrets in sindri.yaml:
#
# Environment Variables (source: env):
#   - ANTHROPIC_API_KEY (required)
#   - GITHUB_TOKEN (required)
#   - LOG_LEVEL
#
# Files (source: file):
#   - TLS_CERT -> /etc/ssl/certs/app.crt (0644)
#   - TLS_KEY -> /etc/ssl/private/app.key (0600)
#
# Vault (source: vault):
#   - DATABASE_PASSWORD <- secret/data/sindri/prod/database:password (required)
#
# S3 (source: s3):
#   - API_SECRET <- anthropic/api-key (required)
```

#### Test Vault Connection

```bash
sindri secrets test-vault

# Output:
# Testing Vault connection...
# [OK] VAULT_ADDR set: https://vault.company.com
# [OK] VAULT_TOKEN available
# [OK] Vault connection successful
# [OK] Token valid (TTL: 3600s)
```

### S3 Commands

#### Initialize S3 Backend

```bash
sindri secrets s3 init \
  --bucket my-secrets \
  --region us-east-1 \
  --create-bucket \
  --output sindri-s3.yaml

# Options:
#   --bucket        S3 bucket name (required)
#   --region        AWS region (required)
#   --endpoint      Custom S3-compatible endpoint
#   --key-file      Master key path (default: .sindri-master.key)
#   --create-bucket Create bucket if not exists
#   --output        Write config to file
```

#### Generate Master Key

```bash
sindri secrets s3 keygen --output ~/.sindri/master.key

# Output:
# Generate Master Key
# Output: /home/user/.sindri/master.key
# Algorithm: age X25519
#
# Generating age keypair...
# Master key generated successfully
# Public key: age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p
#
# IMPORTANT: Keep this key secure and backed up!
# Add to .gitignore to prevent committing to version control
#
#   echo '.sindri-master.key' >> .gitignore
```

#### Push Secret to S3

```bash
# Push from value
sindri secrets s3 push ANTHROPIC_API_KEY --value "sk-ant-api03-..."

# Push from file
sindri secrets s3 push TLS_CERT --from-file ./certs/tls.crt

# Push from stdin
cat secret.txt | sindri secrets s3 push MY_SECRET --stdin

# Push with custom S3 path
sindri secrets s3 push DATABASE_PASSWORD \
  --value "super_secret" \
  --s3-path database/main/password

# Force overwrite existing
sindri secrets s3 push API_KEY --value "new_key" --force
```

#### Pull Secret from S3

```bash
# Pull and display (with confirmation)
sindri secrets s3 pull ANTHROPIC_API_KEY --show

# Export as environment variable
eval $(sindri secrets s3 pull ANTHROPIC_API_KEY --export)

# Write to file
sindri secrets s3 pull TLS_CERT --output ./certs/tls.crt
```

#### Sync Secrets

```bash
# Preview sync (dry run)
sindri secrets s3 sync --dry-run

# Push local to remote
sindri secrets s3 sync --direction push

# Pull remote to local
sindri secrets s3 sync --direction pull

# Bidirectional sync
sindri secrets s3 sync --direction both
```

#### Rotate Master Key

```bash
# Generate new key
sindri secrets s3 keygen --output ~/.sindri/new-master.key

# Rotate secrets to new key
sindri secrets s3 rotate \
  --new-key ~/.sindri/new-master.key \
  --old-key ~/.sindri/master.key

# Add new key without removing old (gradual rollout)
sindri secrets s3 rotate \
  --new-key ~/.sindri/new-master.key \
  --add-only
```

## Provider Integration

### Fly.io

**Mechanism:** `flyctl secrets` command

```bash
# Sindri automatically runs:
flyctl secrets import -a my-app < /dev/shm/sindri-secrets-XXXXX

# For files:
flyctl secrets set TLS_CERT_BASE64=$(base64 < ./certs/tls.crt) -a my-app
```

**File secrets:**

- Encoded as base64 and stored as environment variable
- Decoded in container entrypoint to proper mount path
- Permissions set after decoding

### Docker Compose

**Mechanism:** `env_file` + Docker secrets

```yaml
# Generated docker-compose.yml
services:
  sindri:
    env_file:
      - .env.secrets # Generated from env + vault sources
    secrets:
      - source: tls_cert
        target: /etc/ssl/certs/app.crt
        mode: 0600

secrets:
  tls_cert:
    file: ./certs/tls.crt
```

### Kubernetes

**Mechanism:** Kubernetes `Secret` resources

```yaml
# Environment variable secrets
apiVersion: v1
kind: Secret
metadata:
  name: sindri-secrets
type: Opaque
data:
  ANTHROPIC_API_KEY: c2stYW50LWFwaTA... # base64 encoded

---
# File secrets
apiVersion: v1
kind: Secret
metadata:
  name: sindri-files
type: Opaque
data:
  tls.crt: LS0tLS1CRUdJTi... # base64 encoded
```

### DevPod / DevContainers

**Mechanism:** `containerEnv` and bind mounts

```json
{
  "containerEnv": {
    "ANTHROPIC_API_KEY": "${localEnv:ANTHROPIC_API_KEY}",
    "GITHUB_TOKEN": "${localEnv:GITHUB_TOKEN}"
  },
  "mounts": ["source=${localEnv:HOME}/.ssh,target=/home/developer/.ssh,type=bind,readonly"]
}
```

## Security Best Practices

### 1. Never Commit Secrets

**Add to `.gitignore`:**

```gitignore
# Local environment files
.env.local
.env.*.local

# Master keys
.sindri-master.key
*.key

# Certificate private keys
/certs/*.key
/certs/*.pem

# Secret directories
/secrets/

# Generated files
.env.secrets
```

### 2. Use `.env.local` for Personal Secrets

```bash
# .env (committed - safe defaults)
NODE_ENV=development
LOG_LEVEL=debug

# .env.local (gitignored - personal secrets)
ANTHROPIC_API_KEY=sk-ant-api03-...
GITHUB_TOKEN=ghp_...
DATABASE_PASSWORD=local_dev_password
```

### 3. Mark Production Secrets as Required

```yaml
secrets:
  - name: DATABASE_PASSWORD
    source: vault
    required: true # Deployment fails if not available
```

### 4. Use Restrictive Permissions for Files

```yaml
secrets:
  # Private keys should be 0600
  - name: SSH_PRIVATE_KEY
    source: file
    path: ~/.ssh/id_ed25519
    mountPath: /home/developer/.ssh/id_ed25519
    permissions: "0600"

  # Certificates can be 0644
  - name: TLS_CERT
    source: file
    path: ./certs/tls.crt
    mountPath: /etc/ssl/certs/app.crt
    permissions: "0644"
```

### 5. Rotate Secrets Regularly

**For S3:**

```bash
# Generate new master key
sindri secrets s3 keygen --output ~/.sindri/new-master.key

# Rotate to new key
sindri secrets s3 rotate --new-key ~/.sindri/new-master.key
```

**For Vault:**

```bash
# Vault supports automatic rotation and versioning
vault kv put secret/sindri/prod/database password=new_password
```

### 6. Use Vault or S3 for Production

| Environment | Recommended Source         |
| ----------- | -------------------------- |
| Development | `.env` files               |
| Staging     | S3 or Vault                |
| Production  | **Always use Vault or S3** |

### 7. V3 Memory Safety

V3 automatically zeros secret memory on drop using the `zeroize` crate:

```rust
// SecretValue implements ZeroizeOnDrop
// Secrets are automatically cleared from memory when no longer needed
pub enum SecretValue {
    Env(String),  // Automatically zeroed
    File { content: Vec<u8>, ... },  // Automatically zeroed
}
```

This prevents secrets from lingering in memory (heap inspection attacks).

## Examples

### Pattern 1: SSH Key Management

```yaml
# For SSH INTO the container (public key):
secrets:
  - name: AUTHORIZED_KEYS
    source: env
    fromFile: ~/.ssh/id_ed25519.pub  # Public key content -> env var

# For SSH OUT FROM the container (private key):
secrets:
  - name: DEPLOY_SSH_KEY
    source: file
    path: ~/.ssh/deploy_key_ed25519  # Private key
    mountPath: /home/developer/.ssh/id_ed25519
    permissions: "0600"
```

### Pattern 2: Multi-Environment with S3

```yaml
# Development (local .env)
secrets:
  - name: DATABASE_PASSWORD
    source: env

# Production (S3 with Vault fallback)
secrets:
  backend:
    type: s3
    bucket: prod-secrets
    prefix: secrets/prod/

  secrets:
    - name: DATABASE_PASSWORD
      source: s3
      s3_path: database/password
      fallback: vault
```

### Pattern 3: CI/CD with S3

```yaml
# .github/workflows/deploy.yml
name: Deploy to Fly.io

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Decode master key
        run: |
          echo "${{ secrets.SINDRI_MASTER_KEY }}" | base64 -d > .sindri-master.key
          chmod 600 .sindri-master.key

      - name: Pull secrets from S3
        run: sindri secrets s3 sync --direction pull
        env:
          AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}

      - name: Deploy
        run: sindri deploy --provider fly
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```

### Pattern 4: Team Secret Sharing with S3

```bash
# Team lead initializes
sindri secrets s3 init --bucket team-secrets --region us-east-1
sindri secrets s3 push ANTHROPIC_API_KEY --value "sk-ant-..."

# Share master key securely (1Password, encrypted email, etc.)
# DO NOT commit to git

# Team member onboards
# Save master key to ~/.sindri/master.key (0600)
sindri secrets s3 sync
sindri deploy
```

### Pattern 5: Certificate Management

```yaml
secrets:
  # Public certificate
  - name: TLS_CERT
    source: file
    path: ./certs/server.crt
    mountPath: /etc/ssl/certs/server.crt
    permissions: "0644"

  # Private key (highly sensitive)
  - name: TLS_KEY
    source: file
    path: ./certs/server.key
    mountPath: /etc/ssl/private/server.key
    permissions: "0600"

  # CA bundle
  - name: CA_BUNDLE
    source: file
    path: ./certs/ca-bundle.crt
    mountPath: /etc/ssl/certs/ca-bundle.crt
    permissions: "0644"
```

## Troubleshooting

### Secret Not Found

**Error:**

```text
Error: Required secret ANTHROPIC_API_KEY not found
Checked: $ANTHROPIC_API_KEY, .env.local, .env
```

**Solutions:**

1. Check environment variables: `echo $ANTHROPIC_API_KEY`
2. Check .env files: `cat .env.local`
3. Add to .env.local: `echo "ANTHROPIC_API_KEY=sk-ant-..." >> .env.local`

### File Not Found

**Error:**

```text
Error: Secret file not found: ./certs/tls.crt
```

**Solutions:**

1. Check file exists: `ls -la ./certs/tls.crt`
2. Check path in sindri.yaml is relative to project root
3. Generate certificate if missing

### Vault Authentication Failed

**Error:**

```text
Error: Failed to retrieve DATABASE_PASSWORD from Vault
Path: secret/data/sindri/prod/database, Key: password
```

**Solutions:**

1. Check Vault environment: `echo $VAULT_ADDR $VAULT_TOKEN`
2. Test connection: `vault status`
3. Verify path: `vault kv get secret/sindri/prod/database`
4. Check token expiration: `vault token lookup`

### S3 Access Denied

**Error:**

```text
Error: Failed to access S3 bucket: Access Denied
```

**Solutions:**

1. Check AWS credentials: `aws sts get-caller-identity`
2. Verify bucket exists: `aws s3 ls s3://my-secrets/`
3. Check bucket policy allows access
4. Verify region is correct

### S3 Decryption Failed

**Error:**

```text
Error: Failed to decrypt secret: age decryption failed
```

**Solutions:**

1. Verify correct master key: `sindri secrets s3 pull SECRET --show`
2. Check key file permissions: `ls -la ~/.sindri/master.key`
3. Re-push secret with correct key: `sindri secrets s3 push SECRET --force`

### Permission Denied in Container

**Error:**

```text
Permission denied: /etc/ssl/private/app.key
```

**Solutions:**

1. Check permissions in sindri.yaml:

   ```yaml
   permissions: "0600" # Must be string with quotes
   ```

2. Verify mount path is writable by container user

### Cache Issues

**Error:**

```text
Warning: Using stale cached secret (cache expired)
```

**Solutions:**

1. Clear cache: `rm -rf ~/.sindri/cache/secrets/`
2. Force refresh: `sindri secrets s3 sync --force`
3. Adjust TTL in configuration

## Migration Guide

### Migrating from V2 to V3

V3 is backwards compatible with V2 sindri.yaml configurations. No changes required for basic usage.

### Migrating to S3 Encrypted Storage

**Phase 1: Setup**

```bash
sindri secrets s3 init --bucket my-team-secrets --region us-east-1
```

**Phase 2: Migrate Secrets**

```bash
# Push all secrets from .env.local
sindri secrets s3 push-from-env .env.local

# Or individually
sindri secrets s3 push ANTHROPIC_API_KEY
```

**Phase 3: Update sindri.yaml**

```yaml
secrets:
  backend:
    type: s3
    bucket: my-team-secrets
    region: us-east-1
    encryption:
      key_source: file
      key_file: .sindri-master.key

  secrets:
    - name: ANTHROPIC_API_KEY
      source: s3
      s3_path: anthropic/api-key
      fallback: env # Gradual migration
      required: true
```

**Phase 4: Team Onboarding**

```bash
# Distribute master key securely
# Team members save to ~/.sindri/master.key (0600)
sindri secrets s3 sync
sindri deploy
```

## See Also

- [ADR-015: Secrets Resolver Core Architecture](./architecture/adr/015-secrets-resolver-core-architecture.md)
- [ADR-016: Vault Integration Architecture](./architecture/adr/016-vault-integration-architecture.md)
- [ADR-020: S3 Encrypted Secret Storage](./architecture/adr/020-s3-encrypted-secret-storage.md)
- [Getting Started](./getting-started.md)
