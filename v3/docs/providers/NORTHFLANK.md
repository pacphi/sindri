# Northflank Provider

> **Version:** 3.x
> **Last Updated:** 2026-02

Enterprise PaaS with GPU support, auto-scaling, native pause/resume, and 16+ global regions.

## Overview

The Northflank provider deploys Sindri environments to Northflank's managed Kubernetes platform with:

- **Native pause/resume** - Suspend services to stop compute costs, resume instantly
- **GPU support** - H100, B200, A100, L4, H200, and AMD MI300X GPUs
- **Auto-scaling** - CPU/memory-based horizontal scaling with 15-second metric intervals
- **Persistent volumes** - SSD-backed storage up to 1.5 TB
- **Interactive shell** - Container exec via CLI for direct access
- **Port forwarding** - Access services locally without public exposure
- **16+ regions** - Deploy globally on Northflank's managed cloud or BYOC

**Best for:** GPU workloads, enterprise environments, auto-scaled services, cost-conscious teams

## Prerequisites

| Requirement        | Check Command              | Install / Setup                    |
| ------------------ | -------------------------- | ---------------------------------- |
| Node.js            | `node --version`           | [nodejs.org](https://nodejs.org/)  |
| Northflank CLI     | `northflank --version`     | `npm i @northflank/cli -g`         |
| Northflank account | `northflank list projects` | `northflank login` (opens browser) |

### Authentication

```bash
# Interactive login (opens browser for OAuth)
northflank login

# Or login with an API token directly
northflank login --token <YOUR_TOKEN>

# Or set the token as an environment variable
export NORTHFLANK_API_TOKEN=your_api_token
```

API tokens can be created in the Northflank web UI under your account or team settings.

## Quick Start

```bash
# 1. Install the Northflank CLI
npm i @northflank/cli -g

# 2. Authenticate
northflank login

# 3. Create configuration
cat > sindri.yaml << 'EOF'
version: "3.0"
name: my-sindri-nf

deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: fullstack

providers:
  northflank:
    projectName: sindri-dev
    computePlan: nf-compute-200
EOF

# 4. Deploy
sindri deploy

# 5. Connect
sindri connect
```

## Configuration

### Basic Configuration

```yaml
version: "3.0"
name: sindri-nf

deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: fullstack

providers:
  northflank:
    projectName: sindri-dev
    computePlan: nf-compute-50
```

### Advanced Configuration

```yaml
version: "3.0"
name: sindri-nf-prod

deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 8GB
    cpus: 4
  volumes:
    workspace:
      size: 50GB

extensions:
  profile: enterprise

secrets:
  - name: GITHUB_TOKEN
    source: env
    required: true
  - name: DATABASE_URL
    source: env
    required: true

providers:
  northflank:
    projectName: sindri-production
    serviceName: sindri-workspace
    computePlan: nf-compute-400-16
    region: us-east
    instances: 2
    volumeSizeGb: 50
    volumeMountPath: /workspace
    registryCredentials: ghcr-creds-id
    ports:
      - name: http
        internalPort: 8080
        public: true
        protocol: HTTP
      - name: ssh
        internalPort: 22
        public: false
        protocol: TCP
    autoScaling:
      enabled: true
      minInstances: 1
      maxInstances: 5
      targetCpuUtilization: 70
      targetMemoryUtilization: 80
    healthCheck:
      type: http
      path: /healthz
      port: 8080
      initialDelaySeconds: 15
      periodSeconds: 10
      failureThreshold: 3
```

### GPU Configuration

```yaml
version: "3.0"
name: sindri-gpu

deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-xlarge

extensions:
  profile: fullstack
  additional:
    - python
    - cuda

providers:
  northflank:
    projectName: sindri-ml
    computePlan: nf-compute-800-32
    gpuType: nvidia-h100
    region: us-east
    volumeSizeGb: 100
```

**GPU Tier Mapping:**

| Sindri GPU Tier | Northflank GPU   | VRAM  | Est. Hourly |
| --------------- | ---------------- | ----- | ----------- |
| `gpu-small`     | nvidia-l4        | 24 GB | Varies      |
| `gpu-medium`    | nvidia-a100-40gb | 40 GB | ~$1.42      |
| `gpu-large`     | nvidia-a100-80gb | 80 GB | ~$1.76      |
| `gpu-xlarge`    | nvidia-h100      | 80 GB | ~$2.74      |

Additional GPU types available directly via `gpuType`:

| GPU Type      | VRAM   | Est. Hourly | Regions                                  |
| ------------- | ------ | ----------- | ---------------------------------------- |
| `nvidia-h200` | 141 GB | Varies      | EU-West-NL, US-Central, US-East, US-West |
| `nvidia-b200` | 180 GB | ~$5.87      | Asia-NE, Asia-SE, EU-West-NL, US-East    |
| AMD MI300X    | N/A    | Varies      | Select regions                           |

> GPU deployments require pre-purchased credits. Verify GPU availability in your target region before deploying.

### Auto-Scaling Configuration

```yaml
providers:
  northflank:
    projectName: sindri-dev
    computePlan: nf-compute-200
    autoScaling:
      enabled: true
      minInstances: 1
      maxInstances: 5
      targetCpuUtilization: 70
      targetMemoryUtilization: 80
```

**Auto-scaling behavior:**

- Metrics are checked every **15 seconds**
- Scale-up is **immediate** when thresholds are exceeded
- Scale-down has a **5-minute cooldown** to prevent flapping
- Cannot be used with persistent volumes (volumes limit service to 1 instance)

### Health Check Configuration

```yaml
# HTTP health check
providers:
  northflank:
    healthCheck:
      type: http
      path: /health
      port: 8080
      initialDelaySeconds: 10
      periodSeconds: 15
      failureThreshold: 3

# TCP health check
providers:
  northflank:
    healthCheck:
      type: tcp
      port: 8080

# Command-based health check
providers:
  northflank:
    healthCheck:
      type: command
      command: ["/bin/sh", "-c", "curl -f http://localhost:8080/health"]
      initialDelaySeconds: 30
      failureThreshold: 5
```

## Configuration Reference

| Field                 | Type    | Required | Default         | Description                                                 |
| --------------------- | ------- | -------- | --------------- | ----------------------------------------------------------- |
| `projectName`         | string  | Yes      | -               | Northflank project name (created if it does not exist)      |
| `serviceName`         | string  | No       | `{name}`        | Service name within the project                             |
| `computePlan`         | string  | No       | `nf-compute-50` | Compute plan identifier                                     |
| `gpuType`             | string  | No       | -               | GPU type (e.g., `nvidia-h100`, `nvidia-a100-40gb`)          |
| `instances`           | integer | No       | `1`             | Number of service instances (0 to pause billing)            |
| `region`              | string  | No       | `us-east`       | Region slug (immutable after project creation)              |
| `volumeSizeGb`        | integer | No       | -               | Persistent volume size in GB (limits service to 1 instance) |
| `volumeMountPath`     | string  | No       | `/data`         | Mount path for persistent volume                            |
| `registryCredentials` | string  | No       | -               | Credential ID for pulling private images                    |
| `ports`               | array   | No       | `[]`            | Port configuration (see Ports section)                      |
| `autoScaling`         | object  | No       | disabled        | Horizontal auto-scaling (see Auto-Scaling section)          |
| `healthCheck`         | object  | No       | -               | Health check / liveness probe configuration                 |

### Port Configuration

| Field          | Type    | Required | Default | Description                              |
| -------------- | ------- | -------- | ------- | ---------------------------------------- |
| `name`         | string  | Yes      | -       | Port identifier (e.g., `http`, `ssh`)    |
| `internalPort` | integer | Yes      | -       | Container-internal port number (1-65535) |
| `public`       | boolean | No       | `false` | Expose publicly with auto-TLS            |
| `protocol`     | string  | No       | `HTTP`  | Protocol: `HTTP`, `TCP`, or `UDP`        |

### Auto-Scaling Configuration

| Field                     | Type    | Required | Default | Description                                    |
| ------------------------- | ------- | -------- | ------- | ---------------------------------------------- |
| `enabled`                 | boolean | No       | `false` | Enable horizontal auto-scaling                 |
| `minInstances`            | integer | No       | `1`     | Minimum instance count (scale-down floor)      |
| `maxInstances`            | integer | No       | `3`     | Maximum instance count (scale-up ceiling)      |
| `targetCpuUtilization`    | integer | No       | `70`    | CPU % threshold to trigger scale-up (1-100)    |
| `targetMemoryUtilization` | integer | No       | `80`    | Memory % threshold to trigger scale-up (1-100) |

### Health Check Configuration

| Field                 | Type    | Required        | Default | Description                               |
| --------------------- | ------- | --------------- | ------- | ----------------------------------------- |
| `type`                | string  | No              | `tcp`   | Check method: `http`, `tcp`, or `command` |
| `path`                | string  | If `http`       | -       | HTTP endpoint path                        |
| `port`                | integer | If `http`/`tcp` | -       | Port to check (1-65535)                   |
| `command`             | array   | If `command`    | -       | Command to execute inside container       |
| `initialDelaySeconds` | integer | No              | `10`    | Delay before first check                  |
| `periodSeconds`       | integer | No              | `15`    | Interval between checks                   |
| `failureThreshold`    | integer | No              | `3`     | Consecutive failures before restart       |

## Compute Plans

| Plan                 | vCPU | Memory | Hourly | Monthly |
| -------------------- | ---- | ------ | ------ | ------- |
| `nf-compute-10`      | 0.1  | 256 MB | $0.004 | $2.70   |
| `nf-compute-20`      | 0.2  | 512 MB | $0.008 | $5.40   |
| `nf-compute-50`      | 0.5  | 1 GB   | $0.017 | $12.00  |
| `nf-compute-100-1`   | 1.0  | 1 GB   | $0.025 | $18.00  |
| `nf-compute-100-2`   | 1.0  | 2 GB   | $0.033 | $24.00  |
| `nf-compute-100-4`   | 1.0  | 4 GB   | $0.050 | $36.00  |
| `nf-compute-200`     | 2.0  | 4 GB   | $0.067 | $48.00  |
| `nf-compute-200-8`   | 2.0  | 8 GB   | $0.100 | $72.00  |
| `nf-compute-200-16`  | 2.0  | 16 GB  | $0.167 | $120.00 |
| `nf-compute-400`     | 4.0  | 8 GB   | $0.133 | $96.00  |
| `nf-compute-400-16`  | 4.0  | 16 GB  | $0.200 | $144.00 |
| `nf-compute-800-8`   | 8.0  | 8 GB   | $0.200 | $144.00 |
| `nf-compute-800-16`  | 8.0  | 16 GB  | $0.267 | $192.00 |
| `nf-compute-800-24`  | 8.0  | 24 GB  | $0.333 | $240.00 |
| `nf-compute-800-32`  | 8.0  | 32 GB  | $0.400 | $288.00 |
| `nf-compute-800-40`  | 8.0  | 40 GB  | $0.467 | $336.00 |
| `nf-compute-1200-24` | 12.0 | 24 GB  | $0.400 | $288.00 |
| `nf-compute-1600-32` | 16.0 | 32 GB  | $0.533 | $384.00 |
| `nf-compute-2000-40` | 20.0 | 40 GB  | $0.667 | $480.00 |

**Plan naming convention:** `nf-compute-{vCPU*100}[-{memoryGB}]`

- `nf-compute-200` = 2 vCPU, default memory (4 GB)
- `nf-compute-200-8` = 2 vCPU, 8 GB memory

When no `computePlan` is specified and generic `resources` are set in `sindri.yaml`, the adapter automatically selects the closest matching plan.

## Deployment Commands

```bash
# Deploy (creates project, service, volume)
sindri deploy

# Preview deployment plan (dry-run)
sindri plan

# Check status
sindri status

# Connect (interactive shell via northflank exec)
sindri connect

# Pause service (stops compute billing)
sindri stop

# Resume paused service
sindri start

# Destroy (removes service, optionally project)
sindri destroy
```

## Pause and Resume (Cost Optimization)

Northflank natively supports pausing and resuming services. Paused services do not incur compute costs -- only persistent volume storage is billed.

```bash
# Pause a running service
sindri stop
# Equivalent to: northflank pause service --project <project> --service <service>

# Resume a paused service
sindri start
# Equivalent to: northflank resume service --project <project> --service <service>
```

When you run `sindri connect` on a paused service, the adapter automatically resumes it before establishing the shell session.

## Port Forwarding

Forward remote service ports to your local machine without exposing them publicly:

```bash
# Forward a specific service
northflank forward service --project sindri-dev --service my-workspace

# Forward all services and addons in current context
northflank forward all
```

This is useful for accessing web UIs, databases, or other services running inside the container without configuring public endpoints.

## Secrets Management

Sindri secrets are injected as runtime environment variables during service creation:

```yaml
secrets:
  - name: GITHUB_TOKEN
    source: env
    required: true
  - name: DATABASE_URL
    source: env
    required: true
  - name: API_SECRET
    source: vault
    vaultPath: secret/data/sindri
    vaultKey: api_secret
```

You can also manage secrets directly through the Northflank CLI or API:

```bash
# Northflank also supports secret groups at the project level
# These are inherited by all services in the project
# Manage via the Northflank web UI or API
```

> The Northflank provider supports environment variable secrets only. File-based secrets are not supported and will be skipped with a warning.

## Regions

### Northflank Managed Cloud (16 regions)

| Region Slug               | Location                    |
| ------------------------- | --------------------------- |
| `us-east`                 | US - East                   |
| `us-east-ohio`            | US - East - Ohio            |
| `us-west`                 | US - West                   |
| `us-west-california`      | US - West - California      |
| `us-central`              | US - Central                |
| `canada-central`          | Canada - Central            |
| `europe-west`             | Europe - West (London)      |
| `europe-west-frankfurt`   | Europe - West - Frankfurt   |
| `europe-west-netherlands` | Europe - West - Netherlands |
| `europe-west-zurich`      | Europe - West - Zurich      |
| `asia-east`               | Asia - East                 |
| `asia-northeast`          | Asia - Northeast            |
| `asia-southeast`          | Asia - Southeast            |
| `australia-southeast`     | Australia - Southeast       |
| `africa-south`            | Africa - South              |
| `southamerica-east`       | South America - East        |

> Region is set at project creation and **cannot be changed** afterward. Choose carefully.

### BYOC (Bring Your Own Cloud)

Northflank supports deploying to your own cloud account for 600+ additional regions:

| Cloud Provider | Regions   |
| -------------- | --------- |
| AWS            | 25+       |
| GCP            | 35+       |
| Azure          | 60+       |
| OCI (Oracle)   | Available |
| Civo           | Available |
| CoreWeave      | Available |
| Bare-metal     | Custom    |

BYOC requires an enterprise plan. Resources deploy within your VPC with configurable security groups.

## Troubleshooting

### Authentication Issues

**Symptom:** `northflank list projects` fails or returns unauthorized

**Solution:**

```bash
# Re-authenticate via browser
northflank login

# Or set API token directly
export NORTHFLANK_API_TOKEN=your_api_token

# Verify authentication
northflank list projects
```

### Service Won't Start

**Symptom:** Service stays in "creating" or "error" state

**Solution:**

```bash
# Check service logs
northflank get service details --project <project> --service <service>

# Common causes:
# - Image cannot be pulled (check registry credentials)
# - Compute plan quota exceeded (upgrade plan or delete unused resources)
# - Health check failing (review health check config or increase initialDelaySeconds)
```

### GPU Not Available

**Symptom:** Deployment fails with GPU availability error

**Solution:**

```bash
# 1. Verify GPU availability in your target region
# Not all GPU types are available in all regions

# 2. Try a different region with GPU support
# H100 has the broadest availability (6+ regions)

# 3. Check that you have pre-purchased GPU credits
# GPU usage requires credits on Northflank

# 4. Consider fractional GPU allocation if full GPU is unavailable
```

### Volume and Scaling Conflict

**Symptom:** Warning about instances being limited to 1

**Cause:** Northflank limits services with persistent volumes to a single instance

**Solution:**

- If you need multiple instances, remove the volume configuration
- If you need persistence, accept the single-instance limitation
- Consider using an external database or object storage instead of volumes

### Auto-Scaling Not Working

**Symptom:** Service does not scale up under load

**Solution:**

```bash
# Verify auto-scaling is enabled and properly configured
# Check that:
# 1. autoScaling.enabled is true
# 2. maxInstances > 1
# 3. No persistent volume is attached (volumes limit to 1 instance)
# 4. CPU/memory thresholds are set appropriately
#    (70% CPU and 80% memory are good defaults)

# Check current resource utilization in the Northflank dashboard
```

### Rate Limiting

**Symptom:** API calls fail with 429 status

**Cause:** Northflank's default rate limit is 1000 requests per hour

**Solution:**

- The adapter automatically retries with exponential backoff
- If persistent, wait for the rate limit window to reset (1 hour)
- Contact Northflank support for higher rate limits if needed

## Cost Optimization

1. **Use pause/resume** -- Pause services when not in use (`sindri stop`). Paused services incur no compute costs.

2. **Right-size compute plans** -- Start with `nf-compute-50` (0.5 vCPU, 1 GB) and scale up only when needed.

3. **Enable auto-scaling** -- Let the platform scale instances based on actual demand instead of over-provisioning.

4. **Spot GPU orchestration** -- Northflank supports automated spot GPU scheduling for up to 90% cost reduction on GPU workloads.

5. **Scale to zero** -- Set `instances: 0` to preserve configuration without any compute billing.

6. **Delete unused volumes** -- Volumes are billed even when services are paused.

7. **Choose efficient plans** -- Some plans offer better price-per-resource ratios (e.g., `nf-compute-1200-24` at $0.400/hr provides 12 vCPU vs `nf-compute-800-32` at $0.400/hr with 8 vCPU).

## Comparison to Other Providers

| Feature                | Fly.io                 | E2B                    | Northflank                          |
| ---------------------- | ---------------------- | ---------------------- | ----------------------------------- |
| **GPU support**        | A100, L40s             | None                   | H100, B200, A100, L4, H200, MI300X  |
| **Pause/resume**       | Machine suspend        | Sandbox pause          | Native service pause (instant)      |
| **Auto-scaling**       | Limited                | None                   | CPU/memory-based horizontal scaling |
| **Persistent storage** | Fly Volumes            | None                   | SSD volumes up to 1.5 TB            |
| **Managed databases**  | None                   | None                   | PostgreSQL, MySQL, MongoDB, Redis   |
| **Health checks**      | TCP only               | None                   | HTTP, TCP, command probes           |
| **Regions**            | 30+                    | Limited                | 16 managed + 600+ BYOC              |
| **Connect method**     | SSH                    | WebSocket PTY          | Container exec + port forwarding    |
| **Pricing model**      | Pay-per-second         | Pay-per-second         | Per-second billing                  |
| **BYOC**               | No                     | No                     | AWS, GCP, Azure, OCI, bare-metal    |
| **Config file**        | `fly.toml` (generated) | `e2b.toml` (generated) | None (CLI args only)                |

## Example Scenarios

### Development Environment

A lightweight workspace for daily development:

```yaml
version: "3.0"
name: dev-workspace

deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: fullstack

providers:
  northflank:
    projectName: sindri-dev
    computePlan: nf-compute-50
    volumeSizeGb: 10
    volumeMountPath: /workspace
```

### ML Training with GPU

A GPU-accelerated environment for machine learning:

```yaml
version: "3.0"
name: ml-training

deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-xlarge

extensions:
  profile: fullstack
  additional:
    - python
    - cuda

providers:
  northflank:
    projectName: ml-workloads
    computePlan: nf-compute-800-32
    gpuType: nvidia-a100-80gb
    volumeSizeGb: 100
    volumeMountPath: /workspace
    region: us-east
```

### Production API with Auto-Scaling

A production service with health checks and auto-scaling:

```yaml
version: "3.0"
name: api-service

deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 8GB
    cpus: 4

extensions:
  profile: enterprise

secrets:
  - name: DATABASE_URL
    source: env
    required: true

providers:
  northflank:
    projectName: production
    serviceName: api-primary
    computePlan: nf-compute-400-16
    region: us-east
    instances: 2
    ports:
      - name: http
        internalPort: 8080
        public: true
        protocol: HTTP
      - name: metrics
        internalPort: 9090
        public: false
        protocol: HTTP
    autoScaling:
      enabled: true
      minInstances: 2
      maxInstances: 10
      targetCpuUtilization: 65
      targetMemoryUtilization: 75
    healthCheck:
      type: http
      path: /healthz
      port: 8080
      initialDelaySeconds: 15
      periodSeconds: 10
      failureThreshold: 3
```

### Cost-Optimized Workspace

A workspace that pauses automatically to minimize costs:

```yaml
version: "3.0"
name: budget-dev

deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: minimal

providers:
  northflank:
    projectName: personal-dev
    computePlan: nf-compute-50
    volumeSizeGb: 10
    # Use sindri stop/start for manual pause/resume
    # Paused services cost $0 for compute
```

## Current Limitations

- **Region Immutable:** Project region cannot be changed after creation. Choose carefully during initial deployment.
- **Volume + Scaling Conflict:** Services with persistent volumes are limited to a single instance (cannot use auto-scaling).
- **GPU Requires Credits:** GPU deployments require pre-purchased credits. Verify availability in your target region.
- **Manual Cleanup:** The `destroy` command removes the service but preserves the project. Delete projects manually if no longer needed.
- **Secret Updates:** Updating secrets requires service restart to take effect.

## Related Documentation

- [Provider Overview](README.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [CLI Reference](../CLI.md)
- [Northflank Official Docs](https://northflank.com/docs/)
- [Northflank API Reference](https://northflank.com/docs/v1/api/introduction)
- [Northflank CLI Reference](https://northflank.com/docs/v1/api/use-the-cli)
- [Northflank Pricing](https://northflank.com/pricing)
