# Configuration Reference

Complete reference for `sindri.yaml` configuration file.

## Basic Structure

```yaml
version: 1.0 # Configuration version
name: my-dev-env # Environment name

deployment:
  provider: docker # docker | fly | kubernetes | devpod
  resources: # Resource allocation
    memory: 2GB
    cpus: 1
  volumes: # Persistent storage
    workspace:
      size: 10GB

extensions:
  profile: fullstack # Extension profile OR
  list: # Individual extensions
    - nodejs
    - python
    - docker

providers: # Provider-specific config
  fly: {}
  docker: {}
  kubernetes: {}
  devpod: {}
```

## Top-Level Fields

### version

**Type:** string
**Required:** Yes
**Default:** None

Configuration schema version.

```yaml
version: 1.0
```

### name

**Type:** string
**Required:** Yes
**Default:** None

Unique name for the development environment. Used for:

- Container/app naming
- Volume naming
- Service discovery

**Constraints:**

- Must be lowercase
- Alphanumeric and hyphens only
- Max 63 characters

```yaml
name: my-sindri-dev
```

## Deployment Configuration

### deployment.provider

**Type:** string
**Required:** Yes
**Options:** `docker`, `fly`, `kubernetes`, `devpod`

Target deployment platform.

```yaml
deployment:
  provider: fly
```

### deployment.resources

Resource allocation for the environment.

#### deployment.resources.memory

**Type:** string
**Format:** `<number>GB` or `<number>MB`
**Default:** `2GB`

Memory allocation.

```yaml
deployment:
  resources:
    memory: 4GB
```

**Fly.io tiers:**

- `1GB` - Minimal (~$5-10/mo with auto-suspend)
- `2GB` - Standard (~$10-15/mo)
- `4GB` - Performance (~$30-40/mo)
- `8GB` - High performance

#### deployment.resources.cpus

**Type:** integer
**Default:** `1`

CPU allocation.

```yaml
deployment:
  resources:
    cpus: 2
```

### deployment.volumes

Persistent volume configuration.

#### deployment.volumes.workspace.size

**Type:** string
**Format:** `<number>GB`
**Default:** `10GB`

Persistent workspace volume size.

```yaml
deployment:
  volumes:
    workspace:
      size: 30GB
```

**Cost implications (Fly.io):**

- 10GB: ~$1.50/month
- 30GB: ~$4.50/month
- 100GB: ~$15/month

## Extensions Configuration

Configure which extensions to install.

### Option 1: Use Profile

```yaml
extensions:
  profile: fullstack
```

**Available profiles:**

- `minimal` - nodejs, python
- `fullstack` - nodejs, python, docker, postgres, nodejs-devtools
- `ai-dev` - nodejs, python, ai-toolkit, openskills, monitoring
- `systems` - rust, golang, docker, infra-tools
- `enterprise` - All languages and infrastructure tools

### Option 2: Individual Extensions

```yaml
extensions:
  list:
    - nodejs
    - python
    - docker
    - github-cli
```

**Available extensions:** See [Extension Catalog](EXTENSIONS.md)

### Option 3: Mixed (Profile + Additions)

```yaml
extensions:
  profile: minimal
  additional:
    - docker
    - github-cli
```

## Provider-Specific Configuration

### Docker Provider

```yaml
providers:
  docker:
    compose:
      version: "3.8" # Docker Compose version
    ports:
      - "8080:8080" # Port mappings
      - "3000:3000"
    networks:
      - sindri-network # Custom networks
```

### Fly.io Provider

```yaml
providers:
  fly:
    region: sjc # Fly.io region (sjc, iad, lhr, etc.)
    organization: personal # Fly.io organization
    sshPort: 10022 # External SSH port
    autoStopMachines: true # Auto-suspend when idle
    autoStartMachines: true # Auto-resume on connection
    cpuKind: shared # shared | performance
    highAvailability: false # Multi-machine deployment
```

#### Fly.io Regions

Common regions:

- `sjc` - San Jose, CA (US West)
- `iad` - Ashburn, VA (US East)
- `lhr` - London, UK
- `fra` - Frankfurt, Germany
- `nrt` - Tokyo, Japan
- `syd` - Sydney, Australia

Full list: https://fly.io/docs/reference/regions/

