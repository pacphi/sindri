# RunPod Platform Research Findings

> Research date: 2026-02-16
> Sources: RunPod official documentation, GitHub repositories, blog posts, community resources

---

## Table of Contents

1. [Platform Overview](#1-platform-overview)
2. [Authentication](#2-authentication)
3. [REST API (v1)](#3-rest-api-v1)
4. [GraphQL API](#4-graphql-api)
5. [CLI Tool (runpodctl)](#5-cli-tool-runpodctl)
6. [Python SDK](#6-python-sdk)
7. [GPU Types and IDs](#7-gpu-types-and-ids)
8. [CPU Pods](#8-cpu-pods)
9. [Networking and SSH](#9-networking-and-ssh)
10. [Storage](#10-storage)
11. [Pricing](#11-pricing)
12. [Data Centers](#12-data-centers)
13. [Serverless Endpoints](#13-serverless-endpoints)
14. [Best Practices](#14-best-practices)
15. [Integration Considerations for Sindri](#15-integration-considerations-for-sindri)

---

## 1. Platform Overview

RunPod is a GPU cloud platform offering:

- **GPU Pods**: Persistent GPU/CPU instances for development, training, and inference
- **Serverless Endpoints**: Auto-scaling inference endpoints with pay-per-request pricing
- **Network Volumes**: Persistent NVMe SSD storage independent of compute lifecycle
- **Global Networking**: Pod-to-pod communication across 31+ data centers worldwide

RunPod provides three API interfaces:

1. **REST API** (newer, recommended) at `https://rest.runpod.io/v1`
2. **GraphQL API** (legacy, still supported) at `https://api.runpod.io/graphql`
3. **Python SDK** (`pip install runpod`)

---

## 2. Authentication

### API Key Management

- API keys are generated in the RunPod console under **Settings**
- Keys have configurable permissions for different resource types
- Default spend limit: **$80/hour** across all resources (adjustable upon request)

### Authentication Methods

**REST API:**

```
Authorization: Bearer <RUNPOD_API_KEY>
```

**GraphQL API:**

```
https://api.runpod.io/graphql?api_key=<RUNPOD_API_KEY>
```

Or via header:

```
Authorization: Bearer <RUNPOD_API_KEY>
```

**CLI:**

```bash
runpodctl config --apiKey=<RUNPOD_API_KEY>
```

**Python SDK:**

```python
import runpod
runpod.api_key = "<RUNPOD_API_KEY>"
```

---

## 3. REST API (v1)

### Base URL

```
https://rest.runpod.io/v1
```

### OpenAPI Spec

```
https://rest.runpod.io/v1/openapi.json
```

### Endpoint Categories

| Category                | Description                                    |
| ----------------------- | ---------------------------------------------- |
| Pods                    | Create and manage persistent GPU/CPU instances |
| Serverless Endpoints    | Deploy containerized apps with autoscaling     |
| Network Volumes         | Persistent storage management                  |
| Templates               | Pre-configured Docker image + hardware bundles |
| Container Registry Auth | Private Docker registry connections            |
| Billing and Usage       | Resource metrics and spending data             |

### Pod Endpoints

| Method   | Path                       | Description        |
| -------- | -------------------------- | ------------------ |
| `POST`   | `/v1/pods`                 | Create a new pod   |
| `GET`    | `/v1/pods`                 | List all pods      |
| `GET`    | `/v1/pods/{podId}`         | Get pod details    |
| `POST`   | `/v1/pods/{podId}/start`   | Start/resume a pod |
| `POST`   | `/v1/pods/{podId}/stop`    | Stop a pod         |
| `POST`   | `/v1/pods/{podId}/restart` | Restart a pod      |
| `DELETE` | `/v1/pods/{podId}`         | Terminate a pod    |

### Create Pod Request Body (POST /v1/pods)

#### Core Configuration

| Parameter       | Type    | Default  | Required | Description                       |
| --------------- | ------- | -------- | -------- | --------------------------------- |
| `name`          | string  | "my pod" | No       | Pod name (max 191 chars)          |
| `imageName`     | string  | -        | Yes      | Container image tag               |
| `computeType`   | string  | "GPU"    | No       | `GPU` or `CPU`                    |
| `cloudType`     | string  | "SECURE" | No       | `SECURE` or `COMMUNITY`           |
| `interruptible` | boolean | false    | No       | Enable spot/interruptible pricing |
| `locked`        | boolean | false    | No       | Prevent stopping or resetting     |

#### GPU Configuration (computeType = "GPU")

| Parameter             | Type          | Default        | Description                          |
| --------------------- | ------------- | -------------- | ------------------------------------ |
| `gpuCount`            | integer       | 1              | Number of GPUs                       |
| `gpuTypeIds`          | array[string] | -              | GPU type identifiers (priority list) |
| `gpuTypePriority`     | string        | "availability" | Selection strategy                   |
| `allowedCudaVersions` | array[string] | -              | CUDA version filter                  |
| `minVCPUPerGPU`       | integer       | 2              | Minimum vCPUs per GPU                |
| `minRAMPerGPU`        | integer       | 8              | Minimum RAM (GB) per GPU             |

#### CPU Configuration (computeType = "CPU")

| Parameter           | Type          | Default        | Description            |
| ------------------- | ------------- | -------------- | ---------------------- |
| `vcpuCount`         | integer       | 2              | Number of vCPUs        |
| `cpuFlavorIds`      | array[string] | -              | CPU flavor identifiers |
| `cpuFlavorPriority` | string        | "availability" | Selection strategy     |

#### Storage Configuration

| Parameter           | Type    | Default      | Description                 |
| ------------------- | ------- | ------------ | --------------------------- |
| `containerDiskInGb` | integer | 50           | Ephemeral container storage |
| `volumeInGb`        | integer | 20           | Persistent pod volume       |
| `volumeMountPath`   | string  | "/workspace" | Volume mount point          |
| `networkVolumeId`   | string  | -            | External network volume ID  |

#### Network & Location

| Parameter            | Type          | Default                | Description                |
| -------------------- | ------------- | ---------------------- | -------------------------- |
| `ports`              | array[string] | ["8888/http","22/tcp"] | Port mappings              |
| `dataCenterIds`      | array[string] | all                    | Preferred data centers     |
| `dataCenterPriority` | string        | "availability"         | DC selection strategy      |
| `countryCodes`       | array[string] | -                      | Country filter             |
| `supportPublicIp`    | boolean       | -                      | Require public IP          |
| `globalNetworking`   | boolean       | false                  | Enable cross-DC networking |

#### Docker Configuration

| Parameter          | Type          | Default | Description                       |
| ------------------ | ------------- | ------- | --------------------------------- |
| `dockerEntrypoint` | array[string] | []      | Container entrypoint              |
| `dockerStartCmd`   | array[string] | []      | Container start command           |
| `env`              | object        | {}      | Environment variables (key-value) |

#### Performance Requirements

| Parameter              | Type   | Description            |
| ---------------------- | ------ | ---------------------- |
| `minDiskBandwidthMBps` | number | Minimum disk bandwidth |
| `minDownloadMbps`      | number | Minimum download speed |
| `minUploadMbps`        | number | Minimum upload speed   |

#### Template & Registry

| Parameter                 | Type   | Description                |
| ------------------------- | ------ | -------------------------- |
| `templateId`              | string | Pre-configured template ID |
| `containerRegistryAuthId` | string | Private registry auth ID   |

### Create Pod Response (201)

Returns a Pod object:

```json
{
  "id": "pod-abc123",
  "name": "my-pod",
  "image": "runpod/pytorch:latest",
  "status": "RUNNING",
  "publicIp": "1.2.3.4",
  "portMappings": [...],
  "costPerHr": 0.44,
  "adjustedCostPerHr": 0.44,
  "gpu": { "type": "NVIDIA GeForce RTX 4090", "count": 1 },
  "machine": { ... },
  "volumeInGb": 20,
  "containerDiskInGb": 50
}
```

### Example: Create Pod (curl)

```bash
curl -X POST https://rest.runpod.io/v1/pods \
  -H "Authorization: Bearer $RUNPOD_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-gpu-pod",
    "imageName": "runpod/pytorch:2.1.0-py3.10-cuda11.8.0-devel-ubuntu22.04",
    "gpuTypeIds": ["NVIDIA GeForce RTX 4090"],
    "gpuCount": 1,
    "containerDiskInGb": 50,
    "volumeInGb": 20,
    "volumeMountPath": "/workspace",
    "ports": ["8888/http", "22/tcp"],
    "env": {
      "JUPYTER_PASSWORD": "mypassword"
    }
  }'
```

### Example: List Pods (curl)

```bash
curl https://rest.runpod.io/v1/pods \
  -H "Authorization: Bearer $RUNPOD_API_KEY"
```

### Example: Stop Pod (curl)

```bash
curl -X POST https://rest.runpod.io/v1/pods/{podId}/stop \
  -H "Authorization: Bearer $RUNPOD_API_KEY"
```

### Example: Terminate Pod (curl)

```bash
curl -X DELETE https://rest.runpod.io/v1/pods/{podId} \
  -H "Authorization: Bearer $RUNPOD_API_KEY"
```

---

## 4. GraphQL API

### Endpoint

```
https://api.runpod.io/graphql?api_key=<RUNPOD_API_KEY>
```

### Spec

```
https://graphql-spec.runpod.io/
```

### Key Mutations

#### podFindAndDeployOnDemand

Creates an on-demand (non-interruptible) pod.

```graphql
mutation {
  podFindAndDeployOnDemand(
    input: {
      cloudType: ALL
      gpuCount: 1
      volumeInGb: 20
      containerDiskInGb: 50
      minVcpuCount: 2
      minMemoryInGb: 8
      gpuTypeId: "NVIDIA GeForce RTX 4090"
      name: "my-pod"
      imageName: "runpod/pytorch:latest"
      ports: "8888/http,22/tcp"
      volumeMountPath: "/workspace"
      env: [{ key: "JUPYTER_PASSWORD", value: "mypassword" }]
    }
  ) {
    id
    imageName
    env
    machineId
    machine {
      podHostId
    }
  }
}
```

#### podRentInterruptable

Creates a spot (interruptible) pod with bid pricing.

```graphql
mutation {
  podRentInterruptable(
    input: {
      bidPerGpu: 0.2
      cloudType: ALL
      gpuCount: 1
      volumeInGb: 20
      containerDiskInGb: 50
      gpuTypeId: "NVIDIA GeForce RTX 4090"
      name: "my-spot-pod"
      imageName: "runpod/pytorch:latest"
      ports: "8888/http,22/tcp"
      volumeMountPath: "/workspace"
    }
  ) {
    id
    imageName
    machineId
  }
}
```

#### podResume

Starts a stopped on-demand pod.

```graphql
mutation {
  podResume(input: { podId: "pod-abc123", gpuCount: 1 }) {
    id
    desiredStatus
    imageName
  }
}
```

#### podBidResume

Starts a stopped spot pod with new bid price.

```graphql
mutation {
  podBidResume(input: { podId: "pod-abc123", bidPerGpu: 0.2, gpuCount: 1 }) {
    id
    desiredStatus
  }
}
```

#### podStop

Stops a running pod (preserves volume data).

```graphql
mutation {
  podStop(input: { podId: "pod-abc123" }) {
    id
    desiredStatus
  }
}
```

#### podTerminate

Terminates a pod (deletes all data including volumes).

```graphql
mutation {
  podTerminate(input: { podId: "pod-abc123" })
}
```

### Key Queries

#### List All Pods

```graphql
query {
  myself {
    pods {
      id
      name
      desiredStatus
      costPerHr
      gpuCount
      uptimeSeconds
      runtime {
        gpus {
          gpuUtilPercent
          memoryUtilPercent
        }
      }
    }
  }
}
```

#### Get Single Pod

```graphql
query {
  pod(input: { podId: "pod-abc123" }) {
    id
    name
    desiredStatus
    costPerHr
    imageName
    runtime {
      uptimeInSeconds
      ports {
        ip
        privatePort
        publicPort
        type
      }
      gpus {
        id
        gpuUtilPercent
        memoryUtilPercent
      }
    }
  }
}
```

#### Query GPU Types

```graphql
query {
  gpuTypes {
    id
    displayName
    memoryInGb
  }
}
```

#### Query GPU Type with Pricing

```graphql
query {
  gpuTypes(input: { id: "NVIDIA GeForce RTX 4090" }) {
    id
    displayName
    memoryInGb
    secureCloud
    communityCloud
    lowestPrice(input: { gpuCount: 1 }) {
      minimumBidPrice
      uninterruptablePrice
      minVcpu
      minMemory
    }
  }
}
```

#### Check GPU Availability

```graphql
query {
  gpuTypes {
    id
    displayName
    memoryInGb
    secureCloud
    communityCloud
    lowestPrice(input: { gpuCount: 1 }) {
      minimumBidPrice
      uninterruptablePrice
      stockStatus
    }
  }
}
```

**stockStatus values**: `High`, `Medium`, `Low`, `Out of Stock`

---

## 5. CLI Tool (runpodctl)

### Installation

**Linux/macOS (WSL):**

```bash
wget -qO- cli.runpod.net | sudo bash
```

**macOS (Homebrew):**

```bash
brew install runpod/runpodctl/runpodctl
```

**Windows (PowerShell):**

```powershell
wget https://github.com/runpod/runpodctl/releases/latest/download/runpodctl-windows-amd64.exe -O runpodctl.exe
```

**Note:** All pods come with runpodctl pre-installed with a pod-scoped API key.

### Version

Latest documented version: **v1.14.3**

### Configuration

```bash
runpodctl config --apiKey=<RUNPOD_API_KEY>
```

### Commands

| Command                                 | Description                         |
| --------------------------------------- | ----------------------------------- |
| `runpodctl get pod`                     | List all pods                       |
| `runpodctl get pod {podId}`             | Get specific pod details            |
| `runpodctl create pod`                  | Create a new pod                    |
| `runpodctl start pod {podId}`           | Start an on-demand pod              |
| `runpodctl start pod {podId} --bid=0.3` | Start a spot pod with bid price     |
| `runpodctl stop pod {podId}`            | Stop a pod                          |
| `runpodctl remove pod {podId}`          | Terminate/remove a pod              |
| `runpodctl exec {podId} -- {command}`   | Execute command in a pod            |
| `runpodctl ssh {podId}`                 | SSH into a pod                      |
| `runpodctl send {file}`                 | Send file (generates one-time code) |
| `runpodctl receive {code}`              | Receive file using one-time code    |
| `runpodctl project`                     | Manage RunPod projects              |
| `runpodctl update`                      | Update runpodctl to latest version  |
| `runpodctl version`                     | Show version                        |
| `runpodctl completion`                  | Generate shell completion           |

### File Transfer

The `send` and `receive` commands use one-time codes for security and do not require API keys:

```bash
# On sender machine
runpodctl send data.tar.gz
# Output: Sending 'data.tar.gz' (5.2 GB)
# Code is: 8338-galileo-collect-fidel

# On receiver machine
runpodctl receive 8338-galileo-collect-fidel
```

---

## 6. Python SDK

### Installation

```bash
pip install runpod
```

Requires Python >= 3.8. Latest version released November 19, 2025.

### Pod Management

```python
import runpod
runpod.api_key = "your_api_key"

# List all pods
pods = runpod.get_pods()

# Get specific pod
pod = runpod.get_pod("pod-abc123")

# Create GPU pod
pod = runpod.create_pod(
    name="my-pod",
    image_name="runpod/pytorch:latest",
    gpu_type_id="NVIDIA GeForce RTX 4090",
    gpu_count=1,
    volume_in_gb=20,
    container_disk_in_gb=50,
    ports="8888/http,22/tcp",
    volume_mount_path="/workspace",
    env={"JUPYTER_PASSWORD": "mypassword"}
)

# Create CPU pod
pod = runpod.create_pod(
    name="my-cpu-pod",
    image_name="runpod/stack",
    instance_id="cpu3c-2-4"
)

# Stop pod
runpod.stop_pod("pod-abc123")

# Resume pod
runpod.resume_pod("pod-abc123")

# Terminate pod
runpod.terminate_pod("pod-abc123")
```

### Serverless Worker

```python
import runpod

def handler(job):
    input_data = job["input"]
    # Process the job
    return {"output": "result"}

runpod.serverless.start({"handler": handler})
```

---

## 7. GPU Types and IDs

### Individual GPU Models

| GPU ID (Full Name)                        | Display Name             | VRAM (GB) | Architecture |
| ----------------------------------------- | ------------------------ | --------- | ------------ |
| AMD Instinct MI300X OAM                   | MI300X                   | 192       | CDNA 3       |
| NVIDIA B200                               | B200                     | 180       | Blackwell    |
| NVIDIA H200                               | H200 SXM                 | 141       | Hopper       |
| NVIDIA RTX PRO 6000 Blackwell Server      | RTX PRO 6000 Server      | 96        | Blackwell    |
| NVIDIA RTX PRO 6000 Blackwell Workstation | RTX PRO 6000 Workstation | 96        | Blackwell    |
| NVIDIA H100 NVL                           | H100 NVL                 | 94        | Hopper       |
| NVIDIA A100 80GB PCIe                     | A100 PCIe                | 80        | Ampere       |
| NVIDIA A100-SXM4-80GB                     | A100 SXM                 | 80        | Ampere       |
| NVIDIA H100 80GB HBM3                     | H100 SXM                 | 80        | Hopper       |
| NVIDIA H100 PCIe                          | H100 PCIe                | 80        | Hopper       |
| NVIDIA A40                                | A40                      | 48        | Ampere       |
| NVIDIA L40                                | L40                      | 48        | Ada Lovelace |
| NVIDIA L40S                               | L40S                     | 48        | Ada Lovelace |
| NVIDIA RTX 6000 Ada                       | RTX 6000 Ada             | 48        | Ada Lovelace |
| NVIDIA RTX A6000                          | RTX A6000                | 48        | Ampere       |
| NVIDIA RTX 5090                           | RTX 5090                 | 32        | Blackwell    |
| NVIDIA RTX 5000 Ada                       | RTX 5000 Ada             | 32        | Ada Lovelace |
| Tesla V100-SXM2-32GB                      | V100 SXM2 32GB           | 32        | Volta        |
| NVIDIA A30                                | A30                      | 24        | Ampere       |
| NVIDIA GeForce RTX 3090                   | RTX 3090                 | 24        | Ampere       |
| NVIDIA GeForce RTX 3090 Ti                | RTX 3090 Ti              | 24        | Ampere       |
| NVIDIA GeForce RTX 4090                   | RTX 4090                 | 24        | Ada Lovelace |
| NVIDIA L4                                 | L4                       | 24        | Ada Lovelace |
| NVIDIA RTX A5000                          | RTX A5000                | 24        | Ampere       |
| NVIDIA RTX 4000 Ada                       | RTX 4000 Ada             | 20        | Ada Lovelace |
| NVIDIA RTX 4000 Ada SFF                   | RTX 4000 Ada SFF         | 20        | Ada Lovelace |
| NVIDIA RTX A4500                          | RTX A4500                | 20        | Ampere       |
| NVIDIA GeForce RTX 4080                   | RTX 4080                 | 16        | Ada Lovelace |
| NVIDIA GeForce RTX 4080 SUPER             | RTX 4080 SUPER           | 16        | Ada Lovelace |
| NVIDIA GeForce RTX 5080                   | RTX 5080                 | 16        | Blackwell    |
| NVIDIA RTX 2000 Ada                       | RTX 2000 Ada             | 16        | Ada Lovelace |
| NVIDIA RTX A4000                          | RTX A4000                | 16        | Ampere       |
| Tesla V100-FHHL-16GB                      | V100 FHHL                | 16        | Volta        |
| Tesla V100-PCIE-16GB                      | Tesla V100               | 16        | Volta        |
| Tesla V100-SXM2-16GB                      | V100 SXM2                | 16        | Volta        |
| NVIDIA GeForce RTX 4070 Ti                | RTX 4070 Ti              | 12        | Ada Lovelace |
| NVIDIA GeForce RTX 3080 Ti                | RTX 3080 Ti              | 12        | Ampere       |
| NVIDIA GeForce RTX 3080                   | RTX 3080                 | 10        | Ampere       |
| NVIDIA GeForce RTX 3070                   | RTX 3070                 | 8         | Ampere       |
| NVIDIA RTX A2000                          | RTX A2000                | 6         | Ampere       |

### GPU Pool IDs (Grouped by VRAM)

| Pool ID      | Included Models                  | VRAM (GB) |
| ------------ | -------------------------------- | --------- |
| `AMPERE_16`  | A4000, A4500, RTX 4000, RTX 2000 | 16        |
| `AMPERE_24`  | L4, A5000, RTX 3090              | 24        |
| `ADA_24`     | RTX 4090                         | 24        |
| `AMPERE_48`  | A6000, A40                       | 48        |
| `ADA_48_PRO` | L40, L40S, RTX 6000 Ada          | 48        |
| `AMPERE_80`  | A100                             | 80        |
| `ADA_80_PRO` | H100                             | 80        |
| `HOPPER_141` | H200                             | 141       |

---

## 8. CPU Pods

### Overview

RunPod supports CPU-only pods (no GPU required), useful for preprocessing, API servers, lightweight tasks.

### CPU Flavors

| Flavor            | Description       |
| ----------------- | ----------------- |
| `cpu3c` / `cpu5c` | Compute Optimized |
| General Purpose   | Balanced CPU/RAM  |
| Memory-Optimized  | High RAM per vCPU |

### Configuration

```json
{
  "computeType": "CPU",
  "vcpuCount": 4,
  "cpuFlavorIds": ["cpu3c-2-4"],
  "imageName": "my-image:latest"
}
```

### Recent Enhancements (2025)

- **Docker Runtime**: Replaced Kata Containers for faster startup and lower overhead
- **Network Volume Support**: GA as of March 2025 (previously GPU-only)
- **Limitation**: Docker-in-Docker (nested containers) no longer supported with Docker runtime

---

## 9. Networking and SSH

### Proxy URLs

Format: `https://<podId>-<port>.proxy.runpod.net`

- Available for all pods
- Basic access control via pod ID
- 100-second connection timeout (Cloudflare)
- Useful for web UIs (Jupyter, Gradio, etc.)

### SSH Access

#### Method 1: Proxy SSH (All Pods)

```bash
ssh root@ssh.runpod.io -i ~/.ssh/id_ed25519
```

- Available on all pods
- Does NOT support SCP or SFTP
- Limited to terminal access only

#### Method 2: Public IP SSH (Select Pods)

```bash
ssh root@<PUBLIC_IP> -p <PORT> -i ~/.ssh/id_ed25519
```

- Requires pod with public IP support
- Full SSH capabilities (SCP, SFTP)
- Port 22 must be exposed as TCP

#### SSH Key Setup

1. Generate: `ssh-keygen -t ed25519 -C "email@example.com"`
2. Copy public key: `cat ~/.ssh/id_ed25519.pub`
3. Add to RunPod console Settings > SSH Public Keys
4. Multiple keys separated by newlines

#### File Transfer via SCP (Public IP only)

```bash
# Upload to pod
scp -P <PORT> localfile.txt root@<PUBLIC_IP>:/workspace/

# Download from pod
scp -P <PORT> root@<PUBLIC_IP>:/workspace/file.txt ./
```

### Global Networking

- Pod-to-pod communication across data centers
- Available in 17+ data centers worldwide
- Enabled with `globalNetworking: true` in pod creation

### Port Mapping Format

```
"8888/http"    # HTTP port (accessed via proxy URL)
"22/tcp"       # TCP port (for SSH, direct access)
"5000/http"    # Custom HTTP service
```

---

## 10. Storage

### Storage Types

| Type           | Persistence                         | Billing           | Description                    |
| -------------- | ----------------------------------- | ----------------- | ------------------------------ |
| Container Disk | Ephemeral (lost on stop)            | Running only      | Temporary container filesystem |
| Pod Volume     | Persists on stop, lost on terminate | Running + stopped | Local SSD per-pod volume       |
| Network Volume | Fully persistent                    | Always            | Independent NVMe SSD storage   |

### Container Disk

- Default: 50 GB
- Temporary storage, removed when pod stops
- Only billed while pod is running
- Cost: $0.10/GB/month

### Pod Volume

- Default: 20 GB, mounted at `/workspace`
- Persists across pod stops and restarts
- Removed when pod is terminated
- Cost: $0.10/GB/month (running), $0.20/GB/month (stopped)

### Network Volumes

- Independent of any pod lifecycle
- Backed by high-performance NVMe SSDs
- Transfer speeds: 200-400 MB/s typical, up to 10 GB/s
- Can be enlarged (never reduced)
- Mounted at `/workspace` for pods, `/runpod-volume` for serverless
- **Secure Cloud only** (not available on Community Cloud)
- Must be attached during deployment (not afterward)

**Pricing:**

- Under 1 TB: $0.07/GB/month
- Over 1 TB: $0.05/GB/month

**S3-Compatible API:**
Available in select data centers (EUR-IS-1, EU-RO-1, EU-CZ-1, US-KS-2, US-CA-2) for file operations without active compute.

**Concurrent Access Warning:**
Writing to the same network volume from multiple pods/endpoints simultaneously may cause data corruption. Application logic must handle concurrent access.

### Network Volume Management via REST API

```bash
# Create network volume
curl -X POST https://rest.runpod.io/v1/networkvolumes \
  -H "Authorization: Bearer $RUNPOD_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-volume",
    "size": 100,
    "dataCenterId": "US-CA-2"
  }'
```

---

## 11. Pricing

### Billing Model

- **Per-second billing** for compute and storage
- **No data ingress/egress fees**
- Default spend limit: **$80/hour** (adjustable)

### Pricing Tiers

| Tier                   | Description                        | Savings          |
| ---------------------- | ---------------------------------- | ---------------- |
| On-Demand              | Pay-as-you-go, non-interruptible   | Baseline         |
| Spot (Interruptible)   | Spare capacity, can be interrupted | Up to 60-70% off |
| Savings Plan (3-month) | Upfront commitment                 | ~15-20% off      |
| Savings Plan (6-month) | Upfront commitment                 | ~20-25% off      |

### Representative GPU Pricing (approximate, per GPU/hour)

| GPU       | On-Demand (approx) | Notes                  |
| --------- | ------------------ | ---------------------- |
| RTX 3070  | ~$0.16             | Entry-level            |
| RTX 3090  | ~$0.25             | Community popular      |
| RTX 4090  | ~$0.44             | High perf/cost ratio   |
| L4        | ~$0.38             | Inference optimized    |
| L40S      | ~$0.74             | Professional inference |
| A100 80GB | ~$1.64             | Training standard      |
| H100 SXM  | ~$2.79             | Top-tier training      |
| H200      | ~$3.99             | Latest Hopper          |
| B200      | ~$5+               | Latest Blackwell       |

_Prices vary by data center and availability. Check console for current pricing._

### Storage Pricing

| Storage Type             | Cost           |
| ------------------------ | -------------- |
| Container Disk (running) | $0.10/GB/month |
| Pod Volume (running)     | $0.10/GB/month |
| Pod Volume (stopped)     | $0.20/GB/month |
| Network Volume (< 1TB)   | $0.07/GB/month |
| Network Volume (> 1TB)   | $0.05/GB/month |

---

## 12. Data Centers

### Regions

RunPod operates across 31+ global regions with data centers in:

**North America:**

- US-TX-3, US-TX-4 (Texas)
- US-GA-1, US-GA-2 (Georgia)
- US-IL-1 (Illinois)
- US-WA-1 (Washington)
- US-CA-2 (California)
- US-DE-1 (Delaware)
- US-KS-2 (Kansas)

**Europe:**

- EU-RO-1 (Romania)
- EU-CZ-1 (Czech Republic)
- EU-FR-1 (France)
- EU-NL-1 (Netherlands)
- EU-SE-1 (Sweden)
- EUR-IS-1, EUR-IS-2 (Iceland)

**Asia-Pacific:**

- AP-JP-1 (Japan/Fukushima)

### Cloud Types

| Type        | Description                                                             |
| ----------- | ----------------------------------------------------------------------- |
| `SECURE`    | RunPod-managed data centers, higher security, network volumes supported |
| `COMMUNITY` | Community-hosted GPUs, lower cost, no network volumes                   |
| `ALL`       | Both secure and community                                               |

---

## 13. Serverless Endpoints

### Overview

Deploy containerized applications as auto-scaling inference APIs.

### Key Features

- Auto-scaling from 0 to N workers
- Pay only for compute time used
- Custom Docker images with handler functions
- Supports GPU and CPU workers
- Network volume mounting for model weights

### Endpoint Lifecycle

1. Write handler function with `runpod.serverless.start()`
2. Build Docker image
3. Push to container registry
4. Create endpoint via console, API, or GitHub integration
5. Send requests to endpoint URL

### REST API for Serverless

```bash
# Create endpoint
curl -X POST https://rest.runpod.io/v1/endpoints \
  -H "Authorization: Bearer $RUNPOD_API_KEY" \
  -d '{
    "name": "my-endpoint",
    "templateId": "tmpl-xxx",
    "gpuTypeIds": ["NVIDIA GeForce RTX 4090"],
    "workersMin": 0,
    "workersMax": 5,
    "idleTimeout": 5,
    "executionTimeoutMs": 300000
  }'
```

### Serverless Request Pattern

```bash
# Submit job (async)
curl https://api.runpod.ai/v2/{endpoint_id}/run \
  -H "Authorization: Bearer $RUNPOD_API_KEY" \
  -d '{"input": {"prompt": "Hello, world!"}}'

# Check status
curl https://api.runpod.ai/v2/{endpoint_id}/status/{job_id} \
  -H "Authorization: Bearer $RUNPOD_API_KEY"

# Synchronous (blocks until complete)
curl https://api.runpod.ai/v2/{endpoint_id}/runsync \
  -H "Authorization: Bearer $RUNPOD_API_KEY" \
  -d '{"input": {"prompt": "Hello, world!"}}'
```

---

## 14. Best Practices

### Cost Optimization

1. **Use spot instances** for fault-tolerant workloads (up to 70% savings)
2. **Stop pods when idle** - stopped pods only incur volume storage costs
3. **Right-size GPU selection** - don't over-provision VRAM
4. **Use network volumes** to avoid re-downloading model weights
5. **Set spend limits** to prevent unexpected charges
6. **Use savings plans** for predictable long-running workloads

### Resource Management

1. **Use GPU pool IDs** (e.g., `AMPERE_80`) for flexibility across equivalent GPU types
2. **Check stock status** before creating pods to avoid failures
3. **Set CUDA version constraints** for compatibility
4. **Use `dataCenterPriority: "availability"`** for faster pod deployment
5. **Monitor GPU utilization** via telemetry to optimize workloads

### Deployment Patterns

1. **Dev/Test**: On-demand pods with smaller GPUs for development
2. **Training**: Spot instances with checkpointing for cost-effective training
3. **Inference**: Serverless endpoints with auto-scaling for production
4. **Data Pipeline**: CPU pods for preprocessing, GPU pods for compute
5. **Persistent Workspace**: Network volumes + on-demand pods for reproducible environments

### Storage Strategy

1. Store model weights on network volumes (persist across pod cycles)
2. Use container disk for temporary/scratch data
3. Use S3-compatible API for bulk data operations
4. Avoid concurrent writes to network volumes
5. Keep account funded to prevent volume termination

### Security

1. Use Secure Cloud for sensitive workloads
2. Configure SSH keys (ed25519 recommended) rather than passwords
3. Use private container registries for proprietary images
4. Leverage environment variables for secrets (not baked into images)

---

## 15. Integration Considerations for Sindri

### Recommended API for Sindri Integration

**REST API (v1)** is recommended over GraphQL because:

- Simpler HTTP request/response model
- Standard REST conventions
- Well-documented OpenAPI spec
- Easier to implement in shell scripts (curl)
- Newer and actively developed

### Key Operations Sindri Needs

| Operation       | REST Endpoint                         | GraphQL Mutation               |
| --------------- | ------------------------------------- | ------------------------------ |
| Create GPU Pod  | `POST /v1/pods`                       | `podFindAndDeployOnDemand`     |
| Create Spot Pod | `POST /v1/pods` (interruptible: true) | `podRentInterruptable`         |
| Create CPU Pod  | `POST /v1/pods` (computeType: "CPU")  | N/A (use REST)                 |
| List Pods       | `GET /v1/pods`                        | `myself { pods { ... } }`      |
| Get Pod         | `GET /v1/pods/{id}`                   | `pod(input: { podId: "..." })` |
| Start Pod       | `POST /v1/pods/{id}/start`            | `podResume`                    |
| Stop Pod        | `POST /v1/pods/{id}/stop`             | `podStop`                      |
| Terminate Pod   | `DELETE /v1/pods/{id}`                | `podTerminate`                 |
| Check GPU Avail | Via GraphQL only                      | `gpuTypes { lowestPrice }`     |
| Create Volume   | `POST /v1/networkvolumes`             | N/A                            |

### Configuration Mapping (sindri.yaml -> RunPod API)

```yaml
# Proposed sindri.yaml for RunPod
provider: runpod
region: US-CA-2
instance:
  gpu_type: "NVIDIA GeForce RTX 4090" # maps to gpuTypeIds
  gpu_count: 1 # maps to gpuCount
  gpu_pool: "ADA_24" # alternative: use pool ID
  cloud_type: SECURE # maps to cloudType
  spot: false # maps to interruptible
  bid_per_gpu: 0.3 # maps to bidPerGpu (spot only)
image: "runpod/pytorch:latest" # maps to imageName
storage:
  container_disk_gb: 50 # maps to containerDiskInGb
  volume_gb: 20 # maps to volumeInGb
  volume_mount: "/workspace" # maps to volumeMountPath
  network_volume_id: "vol-xxx" # maps to networkVolumeId
networking:
  ports: # maps to ports
    - "8888/http"
    - "22/tcp"
  public_ip: true # maps to supportPublicIp
  global_networking: false # maps to globalNetworking
env: # maps to env
  JUPYTER_PASSWORD: "mypassword"
```

### Authentication Flow

1. User sets `RUNPOD_API_KEY` environment variable
2. Sindri reads from env or config file
3. All API calls use Bearer token authentication
4. No OAuth or complex auth flows needed

### Error Handling Considerations

- GPU out of stock: Check `stockStatus` before creating, implement retry with fallback GPU types
- Spot interruption: Pod may be terminated at any time, need restart logic
- Insufficient funds: API returns error, display to user
- Rate limits: Not documented, but implement exponential backoff
- Network volume not in same DC: Must match data center IDs

---

## Source URLs

- RunPod Documentation: https://docs.runpod.io/
- GraphQL Pod Management: https://docs.runpod.io/sdks/graphql/manage-pods
- GraphQL API Spec: https://graphql-spec.runpod.io/
- REST API Overview: https://docs.runpod.io/api-reference/overview
- REST API Create Pod: https://docs.runpod.io/api-reference/pods/POST/pods
- GPU Types Reference: https://docs.runpod.io/references/gpu-types
- Pricing: https://docs.runpod.io/pods/pricing
- Network Volumes: https://docs.runpod.io/storage/network-volumes
- SSH Documentation: https://docs.runpod.io/pods/configuration/use-ssh
- CLI (runpodctl): https://github.com/runpod/runpodctl
- Python SDK: https://pypi.org/project/runpod/
- REST API Blog: https://www.runpod.io/blog/runpod-rest-api-gpu-management
- CPU Pods Enhancement: https://www.runpod.io/blog/enhanced-cpu-pods-docker-network
- Global Networking: https://docs.runpod.io/pods/networking
