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
| [Packer](PACKER.md)         | Multi-cloud VM image building       | Golden images, enterprise, pre-baked envs  | Yes (cloud-dependent)     |

### Packer Cloud-Specific Guides

| Cloud Provider                      | Description                               | Builder Type  |
| ----------------------------------- | ----------------------------------------- | ------------- |
| [AWS Packer](packer/AWS.md)         | EC2 AMI images via amazon-ebs builder     | amazon-ebs    |
| [Azure Packer](packer/AZURE.md)     | Managed images with Shared Image Gallery  | azure-arm     |
| [GCP Packer](packer/GCP.md)         | Compute Engine images via googlecompute   | googlecompute |
| [OCI Packer](packer/OCI.md)         | Oracle Cloud Infrastructure custom images | oracle-oci    |
| [Alibaba Packer](packer/ALIBABA.md) | Alibaba Cloud ECS custom images           | alicloud-ecs  |

## Quick Comparison

### Connection Methods

| Provider   | SSH       | WebSocket | IDE Integration        |
| ---------- | --------- | --------- | ---------------------- |
| Docker     | No (exec) | No        | VS Code Dev Containers |
| Fly.io     | Yes       | No        | VS Code Remote SSH     |
| DevPod     | Yes       | No        | VS Code, JetBrains     |
| E2B        | No        | Yes (PTY) | Limited                |
| Kubernetes | No (exec) | No        | VS Code Remote K8s     |
| Packer     | Yes       | No        | VS Code Remote SSH     |

### Cost Model

| Provider   | Pricing              | Auto-Suspend       | Persistence  |
| ---------- | -------------------- | ------------------ | ------------ |
| Docker     | Free (local)         | N/A                | Volumes      |
| Fly.io     | Per-second + storage | Yes (suspend)      | Volumes      |
| DevPod     | Depends on backend   | Depends on backend | Volumes      |
| E2B        | Per-second           | Yes (pause)        | Pause/Resume |
| Kubernetes | Depends on cluster   | Scale to zero      | PVC          |
| Packer     | Build + cloud costs  | Cloud-dependent    | EBS/Disk     |

### Prerequisites

| Provider   | Required                   | Optional               |
| ---------- | -------------------------- | ---------------------- |
| Docker     | Docker Engine, Compose v2  | Sysbox, nvidia-runtime |
| Fly.io     | flyctl CLI, Fly.io account | Dedicated IPv4         |
| DevPod     | devpod CLI, Docker         | kubectl, cloud CLIs    |
| E2B        | e2b CLI, E2B_API_KEY       | -                      |
| Kubernetes | kubectl                    | kind, k3d              |
| Packer     | Packer 1.9+, cloud CLI     | Cloud-specific plugins |

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

### For Pre-built VM Images

**Packer** enables golden image pipelines:

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

- [AWS Packer](packer/AWS.md) - EC2 AMI images
- [Azure Packer](packer/AZURE.md) - Azure Managed Images
- [GCP Packer](packer/GCP.md) - Compute Engine images
- [OCI Packer](packer/OCI.md) - Oracle Cloud images
- [Alibaba Packer](packer/ALIBABA.md) - Alibaba Cloud ECS images

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

### Packer

```bash
sindri packer doctor --cloud aws      # Check prerequisites
sindri packer build --cloud aws       # Build image
sindri packer list --cloud aws        # List images
sindri packer deploy --cloud aws <id> # Deploy from image
sindri packer delete --cloud aws <id> # Delete image
```

## Migration from V2

V3 providers maintain compatibility with V2 configurations while offering enhanced features:

| Feature            | V2           | V3                            |
| ------------------ | ------------ | ----------------------------- |
| Configuration      | sindri.yaml  | sindri.yaml (enhanced schema) |
| Template rendering | Bash scripts | Rust-based Handlebars         |
| Error handling     | Basic        | Comprehensive with recovery   |
| GPU support        | Limited      | Full (provider-dependent)     |
| Auto-suspend       | Fly.io only  | Fly.io, E2B                   |

## Related Documentation

- [Configuration Reference](../CONFIGURATION.md)
- [CLI Reference](../CLI.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [Getting Started](../GETTING_STARTED.md)
