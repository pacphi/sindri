# Docker Provider

> **Version:** 3.x
> **Last Updated:** 2026-01

Local development and testing using Docker Compose.

## Overview

The Docker provider generates `docker-compose.yml` for local development. It's the fastest way to test Sindri configurations without cloud costs.

**Best for:** Local development, testing, offline work, CI/CD pipelines

## Prerequisites

| Requirement    | Version | Check Command            |
| -------------- | ------- | ------------------------ |
| Docker Engine  | 20.10+  | `docker --version`       |
| Docker Compose | v2+     | `docker compose version` |

**Optional:**

- **Sysbox** - Secure Docker-in-Docker without privileged mode
- **nvidia-container-toolkit** - GPU support

## Quick Start

```bash
# 1. Create configuration
cat > sindri.yaml << 'EOF'
version: "1.0"
name: my-sindri-dev

deployment:
  provider: docker
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: fullstack
EOF

# 2. Deploy
sindri deploy

# 3. Connect
sindri connect

# 4. When done
sindri destroy
```

## Configuration

### Basic Configuration

```yaml
# sindri.yaml
version: "1.0"
name: sindri-docker

deployment:
  provider: docker
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: minimal
```

### Advanced Configuration

```yaml
version: "1.0"
name: sindri-docker-dev

deployment:
  provider: docker
  resources:
    memory: 8GB
    cpus: 4

extensions:
  profile: fullstack
  additional:
    - docker
    - monitoring

providers:
  docker:
    ports:
      - "3000:3000"
      - "8080:8080"
      - "5432:5432"
    volumes:
      - "./projects:/alt/home/developer/workspace/projects"
    network: sindri-network
    privileged: false
    dind:
      enabled: true
      mode: auto # auto | sysbox | privileged | socket
```

### Docker-in-Docker (DinD) Modes

| Mode         | Security | Description                                                   |
| ------------ | -------- | ------------------------------------------------------------- |
| `none`       | Highest  | No Docker access inside container                             |
| `socket`     | Medium   | Mounts host Docker socket (shared daemon)                     |
| `sysbox`     | High     | Secure nested containers (requires Sysbox)                    |
| `privileged` | Low      | Full privileged mode (not recommended)                        |
| `auto`       | Auto     | Uses sysbox if available, falls back to privileged if allowed |

### Security Model by DinD Mode

Each DinD mode applies different security controls based on its use case:

| Mode         | no-new-privileges | seccomp | sudo Works? | Use Case                     |
| ------------ | ----------------- | ------- | ----------- | ---------------------------- |
| `socket`     | YES               | default | NO          | Production (shared daemon)   |
| `sysbox`     | NO                | n/a     | YES         | Development (user namespace) |
| `privileged` | NO                | n/a     | YES         | Legacy DinD                  |
| `none`       | NO                | n/a     | YES         | Development (no Docker)      |

**Why `none` mode allows sudo:**

- `none` mode has no Docker access, so there's no privilege escalation path to the host
- Developer has `NOPASSWD` sudo configured in sudoers (intentional for extension installation)
- Blocking sudo only prevents legitimate use (apt package installation) without security benefit
- Production deployments should use `socket` mode which maintains `no-new-privileges`

**Runtime extension installation:**

With `none` mode (the default), you can install extensions that require apt packages at runtime:

```bash
# SSH into the container
sindri connect

# Install extensions with apt dependencies
sindri extension install goose    # Requires libxcb
sindri extension install dotnet   # Requires apt packages
sindri extension install php      # Requires php-fpm
```

**Sysbox Detection:**

The provider automatically detects Sysbox runtime:

```rust
// V3 detection logic
fn has_sysbox(&self) -> bool {
    let output = std::process::Command::new("docker")
        .args(["info", "--format", "{{.Runtimes}}"])
        .output();
    output.map(|o| {
        let stdout = String::from_utf8_lossy(&o.stdout);
        stdout.contains("sysbox-runc")
    }).unwrap_or(false)
}
```

### GPU Configuration

