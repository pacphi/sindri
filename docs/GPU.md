# GPU Configuration Guide

Comprehensive guide to configuring GPU-accelerated Sindri environments for AI/ML, 3D rendering, and compute-intensive workloads.

## Overview

Sindri supports GPU-accelerated deployments across multiple providers using declarative YAML configuration. GPU support enables:

- **AI/ML Training** - Train neural networks and fine-tune models
- **Inference Workloads** - Run large language models and image generation
- **3D Rendering** - GPU-accelerated rendering and visualization
- **Scientific Computing** - CUDA-based computation and simulations

**Supported GPU Vendors:**

- NVIDIA (primary support: CUDA, TensorRT, cuDNN)
- AMD (via ROCm, limited provider support)

## Quick Start

Basic GPU configuration in `sindri.yaml`:

```yaml
deployment:
  provider: fly # or: docker, devpod (aws/gcp/azure)
  resources:
    memory: 16GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium
      count: 1
```

Deploy:

```bash
./cli/sindri deploy
```

## Configuration Reference

### GPU Resource Block

```yaml
deployment:
  resources:
    gpu:
      enabled: boolean # Enable GPU support (default: false)
      type: string # nvidia | amd (default: nvidia)
      count: integer # Number of GPUs: 1-8 (default: 1)
      tier: string # GPU tier (see tiers below)
      memory: string # Minimum GPU memory (e.g., "16GB", "40GB")
```

### GPU Tiers

Sindri abstracts provider-specific GPU types into tiers for easier configuration:

| Tier           | GPU Memory | GPU Types                 | vCPU Range | Use Cases                                   |
| -------------- | ---------- | ------------------------- | ---------- | ------------------------------------------- |
| **gpu-small**  | 8-16 GB    | T4, RTX 3060              | 4-8        | Inference, development, light training      |
| **gpu-medium** | 16-24 GB   | A10G, L4, RTX 4070        | 8-16       | Training, inference, fine-tuning            |
| **gpu-large**  | 40-48 GB   | L40S, A100-40GB, RTX 4090 | 16-32      | Large model training, multi-model inference |
| **gpu-xlarge** | 80+ GB     | A100-80GB, H100           | 32-96      | LLM training, enterprise workloads          |

**Tier Selection Guidelines:**

- **Development/Testing**: `gpu-small` (T4) - $0.50-1.00/hr
- **Production Inference**: `gpu-medium` (A10G) - $1.00-2.00/hr
- **Model Training**: `gpu-large` (A100-40GB) - $2.50-4.00/hr
- **LLM Training**: `gpu-xlarge` (A100-80GB/H100) - $8.00-15.00/hr

## Provider-Specific Configuration

### Fly.io

Fly.io provides A100 and L40S GPUs in select regions.

```yaml
deployment:
  provider: fly
  resources:
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium # Maps to a100-40gb
      count: 1

providers:
  fly:
    region: ord # Chicago - primary GPU region
    # Also available: sjc (San Jose)
    cpuKind: performance
    autoStopMachines: false # Keep GPU instances running
```

**Available Fly.io GPU Regions:**

- `ord` (Chicago, IL) - Primary GPU availability
- `sjc` (San Jose, CA) - Limited GPU availability

**Fly.io GPU Tier Mappings:**

| Tier       | Fly.io GPU | Memory | vCPUs |
| ---------- | ---------- | ------ | ----- |
| gpu-small  | a100-40gb  | 40 GB  | 8     |
| gpu-medium | a100-40gb  | 40 GB  | 16    |
| gpu-large  | l40s       | 48 GB  | 16    |
| gpu-xlarge | a100-80gb  | 80 GB  | 32    |

**Cost Estimate (Fly.io):**

- A100-40GB: ~$2.50/hr (~$1,800/mo continuous)
- L40S: ~$3.00/hr (~$2,160/mo continuous)
- A100-80GB: ~$4.50/hr (~$3,240/mo continuous)

**Notes:**

- Auto-stop not recommended for GPU instances (slow cold starts)
- GPU instances incur charges even when idle if not stopped
- Volume storage billed separately (~$0.15/GB/month)

### Docker (Local Development)

Requires NVIDIA Docker runtime and compatible GPU.

```yaml
deployment:
  provider: docker
  resources:
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-small

providers:
  docker:
    privileged: true # Required for GPU access
```

**Prerequisites:**

