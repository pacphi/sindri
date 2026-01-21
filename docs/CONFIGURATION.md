# Configuration Reference

Complete reference for `sindri.yaml` configuration file.

## Basic Structure

```yaml
version: 1.0 # Configuration version
name: my-dev-env # Environment name

deployment:
  provider: docker # docker | fly | devpod
  resources: # Resource allocation
    memory: 2GB
    cpus: 1
  volumes: # Persistent storage
    workspace:
      size: 10GB

extensions:
  profile: fullstack # Use a profile, OR
  # active:          # List individual extensions (mutually exclusive with profile)
  #   - nodejs
  #   - python
  # additional:      # Extra extensions on top of a profile (optional)
  #   - docker

providers: # Provider-specific config
  fly: {}
  docker: {}
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
**Options:** `docker`, `fly`, `devpod`, `e2b`

Target deployment platform. For Kubernetes deployments, use `devpod` with `type: kubernetes`.

**Provider Comparison:**

| Provider | Best For                          | Access Method    |
| -------- | --------------------------------- | ---------------- |
| `docker` | Local development                 | Direct container |
| `fly`    | Remote cloud dev                  | SSH              |
| `devpod` | IDE integration, K8s, multi-cloud | SSH              |
| `e2b`    | AI sandboxes, rapid prototyping   | WebSocket PTY    |

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

#### deployment.resources.gpu

**Type:** object
**Default:** `{ enabled: false }`

GPU configuration for compute workloads requiring GPU acceleration (AI/ML, 3D rendering, etc.).

```yaml
deployment:
  resources:
    gpu:
      enabled: true
      type: nvidia # nvidia | amd
      count: 1 # Number of GPUs (1-8)
      tier: gpu-medium # gpu-small | gpu-medium | gpu-large | gpu-xlarge
      memory: 16GB # Minimum GPU memory (e.g., 16GB, 24GB)
```

**Quick Reference:**

- `enabled` - Enable GPU support (default: `false`)
- `type` - GPU vendor: `nvidia` or `amd`
- `count` - Number of GPUs: 1-8
- `tier` - GPU tier: `gpu-small` | `gpu-medium` | `gpu-large` | `gpu-xlarge`
- `memory` - Minimum GPU memory required

**See [GPU Configuration Guide](GPU.md)** for comprehensive documentation including:

- Provider-specific GPU configuration (Fly.io, AWS, GCP, Azure, Docker, Kubernetes)
- GPU tier mappings and pricing
- Use case examples (inference, training, rendering)
- Cost optimization strategies
- Troubleshooting guide

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

**Standard profiles:**

- `minimal` - nodejs, python
- `fullstack` - nodejs, python, docker, nodejs-devtools
- `ai-dev` - nodejs, python, golang, ai-toolkit, mdflow, openskills, supabase-cli, linear-mcp, monitoring
- `anthropic-dev` - Full Anthropic toolset (claude-flow, agentic-flow, ai-toolkit, etc.)
- `systems` - rust, golang, docker, infra-tools
- `enterprise` - All languages + jira-mcp, cloud-tools
- `devops` - docker, infra-tools, monitoring, cloud-tools
- `mobile` - nodejs, linear-mcp, supabase-cli

**VisionFlow profiles:**

- `visionflow-core` - Document processing (pdf, docx, xlsx, imagemagick, ffmpeg)
- `visionflow-data-scientist` - AI research and ML tools (perplexity, pytorch, comfyui)
- `visionflow-creative` - 3D modeling and creative tools (blender, qgis, canvas-design)
- `visionflow-full` - All 34 VisionFlow extensions

### Option 2: Individual Extensions

```yaml
extensions:
  active:
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

### E2B Provider

E2B provides ultra-fast cloud sandboxes with ~150ms startup times.

```yaml
providers:
  e2b:
    # Template configuration
    templateAlias: my-sindri-template # Custom template name
    reuseTemplate: true # Reuse existing template

    # Sandbox behavior
    timeout: 3600 # Timeout in seconds (default: 300)
    autoPause: true # Pause on timeout (default: true)
    autoResume: true # Resume paused on connect (default: true)

    # Network configuration
    internetAccess: true # Enable outbound internet
    allowedDomains: # Whitelist domains (empty = all)
      - github.com
      - "*.github.com"
      - api.anthropic.com
    blockedDomains: [] # Blacklist domains
    publicAccess: false # Public URL access to services

    # Metadata for identification
    metadata:
      project: my-project
      environment: development
```

