# V3 Configuration Reference

Complete reference for `sindri.yaml` configuration file in Sindri v3.

## Table of Contents

- [Overview](#overview)
- [File Location](#file-location)
- [Configuration Schema](#configuration-schema)
  - [Top-Level Fields](#top-level-fields)
  - [Deployment Configuration](#deployment-configuration)
  - [Image Configuration](#image-configuration)
  - [Resources Configuration](#resources-configuration)
  - [Extensions Configuration](#extensions-configuration)
  - [Secrets Configuration](#secrets-configuration)
  - [Provider-Specific Configuration](#provider-specific-configuration)
- [Complete Examples](#complete-examples)
- [Environment Variables](#environment-variables)
- [Configuration Validation](#configuration-validation)
- [Migration from V2](#migration-from-v2)

---

## Overview

The `sindri.yaml` configuration file defines your Sindri development environment. It specifies:

- Deployment provider and target infrastructure
- Container image configuration and version management
- Resource allocation (CPU, memory, GPU)
- Extensions and development tools
- Secrets management
- Provider-specific options

Sindri v3 introduces several enhancements over v2:

- **Structured image configuration** with semantic versioning
- **Image signature and provenance verification** via cosign/SLSA
- **S3 secret storage** in addition to env, file, and Vault sources
- **Enhanced GPU configuration** with tier-based selection
- **Local Kubernetes cluster management** with kind/k3d support

---

## File Location

Sindri looks for `sindri.yaml` in the following locations (in order):

1. Current working directory: `./sindri.yaml`
2. User config directory: `~/.config/sindri/sindri.yaml`
3. System config: `/etc/sindri/sindri.yaml`

You can also specify a custom path:

```bash
sindri deploy --config /path/to/sindri.yaml
```

---

## Configuration Schema

### Top-Level Fields

```yaml
version: "3.0" # Required: Configuration schema version
name: my-dev-env # Required: Deployment name

deployment: # Required: Deployment configuration
  provider: docker # Required: Target provider

extensions: # Required: Extension configuration
  profile: minimal # Profile or active list

secrets: [] # Optional: Secrets to inject

providers: {} # Optional: Provider-specific configuration
```

#### version

**Type:** `string`
**Required:** Yes
**Pattern:** `^\d+\.\d+$`

Configuration schema version. V3 uses `"3.0"`.

```yaml
version: "3.0"
```

#### name

**Type:** `string`
**Required:** Yes
**Pattern:** `^[a-z][a-z0-9-]*$`
**Max Length:** 63 characters

Unique name for the development environment. Used for:

- Container/app naming
- Volume naming
- Service discovery
- Kubernetes resources

**Constraints:**

- Must start with a lowercase letter
- Lowercase alphanumeric and hyphens only
- Maximum 63 characters (DNS label limit)

```yaml
name: my-sindri-dev
```

---

### Deployment Configuration

```yaml
deployment:
  provider: docker # Required: Provider type
  image: ghcr.io/org/image:tag # Optional: Legacy image field
  image_config: # Optional: Structured image config (preferred)
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
  resources: # Optional: Resource allocation
    memory: 4GB
    cpus: 2
  volumes: # Optional: Volume configuration
    workspace:
      size: 20GB
```

#### deployment.provider

**Type:** `string`
**Required:** Yes
**Values:** `docker`, `docker-compose`, `fly`, `devpod`, `e2b`, `kubernetes`

Target deployment provider.

| Provider                    | Best For                        | Access Method    | GPU Support |
| --------------------------- | ------------------------------- | ---------------- | ----------- |
| `docker` / `docker-compose` | Local development               | Direct container | Yes         |
| `fly`                       | Remote cloud dev                | SSH              | Yes         |
| `devpod`                    | IDE integration, multi-cloud    | SSH              | Yes         |
| `e2b`                       | AI sandboxes, rapid prototyping | WebSocket PTY    | No          |
| `kubernetes`                | Enterprise, GitOps              | kubectl          | Yes         |

**Note:** `docker` is an alias for `docker-compose`.

```yaml
deployment:
  provider: fly
```

#### deployment.image

**Type:** `string`
**Required:** No (legacy field)

Docker image to deploy. **Deprecated in v3** - use `image_config` instead for structured version management.

```yaml
deployment:
  image: ghcr.io/pacphi/sindri:v3.0.0
```

---

### Image Configuration

V3 introduces structured image configuration with version resolution, signature verification, and provenance attestation.

```yaml
deployment:
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
    tag_override: null
    digest: null
    resolution_strategy: semver
    allow_prerelease: false
    verify_signature: true
    verify_provenance: true
    pull_policy: IfNotPresent
    certificate_identity: null
    certificate_oidc_issuer: null
```

#### image_config.registry

**Type:** `string`
**Required:** Yes (if using image_config)

Container registry URL. Examples:

- `ghcr.io/pacphi/sindri` - GitHub Container Registry
- `docker.io/library/image` - Docker Hub
- `gcr.io/project/image` - Google Container Registry
- `123456789.dkr.ecr.us-west-2.amazonaws.com/image` - AWS ECR

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
```

#### image_config.version

**Type:** `string`
**Required:** No
**Default:** Uses `resolution_strategy`

Semantic version constraint. Supports npm-style constraints:

- `^3.0.0` - Compatible with 3.x.x (>=3.0.0, <4.0.0)
- `~3.1.0` - Approximately 3.1.x (>=3.1.0, <3.2.0)
- `>=3.0.0` - Any version 3.0.0 or higher
- `3.0.0` - Exact version

```yaml
image_config:
  version: "^3.0.0"
```

#### image_config.resolution_strategy

**Type:** `string`
**Required:** No
**Default:** `semver`
**Values:** `semver`, `latest-stable`, `pin-to-cli`, `explicit`

How to resolve the image version:

| Strategy        | Description                                           |
| --------------- | ----------------------------------------------------- |
| `semver`        | Use semantic versioning constraints (default)         |
| `latest-stable` | Always use the newest stable (non-prerelease) version |
| `pin-to-cli`    | Use the same version as the CLI binary                |
| `explicit`      | Use explicit `tag_override` or `digest`               |

```yaml
image_config:
  resolution_strategy: latest-stable
```

#### image_config.tag_override

**Type:** `string`
**Required:** No

Explicit tag override. Ignores version constraint when set.

```yaml
image_config:
  tag_override: v3.0.0-beta.1
  resolution_strategy: explicit
```

#### image_config.digest

**Type:** `string`
**Required:** No
**Pattern:** `sha256:[a-f0-9]{64}`

Pin to specific image digest for immutable deployments. Overrides both `version` and `tag_override`.

```yaml
image_config:
  digest: sha256:abc123def456...
  resolution_strategy: explicit
```

#### image_config.allow_prerelease

**Type:** `boolean`
**Required:** No
**Default:** `false`

Allow prerelease versions (alpha, beta, rc) in version resolution.

```yaml
image_config:
  version: "^3.0.0"
  allow_prerelease: true # Allows 3.1.0-alpha.1, etc.
```

#### image_config.verify_signature

**Type:** `boolean`
**Required:** No
**Default:** `true`

Verify container image signature using cosign before deployment.

```yaml
image_config:
  verify_signature: true
```

#### image_config.verify_provenance

**Type:** `boolean`
**Required:** No
**Default:** `true`

Verify SLSA provenance attestation.

```yaml
image_config:
  verify_provenance: true
```

#### image_config.pull_policy

**Type:** `string`
**Required:** No
**Default:** `IfNotPresent`
**Values:** `Always`, `IfNotPresent`, `Never`

Container image pull policy:

| Policy         | Description                                |
| -------------- | ------------------------------------------ |
| `Always`       | Always pull the image from registry        |
| `IfNotPresent` | Only pull if not present locally (default) |
| `Never`        | Never pull, use local only                 |

```yaml
image_config:
  pull_policy: Always
```

#### image_config.certificate_identity

**Type:** `string`
**Required:** No

Certificate identity regexp for signature verification.

```yaml
image_config:
  certificate_identity: "https://github.com/pacphi/sindri.*"
```

#### image_config.certificate_oidc_issuer

**Type:** `string`
**Required:** No

OIDC issuer for signature verification.

```yaml
image_config:
  certificate_oidc_issuer: "https://token.actions.githubusercontent.com"
```

---

### Resources Configuration

```yaml
deployment:
  resources:
    memory: 4GB
    cpus: 2
    gpu:
      enabled: true
      type: nvidia
      count: 1
      tier: gpu-medium
      memory: 16GB
```

#### resources.memory

**Type:** `string`
**Required:** No
**Pattern:** `^\d+(MB|GB)$`
**Default:** `2GB`

Memory allocation for the environment.

**Fly.io pricing tiers:**

| Memory | Approximate Cost (with auto-suspend) |
| ------ | ------------------------------------ |
| 1GB    | ~$5-10/month                         |
| 2GB    | ~$10-15/month                        |
| 4GB    | ~$30-40/month                        |
| 8GB    | ~$60-80/month                        |

```yaml
resources:
  memory: 4GB
```

#### resources.cpus

**Type:** `integer`
**Required:** No
**Minimum:** 1
**Default:** 1

CPU core allocation.

```yaml
resources:
  cpus: 2
```

#### resources.gpu

GPU configuration for compute workloads.

```yaml
resources:
  gpu:
    enabled: true # Enable GPU (default: false)
    type: nvidia # nvidia | amd (default: nvidia)
    count: 1 # Number of GPUs: 1-8 (default: 1)
    tier: gpu-medium # GPU tier for auto-selection
    memory: 16GB # Minimum GPU memory
```

##### gpu.enabled

**Type:** `boolean`
**Default:** `false`

Enable GPU support. Must be set to `true` to use GPU resources.

##### gpu.type

**Type:** `string`
**Values:** `nvidia`, `amd`
**Default:** `nvidia`

GPU vendor type.

##### gpu.count

**Type:** `integer`
**Range:** 1-8
**Default:** 1

Number of GPUs to request.

##### gpu.tier

**Type:** `string`
**Values:** `gpu-small`, `gpu-medium`, `gpu-large`, `gpu-xlarge`

GPU tier for automatic instance selection. Maps to provider-specific GPU types.

| Tier         | Typical GPU     | Use Case                  |
| ------------ | --------------- | ------------------------- |
| `gpu-small`  | T4, A10         | Inference, light training |
| `gpu-medium` | A10G, L4        | Medium training           |
| `gpu-large`  | A100 40GB       | Large training            |
| `gpu-xlarge` | A100 80GB, H100 | Maximum performance       |

##### gpu.memory

**Type:** `string`
**Pattern:** `^\d+(GB|MB)$`

Minimum GPU memory required.

**Note:** E2B provider does not support GPU. GPU configuration will be rejected.

---

### Volumes Configuration

```yaml
deployment:
  volumes:
    workspace:
      path: /home/developer/workspace
      size: 20GB
```

#### volumes.workspace.path

**Type:** `string`
**Default:** `/home/developer/workspace`

Container path for workspace volume mount.

#### volumes.workspace.size

**Type:** `string`
**Pattern:** `^\d+(MB|GB)$`
**Default:** `10GB`

Persistent workspace volume size.

**Fly.io volume pricing:**

- 10GB: ~$1.50/month
- 30GB: ~$4.50/month
- 100GB: ~$15/month

---

### Extensions Configuration

```yaml
extensions:
  profile: fullstack # Use a profile
  # OR
  active: # Explicit extension list
    - nodejs
    - python
    - docker
  additional: # Additions to a profile
    - github-cli
  auto_install: true # Auto-install on startup
```

**Mutually exclusive:** Use either `profile` OR `active`, not both.

#### extensions.profile

**Type:** `string`
**Required:** Either `profile` or `active` is required

Pre-configured extension profile.

**Standard profiles:**

| Profile         | Description            | Extensions                                             |
| --------------- | ---------------------- | ------------------------------------------------------ |
| `minimal`       | Basic development      | nodejs, python                                         |
| `fullstack`     | Full-stack web         | nodejs, python, docker, nodejs-devtools                |
| `ai-dev`        | AI/ML development      | nodejs, python, golang, ai-toolkit, mdflow, openskills |
| `anthropic-dev` | Anthropic toolset      | claude-flow, agentic-flow, ai-toolkit                  |
| `systems`       | Systems programming    | rust, golang, docker, infra-tools                      |
| `enterprise`    | Enterprise development | All languages + jira-mcp, cloud-tools                  |
| `devops`        | DevOps/SRE             | docker, infra-tools, monitoring, cloud-tools           |
| `mobile`        | Mobile development     | nodejs, linear-mcp, supabase-cli                       |

**VisionFlow profiles:**

| Profile                     | Description                                                |
| --------------------------- | ---------------------------------------------------------- |
| `visionflow-core`           | Document processing (pdf, docx, xlsx, imagemagick, ffmpeg) |
| `visionflow-data-scientist` | AI research and ML tools (perplexity, pytorch, comfyui)    |
| `visionflow-creative`       | 3D modeling and creative tools (blender, qgis)             |
| `visionflow-full`           | All 34 VisionFlow extensions                               |

```yaml
extensions:
  profile: fullstack
```

#### extensions.active

**Type:** `array` of `string`
**Required:** Either `profile` or `active` is required

Explicit list of extensions to install.

```yaml
extensions:
  active:
    - nodejs
    - python
    - docker
    - github-cli
```

#### extensions.additional

**Type:** `array` of `string`
**Required:** No

Additional extensions to install on top of a profile.

```yaml
extensions:
  profile: minimal
  additional:
    - docker
    - github-cli
```

#### extensions.auto_install

**Type:** `boolean`
**Default:** `true`

Automatically install extensions on container startup. Set to `false` for manual control or CI testing.

```yaml
extensions:
  profile: minimal
  auto_install: false
```

---

### Secrets Configuration

Secrets are injected into the deployment from various sources.

```yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: env
    required: true

  - name: GITHUB_TOKEN
    source: env
    fromFile: ~/.config/gh/token
    required: false

  - name: SSH_KEY
    source: file
    path: ~/.ssh/id_rsa
    mountPath: /home/developer/.ssh/id_rsa
    permissions: "0600"

  - name: DB_PASSWORD
    source: vault
    vaultPath: secret/data/myapp
    vaultKey: db_password
    vaultMount: secret
    required: true

  - name: CONFIG_FILE
    source: s3
    s3Path: s3://my-bucket/secrets/config.json
    required: true
```

#### Secret Object Fields

##### name

**Type:** `string`
**Required:** Yes
**Pattern:** `^[A-Z][A-Z0-9_]*$`

Environment variable name for the secret.

##### source

**Type:** `string`
**Required:** Yes
**Values:** `env`, `file`, `vault`, `s3`

Source type for the secret.

| Source  | Description                          |
| ------- | ------------------------------------ |
| `env`   | Environment variable or file content |
| `file`  | Mount file into container            |
| `vault` | HashiCorp Vault secret               |
| `s3`    | AWS S3 encrypted secret              |

##### required

**Type:** `boolean`
**Default:** `false`

Fail deployment if secret is not available.

##### fromFile (source: env)

**Type:** `string`

Read secret value from file content. Supports `~` expansion. Falls back to environment variable if file not found.

```yaml
- name: GITHUB_TOKEN
  source: env
  fromFile: ~/.config/gh/token
```

##### path, mountPath (source: file)

Mount a file into the container.

```yaml
- name: SSH_KEY
  source: file
  path: ~/.ssh/id_rsa
  mountPath: /home/developer/.ssh/id_rsa
  permissions: "0600"
```

##### permissions (source: file)

**Type:** `string`
**Pattern:** `^0[0-7]{3}$`
**Default:** `0644`

Unix file permissions in octal.

##### vaultPath, vaultKey, vaultMount (source: vault)

HashiCorp Vault secret reference.

```yaml
- name: DB_PASSWORD
  source: vault
  vaultPath: secret/data/myapp
  vaultKey: db_password
  vaultMount: secret # Default: "secret"
```

##### s3Path (source: s3)

S3 path for encrypted secret.

```yaml
- name: CONFIG
  source: s3
  s3Path: s3://my-bucket/secrets/config.json
```

---

### Provider-Specific Configuration

Provider-specific options are under the `providers` key.

#### Docker Provider

```yaml
providers:
  docker:
    network: bridge # bridge | host | none
    restart: unless-stopped # no | always | on-failure | unless-stopped
    ports: # Additional port mappings
      - "8080:8080"
      - "3000:3000"
    privileged: false # Run in privileged mode (not recommended)
    extraHosts: # Extra /etc/hosts entries
      - "host.docker.internal:host-gateway"
    runtime: auto # runc | sysbox-runc | auto
    dind: # Docker-in-Docker configuration
      enabled: true
      mode: auto # sysbox | privileged | socket | auto
      storageDriver: auto # auto | overlay2 | fuse-overlayfs | vfs
      storageSize: 20GB
```

##### docker.network

**Type:** `string`
**Values:** `bridge`, `host`, `none`
**Default:** `bridge`

Docker network mode.

##### docker.restart

**Type:** `string`
**Values:** `no`, `always`, `on-failure`, `unless-stopped`
**Default:** `unless-stopped`

Container restart policy.

##### docker.runtime

**Type:** `string`
**Values:** `runc`, `sysbox-runc`, `auto`
**Default:** `auto`

Container runtime. `auto` uses sysbox-runc if available, falls back to runc.

##### docker.dind

Docker-in-Docker configuration.

| Option          | Type    | Default | Description                                                |
| --------------- | ------- | ------- | ---------------------------------------------------------- |
| `enabled`       | boolean | `false` | Enable DinD                                                |
| `mode`          | string  | `auto`  | `sysbox` (secure), `privileged` (legacy), `socket`, `auto` |
| `storageDriver` | string  | `auto`  | `auto`, `overlay2`, `fuse-overlayfs`, `vfs`                |
| `storageSize`   | string  | `20GB`  | Storage volume size limit                                  |

---

#### Fly.io Provider

```yaml
providers:
  fly:
    region: sjc # Fly.io region code
    organization: personal # Fly.io organization
    sshPort: 10022 # External SSH port (1024-65535)
    autoStopMachines: true # Auto-suspend when idle
    autoStartMachines: true # Auto-resume on connection
    cpuKind: shared # shared | performance
    highAvailability: false # Multi-machine deployment
```

##### fly.region

**Type:** `string`
**Default:** `sjc`

Fly.io region code.

| Region | Location               |
| ------ | ---------------------- |
| `sjc`  | San Jose, CA (US West) |
| `iad`  | Ashburn, VA (US East)  |
| `lhr`  | London, UK             |
| `fra`  | Frankfurt, Germany     |
| `nrt`  | Tokyo, Japan           |
| `syd`  | Sydney, Australia      |

Full list: https://fly.io/docs/reference/regions/

##### fly.autoStopMachines / fly.autoStartMachines

**Type:** `boolean`
**Default:** `true`

Auto-suspend machines after 5 minutes idle / auto-resume on SSH connection.

**Cost optimization:** Pay only for active time when enabled.

##### fly.cpuKind

**Type:** `string`
**Values:** `shared`, `performance`
**Default:** `shared`

- `shared` - Shared CPU (default, cost-effective)
- `performance` - Dedicated CPU (faster, higher cost)

---

#### Kubernetes Provider

```yaml
providers:
  kubernetes:
    namespace: default
    storageClass: standard
    ingress:
      enabled: false
      hostname: dev.example.com
```

##### kubernetes.namespace

**Type:** `string`
**Default:** `default`

Kubernetes namespace for deployment.

##### kubernetes.storageClass

**Type:** `string`

Kubernetes storage class for persistent volumes.

##### kubernetes.ingress

Ingress configuration for external access.

```yaml
ingress:
  enabled: true
  hostname: dev.example.com
```

---

#### DevPod Provider

DevPod supports multiple cloud backends.

```yaml
providers:
  devpod:
    type: aws # Required: aws | gcp | azure | digitalocean | kubernetes | ssh | docker
    buildRepository: ghcr.io/myorg/sindri # Registry for image push
    aws: # AWS-specific options
      region: us-west-2
      instanceType: c5.xlarge
      diskSize: 40
      useSpot: false
    # gcp, azure, digitalocean, kubernetes, ssh, docker sections...
```

##### devpod.type

**Type:** `string`
**Required:** Yes
**Values:** `aws`, `gcp`, `azure`, `digitalocean`, `kubernetes`, `ssh`, `docker`

DevPod provider type.

##### devpod.buildRepository

**Type:** `string`

Docker registry URL for image push. Required for non-local Kubernetes and cloud providers.

##### devpod.aws

AWS EC2 configuration.

```yaml
aws:
  region: us-west-2 # Default: us-west-2
  instanceType: c5.xlarge # Default: c5.xlarge
  diskSize: 40 # Root volume in GB (default: 40)
  useSpot: false # Use spot instances
  subnetId: subnet-xxx # VPC subnet ID
  securityGroupId: sg-xxx # Security group ID
```

##### devpod.gcp

GCP Compute Engine configuration.

```yaml
gcp:
  project: my-project # GCP project ID
  zone: us-central1-a # Default: us-central1-a
  machineType: e2-standard-4 # Default: e2-standard-4
  diskSize: 40 # Boot disk in GB (default: 40)
  diskType: pd-balanced # pd-standard | pd-balanced | pd-ssd
```

##### devpod.azure

Azure VM configuration.

```yaml
azure:
  subscription: xxx-xxx # Azure subscription ID
  resourceGroup: devpod-rg # Default: devpod-resources
  location: eastus # Default: eastus
  vmSize: Standard_D4s_v3 # Default: Standard_D4s_v3
  diskSize: 40 # OS disk in GB (default: 40)
```

##### devpod.digitalocean

DigitalOcean Droplet configuration.

```yaml
digitalocean:
  region: nyc3 # Default: nyc3
  size: s-4vcpu-8gb # Default: s-4vcpu-8gb
  diskSize: 40 # Block storage in GB (optional)
```

##### devpod.kubernetes

Kubernetes pod configuration.

```yaml
kubernetes:
  namespace: devpod # Default: devpod
  storageClass: standard # Storage class
  context: my-cluster # Kubernetes context
  nodeSelector: # Node selector labels
    gpu: "true"
```

##### devpod.ssh

SSH provider for existing machines.

```yaml
ssh:
  host: dev.example.com
  user: root # Default: root
  port: 22 # Default: 22
  keyPath: ~/.ssh/id_rsa # Default: ~/.ssh/id_rsa
```

---

#### E2B Provider

E2B provides ultra-fast cloud sandboxes with ~150ms startup times.

```yaml
providers:
  e2b:
    templateAlias: my-sindri-template
    reuseTemplate: true
    timeout: 3600 # Timeout in seconds (60-86400)
    autoPause: true # Pause on timeout
    autoResume: true # Resume on connect
    internetAccess: true # Outbound internet
    allowedDomains: # Whitelist domains (empty = all)
      - github.com
      - "*.github.com"
    blockedDomains: [] # Blacklist domains
    publicAccess: false # Public URL access
    metadata: # Custom metadata
      project: my-project
      environment: development
    team: my-team # E2B team for billing
    buildOnDeploy: false # Force rebuild on deploy
```

**Important:** E2B does not support GPU configuration.

---

#### Local Kubernetes (kind/k3d)

```yaml
providers:
  k8s:
    provider: kind # kind | k3d
    clusterName: sindri-dev
    version: v1.35.0 # Kubernetes version
    nodes: 1 # 1-10 nodes
    kind: # kind-specific options
      image: kindest/node:v1.35.0
      configFile: ./kind-config.yaml
    k3d: # k3d-specific options
      image: rancher/k3s:v1.35.0-k3s1
      registry:
        enabled: true
        name: k3d-registry
        port: 5000
```

---

## Complete Examples

### Minimal Local Docker

```yaml
version: "3.0"
name: sindri-local

deployment:
  provider: docker
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: minimal
```

### Fly.io Production

```yaml
version: "3.0"
name: sindri-prod

deployment:
  provider: fly
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
    verify_signature: true
    verify_provenance: true
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
    - monitoring

secrets:
  - name: ANTHROPIC_API_KEY
    source: env
    required: true
  - name: GITHUB_TOKEN
    source: env
    fromFile: ~/.config/gh/token

providers:
  fly:
    region: sjc
    organization: personal
    sshPort: 10022
    autoStopMachines: true
    autoStartMachines: true
    cpuKind: shared
```

### Kubernetes via DevPod

```yaml
version: "3.0"
name: sindri-k8s

deployment:
  provider: devpod
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
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
    buildRepository: ghcr.io/myorg/sindri
    kubernetes:
      namespace: dev-environments
      storageClass: fast-ssd
      context: my-production-cluster
```

### GPU-Enabled ML Development

```yaml
version: "3.0"
name: ml-dev

deployment:
  provider: devpod
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      count: 1
      tier: gpu-large
      memory: 40GB

extensions:
  profile: ai-dev
  additional:
    - pytorch
    - jupyter

providers:
  devpod:
    type: aws
    aws:
      region: us-west-2
      instanceType: p3.2xlarge
      diskSize: 100
```

### E2B AI Sandbox

```yaml
version: "3.0"
name: ai-sandbox

deployment:
  provider: e2b
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: ai-dev

providers:
  e2b:
    timeout: 3600
    autoPause: true
    internetAccess: true
    allowedDomains:
      - api.anthropic.com
      - api.openai.com
      - github.com
    metadata:
      purpose: ai-development
```

---

## Environment Variables

Environment variables in the container are set via:

1. **Secrets** - Injected via `secrets` configuration
2. **Provider-specific** - Set by the deployment provider
3. **Extension-specific** - Set by installed extensions

**Common environment variables:**

| Variable          | Description         |
| ----------------- | ------------------- |
| `SINDRI_NAME`     | Deployment name     |
| `SINDRI_PROVIDER` | Deployment provider |
| `SINDRI_VERSION`  | Sindri version      |
| `HOME`            | User home directory |
| `WORKSPACE`       | Workspace directory |

---

## Configuration Validation

### CLI Validation

```bash
# Validate configuration
sindri config validate

# Validate specific file
sindri config validate --file ./sindri.yaml

# Validate with extension checking
sindri config validate --check-extensions
```

### Schema Location

JSON schema for validation:

```
v3/schemas/sindri.schema.json
```

### Common Validation Errors

| Error                       | Cause                            | Solution                                  |
| --------------------------- | -------------------------------- | ----------------------------------------- |
| `name: pattern mismatch`    | Name contains invalid characters | Use lowercase, alphanumeric, hyphens only |
| `provider: unknown`         | Invalid provider value           | Use: docker, fly, devpod, e2b, kubernetes |
| `extensions: oneOf failed`  | Both profile and active set      | Use profile OR active, not both           |
| `version: pattern mismatch` | Invalid version format           | Use `X.Y` format (e.g., "3.0")            |

---

## Migration from V2

### Key Changes

1. **Version field**: Change `1.0` to `3.0`
2. **Image configuration**: Use `image_config` instead of `image`
3. **Provider field**: Moved from `deployment.provider` (same location, same values)

### Migration Example

**V2 Configuration:**

```yaml
version: 1.0
name: my-env

deployment:
  provider: fly
  image: ghcr.io/pacphi/sindri:v2.0.0
  resources:
    memory: 4GB
    cpus: 2
```

**V3 Configuration:**

```yaml
version: "3.0"
name: my-env

deployment:
  provider: fly
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
    verify_signature: true
  resources:
    memory: 4GB
    cpus: 2
```

### Migration Command

```bash
# Check migration status
sindri migrate status

# Run migration with backup
sindri migrate run --backup

# Rollback if needed
sindri migrate rollback
```

---

## Related Documentation

- [Getting Started](getting-started.md)
- [Image Management](image-management.md)
- [Architecture Decision Records](architecture/adr/README.md)
