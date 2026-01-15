#!/bin/bash
set -euo pipefail

# ollama install script - Installs Ollama LLM runtime
# Uses official installer with extended timeout for large binary download

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Ollama..."

# Check if running in CI mode - skip large downloads
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
    print_warning "CI mode detected - skipping Ollama installation (large binary)"
    print_status "Ollama can be installed manually with: curl -fsSL https://ollama.com/install.sh | sh"
    exit 0
fi

# ------------------------------------------------------------------------------
# GPU Detection and Warning
# ------------------------------------------------------------------------------
# Check if GPU is available. Ollama works without GPU but inference is slower.
# This allows users to run smaller models on CPU-only machines.

check_ollama_gpu() {
    local gpu_available=false

    # Check for NVIDIA GPU (most common for Ollama)
    if command_exists nvidia-smi; then
        if nvidia-smi --list-gpus &>/dev/null; then
            gpu_available=true
            local gpu_count
            gpu_count=$(nvidia-smi --list-gpus 2>/dev/null | wc -l)
            print_success "NVIDIA GPU detected ($gpu_count GPU(s) available)"
        fi
    fi

    # Check for AMD GPU (ROCm)
    if [[ "$gpu_available" == "false" ]] && command_exists rocm-smi; then
        if rocm-smi --showproductname &>/dev/null; then
            gpu_available=true
            print_success "AMD GPU detected (ROCm available)"
        fi
    fi

    # No GPU detected - warn but continue
    if [[ "$gpu_available" == "false" ]]; then
        print_warning "========================================================"
        print_warning "NO GPU DETECTED - Ollama will run in CPU-only mode"
        print_warning "========================================================"
        print_status ""
        print_status "Performance will be significantly slower without a GPU."
        print_status "However, smaller models work reasonably well on CPU:"
        print_status ""
        print_status "  Recommended CPU-friendly models:"
        print_status "    - tinyllama (638MB)     - Fast, basic tasks"
        print_status "    - phi3:mini (2.3GB)     - Good balance of speed/quality"
        print_status "    - gemma2:2b (1.6GB)     - Google's efficient small model"
        print_status "    - llama3.2:1b (1.3GB)   - Meta's smallest Llama"
        print_status "    - qwen2.5:0.5b (397MB)  - Very fast, basic tasks"
        print_status ""
        print_status "  To add GPU support later:"
        print_status "    - Docker: Install nvidia-container-toolkit"
        print_status "    - Fly.io: Configure gpu in sindri.yaml deployment.resources"
        print_status "    - Cloud: Use GPU-enabled instance types"
        print_status ""
        print_status "Continuing with CPU-only installation..."
        print_status ""
    fi
}

# Run GPU check
check_ollama_gpu

# Check if already installed
if command_exists ollama; then
    current_version=$(ollama --version 2>/dev/null | head -1 || echo "unknown")
    print_warning "Ollama already installed: $current_version"
    print_status "To upgrade, run: curl -fsSL https://ollama.com/install.sh | sh"
    exit 0
fi

# Install Ollama via direct tarball download to user-local directory (C-5 security compliance)
# This avoids the official installer's internal sudo calls
# Official docs: https://docs.ollama.com/linux
print_status "Downloading Ollama tarball (this may take several minutes)..."
print_status "The tarball is approximately 800MB - download time depends on network speed"

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64) OLLAMA_ARCH="amd64" ;;
  aarch64|arm64) OLLAMA_ARCH="arm64" ;;
  *) print_error "Unsupported architecture: $ARCH"; exit 1 ;;
esac

OLLAMA_URL="https://ollama.com/download/ollama-linux-${OLLAMA_ARCH}.tar.zst"
OLLAMA_TARBALL="/tmp/ollama-linux-${OLLAMA_ARCH}.tar.zst"
OLLAMA_BIN="$HOME/.local/bin/ollama"

# Ensure user-local bin directory exists
mkdir -p "$HOME/.local/bin"

# Download and extract ollama tarball
if curl -fsSL "$OLLAMA_URL" -o "$OLLAMA_TARBALL"; then
    # Extract just the ollama binary from the tarball (it's in bin/ollama)
    if tar -I zstd -xf "$OLLAMA_TARBALL" -C /tmp; then
        if [[ -f "/tmp/bin/ollama" ]]; then
            mv /tmp/bin/ollama "$OLLAMA_BIN"
            chmod +x "$OLLAMA_BIN"
            rm -rf /tmp/bin "$OLLAMA_TARBALL"

            if "$OLLAMA_BIN" --version &>/dev/null; then
                installed_version=$("$OLLAMA_BIN" --version 2>/dev/null | head -1 || echo "unknown")
                print_success "Ollama installed to ~/.local/bin: $installed_version"
                print_status "Start Ollama server with: ollama serve"
                print_status "Or run in background: nohup ollama serve > ~/ollama.log 2>&1 &"
                print_status "Pull a model with: ollama pull llama3.2"
            else
                print_error "Ollama binary installed but verification failed"
                rm -f "$OLLAMA_BIN"
                exit 1
            fi
        else
            print_error "Ollama binary not found in tarball at expected location /tmp/bin/ollama"
            rm -rf /tmp/bin "$OLLAMA_TARBALL"
            exit 1
        fi
    else
        print_error "Failed to extract Ollama tarball"
        rm -f "$OLLAMA_TARBALL"
        exit 1
    fi
else
    print_error "Failed to download Ollama tarball from $OLLAMA_URL"
    exit 1
fi

# Create workspace directory for models info
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"
mkdir -p "${WORKSPACE}/extensions/ollama"

print_success "Ollama installation complete"
