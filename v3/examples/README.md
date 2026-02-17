# Sindri v3 Provider Examples

This directory contains example `sindri.yaml` configurations for different providers and use cases.

## RunPod Examples

| File                        | Description                                    |
| --------------------------- | ---------------------------------------------- |
| `runpod-gpu-basic.yaml`     | Basic GPU development with NVIDIA RTX A4000    |
| `runpod-a100-training.yaml` | A100 80GB training with network volume         |
| `runpod-spot.yaml`          | Spot (interruptible) instance for cost savings |
| `runpod-cpu-only.yaml`      | CPU-only pod without GPU                       |

## Northflank Examples

| File                          | Description                                    |
| ----------------------------- | ---------------------------------------------- |
| `northflank-basic.yaml`       | Basic development setup (2 vCPU, 4 GB)         |
| `northflank-gpu.yaml`         | GPU-enabled ML training with A100              |
| `northflank-autoscaling.yaml` | Auto-scaling production API with health checks |
| `northflank-full.yaml`        | Enterprise setup with all features             |

## Cross-Provider

| File                       | Description                                                 |
| -------------------------- | ----------------------------------------------------------- |
| `provider-comparison.yaml` | Same workload configured for RunPod, Northflank, and Fly.io |

## Usage

```bash
# Copy an example to your project root
cp examples/runpod-gpu-basic.yaml sindri.yaml

# Edit the configuration for your needs
vim sindri.yaml

# Deploy
sindri deploy
```

## Quick Reference

| Use Case    | RunPod Example         | Northflank Example              |
| ----------- | ---------------------- | ------------------------------- |
| Basic Dev   | `runpod-gpu-basic`     | `northflank-basic`              |
| ML Training | `runpod-a100-training` | `northflank-gpu`                |
| Production  | `runpod-spot`          | `northflank-autoscaling`        |
| CPU Only    | `runpod-cpu-only`      | `northflank-basic` (remove GPU) |

## Prerequisites

### RunPod

- Set the `RUNPOD_API_KEY` environment variable
- Get your key at https://www.runpod.io/console/user/settings

### Northflank

- Install the CLI: `npm i @northflank/cli -g`
- Authenticate: `northflank login` or set `NORTHFLANK_API_TOKEN`