```yaml
deployment:
  provider: docker
  resources:
    memory: 16GB
    cpus: 4
    gpu:
      enabled: true
      type: nvidia
```

**Prerequisites for GPU:**

```bash
# Install nvidia-container-toolkit
# Ubuntu/Debian
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
  sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
  sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo apt-get update && sudo apt-get install -y nvidia-container-toolkit
sudo systemctl restart docker
```

## Dockerfile Build Support

The Docker provider supports building images from Dockerfile when no pre-built image is specified:

- If `image` or `image_config` is specified - Uses the specified image
- If no image specified but Dockerfile exists - Builds from Dockerfile
- If neither exists - Uses default `ghcr.io/pacphi/sindri:latest`

### Dockerfile Search Order

When building, the provider searches for Dockerfile in this order:

1. `./Dockerfile` (project root - default)
2. `./v3/Dockerfile` (Sindri v3 specific - fallback)
3. `./deploy/Dockerfile` (deploy-specific - fallback)

### Force Rebuild

Use `sindri deploy --force` to rebuild even if an image exists locally.

```bash
# Normal deploy (uses cached image if present)
sindri deploy

# Force rebuild from Dockerfile
sindri deploy --force
```

### Image Deployment Options

#### Option 1: Pre-built Image (Recommended for Users)

```yaml
deployment:
  provider: docker
  image: ghcr.io/pacphi/sindri:3.0.0
```

#### Option 2: Image Version Resolution

```yaml
deployment:
  provider: docker
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
    resolution_strategy: semver
```

#### Option 3: Build from Source (For Sindri Developers)