```bash
# Install NVIDIA drivers
sudo apt install nvidia-driver-535

# Install NVIDIA Container Toolkit
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | \
  sudo tee /etc/apt/sources.list.d/nvidia-docker.list
sudo apt update && sudo apt install -y nvidia-container-toolkit

# Restart Docker
sudo systemctl restart docker

# Verify GPU access
docker run --rm --gpus all nvidia/cuda:11.8.0-base-ubuntu22.04 nvidia-smi
```

**Docker GPU Tier Mappings:**

Uses host GPU directly - tier names for consistency:

| Tier       | Typical GPUs              |
| ---------- | ------------------------- |
| gpu-small  | GTX 1080, RTX 3060, T4    |
| gpu-medium | RTX 3090, RTX 4070, A10G  |
| gpu-large  | RTX 4090, L40S, A100-40GB |
| gpu-xlarge | A100-80GB, H100           |

### AWS (via DevPod)

AWS EC2 GPU instances via DevPod.

```yaml
deployment:
  provider: devpod
  resources:
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium

providers:
  devpod:
    type: aws
    buildRepository: ghcr.io/myorg/sindri # Required for cloud
    aws:
      region: us-west-2
      # Instance type auto-selected based on tier
      diskSize: 100
      useSpot: false # Spot instances not recommended for GPU
```

**AWS GPU Tier Mappings:**

| Tier       | Instance Type | GPU            | vCPUs | Memory  | Cost (us-west-2) |
| ---------- | ------------- | -------------- | ----- | ------- | ---------------- |
| gpu-small  | g4dn.xlarge   | T4 (16GB)      | 4     | 16 GB   | ~$0.526/hr       |
| gpu-medium | g5.2xlarge    | A10G (24GB)    | 8     | 32 GB   | ~$1.212/hr       |
| gpu-large  | g5.4xlarge    | A10G (24GB)    | 16    | 64 GB   | ~$1.624/hr       |
| gpu-xlarge | p4d.24xlarge  | A100x8 (320GB) | 96    | 1152 GB | ~$32.77/hr       |

**Available AWS GPU Regions:**

- us-east-1, us-east-2, us-west-2 (best GPU availability)
- eu-west-1, eu-central-1
- ap-southeast-1, ap-northeast-1

### GCP (via DevPod)

Google Cloud Compute Engine with GPU accelerators.

```yaml
deployment:
  provider: devpod
  resources:
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium

providers:
  devpod:
    type: gcp
    buildRepository: ghcr.io/myorg/sindri
    gcp:
      project: my-project-id
      zone: us-central1-a # GPU availability varies by zone
      diskSize: 100
      diskType: pd-ssd # Recommended for GPU workloads
```

**GCP GPU Tier Mappings:**

| Tier       | Machine Type   | GPU Accelerator        | vCPUs | Memory  | Cost (us-central1) |
| ---------- | -------------- | ---------------------- | ----- | ------- | ------------------ |
| gpu-small  | n1-standard-4  | nvidia-tesla-t4 (x1)   | 4     | 15 GB   | ~$0.49/hr          |
| gpu-medium | n1-standard-8  | nvidia-tesla-a10g (x1) | 8     | 30 GB   | ~$1.28/hr          |
| gpu-large  | g2-standard-16 | nvidia-l4 (x1)         | 16    | 64 GB   | ~$1.65/hr          |
| gpu-xlarge | a2-megagpu-16g | nvidia-a100-80gb (x16) | 96    | 1360 GB | ~$55/hr            |

**Available GCP GPU Zones:**

- us-central1-a, us-central1-b, us-central1-c
- us-west1-b, us-east1-c
- europe-west4-a, asia-southeast1-c

### Azure (via DevPod)

Azure Virtual Machines with GPU support.

```yaml
deployment:
  provider: devpod
  resources:
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium

providers:
  devpod:
    type: azure
    buildRepository: ghcr.io/myorg/sindri
    azure:
      subscription: xxx-xxx-xxx
      resourceGroup: devpod-resources
      location: eastus
      diskSize: 100
```

**Azure GPU Tier Mappings:**

| Tier       | VM Size                   | GPU            | vCPUs | Memory  | Cost (East US) |
| ---------- | ------------------------- | -------------- | ----- | ------- | -------------- |
| gpu-small  | Standard_NC4as_T4_v3      | T4 (16GB)      | 4     | 28 GB   | ~$0.526/hr     |
| gpu-medium | Standard_NC8as_T4_v3      | T4 (16GB)      | 8     | 56 GB   | ~$0.904/hr     |
| gpu-large  | Standard_NC24ads_A100_v4  | A100 (80GB)    | 24    | 220 GB  | ~$3.67/hr      |
| gpu-xlarge | Standard_ND96amsr_A100_v4 | A100x8 (640GB) | 96    | 1900 GB | ~$27.20/hr     |