#### Fly.io Auto-Suspend

```yaml
providers:
  fly:
    autoStopMachines: true # Suspend after 5 min idle
    autoStartMachines: true # Auto-resume on SSH/connection
```

**Cost savings:** Pay only for active time, not idle time.

#### Fly.io CPU Types

```yaml
providers:
  fly:
    cpuKind: shared # Shared CPU (default, cheaper)
    # cpuKind: performance   # Dedicated CPU (faster, pricier)
```

**Shared CPU:** Good for development, auto-suspend workloads.
**Performance CPU:** For CPU-intensive work, production.

### Kubernetes Provider

```yaml
providers:
  kubernetes:
    namespace: dev-envs # Kubernetes namespace
    storageClass: standard # StorageClass for PVC
    imagePullPolicy: IfNotPresent # Image pull policy
    ingressEnabled: false # Enable Ingress
    ingressHost: dev.example.com # Ingress hostname
```

### DevPod Provider

```yaml
providers:
  devpod:
    vscodeExtensions: # VS Code extensions
      - ms-python.python
      - golang.go
      - rust-lang.rust-analyzer
    forwardPorts: # Port forwarding
      - 3000
      - 8080
    features: # DevContainer features
      - ghcr.io/devcontainers/features/github-cli:1
      - ghcr.io/devcontainers/features/docker-in-docker:2
```

## Environment Variables

Set environment variables for the container.

```yaml
environment:
  NODE_ENV: development
  DEBUG: "true"
  CUSTOM_VAR: value
```

**Note:** Secrets should be managed via provider mechanisms (Fly secrets, K8s secrets), not in `sindri.yaml`.

## Secrets Management

Secrets are **not** stored in `sindri.yaml`. Use provider-specific mechanisms:

### Fly.io Secrets

```bash
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a my-app
flyctl secrets set GITHUB_TOKEN=ghp_... -a my-app
```

### Kubernetes Secrets

```bash
kubectl create secret generic sindri-secrets \
  --from-literal=ANTHROPIC_API_KEY=sk-ant-... \
  --namespace=dev-envs
```

### Docker Compose (.env file)

```bash
# .env (not committed to git)
ANTHROPIC_API_KEY=sk-ant-...
GITHUB_TOKEN=ghp_...
```

## Complete Example: Minimal Local

```yaml
version: 1.0
name: sindri-local

deployment:
  provider: docker
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: minimal
```

## Complete Example: Fly.io Production

```yaml
version: 1.0
name: sindri-prod

deployment:
  provider: fly
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 30GB

extensions:
  profile: fullstack
  additional:
    - infra-tools
    - cloud-tools
    - monitoring

providers:
  fly:
    region: sjc
    organization: personal
    sshPort: 10022
    autoStopMachines: true
    autoStartMachines: true
    cpuKind: shared
```

## Complete Example: Kubernetes

```yaml
version: 1.0
name: sindri-k8s

deployment:
  provider: kubernetes
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 50GB

extensions:
  profile: enterprise

providers:
  kubernetes:
    namespace: dev-environments
    storageClass: fast-ssd
    imagePullPolicy: Always
    ingressEnabled: true
    ingressHost: sindri.dev.company.com

environment:
  ENVIRONMENT: production
  LOG_LEVEL: info
```

## Complete Example: DevPod

```yaml
version: 1.0
name: sindri-devpod

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: fullstack

providers:
  devpod:
    vscodeExtensions:
      - ms-python.python
      - dbaeumer.vscode-eslint
      - esbenp.prettier-vscode
      - golang.go
    forwardPorts:
      - 3000
      - 5432
      - 6379
    features:
      - ghcr.io/devcontainers/features/github-cli:1
      - ghcr.io/devcontainers/features/docker-in-docker:2
```

## Configuration Validation

Validate your configuration:

```bash
./cli/sindri config validate
```

Validates against JSON schema: `docker/lib/schemas/sindri.schema.json`

## Schema Location

JSON schema for validation:

```text
docker/lib/schemas/sindri.schema.json
```

## Related Documentation

- [Quickstart](QUICKSTART.md)
- [Extension Catalog](EXTENSIONS.md)
- [Fly.io Deployment](FLY_DEPLOYMENT.md)
- [DevPod Integration](DEVPOD_INTEGRATION.md)