> **Important:** This clones from GitHub - your changes must be pushed first!
> For testing local/uncommitted changes, use `make v3-cycle-fast` instead.
> See [MAINTAINER_GUIDE.md](../MAINTAINER_GUIDE.md#two-development-paths) for the full guide.

**Using CLI flag:**

```bash
# First push your changes, then:
sindri deploy --from-source
```

**Using YAML configuration:**

```yaml
deployment:
  provider: docker
  buildFromSource:
    enabled: true
    gitRef: "main" # Optional: branch, tag, or commit SHA (defaults to main)
```

**Custom branch/commit:**

```yaml
deployment:
  provider: docker
  buildFromSource:
    enabled: true
    gitRef: "feature/my-feature" # Test your pushed feature branch
```

This clones from GitHub and builds inside Docker for Linux compatibility. The image is tagged as `sindri:{version}-{gitsha}` for traceability.

#### Option 4: Local Development (No Push Required)

For testing uncommitted local changes without pushing to GitHub:

```bash
make v3-cycle-fast CONFIG=sindri.yaml
```

This uses your local working directory files directly. See [MAINTAINER_GUIDE.md](../MAINTAINER_GUIDE.md#two-development-paths) for details.

## Deployment Commands

```bash
# Deploy (generates docker-compose.yml, starts container)
sindri deploy

# Preview deployment plan
sindri plan

# Check status
sindri status

# Connect to container
sindri connect

# Stop container (preserves volumes)
sindri stop

# Start stopped container
sindri start

# Destroy (removes container and volumes)
sindri destroy
sindri destroy --force  # Skip confirmation
```

## What Gets Generated

### docker-compose.yml

The provider generates a complete `docker-compose.yml`:

```yaml
# Generated by Sindri V3
services:
  sindri-docker:
    image: sindri:latest
    container_name: sindri-docker
    hostname: sindri-docker
    environment:
      - HOME=/alt/home/developer
      - WORKSPACE=/alt/home/developer/workspace
      - INSTALL_PROFILE=fullstack
    volumes:
      - dev_home:/alt/home/developer
    ports:
      - "3000:3000"
    deploy:
      resources:
        limits:
          cpus: "2"
          memory: 4G
        reservations:
          cpus: "1"
          memory: 2G

volumes:
  dev_home:
    driver: local

networks:
  sindri-network:
    driver: bridge
```

## Secrets Management

**Sindri automatically resolves and injects secrets from your `sindri.yaml` configuration** before deployment.

### Automatic Secrets Resolution

When you run `sindri deploy`, the Docker provider:

1. **Resolves secrets** from all configured sources (env, vault, s3, file)
2. **Creates `.env.secrets`** file with resolved environment variable secrets
3. **References the file** in generated `docker-compose.yml` via `env_file`
4. **Cleans up** the secrets file after container starts

**Preflight check output:**

```
Found environment files in /path/to/project: .env.local, .env
Secrets will be resolved with priority: shell env > .env.local > .env
```

### Using .env Files

Create `.env` in the same directory as `sindri.yaml` (add to `.gitignore`):

```bash
# .env (committed - safe defaults)
NODE_ENV=development
LOG_LEVEL=info

# .env.local (gitignored - personal secrets)
ANTHROPIC_API_KEY=sk-ant-...
GITHUB_TOKEN=ghp_...
GIT_USER_NAME=Your Name
GIT_USER_EMAIL=you@example.com
```

**Resolution priority**: `shell env > .env.local > .env > vault > s3`

### Using Custom .env File Path

```bash
# Deploy with custom .env file
sindri deploy --env-file config/production.env

# Use absolute path
sindri deploy --env-file /secrets/.env
```

### Configure Secrets in sindri.yaml

```yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: env
    required: true

  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/myapp
    vaultKey: password
    required: true

  - name: S3_BACKUP_KEY
    source: s3
    s3_path: backup/api-key
```

The Docker provider will:

- Load `ANTHROPIC_API_KEY` from `.env` or shell environment
- Fetch `DATABASE_PASSWORD` from HashiCorp Vault
- Pull `S3_BACKUP_KEY` from encrypted S3 storage
- Write all secrets to `.env.secrets`
- Mount the file into the container

### Generated docker-compose.yml

```yaml
services:
  sindri:
    env_file:
      - .env.secrets # Auto-generated, contains resolved secrets
    # ... rest of config
```

### Supported Secret Types

✅ **Environment variables** (`source: env`, `vault`, `s3`)
⚠️ **File secrets** Not currently supported - use manual mounts

### Environment Variable Override

You can still override secrets at deploy time:

```bash
ANTHROPIC_API_KEY=sk-ant-override sindri deploy
```

Shell environment variables have the **highest priority** in secret resolution.

### Security Best Practices

1. **Add to .gitignore:**

   ```gitignore
   .env.local
   .env.*.local
   .env.secrets
   ```

2. **Use .env.local for personal secrets** (never commit)

3. **Mark production secrets as required:**

   ```yaml
   secrets:
     - name: DATABASE_PASSWORD
       source: vault
       required: true
   ```

4. **Use Vault or S3 for production** (not .env files)

See [SECRETS_MANAGEMENT.md](../SECRETS_MANAGEMENT.md) for complete documentation.

## Volume Management

### Default Volume

The provider creates a persistent volume for `/alt/home/developer`:

```yaml
volumes:
  dev_home:
    driver: local
```

### Custom Volume Mounts

```yaml
providers:
  docker:
    volumes:
      # Mount local projects
      - "./projects:/alt/home/developer/workspace/projects"
      # Mount SSH keys (read-only)
      - "~/.ssh:/alt/home/developer/.ssh:ro"
      # Mount git config (read-only)
      - "~/.gitconfig:/alt/home/developer/.gitconfig:ro"
```

### Volume Operations

```bash
# List volumes
docker volume ls | grep sindri

# Inspect volume
docker volume inspect sindri-docker_dev_home

# Backup volume
docker run --rm -v sindri-docker_dev_home:/data -v $(pwd):/backup \
  alpine tar czf /backup/home-backup.tar.gz -C /data .

# Restore volume
docker run --rm -v sindri-docker_dev_home:/data -v $(pwd):/backup \
  alpine tar xzf /backup/home-backup.tar.gz -C /data
```

## Port Forwarding

### Automatic Extension Ports

Extensions that declare `service.ports` in their `extension.yaml` have their ports automatically mapped in the generated `docker-compose.yml`. No manual configuration is needed.

For example, installing the **paperclip** extension (which declares ports 3100 and 5432) automatically generates:

```yaml
# In generated docker-compose.yml
ports:
  - "3100:3100" # paperclip: web-ui (http, web-ui)
  - "5432:5432" # paperclip: database (tcp)
```

Other extensions with automatic port mapping include:

| Extension       | Port(s)    | Protocol  | Description            |
| --------------- | ---------- | --------- | ---------------------- |
| paperclip       | 3100, 5432 | http, tcp | Dashboard + PostgreSQL |
| excalidraw-mcp  | 3000       | http      | Canvas server          |
| guacamole       | 8080, 3389 | http, tcp | Web UI + RDP           |
| openclaw        | 18789      | http      | Control UI             |
| ollama          | 11434      | http      | REST API               |
| claude-code-mux | 13456      | http      | Routing proxy          |
| xfce-ubuntu     | 3389       | tcp       | xRDP access            |

### Manual Port Overrides

Manual ports in `sindri.yaml` take precedence over extension-declared ports. Use this to remap defaults:

```yaml
providers:
  docker:
    ports:
      - "3000:3000" # Web app
      - "8080:8080" # API
      - "5432:5432" # PostgreSQL
      - "6379:6379" # Redis
      - "27017:27017" # MongoDB
```

### Dynamic Ports

```yaml
providers:
  docker:
    ports:
      - "3000" # Assigns random host port
```

Check assigned port:

```bash
docker compose port sindri-docker 3000
```

## Networking

### Default Network

```yaml
networks:
  sindri-network:
    driver: bridge
```

### Connect to Existing Network

```yaml
providers:
  docker:
    network: my-existing-network
    networkExternal: true
```

### Multiple Networks

```yaml
providers:
  docker:
    networks:
      - sindri-network
      - database-network
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker compose logs sindri-docker

# Check container status
docker compose ps

# Inspect container
docker inspect sindri-docker

# Check generated compose file
cat docker-compose.yml
```

### Permission Issues

```bash
# Check volume permissions
docker exec sindri-docker ls -la /alt/home/developer

# Fix ownership (if needed)
docker exec -u root sindri-docker chown -R developer:developer /alt/home/developer
```

### Out of Memory

Increase memory limit:

```yaml
deployment:
  resources:
    memory: 8GB
```

Or check what's using memory:

```bash
docker stats sindri-docker
```

### Port Already in Use

```bash
# Find what's using the port
lsof -i :3000

# Use different port
providers:
  docker:
    ports:
      - "3001:3000"  # Map to different host port
```

### Docker Daemon Connection Issues

```bash
# Check Docker daemon
docker info

# Check Docker socket permissions
ls -la /var/run/docker.sock

# Add user to docker group (if needed)
sudo usermod -aG docker $USER
```

## Best Practices

1. **Use .env files** - Never hardcode secrets in sindri.yaml
2. **Mount SSH keys read-only** - `~/.ssh:/home/developer/.ssh:ro`
3. **Set resource limits** - Prevent runaway containers
4. **Use named volumes** - For persistent data
5. **Test locally first** - Before deploying to cloud providers
6. **Use Sysbox for DinD** - More secure than privileged mode

## Resource Guidelines

| Use Case             | Memory | CPUs | Disk   |
| -------------------- | ------ | ---- | ------ |
| Minimal testing      | 2GB    | 1    | 10GB   |
| Standard development | 4GB    | 2    | 30GB   |
| Full-stack with DB   | 8GB    | 4    | 50GB   |
| AI/ML workloads      | 16GB+  | 8+   | 100GB+ |

## Cost

**$0** - Uses local machine resources only.

## Hardcoded Defaults

> **For maintainers:** The following values are either hardcoded in the generated `docker-compose.yml` (not configurable via `sindri.yaml`) or are fallback defaults applied when a field is omitted from the config.

### Non-Configurable Template Values

**Source:** `sindri-providers/src/templates/docker-compose.yml.tera`

| Value                                                                      | Template Line  | Description                                  |
| -------------------------------------------------------------------------- | -------------- | -------------------------------------------- |
| `INIT_WORKSPACE=true`                                                      | 28             | Always initializes workspace                 |
| `/alt/home/developer`                                                      | 16             | Home directory volume mount path             |
| `/var/lib/docker`                                                          | 18             | Docker daemon storage (privileged DinD mode) |
| `/var/run/docker.sock`                                                     | 21             | Docker socket path (socket DinD mode)        |
| `runtime: nvidia`                                                          | 47             | GPU runtime name                             |
| GPU capabilities `[gpu, compute, utility]`                                 | 57             | NVIDIA capabilities list                     |
| `/tmp:size=2G,mode=1777,noexec,nosuid,nodev`                               | 68, 72, 78, 94 | tmpfs mount (all modes)                      |
| `no-new-privileges:true`                                                   | 76             | Security option (socket mode)                |
| Group `docker`                                                             | 80             | Added group in socket mode                   |
| Capabilities drop `ALL`, add `CHOWN, DAC_OVERRIDE, FOWNER, SETUID, SETGID` | 85–92          | Linux capabilities (none mode)               |
| `stdin_open: true`                                                         | 96             | Interactive stdin                            |
| `tty: true`                                                                | 97             | Pseudo-TTY                                   |
| `driver: local`                                                            | 113, 116       | Volume driver type                           |

**Source:** `sindri-providers/src/templates/docker-compose.dind.yml.tera`

| Value                           | Template Line | Description                            |
| ------------------------------- | ------------- | -------------------------------------- |
| `runtime: sysbox-runc`          | 11            | Sysbox runtime name                    |
| `privileged: true`              | 13            | Privileged mode (when DinD privileged) |
| `INIT_WORKSPACE=true`           | 27            | Always initializes workspace           |
| `stdin_open: true`, `tty: true` | 43–44         | Interactive terminal                   |
| `driver: local`                 | 52            | Volume driver type                     |

### Configurable Fallback Defaults

**Source:** `sindri-providers/src/templates/context.rs` — `from_config()`

| Field               | Default          | Line    | Override in `sindri.yaml`             |
| ------------------- | ---------------- | ------- | ------------------------------------- |
| Memory              | `"4GB"`          | 97      | `deployment.resources.memory`         |
| CPUs                | `2`              | 100     | `deployment.resources.cpus`           |
| Volume size         | `"10GB"`         | 109     | `deployment.volumes.workspace.size`   |
| GPU type            | `"nvidia"`       | 118     | `deployment.resources.gpu.type`       |
| Network mode        | `"bridge"`       | 133     | `providers.docker.network`            |
| DinD storage size   | `"20GB"`         | 153     | `providers.docker.dind.storageSize`   |
| DinD storage driver | `"auto"`         | 154     | `providers.docker.dind.storageDriver` |
| Secrets file        | `".env.secrets"` | 197     | Not configurable                      |
| Privileged          | `false`          | 128–129 | `providers.docker.privileged`         |

**Source:** `sindri-providers/src/docker.rs`

| Field                    | Default                                    | Line     | Description            |
| ------------------------ | ------------------------------------------ | -------- | ---------------------- |
| Secrets file permissions | `0o600`                                    | 453      | Owner read/write only  |
| Dockerfile names         | `"Dockerfile"` / `"Dockerfile.dev"`        | 600, 602 | Selected by build mode |
| User/home in exec        | `-u developer`, `HOME=/alt/home/developer` | 775, 810 | Connection environment |

### Computed / Derived Values

| Value          | Computation                                                   | Source                          |
| -------------- | ------------------------------------------------------------- | ------------------------------- |
| DinD mode      | Auto-detected from host (`sysbox-runc` binary, Docker socket) | `docker.rs::detect_dind_mode()` |
| Image tag      | `sindri:{cli_version}-{gitsha}`                               | `docker.rs:593–596`             |
| Volume names   | `{name}_home`, `{name}_docker`                                | `docker.rs:347–350`             |
| Container name | Derived from config `name` field                              | `docker.rs:714–718`             |

## Related Documentation

- [Provider Overview](README.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [CLI Reference](../CLI.md)