**Available Azure GPU Regions:**

- eastus, westus2, southcentralus
- westeurope, northeurope
- southeastasia, japaneast

### Kubernetes (via DevPod)

Deploy to existing Kubernetes clusters with GPU node pools.

```yaml
deployment:
  provider: devpod
  resources:
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium

providers:
  devpod:
    type: kubernetes
    buildRepository: ghcr.io/myorg/sindri
    kubernetes:
      namespace: devpod
      context: my-gpu-cluster
      storageClass: fast-ssd
```

**Kubernetes GPU Configuration:**

Requires GPU nodes with NVIDIA device plugin:

```bash
# Install NVIDIA device plugin
kubectl create -f https://raw.githubusercontent.com/NVIDIA/k8s-device-plugin/v0.14.0/nvidia-device-plugin.yml

# Verify GPU nodes
kubectl get nodes -L nvidia.com/gpu
```

**GPU Node Selectors:**

| Tier       | Node Selector                | GPU Count |
| ---------- | ---------------------------- | --------- |
| gpu-small  | accelerator: nvidia-tesla-t4 | 1         |
| gpu-medium | accelerator: nvidia-a10g     | 1         |
| gpu-large  | accelerator: nvidia-l40s     | 1         |
| gpu-xlarge | accelerator: nvidia-a100     | 8         |

## Use Case Examples

### AI/ML Inference

Small model inference (e.g., Stable Diffusion, small LLMs):

```yaml
version: 1.0
name: ml-inference
deployment:
  provider: fly
  resources:
    memory: 16GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-small # T4 sufficient for inference
      count: 1
extensions:
  profile: ai-dev # Includes Python, PyTorch, TensorFlow
  additional:
    - vf-comfyui # Stable Diffusion UI
```

**Cost**: ~$1.00/hr (~$720/mo)

### Model Fine-Tuning

Fine-tuning medium models (e.g., 7B parameter models):

```yaml
version: 1.0
name: model-training
deployment:
  provider: devpod
  resources:
    memory: 64GB
    cpus: 16
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium # A10G/L4
      count: 1
  volumes:
    workspace:
      size: 500GB # Large dataset storage
providers:
  devpod:
    type: aws
    buildRepository: ghcr.io/myorg/sindri
    aws:
      region: us-west-2
      diskSize: 500
extensions:
  profile: ai-dev
  additional:
    - vf-pytorch-ml
```

**Cost (AWS)**: ~$1.21/hr (~$870/mo)

### Large Model Training

Training/fine-tuning large models (13B-70B parameters):

```yaml
version: 1.0
name: llm-training
deployment:
  provider: devpod
  resources:
    memory: 128GB
    cpus: 32
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-large # A100-40GB or L40S
      count: 1
  volumes:
    workspace:
      size: 1TB
providers:
  devpod:
    type: gcp
    buildRepository: ghcr.io/myorg/sindri
    gcp:
      project: my-project
      zone: us-central1-a
      diskSize: 1000
      diskType: pd-ssd
extensions:
  profile: ai-dev
  additional:
    - vf-pytorch-ml
    - vf-deepspeed # Distributed training
```

**Cost (GCP)**: ~$1.65/hr (~$1,188/mo)

### 3D Rendering

GPU-accelerated 3D rendering and visualization:

```yaml
version: 1.0
name: 3d-rendering
deployment:
  provider: docker # Local workstation with RTX GPU
  resources:
    memory: 32GB
    cpus: 16
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-large # RTX 4090 or similar
      count: 1
providers:
  docker:
    privileged: true
extensions:
  profile: visionflow-creative
  additional:
    - vf-pbr-rendering
    - vf-blender
```

**Cost**: Hardware dependent (local GPU)

### Multi-GPU Training

Distributed training across multiple GPUs:

```yaml
version: 1.0
name: distributed-training
deployment:
  provider: devpod
  resources:
    memory: 1152GB
    cpus: 96
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-xlarge # 8x A100
      count: 8
  volumes:
    workspace:
      size: 2TB
providers:
  devpod:
    type: aws
    buildRepository: ghcr.io/myorg/sindri
    aws:
      region: us-west-2
      # Uses p4d.24xlarge automatically
extensions:
  profile: ai-dev
  additional:
    - vf-pytorch-ml
    - vf-deepspeed
    - vf-horovod # Multi-GPU orchestration
```

**Cost (AWS)**: ~$32.77/hr (~$23,600/mo)

## GPU-Dependent Extensions

