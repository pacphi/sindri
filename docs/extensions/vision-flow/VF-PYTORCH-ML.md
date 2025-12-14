# VF-PyTorch-ML

PyTorch deep learning framework with CUDA support.

## Overview

| Property         | Value                      |
| ---------------- | -------------------------- |
| **Category**     | ai                         |
| **Version**      | 1.0.0                      |
| **Installation** | script                     |
| **Disk Space**   | 5000 MB                    |
| **Memory**       | 8192 MB                    |
| **Dependencies** | [python](../PYTHON.md)     |
| **GPU**          | Recommended (NVIDIA, 8GB+) |

## Description

PyTorch deep learning framework with CUDA support (from VisionFlow) - provides comprehensive ML/AI capabilities with GPU acceleration. Includes transformers, datasets, and common ML libraries.

## Installed Tools

| Tool           | Type      | Description               |
| -------------- | --------- | ------------------------- |
| `pytorch`      | framework | Deep learning framework   |
| `transformers` | library   | Hugging Face transformers |
| `datasets`     | library   | Dataset management        |
| `accelerate`   | library   | Distributed training      |

## Configuration

### Environment Variables

| Variable               | Value | Scope  |
| ---------------------- | ----- | ------ |
| `CUDA_VISIBLE_DEVICES` | `0`   | bashrc |

### Templates

| Template   | Destination                           | Description         |
| ---------- | ------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-pytorch-ml/SKILL.md` | Skill documentation |

## GPU Requirements

- **Required**: No (CPU fallback available)
- **Recommended**: Yes (NVIDIA GPU with 8GB+ VRAM)
- **Type**: nvidia
- **Min Memory**: 8192 MB

## Network Requirements

- `pypi.org` - Python packages
- `download.pytorch.org` - PyTorch downloads
- `huggingface.co` - Model hub

## Installation

```bash
extension-manager install vf-pytorch-ml
```

**Note:** Automatically detects GPU and installs CUDA-enabled or CPU-only PyTorch.

## Validation

```bash
python3 -c "import torch; print(torch.__version__)"
```

Expected output pattern: `\d+\.\d+`

To check CUDA availability:

```bash
python3 -c "import torch; print(f'CUDA available: {torch.cuda.is_available()}')"
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-pytorch-ml
```

## Removal

```bash
extension-manager remove vf-pytorch-ml
```

Removes:

- `~/extensions/vf-pytorch-ml`
- `~/.cache/torch`
- `~/.cache/huggingface`

## Related Extensions

- [vf-comfyui](VF-COMFYUI.md) - Image generation (uses PyTorch)
- [vf-jupyter-notebooks](VF-JUPYTER-NOTEBOOKS.md) - Interactive notebooks
- [python](../PYTHON.md) - Python runtime (required)

## Additional Notes

- Includes common ML libraries: numpy, scipy, scikit-learn, matplotlib, pandas
- Supports both CUDA 12.1 (GPU) and CPU-only installations
- Model cache stored in `~/.cache/huggingface`
