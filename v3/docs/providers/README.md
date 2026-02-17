# Sindri V3 Provider Documentation

> **Version:** 3.x
> **Last Updated:** 2026-01

This directory contains documentation for Sindri V3 deployment providers.

## Available Providers

| Provider                    | Description                         | Best For                                   | GPU Support               |
| --------------------------- | ----------------------------------- | ------------------------------------------ | ------------------------- |
| [Docker](DOCKER.md)         | Local containers via Docker Compose | Local development, testing, CI/CD          | Yes (nvidia-runtime)      |
| [Fly.io](FLY.md)            | Cloud deployment with auto-suspend  | Remote development, cost-effective cloud   | Yes (A100, L40s)          |
| [DevPod](DEVPOD.md)         | Multi-cloud DevContainers           | IDE integration, multi-cloud flexibility   | Yes (via cloud providers) |
| [E2B](E2B.md)               | Cloud sandboxes with pause/resume   | AI sandboxes, fast startup, pay-per-second | No                        |
| [Kubernetes](KUBERNETES.md) | Container orchestration             | Enterprise, multi-tenant, CI/CD            | Yes (node selectors)      |
| [RunPod](RUNPOD.md)         | GPU cloud with 40+ GPU types        | GPU-intensive ML, cost-sensitive research  | Yes (40+ GPU types)       |
| [Northflank](NORTHFLANK.md) | PaaS with auto-scaling and GPUs     | Auto-scaling apps, enterprise deployments  | Yes (L4, A100, H100+)     |
| [VM Images](VM.md)          | Multi-cloud VM image building       | Golden images, enterprise, pre-baked envs  | Yes (cloud-dependent)     |

### VM Image Cloud-Specific Guides

| Cloud Provider                     | Description                               | Builder Type  |
| ---------------------------------- | ----------------------------------------- | ------------- |
| [AWS VM](vm/AWS.md)                | EC2 AMI images via amazon-ebs builder     | amazon-ebs    |
| [Azure VM](vm/AZURE.md)            | Managed images with Shared Image Gallery  | azure-arm     |
| [GCP VM](vm/GCP.md)                | Compute Engine images via googlecompute   | googlecompute |
| [OCI VM](vm/OCI.md)                | Oracle Cloud Infrastructure custom images | oracle-oci    |
| [Alibaba VM](vm/ALIBABA.md)        | Alibaba Cloud ECS custom images           | alicloud-ecs  |
| [Security Guide](vm/SECURITY.md)   | CIS hardening, OpenSCAP, encryption       | -             |
| [Distribution](vm/DISTRIBUTION.md) | Multi-cloud image sharing and publication | -             |

## Quick Comparison

### Connection Methods

| Provider   | SSH       | WebSocket | IDE Integration        |
| ---------- | --------- | --------- | ---------------------- |
| Docker     | No (exec) | No        | VS Code Dev Containers |
| Fly.io     | Yes       | No        | VS Code Remote SSH     |
| DevPod     | Yes       | No        | VS Code, JetBrains     |
| E2B        | No        | Yes (PTY) | Limited                |
| Kubernetes | No (exec) | No        | VS Code Remote K8s     |
| RunPod     | Yes       | No        | VS Code Remote SSH     |
| Northflank | No (exec) | No        | Port forwarding        |
| VM Images  | Yes       | No        | VS Code Remote SSH     |

### Cost Model

| Provider   | Pricing              | Auto-Suspend       | Persistence     |
| ---------- | -------------------- | ------------------ | --------------- |
| Docker     | Free (local)         | N/A                | Volumes         |
| Fly.io     | Per-second + storage | Yes (suspend)      | Volumes         |
| DevPod     | Depends on backend   | Depends on backend | Volumes         |
| E2B        | Per-second           | Yes (pause)        | Pause/Resume    |
| Kubernetes | Depends on cluster   | Scale to zero      | PVC             |
| RunPod     | Per-second           | No (stop/start)    | Network volumes |
| Northflank | Per-second           | Yes (pause/resume) | SSD volumes     |
| VM Images  | Build + cloud costs  | Cloud-dependent    | EBS/Disk        |

### Prerequisites

| Provider   | Required                   | Optional               |
| ---------- | -------------------------- | ---------------------- |
| Docker     | Docker Engine, Compose v2  | Sysbox, nvidia-runtime |
| Fly.io     | flyctl CLI, Fly.io account | Dedicated IPv4         |
| DevPod     | devpod CLI, Docker         | kubectl, cloud CLIs    |
| E2B        | e2b CLI, E2B_API_KEY       | -                      |
| Kubernetes | kubectl                    | kind, k3d              |
| RunPod     | RUNPOD_API_KEY             | SSH client             |
| Northflank | Northflank CLI, account    | Node.js                |
| VM Images  | Packer 1.9+, cloud CLI     | Cloud-specific plugins |

## Choosing a Provider

### For Local Development

**Docker** is the recommended choice:

- Zero cloud costs
- Fast iteration
- Works offline
- Full control over environment

```yaml
deployment:
  provider: docker
```

### For Remote Development

**Fly.io** offers the best balance:

- Auto-suspend saves costs
- Global regions for low latency
- Persistent volumes
- SSH access for VS Code Remote

```yaml
deployment:
  provider: fly

providers:
  fly:
    region: sjc
    autoStopMachines: true
```

### For IDE Integration

**DevPod** provides the richest IDE experience:

- VS Code Dev Containers
- JetBrains Gateway
- Multi-cloud backends
- DevContainer standard

```yaml
deployment:
  provider: devpod

providers:
  devpod:
    type: docker # or kubernetes, aws, gcp, azure
```

### For AI/Agent Sandboxes

**E2B** is optimized for ephemeral execution:

- ~150ms startup from snapshots
- Pause/resume preserves state
- Pay only for active time
- WebSocket access through firewalls

```yaml
deployment:
  provider: e2b

providers:
  e2b:
    timeout: 3600
    autoPause: true
```

### For Enterprise/Production

**Kubernetes** offers the most control:

- Multi-tenant isolation
- Resource quotas
- RBAC integration
- GitOps workflows

```yaml
deployment:
  provider: kubernetes

providers:
  kubernetes:
    namespace: dev-environments
    storageClass: fast-ssd
```

### For GPU-Intensive ML Workloads

**RunPod** has the widest GPU selection:

- 40+ GPU types from RTX 3070 to H200/B200
- Spot pricing for 60-70% savings
- Per-second billing, no minimums
- Network volumes for persistent model storage
- No CLI required (direct REST API)

```yaml
deployment:
  provider: runpod

providers:
  runpod:
    gpuType: "NVIDIA RTX A4000"
    containerDiskGb: 50
    volumeSizeGb: 20
```

### For Auto-Scaling Production Apps

**Northflank** provides managed Kubernetes with auto-scaling:

- Native pause/resume (zero compute cost when paused)
- CPU/memory-based horizontal auto-scaling
- GPU support (L4, A100, H100, H200, B200)
- Health checks (HTTP, TCP, command)
- 16 managed regions + 600+ BYOC

```yaml
deployment:
  provider: northflank

providers:
  northflank:
    projectName: sindri-dev
    computePlan: nf-compute-200
    autoScaling:
      enabled: true
      minInstances: 1
      maxInstances: 5
```

### For Pre-built VM Images

**VM Images** enables golden image pipelines:

- Multi-cloud support (AWS, Azure, GCP, OCI, Alibaba)
- Pre-baked environments for fast instance launches
- Consistent images across teams
- CI/CD integration for automated builds

```yaml
deployment:
  provider: packer

providers:
  packer:
    cloud: aws
    profile: fullstack
    region: us-west-2
```

See cloud-specific guides:

- [AWS VM](vm/AWS.md) - EC2 AMI images
- [Azure VM](vm/AZURE.md) - Azure Managed Images
- [GCP VM](vm/GCP.md) - Compute Engine images
- [OCI VM](vm/OCI.md) - Oracle Cloud images
- [Alibaba VM](vm/ALIBABA.md) - Alibaba Cloud ECS images

## Common Workflows

### Deploy

```bash
# Deploy with configured provider
sindri deploy

# Deploy with specific provider
sindri deploy --provider fly

# Preview deployment plan
sindri plan
```

### Connect

```bash
# Connect to deployed environment
sindri connect

# Check status
sindri status
```

### Lifecycle Management

```bash
# Stop (preserves state)
sindri stop

# Start (resume)
sindri start

# Destroy (removes all resources)
sindri destroy
```

## Provider-Specific Commands

### Fly.io

```bash
flyctl status -a <app-name>
flyctl logs -a <app-name>
flyctl ssh console -a <app-name>
```

### DevPod

```bash
devpod list
devpod ssh <workspace>
devpod stop <workspace>
```

### E2B

```bash
e2b sandbox list
e2b sandbox terminal <id>
e2b sandbox pause <id>
```

### Kubernetes

```bash
kubectl get pods -n <namespace>
kubectl logs <pod> -n <namespace>
kubectl exec -it <pod> -n <namespace> -- /bin/bash
```

### RunPod

```bash
sindri deploy                          # Create pod
sindri status                          # Check pod status
sindri connect                         # SSH to pod
sindri stop                            # Stop pod (preserves volume)
sindri start                           # Resume stopped pod
sindri destroy                         # Terminate pod permanently
```

### Northflank

```bash
sindri deploy                          # Create project + service
sindri status                          # Check service status
sindri connect                         # Interactive shell via exec
sindri stop                            # Pause service (no compute cost)
sindri start                           # Resume paused service
sindri destroy                         # Remove service
northflank forward service --project <project> --service <service>  # Port forward
```

### VM Images

```bash
sindri vm doctor --cloud aws      # Check prerequisites
sindri vm build --cloud aws       # Build image
sindri vm list --cloud aws        # List images
sindri vm deploy --cloud aws <id> # Deploy from image
sindri vm delete --cloud aws <id> # Delete image
```

## Migration from V2

V3 providers maintain compatibility with V2 configurations while offering enhanced features:

| Feature            | V2           | V3                                       |
| ------------------ | ------------ | ---------------------------------------- |
| Configuration      | sindri.yaml  | sindri.yaml (enhanced schema)            |
| Template rendering | Bash scripts | Rust-based Handlebars                    |
| Error handling     | Basic        | Comprehensive with recovery              |
| GPU support        | Limited      | Full (RunPod 40+ GPUs, Northflank H100+) |
| Auto-suspend       | Fly.io only  | Fly.io, E2B, Northflank                  |

## Related Documentation

- [Provider Architecture](ARCHITECTURE.md) -- How providers translate config into deployments
- [Configuration Reference](../CONFIGURATION.md)
- [CLI Reference](../CLI.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [Getting Started](../GETTING_STARTED.md)
