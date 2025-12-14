#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-pytorch-ml
# VisionFlow capability: PyTorch deep learning framework

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-pytorch-ml"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-pytorch-ml/resources"

print_status "Installing PyTorch ML framework..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Detect CUDA availability and install appropriate PyTorch
if command -v nvidia-smi &>/dev/null; then
    print_status "NVIDIA GPU detected, installing CUDA-enabled PyTorch..."
    pip install --user torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121
else
    print_status "No GPU detected, installing CPU-only PyTorch..."
    pip install --user torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu
fi

# Install common ML libraries
pip install --user \
    transformers \
    datasets \
    accelerate \
    safetensors \
    numpy \
    scipy \
    scikit-learn \
    matplotlib \
    pandas

print_success "vf-pytorch-ml installed successfully"

# Verify installation
python3 -c "import torch; print(f'PyTorch {torch.__version__}'); print(f'CUDA available: {torch.cuda.is_available()}')"
