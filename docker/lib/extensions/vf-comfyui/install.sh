#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-comfyui
# VisionFlow capability: ComfyUI image generation

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-comfyui"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-comfyui/resources"

print_status "Installing ComfyUI..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Clone ComfyUI if not using local copy
if [[ ! -d "${EXTENSION_DIR}/ComfyUI" ]]; then
    print_status "Cloning ComfyUI..."
    git clone https://github.com/comfyanonymous/ComfyUI.git "${EXTENSION_DIR}/ComfyUI" || true
fi

# Install Python dependencies
pip install --user torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121 || \
    pip install --user torch torchvision torchaudio

# Install ComfyUI requirements
if [[ -f "${EXTENSION_DIR}/ComfyUI/requirements.txt" ]]; then
    pip install --user -r "${EXTENSION_DIR}/ComfyUI/requirements.txt"
fi

# Install MCP dependencies
pip install --user mcp pydantic httpx websockets

print_success "vf-comfyui installed successfully"
print_warning "Requires GPU (NVIDIA, 8GB+ VRAM)"
print_status "Port: 8188"