These extensions require or benefit from GPU acceleration:

### AI/ML Extensions

- `vf-pytorch-ml` - PyTorch with CUDA support
- `vf-tensorflow` - TensorFlow GPU
- `vf-comfyui` - Stable Diffusion UI (requires 8GB+ VRAM)
- `vf-deepspeed` - Distributed training framework
- `vf-horovod` - Multi-GPU training

### 3D Rendering Extensions

- `vf-pbr-rendering` - Physically-based rendering
- `vf-blender` - 3D modeling with GPU rendering
- `vf-unreal-engine` - Game engine (requires 16GB+ VRAM)

### Data Science Extensions

- `vf-rapids` - GPU-accelerated data science (NVIDIA only)
- `vf-cudf` - GPU DataFrames

## Cost Optimization

### Best Practices

1. **Use auto-stop when possible** (not recommended for Fly.io GPU instances due to slow cold starts)
2. **Right-size your tier**: Don't overprovision
3. **Use spot instances** for non-critical workloads (AWS, GCP, Azure)
4. **Local development first**: Test on Docker locally before cloud deployment
5. **Schedule workloads**: Run training during off-peak hours if possible

### Cost Comparison Matrix

Monthly cost estimate for continuous operation (730 hrs/month):

| Tier       | Fly.io | AWS     | GCP     | Azure   |
| ---------- | ------ | ------- | ------- | ------- |
| gpu-small  | $1,825 | $384    | $358    | $384    |
| gpu-medium | $1,825 | $885    | $934    | $660    |
| gpu-large  | $2,190 | $1,186  | $1,205  | $2,680  |
| gpu-xlarge | $3,285 | $23,922 | $40,150 | $19,856 |

**Spot Instance Savings** (AWS/GCP/Azure): 50-90% discount, subject to interruption

## Monitoring GPU Usage

### Inside Container

```bash
# Check GPU status
nvidia-smi

# Monitor GPU in real-time
watch -n 1 nvidia-smi

# GPU utilization
nvidia-smi --query-gpu=utilization.gpu --format=csv

# GPU memory usage
nvidia-smi --query-gpu=memory.used,memory.total --format=csv
```

### PyTorch GPU Check

```python
import torch

print(f"CUDA available: {torch.cuda.is_available()}")
print(f"CUDA version: {torch.version.cuda}")
print(f"GPU count: {torch.cuda.device_count()}")
print(f"GPU name: {torch.cuda.get_device_name(0)}")
```

## Troubleshooting

### GPU Not Detected

**Docker:**

```bash
# Verify nvidia-docker runtime
docker run --rm --gpus all nvidia/cuda:11.8.0-base-ubuntu22.04 nvidia-smi

# Check Docker daemon config
cat /etc/docker/daemon.json
# Should include: "default-runtime": "nvidia"
```

**Fly.io:**

```bash
# Check GPU allocation
flyctl status -a your-app-name

# SSH into container
flyctl ssh console -a your-app-name
nvidia-smi
```

### Out of Memory Errors

1. **Reduce batch size** in training scripts
2. **Enable gradient checkpointing** (PyTorch/TensorFlow)
3. **Upgrade to larger tier** (more VRAM)
4. **Use mixed precision training** (FP16/BF16)

### Slow Performance

1. **Check GPU utilization**: `nvidia-smi` - should be >80%
2. **Verify CUDA version**: Match PyTorch/TensorFlow CUDA version
3. **Enable cudnn benchmarking**: `torch.backends.cudnn.benchmark = True`
4. **Check for CPU bottlenecks**: Data loading, preprocessing

### Driver Version Mismatches

```bash
# Check driver version
nvidia-smi | grep "Driver Version"

# CUDA compatibility
# Driver 525.x: CUDA 12.0
# Driver 520.x: CUDA 11.8
# Driver 515.x: CUDA 11.7
```

## See Also

- [Configuration Reference](CONFIGURATION.md) - Full sindri.yaml reference
- [Provider Guides](providers/) - Provider-specific documentation
- [AI/ML Extensions](EXTENSIONS.md#ai-tools) - GPU-accelerated extensions
- [vm-sizes.yaml](../docker/lib/vm-sizes.yaml) - Complete GPU tier mappings

## GPU Pricing References

- **Fly.io**: https://fly.io/docs/about/pricing/#gpus
- **AWS EC2**: https://aws.amazon.com/ec2/instance-types/
- **GCP Compute**: https://cloud.google.com/compute/gpus-pricing
- **Azure VMs**: https://azure.microsoft.com/en-us/pricing/details/virtual-machines/
