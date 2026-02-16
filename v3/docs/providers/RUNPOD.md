# RunPod Provider

> **Version:** 3.x
> **Last Updated:** 2026-02
> **Status:** Draft (pre-implementation)

GPU cloud deployment with 40+ GPU types, spot pricing, persistent network volumes, and SSH proxy access.

## Overview

The RunPod provider deploys Sindri environments to RunPod's GPU cloud infrastructure with:

- **Extensive GPU catalog** - 40+ GPU types from RTX 3070 to H200 and B200
- **Spot pricing** - Save 60-70% with interruptible instances
- **Three-tier storage** - Container disk, pod volumes, and persistent network volumes
- **SSH access** - Proxy SSH (all pods) and full SSH via public IP
- **Per-second billing** - No minimum commitments, no ingress/egress fees
- **CPU-only pods** - Run non-GPU workloads at low cost
- **Global reach** - 31+ data centers across North America, Europe, and Asia-Pacific

**Best for:** ML/AI development, GPU-intensive workloads, cost-sensitive research, large model training

## Prerequisites

| Requirement           | Check                  | Setup                                                                     |
| --------------------- | ---------------------- | ------------------------------------------------------------------------- |
| RunPod API key        | `echo $RUNPOD_API_KEY` | Get key at [RunPod Settings](https://www.runpod.io/console/user/settings) |
| SSH client (optional) | `ssh -V`               | Install OpenSSH client                                                    |

Unlike other Sindri providers, RunPod uses a direct REST API integration -- no CLI tool installation is required. Authentication is handled entirely through the `RUNPOD_API_KEY` environment variable.

### Setting Up Authentication

```bash
# Set the API key (add to your shell profile for persistence)
export RUNPOD_API_KEY=your_api_key_here

# Verify the key works
curl -s -H "Authorization: Bearer $RUNPOD_API_KEY" \
  https://rest.runpod.io/v1/pods | head -c 100
```

### SSH Key Setup (for pod connections)

```bash
# Generate an SSH key if you don't have one
ssh-keygen -t ed25519 -C "your_email@example.com"

# Add public key to RunPod console:
# Settings > SSH Public Keys > paste contents of ~/.ssh/id_ed25519.pub
```

## Quick Start

```bash
# 1. Set API key
export RUNPOD_API_KEY=your_api_key_here

# 2. Create configuration
cat > sindri.yaml << 'EOF'
version: "3.0"
name: my-sindri-gpu

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 16GB
    cpus: 4
    gpu:
      enabled: true

extensions:
  profile: fullstack

providers:
  runpod:
    gpuType: "NVIDIA GeForce RTX 4090"
    containerDiskGb: 50
    volumeSizeGb: 20
    cloudType: SECURE
EOF

# 3. Deploy
sindri deploy

# 4. Connect
sindri connect

# 5. Stop when done (preserves pod volume)
sindri stop
```

## Configuration

### Basic GPU Configuration

```yaml
version: "3.0"
name: my-gpu-env

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 16GB
    cpus: 4
    gpu:
      enabled: true

extensions:
  profile: fullstack

providers:
  runpod:
    gpuType: "NVIDIA RTX A4000"
    containerDiskGb: 50
    volumeSizeGb: 20
```

### CPU-Only Configuration

```yaml
version: "3.0"
name: my-cpu-env

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: minimal

providers:
  runpod:
    cpuOnly: true
    cpuInstanceId: "cpu3c-2-4"
    containerDiskGb: 20
    volumeSizeGb: 0
```

### Spot Pricing Configuration

```yaml
providers:
  runpod:
    gpuType: "NVIDIA A100-SXM4-80GB"
    spotBid: 1.50
    cloudType: COMMUNITY
    containerDiskGb: 50
    volumeSizeGb: 100
```

### Network Volume (Fully Persistent Storage)

```yaml
providers:
  runpod:
    gpuType: "NVIDIA GeForce RTX 4090"
    networkVolumeId: "vol-abc123"
    volumeMountPath: /workspace
    cloudType: SECURE
```

### Advanced Configuration

```yaml
version: "3.0"
name: ml-training-env

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      tier: gpu-large

extensions:
  profile: ml

secrets:
  - name: HF_TOKEN
    source: env
    required: true
  - name: WANDB_API_KEY
    source: env

providers:
  runpod:
    gpuType: "NVIDIA A100-SXM4-80GB"
    gpuCount: 2
    containerDiskGb: 200
    volumeSizeGb: 100
    volumeMountPath: "/workspace"
    cloudType: SECURE
    region: "US-CA-2"
    spot: false
    ports:
      - "8888/http"
      - "22/tcp"
    publicIp: true
```

## Configuration Reference

| Field             | Type    | Required     | Default                   | Description                                             |
| ----------------- | ------- | ------------ | ------------------------- | ------------------------------------------------------- |
| `gpuType`         | string  | For GPU pods | -                         | GPU type identifier (e.g., `"NVIDIA GeForce RTX 4090"`) |
| `gpuCount`        | integer | No           | `1`                       | Number of GPUs (1-16)                                   |
| `cpuOnly`         | boolean | No           | `false`                   | Use CPU-only pod (no GPU)                               |
| `vcpuCount`       | integer | For CPU pods | `4`                       | Number of vCPUs for CPU pods                            |
| `containerDiskGb` | integer | No           | `50`                      | Container disk size in GB (ephemeral)                   |
| `volumeSizeGb`    | integer | No           | `20`                      | Pod volume size in GB (persists on stop)                |
| `volumeMountPath` | string  | No           | `/workspace`              | Mount path for volume                                   |
| `cloudType`       | string  | No           | `SECURE`                  | `SECURE` or `COMMUNITY`                                 |
| `region`          | string  | No           | Auto                      | Data center ID (e.g., `"US-CA-2"`, `"EU-RO-1"`)         |
| `spot`            | boolean | No           | `false`                   | Use spot/interruptible pricing                          |
| `ports`           | array   | No           | `["8888/http", "22/tcp"]` | Port mappings (`port/protocol`)                         |
| `networkVolumeId` | string  | No           | -                         | Pre-created network volume ID                           |
| `publicIp`        | boolean | No           | `false`                   | Request public IP for full SSH/SCP                      |

### GPU Tier Mapping

When using the generic `deployment.resources.gpu.tier` field, Sindri maps to RunPod GPU pool IDs:

| Sindri Tier  | RunPod Pool ID | Included GPUs | VRAM  |
| ------------ | -------------- | ------------- | ----- |
| `gpu-small`  | `ADA_24`       | RTX 4090      | 24 GB |
| `gpu-medium` | `AMPERE_48`    | A6000, A40    | 48 GB |
| `gpu-large`  | `AMPERE_80`    | A100          | 80 GB |
| `gpu-xlarge` | `ADA_80_PRO`   | H100          | 80 GB |

For precise control, use the `gpuType` field in the `providers.runpod` section instead.

### GPU Types and Pricing

Individual GPU models available on RunPod:

| GPU Type String                 | Display Name   | VRAM   | Approx. On-Demand |
| ------------------------------- | -------------- | ------ | ----------------- |
| `NVIDIA GeForce RTX 3070`       | RTX 3070       | 8 GB   | ~$0.16/hr         |
| `NVIDIA GeForce RTX 3080`       | RTX 3080       | 10 GB  | ~$0.19/hr         |
| `NVIDIA GeForce RTX 3090`       | RTX 3090       | 24 GB  | ~$0.25/hr         |
| `NVIDIA GeForce RTX 4080 SUPER` | RTX 4080 SUPER | 16 GB  | ~$0.35/hr         |
| `NVIDIA GeForce RTX 4090`       | RTX 4090       | 24 GB  | ~$0.44/hr         |
| `NVIDIA RTX A4000`              | RTX A4000      | 16 GB  | ~$0.38/hr         |
| `NVIDIA RTX A5000`              | RTX A5000      | 24 GB  | ~$0.42/hr         |
| `NVIDIA RTX A6000`              | RTX A6000      | 48 GB  | ~$0.59/hr         |
| `NVIDIA RTX 4000 Ada`           | RTX 4000 Ada   | 20 GB  | ~$0.38/hr         |
| `NVIDIA RTX 5000 Ada`           | RTX 5000 Ada   | 32 GB  | ~$0.52/hr         |
| `NVIDIA RTX 6000 Ada`           | RTX 6000 Ada   | 48 GB  | ~$0.74/hr         |
| `NVIDIA L4`                     | L4             | 24 GB  | ~$0.38/hr         |
| `NVIDIA A40`                    | A40            | 48 GB  | ~$0.69/hr         |
| `NVIDIA L40S`                   | L40S           | 48 GB  | ~$0.74/hr         |
| `NVIDIA A100 80GB PCIe`         | A100 PCIe      | 80 GB  | ~$1.64/hr         |
| `NVIDIA A100-SXM4-80GB`         | A100 SXM       | 80 GB  | ~$1.64/hr         |
| `NVIDIA H100 80GB HBM3`         | H100 SXM       | 80 GB  | ~$2.79/hr         |
| `NVIDIA H100 PCIe`              | H100 PCIe      | 80 GB  | ~$2.49/hr         |
| `NVIDIA H200`                   | H200 SXM       | 141 GB | ~$3.99/hr         |
| `NVIDIA RTX 5090`               | RTX 5090       | 32 GB  | ~$0.69/hr         |
| `NVIDIA B200`                   | B200           | 180 GB | ~$5.00+/hr        |

_Prices are approximate and vary by data center and availability. Check the [RunPod console](https://www.runpod.io/console/gpu-cloud) for current pricing._

### GPU Pool IDs

For flexible GPU selection, use pool IDs instead of specific GPU type strings. RunPod will assign any available GPU within the pool:

| Pool ID      | Included Models                  | VRAM   |
| ------------ | -------------------------------- | ------ |
| `AMPERE_16`  | A4000, A4500, RTX 4000, RTX 2000 | 16 GB  |
| `AMPERE_24`  | L4, A5000, RTX 3090              | 24 GB  |
| `ADA_24`     | RTX 4090                         | 24 GB  |
| `AMPERE_48`  | A6000, A40                       | 48 GB  |
| `ADA_48_PRO` | L40, L40S, RTX 6000 Ada          | 48 GB  |
| `AMPERE_80`  | A100                             | 80 GB  |
| `ADA_80_PRO` | H100                             | 80 GB  |
| `HOPPER_141` | H200                             | 141 GB |

```yaml
# Use pool ID for better availability
providers:
  runpod:
    gpuType: "ADA_24" # Any 24GB Ada GPU (e.g., RTX 4090)
```

### CPU Instance Types

| Flavor       | Description                          | Approx. Cost |
| ------------ | ------------------------------------ | ------------ |
| `cpu3c-2-4`  | 2 vCPU, 4 GB RAM                     | ~$0.05/hr    |
| `cpu3c-4-8`  | 4 vCPU, 8 GB RAM                     | ~$0.10/hr    |
| `cpu3c-8-16` | 8 vCPU, 16 GB RAM                    | ~$0.20/hr    |
| `cpu5c-2-4`  | 2 vCPU, 4 GB RAM (compute-optimized) | ~$0.06/hr    |
| `cpu5c-4-8`  | 4 vCPU, 8 GB RAM (compute-optimized) | ~$0.12/hr    |

## Deployment Commands

```bash
# Deploy (creates pod)
sindri deploy

# Preview deployment plan
sindri plan

# Check status
sindri status

# Connect to pod (SSH)
sindri connect

# Stop pod (retains pod volume data)
sindri stop

# Start stopped pod (resumes)
sindri start

# Destroy pod (permanent, loses pod volume)
sindri destroy
```

## Storage

### Three-Tier Storage Model

RunPod provides three storage tiers with different persistence characteristics:

```
+-------------------+     +-------------------+     +-------------------+
| Container Disk    |     | Pod Volume        |     | Network Volume    |
| (Ephemeral)       |     | (Stop-persistent) |     | (Fully persistent)|
|                   |     |                   |     |                   |
| - Default: 50 GB  |     | - Default: 20 GB  |     | - User-managed    |
| - Lost on stop    |     | - Mount: /workspace|     | - Mount: /workspace|
| - $0.10/GB/month  |     | - Survives stop   |     | - Survives all    |
| - Scratch data    |     | - Lost on terminate|    | - $0.07/GB/month  |
+-------------------+     +-------------------+     +-------------------+
```

### Storage Comparison

| Storage Type           | Cost             | Persists on Stop | Persists on Terminate | Use Case                |
| ---------------------- | ---------------- | ---------------- | --------------------- | ----------------------- |
| Container Disk         | $0.10/GB/mo      | No               | No                    | Temporary/scratch data  |
| Pod Volume             | $0.10-0.20/GB/mo | Yes              | No                    | Session data, code      |
| Network Volume (<1 TB) | $0.07/GB/mo      | Yes              | Yes                   | Model weights, datasets |
| Network Volume (>1 TB) | $0.05/GB/mo      | Yes              | Yes                   | Large datasets          |

### Lifecycle Behavior

| Event            | Container Disk | Pod Volume    | Network Volume |
| ---------------- | -------------- | ------------- | -------------- |
| Pod running      | Available      | Available     | Available      |
| `sindri stop`    | **Lost**       | **Preserved** | **Preserved**  |
| `sindri start`   | Fresh          | Reattached    | Reattached     |
| `sindri destroy` | **Lost**       | **Lost**      | **Preserved**  |

### Using Network Volumes

Network volumes are fully persistent and survive pod termination. They must be created before use:

```bash
# Create a network volume via the RunPod API
curl -X POST https://rest.runpod.io/v1/networkvolumes \
  -H "Authorization: Bearer $RUNPOD_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-sindri-volume",
    "size": 100,
    "dataCenterId": "US-CA-2"
  }'
```

Then reference it in your `sindri.yaml`:

```yaml
providers:
  runpod:
    gpuType: "NVIDIA GeForce RTX 4090"
    networkVolumeId: "vol-abc123"
    volumeMountPath: /workspace
```

**Important constraints:**

- Network volumes are only available on Secure Cloud (not Community Cloud)
- The pod and network volume must be in the same data center
- Concurrent writes from multiple pods may cause data corruption
- Network volumes can be enlarged but never reduced

## Connection Methods

### Method 1: sindri connect (Recommended)

```bash
sindri connect
```

This automatically selects the best available connection method for your pod.

### Method 2: SSH via Proxy (All Pods)

Available on all pods without public IP:

```bash
ssh root@ssh.runpod.io -i ~/.ssh/id_ed25519
```

**Limitations:**

- Terminal access only (no SCP or SFTP)
- Requires SSH key to be registered in RunPod console

### Method 3: SSH via Public IP (Full SSH)

Available on pods created with `publicIp: true`:

```bash
ssh root@<PUBLIC_IP> -p <PORT> -i ~/.ssh/id_ed25519
```

**Benefits:**

- Full SSH capabilities including SCP and SFTP
- Direct connection without proxy

**SSH config entry:**

```
Host sindri-runpod
    HostName <PUBLIC_IP>
    Port <PORT>
    User root
    IdentityFile ~/.ssh/id_ed25519
    StrictHostKeyChecking no
```

### Method 4: Web Terminal

Access the RunPod web terminal at:

```
https://www.runpod.io/console/pods/<POD_ID>/terminal
```

### Method 5: Proxy URLs (HTTP Services)

Any HTTP port exposed in the configuration is accessible via:

```
https://<POD_ID>-<PORT>.proxy.runpod.net
```

For example, a Jupyter notebook on port 8888:

```
https://abc123-8888.proxy.runpod.net
```

## Secrets Management

Secrets are injected as environment variables at pod creation time via the RunPod API. Configure them in your `sindri.yaml`:

```yaml
secrets:
  - name: HF_TOKEN
    source: env
    required: true
  - name: WANDB_API_KEY
    source: env
  - name: GITHUB_TOKEN
    source: env
```

**Important:** Secrets are set at pod creation. Updating secrets requires terminating and recreating the pod (unlike Fly.io which supports live secret updates). Plan your secrets before deployment.

## Advanced Features

### Spot Pricing (Cost Optimization)

Spot instances use spare GPU capacity at significant discounts (up to 60-70% off) but may be interrupted when demand increases:

```yaml
providers:
  runpod:
    gpuType: "NVIDIA GeForce RTX 4090"
    spot: true
    cloudType: COMMUNITY
```

**When to use spot:**

- Development and experimentation
- Fault-tolerant training with checkpoints
- Batch processing that can be restarted
- Cost-sensitive research

**When NOT to use spot:**

- Long-running training without checkpointing
- Production inference
- Interactive development requiring uptime guarantees

### Multi-GPU Pods

Request multiple GPUs for distributed training:

```yaml
providers:
  runpod:
    gpuType: "NVIDIA A100-SXM4-80GB"
    gpuCount: 4
    containerDiskGb: 200
    volumeSizeGb: 500
```

### Port Exposure

Expose HTTP ports via RunPod proxy URLs:

```yaml
providers:
  runpod:
    ports:
      - "8888/http" # Jupyter: https://<podId>-8888.proxy.runpod.net
      - "3000/http" # App: https://<podId>-3000.proxy.runpod.net
      - "22/tcp" # SSH
```

**Proxy URL format:** `https://<podId>-<port>.proxy.runpod.net`

**Note:** Proxy URLs have a 100-second connection timeout (Cloudflare). For long-running connections, use SSH or a public IP.

### File Transfer

**With public IP (SCP/SFTP):**

```bash
# Upload
scp -P <PORT> local_file.tar.gz root@<PUBLIC_IP>:/workspace/

# Download
scp -P <PORT> root@<PUBLIC_IP>:/workspace/results.tar.gz ./
```

**Without public IP (runpodctl send/receive):**

```bash
# On local machine
pip install runpod  # or use runpodctl CLI

# On pod (runpodctl is pre-installed)
runpodctl send /workspace/model.bin
# Output: Code is: 1234-word-word-word

# On local machine
runpodctl receive 1234-word-word-word
```

**Via network volumes:** Mount the same network volume on different pods to share data without transfer.

## Regions

### Data Centers

RunPod operates 31+ data centers globally:

**North America:**

| Region ID        | Location   |
| ---------------- | ---------- |
| US-TX-3, US-TX-4 | Texas      |
| US-GA-1, US-GA-2 | Georgia    |
| US-IL-1          | Illinois   |
| US-WA-1          | Washington |
| US-CA-2          | California |
| US-DE-1          | Delaware   |
| US-KS-2          | Kansas     |

**Europe:**

| Region ID          | Location       |
| ------------------ | -------------- |
| EU-RO-1            | Romania        |
| EU-CZ-1            | Czech Republic |
| EU-FR-1            | France         |
| EU-NL-1            | Netherlands    |
| EU-SE-1            | Sweden         |
| EUR-IS-1, EUR-IS-2 | Iceland        |

**Asia-Pacific:**

| Region ID | Location |
| --------- | -------- |
| AP-JP-1   | Japan    |

### Cloud Types

| Type        | Description                 | Network Volumes | Pricing    |
| ----------- | --------------------------- | --------------- | ---------- |
| `SECURE`    | RunPod-managed data centers | Supported       | Standard   |
| `COMMUNITY` | Community-hosted GPUs       | Not supported   | Lower cost |

## Troubleshooting

### API Key Issues

**Symptom:** `RunPod API authentication failed (401 Unauthorized)`

**Solution:**

```bash
# Verify the key is set
echo $RUNPOD_API_KEY

# Set the key
export RUNPOD_API_KEY=your_api_key_here

# Test the key
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $RUNPOD_API_KEY" \
  https://rest.runpod.io/v1/pods
# Should return 200
```

### GPU Not Available

**Symptom:** `GPU type not available` or pod creation fails

**Solution:**

1. Use a GPU pool ID for flexible selection:
   ```yaml
   gpuType: "ADA_24" # Any 24GB Ada GPU
   ```
2. Remove the region constraint to allow any data center
3. Try Community Cloud for wider availability:
   ```yaml
   cloudType: COMMUNITY
   ```
4. Use spot pricing for better availability:
   ```yaml
   spot: true
   ```
5. Try a different GPU type with similar specs

### Pod Creation Fails

**Symptom:** 400 Bad Request from RunPod API

**Checklist:**

- Verify the GPU type string matches exactly (case-sensitive)
- Check account balance has sufficient credits
- Ensure container image exists and is accessible
- Verify `containerDiskGb` is large enough for your image
- Try a different `cloudType` (SECURE vs COMMUNITY)

### Connection Issues

**Symptom:** Cannot SSH to pod

**Solution:**

1. Verify pod is in RUNNING state:
   ```bash
   sindri status
   ```
2. Check SSH key is registered in RunPod console
3. For proxy SSH, use `root@ssh.runpod.io`
4. For public IP SSH, verify the mapped port number
5. Try the web terminal as a fallback:
   ```
   https://www.runpod.io/console/pods/<POD_ID>/terminal
   ```

### Volume Issues

**Symptom:** Data lost after stop or terminate

**Understanding:**

- **Container disk** is ephemeral -- lost on stop
- **Pod volume** persists on stop, lost on terminate
- **Network volume** is fully persistent

**For critical data**, use a network volume:

```yaml
providers:
  runpod:
    networkVolumeId: "vol-abc123"
```

### Rate Limiting

**Symptom:** `RunPod API rate limit exceeded (429)`

**Solution:** Wait a moment and retry. The Sindri adapter will automatically surface this error. If persistent, reduce the frequency of status checks.

## Cost Optimization

### Pricing Strategies

1. **Use Spot Instances** -- Save 60-70% for interruptible workloads
2. **Stop vs Terminate** -- Stop retains pod volume data, only pays for storage ($0.10-0.20/GB/mo). Terminate deletes everything.
3. **Right-size GPUs** -- Choose the smallest GPU that meets your VRAM requirements
4. **Community Cloud** -- 10-30% cheaper than Secure Cloud for development workloads
5. **GPU Pool IDs** -- More availability means less waiting and potentially better pricing
6. **Network Volumes** -- Store model weights once, attach to any pod. Avoid re-downloading

### Cost Estimates

| Configuration      | Approx. Hourly | Monthly (8h/day, 22 days) |
| ------------------ | -------------- | ------------------------- |
| CPU (2 vCPU, 4 GB) | $0.05          | $8.80                     |
| RTX 4090 (24 GB)   | $0.44          | $77.44                    |
| A100 80 GB         | $1.64          | $288.64                   |
| H100 SXM 80 GB     | $2.79          | $491.04                   |
| 4x A100 80 GB      | $6.56          | $1,154.56                 |

_Storage costs are additional. Stopped pods incur volume storage charges._

## Example Scenarios

### ML Development

A cost-effective setup for everyday ML experimentation:

```yaml
version: "3.0"
name: ml-dev

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    gpu:
      enabled: true

extensions:
  profile: full

providers:
  runpod:
    gpuType: "NVIDIA RTX A4000"
    containerDiskGb: 50
    volumeSizeGb: 50
    cloudType: COMMUNITY
```

### Large Model Training

Multi-GPU setup with persistent storage for training runs:

```yaml
version: "3.0"
name: training-cluster

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 64GB
    cpus: 16
    gpu:
      enabled: true
      tier: gpu-large

extensions:
  profile: ml

secrets:
  - name: WANDB_API_KEY
    source: env

providers:
  runpod:
    gpuType: "NVIDIA A100-SXM4-80GB"
    gpuCount: 2
    containerDiskGb: 200
    volumeSizeGb: 500
    networkVolumeId: "vol-training-data"
    cloudType: SECURE
    region: "US-CA-2"
```

### Budget Research

Spot instances for cost-sensitive academic or personal research:

```yaml
version: "3.0"
name: research-env

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    gpu:
      enabled: true

extensions:
  profile: fullstack

providers:
  runpod:
    gpuType: "NVIDIA GeForce RTX 3090"
    spot: true
    cloudType: COMMUNITY
    containerDiskGb: 30
    volumeSizeGb: 20
```

### CPU-Only Data Processing

No GPU needed for preprocessing, data pipelines, or API development:

```yaml
version: "3.0"
name: data-pipeline

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 16GB
    cpus: 8

extensions:
  profile: base

providers:
  runpod:
    cpuOnly: true
    vcpuCount: 8
    containerDiskGb: 50
    volumeSizeGb: 100
```

### Inference Server with Port Exposure

Serve a model with an HTTP endpoint:

```yaml
version: "3.0"
name: inference-server

deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    gpu:
      enabled: true

providers:
  runpod:
    gpuType: "NVIDIA L4"
    containerDiskGb: 50
    volumeSizeGb: 50
    ports:
      - "8080/http"
      - "8888/http"
      - "22/tcp"
    publicIp: true
```

Access the inference API at `https://<podId>-8080.proxy.runpod.net`.

## Comparison to Other Providers

| Feature             | Docker         | Fly.io        | E2B            | RunPod                           |
| ------------------- | -------------- | ------------- | -------------- | -------------------------------- |
| GPU support         | Runtime check  | A100, L40s    | No             | **40+ GPU types**                |
| GPU selection       | Host GPU       | 4 tiers       | N/A            | **Specific model or pool**       |
| Spot pricing        | N/A            | No            | No             | **Yes (60-70% savings)**         |
| Auto-suspend        | N/A            | Yes           | Yes (pause)    | Yes (stop/start)                 |
| Storage             | Docker volumes | Fly volumes   | No             | **3-tier (disk/volume/network)** |
| SSH access          | docker exec    | flyctl ssh    | No (WebSocket) | **Proxy + public IP**            |
| External CLI        | docker         | flyctl        | e2b            | **None required**                |
| Ingress/egress fees | Free (local)   | Free (100 GB) | N/A            | **Free**                         |
| Billing model       | Free (local)   | Per-second    | Per-second     | **Per-second**                   |
| Best for            | Local dev      | Remote dev    | AI sandboxes   | **GPU workloads**                |

## Architecture Notes

### API Integration

The RunPod provider uses direct HTTP REST API calls (`reqwest`) instead of shelling out to a CLI tool. This means:

- No external CLI dependency to install
- Structured error handling from API responses
- No subprocess spawn overhead
- Easier to test with HTTP mocking

### Sindri Lifecycle Mapping

| Sindri Command   | RunPod API Call            | Description                        |
| ---------------- | -------------------------- | ---------------------------------- |
| `sindri deploy`  | `POST /v1/pods`            | Create and start a new pod         |
| `sindri status`  | `GET /v1/pods/{id}`        | Query pod state                    |
| `sindri connect` | N/A (opens SSH)            | Connect via SSH proxy or public IP |
| `sindri stop`    | `POST /v1/pods/{id}/stop`  | Stop pod (preserve volume)         |
| `sindri start`   | `POST /v1/pods/{id}/start` | Resume a stopped pod               |
| `sindri destroy` | `DELETE /v1/pods/{id}`     | Terminate pod permanently          |

### Pod State Mapping

| RunPod Status | Sindri State | Description                    |
| ------------- | ------------ | ------------------------------ |
| `CREATED`     | Creating     | Pod created, not yet running   |
| `STARTING`    | Creating     | Pod is booting up              |
| `RUNNING`     | Running      | Pod is ready for use           |
| `EXITED`      | Stopped      | Pod stopped (volume preserved) |
| `STOPPED`     | Stopped      | Pod explicitly stopped         |
| `TERMINATED`  | NotDeployed  | Pod permanently removed        |
| `ERROR`       | Error        | Pod in error state             |

## Related Documentation

- [Provider Overview](README.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [CLI Reference](../CLI.md)
- [RunPod Official Docs](https://docs.runpod.io/)
- [RunPod REST API Reference](https://docs.runpod.io/api-reference/overview)
- [RunPod GPU Types](https://docs.runpod.io/references/gpu-types)