**E2B Options:**

| Option           | Type    | Default  | Description                        |
| ---------------- | ------- | -------- | ---------------------------------- |
| `templateAlias`  | string  | `{name}` | Template identifier                |
| `reuseTemplate`  | boolean | `true`   | Reuse existing template            |
| `timeout`        | integer | `300`    | Sandbox timeout (60-86400 seconds) |
| `autoPause`      | boolean | `true`   | Pause on timeout instead of kill   |
| `autoResume`     | boolean | `true`   | Auto-resume on connect             |
| `internetAccess` | boolean | `true`   | Enable outbound internet           |
| `allowedDomains` | array   | `[]`     | Whitelist domains                  |
| `blockedDomains` | array   | `[]`     | Blacklist domains                  |
| `publicAccess`   | boolean | `false`  | Allow public URL access            |
| `metadata`       | object  | `{}`     | Custom key-value pairs             |

**Important:** E2B does not support GPU. GPU configuration will be rejected.

See [E2B Provider Guide](providers/E2B.md) for full documentation.

### DevPod Multi-Backend Support

DevPod can deploy to multiple cloud backends. Use `providers.devpod.type` to select:

```yaml
providers:
  devpod:
    type: kubernetes # docker | aws | gcp | azure | digitalocean | kubernetes | ssh
    kubernetes:
      namespace: sindri-dev
      storageClass: standard
      context: my-cluster # Optional: specific kubeconfig context
```

**Available DevPod backends:**

| Type           | Backend                | Example Config                  |
| -------------- | ---------------------- | ------------------------------- |
| `docker`       | Local Docker (default) | `examples/devpod/`              |
| `aws`          | AWS EC2                | `examples/devpod/aws/`          |
| `gcp`          | GCP Compute            | `examples/devpod/gcp/`          |
| `azure`        | Azure VMs              | `examples/devpod/azure/`        |
| `digitalocean` | DigitalOcean Droplets  | `examples/devpod/digitalocean/` |
| `kubernetes`   | Kubernetes pods        | `examples/devpod/kubernetes/`   |
| `ssh`          | Any SSH host           | N/A                             |

### Cloud Provider Regions

#### AWS Regions

**Recommended Regions:**

| Region         | Location    | GPU Availability | Notes                    |
| -------------- | ----------- | ---------------- | ------------------------ |
| us-east-1      | N. Virginia | Excellent        | Largest, most services   |
| us-east-2      | Ohio        | Good             | Lower latency to Midwest |
| us-west-2      | Oregon      | Excellent        | Best GPU availability    |
| eu-west-1      | Ireland     | Good             | EU primary               |
| eu-central-1   | Frankfurt   | Good             | EU central               |
| ap-southeast-1 | Singapore   | Fair             | Asia Pacific             |
| ap-northeast-1 | Tokyo       | Good             | Japan                    |

**Configuration:**

```yaml
providers:
  devpod:
    type: aws
    aws:
      region: us-west-2 # Choose based on proximity and GPU needs
      instanceType: t3.medium
```

**Full list:** https://aws.amazon.com/about-aws/global-infrastructure/regions_az/

#### GCP Zones

**Recommended Zones:**

| Zone              | Location       | GPU Availability | Notes                 |
| ----------------- | -------------- | ---------------- | --------------------- |
| us-central1-a     | Iowa           | Excellent        | Best GPU availability |
| us-central1-b     | Iowa           | Excellent        | High availability     |
| us-west1-b        | Oregon         | Good             | West Coast            |
| us-east1-c        | South Carolina | Good             | East Coast            |
| europe-west4-a    | Netherlands    | Good             | EU primary            |
| asia-southeast1-c | Singapore      | Fair             | Asia Pacific          |

**Configuration:**

```yaml
providers:
  devpod:
    type: gcp
    gcp:
      zone: us-central1-a # Best for GPU workloads
      machineType: e2-medium
```

**Note:** GCP uses zones (region + availability zone). GPU availability varies significantly by zone.

**Full list:** https://cloud.google.com/compute/docs/regions-zones

