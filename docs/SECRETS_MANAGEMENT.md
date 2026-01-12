# Secrets Management

Comprehensive guide to managing secrets in Sindri across all deployment providers.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Secret Sources](#secret-sources)
  - [Environment Variables](#environment-variables-source-env)
  - [Files](#files-source-file)
  - [HashiCorp Vault](#hashicorp-vault-source-vault)
- [Provider-Specific Behavior](#provider-specific-behavior)
- [Configuration Reference](#configuration-reference)
- [Security Best Practices](#security-best-practices)
- [Extension-Specific API Keys](#extension-specific-api-keys)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)

## Overview

Sindri provides a unified, declarative approach to secrets management that works consistently across all deployment providers (Fly.io, Docker, Kubernetes, DevPod).

### Design Principles

1. **Zero Configuration Default** - Works out of the box with `.env` files
2. **Progressive Enhancement** - Supports advanced sources (Vault, files) when needed
3. **Provider Awareness** - Leverages each provider's native secret management
4. **Security by Default** - Never logs or exposes secrets
5. **Developer Friendly** - Clear feedback, helpful errors, validation

### Supported Secret Sources

| Source  | Use Case                             | Example                     |
| ------- | ------------------------------------ | --------------------------- |
| `env`   | API keys, tokens, passwords          | `.env` files, shell exports |
| `file`  | Certificates, SSH keys, config files | TLS certs, private keys     |
| `vault` | Production secrets, rotation         | HashiCorp Vault KV store    |

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

**3. Deploy:**

```bash
sindri deploy
# Secrets automatically resolved from .env.local and .env
```

That's it! Sindri will automatically:

- Read secrets from `.env.local` (first priority)
- Fall back to `.env` if not found
- Inject them into your deployment using provider-native mechanisms

## Secret Sources

### Environment Variables (`source: env`)

Resolve secrets from environment variables, `.env` files, shell exports, or local files.

#### Resolution Priority

Secrets are resolved in this order (highest priority first):

1. **Shell environment variables** - `export ANTHROPIC_API_KEY=...`
2. **`.env.local`** - Gitignored, personal secrets
3. **`.env`** - Committed, shared defaults
4. **`fromFile`** - Read content from a local file (if specified)

#### Environment Variable Configuration

```yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: env

  - name: DATABASE_PASSWORD
    source: env
    required: true # Fail deployment if missing
```

#### The `fromFile` Property

Use `fromFile` to read secret content directly from a local file. This is ideal for SSH public keys and other file-based credentials where you don't want to manually `export` or maintain `.env` files:

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
| **Purpose**        | Read file content → env var        | Mount file into container  |
| **Result**         | Sets environment variable          | File exists at `mountPath` |
| **Use case**       | SSH public keys, API keys in files | Certificates, config files |
| **Container path** | N/A                                | Required via `mountPath`   |

**When to use `fromFile`:**

- SSH authorized keys (`AUTHORIZED_KEYS`)
- API keys stored in files
- Any secret where you need the _content_ as an environment variable

**When to use `source: file`:**

- TLS certificates that apps read from disk
- Config files that must exist at specific paths
- Private keys for outbound SSH connections

#### Environment Variable Usage Patterns

- **Pattern 1: Local development with .env files**

```bash
# .env.local (gitignored)
ANTHROPIC_API_KEY=sk-ant-api03-xxx
GITHUB_TOKEN=ghp_xxx

sindri deploy --provider docker
```

- **Pattern 2: CI/CD with environment variables**

```bash
# GitHub Actions / CI environment
export ANTHROPIC_API_KEY="${{ secrets.ANTHROPIC_API_KEY }}"
export DATABASE_PASSWORD="${{ secrets.DB_PASSWORD }}"

sindri deploy --provider fly
```

- **Pattern 3: Runtime override**

```bash
# Override specific secret at deploy time
GITHUB_TOKEN=ghp_override_token sindri deploy
```

#### Required Secrets

Mark secrets as required to fail early if missing:

```yaml
secrets:
  - name: DATABASE_PASSWORD
    source: env
    required: true # Deployment fails if not found

  - name: OPTIONAL_API_KEY
    source: env
    required: false # Warning only, deployment continues
```

**Error output:**

```bash
sindri deploy
# ✗ Error: Required secret DATABASE_PASSWORD not found
# Checked: $DATABASE_PASSWORD, .env.local, .env
# Hint: Add to .env.local or export DATABASE_PASSWORD=...
```

### Files (`source: file`)

Use files for certificates, SSH keys, and configuration files that need to be mounted in the container.

#### File Source Configuration

```yaml
secrets:
  - name: TLS_CERT
    source: file
    path: ./certs/tls.crt
    mountPath: /etc/ssl/certs/app.crt

  - name: SSH_PRIVATE_KEY
    source: file
    path: ~/.ssh/id_ed25519
    mountPath: /home/developer/.ssh/id_ed25519
    permissions: "0600" # File permissions in container
```

#### File Configuration Fields

| Field         | Required | Description                              | Default  |
| ------------- | -------- | ---------------------------------------- | -------- |
| `path`        | Yes      | Local file path (supports `~` expansion) | -        |
| `mountPath`   | Yes      | Destination path in container            | -        |
| `permissions` | No       | Unix file permissions (octal)            | `"0644"` |

#### File Usage Patterns

- **Pattern 1: TLS Certificates**

```yaml
secrets:
  - name: TLS_CERT
    source: file
    path: ./certs/production-tls.crt
    mountPath: /etc/ssl/certs/app.crt

  - name: TLS_KEY
    source: file
    path: ./certs/production-tls.key
    mountPath: /etc/ssl/private/app.key
    permissions: "0600" # Private key - restrictive permissions
```

- **Pattern 2: SSH Deploy Key**

```yaml
secrets:
  - name: DEPLOY_SSH_KEY
    source: file
    path: ~/.ssh/deploy_key_ed25519
    mountPath: /home/developer/.ssh/id_ed25519
    permissions: "0600"
```

- **Pattern 3: Application Config**

```yaml
secrets:
  - name: APP_CONFIG
    source: file
    path: ./config/production.json
    mountPath: /workspace/config/production.json
    permissions: "0644"
```

#### File Provider Handling

- **Fly.io**: File content encoded as base64, stored as secret, decoded at container start
- **Docker**: Mounted as Docker secret with specified permissions
- **Kubernetes**: Stored in Secret resource, mounted as volume
- **DevPod**: Bind-mounted from local filesystem

### HashiCorp Vault (`source: vault`)

Integrate with HashiCorp Vault for production secret management and rotation.

#### Prerequisites

1. **Install Vault CLI:**

   ```bash
   # macOS
   brew install vault

   # Linux
   wget https://releases.hashicorp.com/vault/1.15.0/vault_1.15.0_linux_amd64.zip
   unzip vault_1.15.0_linux_amd64.zip
   sudo mv vault /usr/local/bin/
   ```

2. **Configure Vault access:**

   ```bash
   export VAULT_ADDR='https://vault.company.com'
   export VAULT_TOKEN='hvs.xxxxx'

   # Or use ~/.vault-token file
   echo 'hvs.xxxxx' > ~/.vault-token
   ```

3. **Verify connection:**

   ```bash
   vault status
   ```

#### Vault Source Configuration

```yaml
secrets:
  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: password
    vaultMount: secret # Optional, default: secret

  - name: API_SECRET_KEY
    source: vault
    vaultPath: secret/data/sindri/prod/api
    vaultKey: secret_key
    required: true
```

#### Vault Configuration Fields

| Field        | Required | Description                                 | Default    |
| ------------ | -------- | ------------------------------------------- | ---------- |
| `vaultPath`  | Yes      | Full KV path (e.g., `secret/data/app/prod`) | -          |
| `vaultKey`   | Yes      | Key within the secret                       | -          |
| `vaultMount` | No       | KV mount point                              | `"secret"` |

#### Understanding Vault Path Structure

**KV v2 (recommended):**

```text
secret/data/sindri/prod/database
└─┬──┘ └┬─┘ └────┬────┘ └──┬───┘
  │     │        │          └─ Secret name
  │     │        └─ Environment
  │     └─ "data" for KV v2
  └─ Mount point
```

**Example Vault setup:**

```bash
# Write secret to Vault
vault kv put secret/sindri/prod/database \
  password=super_secret_password \
  host=db.example.com \
  port=5432

# Read secret (for testing)
vault kv get -mount=secret -field=password secret/sindri/prod/database
```

#### Vault Usage Patterns

- **Pattern 1: Database credentials**

```yaml
secrets:
  - name: DATABASE_URL
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: url

  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: password
    required: true
```

- **Pattern 2: Multi-environment**

```yaml
# Development (uses .env)
secrets:
  - name: API_KEY
    source: env
# Production (uses Vault)
# Override sindri.yaml per environment or use conditionals
```

#### Pattern 3: Mixed sources

```yaml
secrets:
  # Production secrets from Vault
  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/prod/db
    vaultKey: password

  # Certificates from files
  - name: TLS_CERT
    source: file
    path: ./certs/prod.crt
    mountPath: /etc/ssl/certs/app.crt

  # Non-sensitive config from env
  - name: LOG_LEVEL
    source: env
```

## Provider-Specific Behavior

### Fly.io

**Mechanism:** `flyctl secrets` command

**How it works:**

1. Sindri resolves all secrets from configured sources
2. Generates temporary secrets file (in-memory)
3. Runs `flyctl secrets import` to set secrets atomically
4. Cleans up temporary file

**File secrets:**

- Encoded as base64 and stored as environment variable
- Decoded in container entrypoint to proper mount path
- Permissions set after decoding

**Example:**

```bash
# Sindri automatically runs:
flyctl secrets import -a my-app < /dev/shm/sindri-secrets-XXXXX

# For files:
flyctl secrets set TLS_CERT_BASE64=$(base64 < ./certs/tls.crt) -a my-app
```

### Docker Compose

**Mechanism:** `env_file` + Docker secrets

**How it works:**

1. Environment secrets → `.env.secrets` (gitignored, generated)
2. File secrets → Docker secrets section
3. Generated `docker-compose.yml` includes both

**Generated config:**

```yaml
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

**How it works:**

1. Creates `Secret` resource with all secrets
2. Mounts as environment variables or volumes
3. Applies RBAC for secret access

**Generated resources:**

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

---
# Pod using secrets
apiVersion: v1
kind: Pod
spec:
  containers:
    - name: sindri
      envFrom:
        - secretRef:
            name: sindri-secrets
      volumeMounts:
        - name: tls-cert
          mountPath: /etc/ssl/certs/app.crt
          subPath: tls.crt
          readOnly: true
  volumes:
    - name: tls-cert
      secret:
        secretName: sindri-files
        defaultMode: 0600
```

### DevPod / DevContainers

**Mechanism:** `containerEnv` and bind mounts

**How it works:**

1. Reads from local environment/files
2. Generates `devcontainer.json` with proper references
3. VS Code/DevPod handles injection

**Generated config:**

```json
{
  "containerEnv": {
    "ANTHROPIC_API_KEY": "${localEnv:ANTHROPIC_API_KEY}",
    "GITHUB_TOKEN": "${localEnv:GITHUB_TOKEN}"
  },
  "mounts": ["source=${localEnv:HOME}/.ssh,target=/home/developer/.ssh,type=bind,readonly"]
}
```

## Configuration Reference

### Secret Object Schema

```yaml
secrets:
  - name: string # Required: Environment variable name
    source: env|file|vault # Required: Secret source type
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
```

### Complete Example

```yaml
version: 1.0
name: my-production-app

deployment:
  provider: fly
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: enterprise

secrets:
  # API keys from Vault
  - name: ANTHROPIC_API_KEY
    source: vault
    vaultPath: secret/data/sindri/prod/anthropic
    vaultKey: api_key
    required: true

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

  # SSH key for git operations
  - name: DEPLOY_SSH_KEY
    source: file
    path: ~/.ssh/deploy_key_ed25519
    mountPath: /home/developer/.ssh/id_ed25519
    permissions: "0600"

  # Non-sensitive config from env
  - name: LOG_LEVEL
    source: env
    required: false

  - name: ENVIRONMENT
    source: env
    required: true
```

## Security Best Practices

### 1. Never Commit Secrets

**Add to `.gitignore`:**

```gitignore
# Local environment files
.env.local
.env.*.local

# Certificate private keys
/certs/*.key
/certs/*.pem

# Secret directories
/secrets/

# Generated files
.env.secrets
```

**Sindri automatically generates this during `sindri config init`.**

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

**For Vault (recommended):**

Vault supports automatic rotation and versioning.

**For environment variables:**

```bash
# Update .env.local
vim .env.local

# Redeploy
sindri deploy
```

**For Fly.io:**

```bash
# Update secret
flyctl secrets set ANTHROPIC_API_KEY=new_key -a my-app

# Restart to apply
flyctl apps restart my-app
```

### 6. Validate Before Deploying

```bash
# Validate secrets configuration
sindri secrets validate

# Output:
# ✓ ANTHROPIC_API_KEY (env): Found in .env.local
# ✓ TLS_CERT (file): Found at ./certs/tls.crt (1.2 KB)
# ✓ DATABASE_PASSWORD (vault): Retrieved from vault:secret/data/sindri/prod/db
# ⚠ OPTIONAL_API_KEY (env): Not found (optional, continuing)
```

### 7. Use Vault for Production

Development → `.env` files
Staging → Vault or provider secrets
Production → **Always use Vault**

### 8. Audit Secret Access

**Vault provides audit logs:**

```bash
# Enable audit device
vault audit enable file file_path=/var/log/vault_audit.log

# Review access
cat /var/log/vault_audit.log | jq '.request.path'
```

**Kubernetes RBAC:**

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: secret-reader
rules:
  - apiGroups: [""]
    resources: ["secrets"]
    resourceNames: ["sindri-secrets"] # Specific secret only
    verbs: ["get"]
```

## Extension-Specific API Keys

Many Sindri extensions require API keys or tokens to function. This reference lists all extensions that need secrets configured.

### AI & Machine Learning

| Extension                  | Secret Name             | Description                              | Where to Get                                                  |
| -------------------------- | ----------------------- | ---------------------------------------- | ------------------------------------------------------------- |
| `claude-auth-with-api-key` | `ANTHROPIC_API_KEY`     | Claude API access                        | [Anthropic Console](https://console.anthropic.com/)           |
| `vf-gemini-flow`           | `GOOGLE_GEMINI_API_KEY` | Google Gemini multi-agent orchestration  | [Google AI Studio](https://makersuite.google.com/app/apikey)  |
| `vf-perplexity`            | `PERPLEXITY_API_KEY`    | Perplexity AI real-time web research     | [Perplexity Settings](https://www.perplexity.ai/settings/api) |
| `vf-deepseek-reasoning`    | `DEEPSEEK_API_KEY`      | DeepSeek AI reasoning MCP server         | [DeepSeek Platform](https://platform.deepseek.com/)           |
| `vf-zai-service`           | `ZAI_ANTHROPIC_API_KEY` | Cost-effective Claude API wrapper        | [Anthropic Console](https://console.anthropic.com/)           |
| `vf-ontology-enrich`       | `PERPLEXITY_API_KEY`    | AI ontology enrichment (uses Perplexity) | [Perplexity Settings](https://www.perplexity.ai/settings/api) |
| `ai-toolkit`               | `GOOGLE_GEMINI_API_KEY` | Multi-AI toolkit (Gemini support)        | [Google AI Studio](https://makersuite.google.com/app/apikey)  |
| `claudish`                 | `OPENROUTER_API_KEY`    | OpenRouter multi-provider gateway        | [OpenRouter Keys](https://openrouter.ai/keys)                 |

### Project Management

| Extension    | Secret Name      | Description                              | Where to Get                                                                        |
| ------------ | ---------------- | ---------------------------------------- | ----------------------------------------------------------------------------------- |
| `linear-mcp` | `LINEAR_API_KEY` | Linear.app AI-powered project management | [Linear API Settings](https://linear.app/settings/api)                              |
| `jira-mcp`   | `JIRA_URL`       | Atlassian Jira base URL                  | Your Jira instance (e.g., `https://company.atlassian.net`)                          |
| `jira-mcp`   | `JIRA_USERNAME`  | Atlassian account email                  | Your Atlassian account                                                              |
| `jira-mcp`   | `JIRA_API_TOKEN` | Atlassian API token                      | [Atlassian API Tokens](https://id.atlassian.com/manage-profile/security/api-tokens) |

### Package Registries

| Secret Name  | Description                                               | Where to Get                                                                           |
| ------------ | --------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `NPM_TOKEN`  | npm registry auth (bypasses rate limits, **recommended**) | [npm Access Tokens](https://www.npmjs.com/settings/~/tokens) (read-only Classic token) |
| `PYPI_TOKEN` | PyPI for publishing packages                              | [PyPI API Tokens](https://pypi.org/manage/account/token/)                              |

> **Why NPM_TOKEN is recommended:** Anonymous npm requests are heavily rate-limited. During extension installation, mise resolves npm package versions which can timeout (20s default) when rate-limited. Setting `NPM_TOKEN` provides authenticated access with much higher limits, preventing "timed out" errors during tool installation.

### Infrastructure

| Extension           | Secret Name             | Description                                     | Where to Get                                                        |
| ------------------- | ----------------------- | ----------------------------------------------- | ------------------------------------------------------------------- |
| `supabase-cli`      | `SUPABASE_ACCESS_TOKEN` | Supabase CLI authentication                     | [Supabase Dashboard](https://supabase.com/dashboard/account/tokens) |
| `vf-management-api` | `MANAGEMENT_API_KEY`    | VisionFlow task orchestration API (has default) | Self-configured                                                     |

### Example: AI Extensions Configuration

```yaml
secrets:
  # Core Claude access
  - name: ANTHROPIC_API_KEY
    source: env
    required: true

  # Additional AI providers (enable as needed)
  - name: GOOGLE_GEMINI_API_KEY
    source: env
  - name: PERPLEXITY_API_KEY
    source: env
  - name: DEEPSEEK_API_KEY
    source: env
```

### Example: Project Management Configuration

```yaml
secrets:
  # Linear integration
  - name: LINEAR_API_KEY
    source: env

  # Jira integration (requires all three)
  - name: JIRA_URL
    source: env
  - name: JIRA_USERNAME
    source: env
  - name: JIRA_API_TOKEN
    source: env
```

### Example: Package Registries Configuration

```yaml
secrets:
  # npm authentication (recommended for reliable tool installation)
  - name: NPM_TOKEN
    source: env
```

## Common Patterns

### Pattern 0: SSH Key Management

Understanding SSH key direction is critical for secure configuration:

```text
┌─────────────────┐                    ┌─────────────────────┐
│  Your Laptop    │ ──── SSH ────────► │   Sindri Container  │
│                 │                    │                     │
│ Private Key     │    Authenticates   │ Public Key          │
│ ~/.ssh/id_ed25519│    with public key │ AUTHORIZED_KEYS     │
└─────────────────┘                    └─────────────────────┘
```

**For SSH INTO the container (most common):**

- Your laptop keeps the **private key** (`~/.ssh/id_ed25519`)
- Container needs the **public key** (`AUTHORIZED_KEYS`)
- Use `fromFile` to read your public key:

```yaml
secrets:
  - name: AUTHORIZED_KEYS
    source: env
    fromFile: ~/.ssh/id_ed25519.pub # Public key content → env var
```

**For SSH OUT FROM the container (git clone, deploy):**

- Container needs a **private key** for outbound connections
- Mount the private key as a file:

```yaml
secrets:
  # Mount deploy key for git operations inside container
  - name: DEPLOY_SSH_KEY
    source: file
    path: ~/.ssh/deploy_key_ed25519 # Private key
    mountPath: /alt/home/developer/.ssh/id_ed25519
    permissions: "0600" # Restrictive permissions required
```

**Security note:** Never mount your personal private key into a container. Use dedicated deploy keys with limited scope.

### Pattern 1: Multi-Environment Configuration

**Directory structure:**

```text
project/
├── sindri.yaml            # Base configuration
├── sindri.dev.yaml        # Development overrides
├── sindri.prod.yaml       # Production overrides
├── .env                   # Shared defaults
├── .env.local             # Local development secrets
└── certs/
    ├── dev-tls.crt
    └── prod-tls.crt
```

**Development deployment:**

```bash
cp sindri.dev.yaml sindri.yaml
sindri deploy --provider docker
```

**Production deployment:**

```bash
cp sindri.prod.yaml sindri.yaml
sindri deploy --provider fly
```

### Pattern 2: GitHub Actions Integration

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
      - uses: actions/checkout@v6

      - name: Setup secrets
        run: |
          echo "ANTHROPIC_API_KEY=${{ secrets.ANTHROPIC_API_KEY }}" > .env.local
          echo "DATABASE_PASSWORD=${{ secrets.DATABASE_PASSWORD }}" >> .env.local

      - name: Deploy
        run: |
          sindri deploy --provider fly
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```

### Pattern 3: Vault with Multiple Environments

```yaml
# Production
secrets:
  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: password

# Staging
secrets:
  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/staging/database
    vaultKey: password

# Development (no Vault)
secrets:
  - name: DATABASE_PASSWORD
    source: env  # Uses .env.local
```

### Pattern 4: Certificate Management

```yaml
secrets:
  # Public certificate (less sensitive)
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

### Pattern 5: Database Connection Secrets

```yaml
secrets:
  # Option 1: Full connection URL
  - name: DATABASE_URL
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: url

  # Option 2: Individual components
  - name: DB_HOST
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: host

  - name: DB_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: password
    required: true

  - name: DB_USER
    source: env # Non-sensitive

  - name: DB_NAME
    source: env # Non-sensitive
```

## Troubleshooting

### Secret Not Found

**Error:**

```text
✗ Error: Required secret ANTHROPIC_API_KEY not found
Checked: $ANTHROPIC_API_KEY, .env.local, .env
```

**Solutions:**

1. **Check environment variables:**

   ```bash
   echo $ANTHROPIC_API_KEY
   ```

2. **Check .env files:**

   ```bash
   cat .env
   cat .env.local
   ```

3. **Add to .env.local:**

   ```bash
   echo "ANTHROPIC_API_KEY=sk-ant-..." >> .env.local
   ```

### File Not Found

**Error:**

```text
✗ Error: Secret file not found: ./certs/tls.crt
```

**Solutions:**

1. **Check file exists:**

   ```bash
   ls -la ./certs/tls.crt
   ```

2. **Check path in sindri.yaml:**

   ```yaml
   secrets:
     - name: TLS_CERT
       path: ./certs/tls.crt # Relative to project root
   ```

3. **Generate certificate if missing:**

   ```bash
   openssl req -x509 -newkey rsa:4096 -keyout certs/tls.key -out certs/tls.crt -days 365 -nodes
   ```

### Vault Authentication Failed

**Error:**

```text
✗ Error: Failed to retrieve DATABASE_PASSWORD from Vault
Path: secret/data/sindri/prod/database, Key: password
```

**Solutions:**

1. **Check Vault environment variables:**

   ```bash
   echo $VAULT_ADDR
   echo $VAULT_TOKEN
   ```

2. **Test Vault connection:**

   ```bash
   vault status
   vault kv get -mount=secret -field=password secret/sindri/prod/database
   ```

3. **Verify Vault path:**

   ```bash
   # List secrets at path
   vault kv list secret/sindri/prod

   # Get full secret
   vault kv get secret/sindri/prod/database
   ```

4. **Check Vault token expiration:**

   ```bash
   vault token lookup
   ```

### Permission Denied in Container

**Error:**

```text
Permission denied: /etc/ssl/private/app.key
```

**Solutions:**

1. **Check file permissions in sindri.yaml:**

   ```yaml
   secrets:
     - name: TLS_KEY
       permissions: "0600" # Must be string with quotes
   ```

2. **Verify file ownership:**

   Files are owned by `developer` user (uid 1001). Ensure mount path is writable:

   ```yaml
   # Good: User writable locations
   mountPath: /home/developer/.ssh/id_ed25519
   mountPath: /workspace/config/secret.json

   # Bad: System locations (may need entrypoint script)
   mountPath: /etc/ssl/private/app.key  # Needs special handling
   ```

### Secret Not Updating in Fly.io

**Issue:** Changed secret but container still has old value.

**Solution:**

Fly.io caches secrets. Force restart:

```bash
# Set secret
flyctl secrets set ANTHROPIC_API_KEY=new_value -a my-app

# Restart (automatically triggered by secret change)
# But if not, manually restart:
flyctl apps restart my-app
```

### Base64 Encoding Issues (File Secrets on Fly.io)

**Issue:** File content corrupted when decoded.

**Solution:**

Verify base64 encoding/decoding:

```bash
# Encode
base64 < ./certs/tls.crt

# Decode and verify
echo "..." | base64 -d | diff - ./certs/tls.crt
```

Sindri handles this automatically, but for manual debugging:

```bash
# Check secret in Fly.io
flyctl ssh console -a my-app
echo $TLS_CERT_BASE64 | base64 -d
```

## CLI Commands

### Validate Secrets

```bash
sindri secrets validate

# Output:
# Validating secrets from sindri.yaml...
# ✓ ANTHROPIC_API_KEY (env): Found in .env.local
# ✓ GITHUB_TOKEN (env): Found in shell environment
# ✓ TLS_CERT (file): Found at ./certs/tls.crt (1.2 KB)
# ✓ DATABASE_PASSWORD (vault): Retrieved successfully
# ⚠ OPTIONAL_KEY (env): Not found (optional, will not be set)
```

### List Configured Secrets

```bash
sindri secrets list

# Output:
# Configured secrets in sindri.yaml:
#
# Environment Variables (source: env):
#   • ANTHROPIC_API_KEY (required)
#   • GITHUB_TOKEN (required)
#   • LOG_LEVEL
#
# Files (source: file):
#   • TLS_CERT → /etc/ssl/certs/app.crt (0644)
#   • TLS_KEY → /etc/ssl/private/app.key (0600)
#
# Vault (source: vault):
#   • DATABASE_PASSWORD ← secret/data/sindri/prod/database:password (required)
```

### Test Vault Connection

```bash
sindri secrets test-vault

# Output:
# Testing Vault connection...
# ✓ Vault CLI installed
# ✓ VAULT_ADDR set: https://vault.company.com
# ✓ VAULT_TOKEN set
# ✓ Vault connection successful
# ✓ KV v2 engine enabled at secret/
#
# Testing secret retrieval:
# ✓ secret/data/sindri/prod/database:password
```

### Encode File for Manual Setting

```bash
sindri secrets encode-file ./certs/tls.crt

# Output:
# Base64-encoded content (copy for manual secret setting):
# LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t...
#
# To set manually on Fly.io:
# flyctl secrets set TLS_CERT_BASE64='LS0tLS...' -a my-app
```

## Migration Guide

### Migrating from Manual Secret Management

**Before (manual flyctl commands):**

```bash
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a my-app
flyctl secrets set GITHUB_TOKEN=ghp_... -a my-app
flyctl secrets set DATABASE_PASSWORD=xxx -a my-app
```

**After (sindri.yaml):**

```yaml
# sindri.yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: env
  - name: GITHUB_TOKEN
    source: env
  - name: DATABASE_PASSWORD
    source: env

# .env.local
ANTHROPIC_API_KEY=sk-ant-...
GITHUB_TOKEN=ghp_...
DATABASE_PASSWORD=xxx
```

```bash
sindri deploy  # Secrets automatically injected
```

### Migrating to Vault

#### Step 1: Move secrets to Vault

```bash
# Write secrets to Vault
vault kv put secret/sindri/prod/api \
  anthropic_key=sk-ant-... \
  github_token=ghp_...

vault kv put secret/sindri/prod/database \
  password=xxx
```

#### Step 2: Update sindri.yaml

```yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: vault # Changed from env
    vaultPath: secret/data/sindri/prod/api
    vaultKey: anthropic_key

  - name: GITHUB_TOKEN
    source: vault
    vaultPath: secret/data/sindri/prod/api
    vaultKey: github_token

  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: password
```

#### Step 3: Deploy with Vault

```bash
export VAULT_ADDR='https://vault.company.com'
export VAULT_TOKEN='hvs.xxx'

sindri deploy
```

## See Also

- [Configuration Reference](CONFIGURATION.md) - Complete sindri.yaml reference
- [Security Best Practices](SECURITY.md) - Security guidelines
- [Fly Deployment](providers/FLY.md) - Fly.io-specific deployment
- [Architecture](ARCHITECTURE.md) - System architecture overview