#### Azure Regions

**Recommended Regions:**

| Region         | Location    | GPU Availability | Notes                |
| -------------- | ----------- | ---------------- | -------------------- |
| eastus         | Virginia    | Excellent        | Largest Azure region |
| eastus2        | Virginia    | Good             | Backup to eastus     |
| westus2        | Washington  | Good             | West Coast           |
| southcentralus | Texas       | Good             | Central US           |
| westeurope     | Netherlands | Good             | EU primary           |
| northeurope    | Ireland     | Good             | EU backup            |
| southeastasia  | Singapore   | Fair             | Asia Pacific         |
| japaneast      | Tokyo       | Fair             | Japan                |

**Configuration:**

```yaml
providers:
  devpod:
    type: azure
    azure:
      location: eastus # Choose based on proximity and services
      vmSize: Standard_B2s
```

**Full list:** https://azure.microsoft.com/en-us/explore/global-infrastructure/geographies/

#### DigitalOcean Regions

**Available Regions:**

| Region | Location      | Notes           |
| ------ | ------------- | --------------- |
| nyc1   | New York 1    | US East (older) |
| nyc3   | New York 3    | US East (newer) |
| sfo3   | San Francisco | US West         |
| tor1   | Toronto       | Canada          |
| lon1   | London        | Europe          |
| fra1   | Frankfurt     | Europe          |
| ams3   | Amsterdam     | Europe          |
| sgp1   | Singapore     | Asia Pacific    |
| blr1   | Bangalore     | India           |

**Configuration:**

```yaml
providers:
  devpod:
    type: digitalocean
    digitalocean:
      region: nyc3 # Recommended for US East
      size: s-2vcpu-4gb
```

**Note:** DigitalOcean does not offer GPU instances.

**Full list:** https://docs.digitalocean.com/products/platform/availability-matrix/

### Region Selection Guidelines

**Latency Optimization:**

- Choose region closest to your team's location
- Use ping tests to verify latency
- Consider data sovereignty requirements (GDPR, etc.)

**Cost Optimization:**

- Regions may have different pricing
- GPU availability affects cost (scarcity = higher prices)
- Consider data transfer costs between regions

**Feature Availability:**

- Not all VM types available in all regions
- GPU types vary by region (see [GPU.md](GPU.md))
- Some services/features are region-specific

**Testing Latency:**

```bash
# Test ping to various regions
ping -c 3 compute.us-west-2.amazonaws.com
ping -c 3 us-central1.gce.cloud.google.com
ping -c 3 eastus.management.azure.com
```

## Kubernetes Deployment

Sindri deploys to Kubernetes via DevPod. Use `provider: devpod` with `type: kubernetes`:

```yaml
deployment:
  provider: devpod

providers:
  devpod:
    type: kubernetes
    kubernetes:
      namespace: sindri-test
      storageClass: standard
      context: my-cluster # Optional: specific kubeconfig context
```

**Features:**

- DevContainer compatibility and IDE integration
- Automatic image handling for local clusters (kind/k3d)
- CI/CD testing support with auto-provisioned clusters

For manual Kubernetes deployment without DevPod (GitOps, enterprise policies),
see [Appendix A in the Kubernetes Guide](providers/KUBERNETES.md#appendix-a-manual-kubernetes-deployment).

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

## Complete Example: Kubernetes (via DevPod)

```yaml
version: 1.0
name: sindri-k8s

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 50GB

extensions:
  profile: enterprise

providers:
  devpod:
    type: kubernetes
    kubernetes:
      namespace: dev-environments
      storageClass: fast-ssd
      context: my-production-cluster

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
./v2/cli/sindri config validate
```

Validates against JSON schema: `v2/docker/lib/schemas/sindri.schema.json`

## Schema Location

JSON schema for validation:

```text
v2/docker/lib/schemas/sindri.schema.json
```

## Related Documentation

- [Quickstart](QUICKSTART.md)
- [Extension Catalog](EXTENSIONS.md)
- [Docker Deployment](providers/DOCKER.md)
- [Fly.io Deployment](providers/FLY.md)
- [DevPod Integration](providers/DEVPOD.md)
- [E2B Cloud Sandboxes](providers/E2B.md)
- [Kubernetes Deployment](providers/KUBERNETES.md)
