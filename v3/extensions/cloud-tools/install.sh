#!/bin/bash
set -euo pipefail

# cloud-tools install script - Simplified for YAML-driven architecture
# This script focuses on installation logic only. All metadata, validation,
# and configuration is handled by extension.yaml

print_status "Installing cloud provider CLI tools..."

# Load pinned versions from co-located config
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=versions.env
source "$SCRIPT_DIR/versions.env"

# Detect architecture for binary downloads
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64) AWS_ARCH="x86_64"; ALI_ARCH="amd64"; DO_ARCH="amd64" ;;
  aarch64|arm64) AWS_ARCH="aarch64"; ALI_ARCH="arm64"; DO_ARCH="arm64" ;;
  *) print_warning "Unsupported architecture: $ARCH"; AWS_ARCH="x86_64"; ALI_ARCH="amd64"; DO_ARCH="amd64" ;;
esac

# Check Python availability for Azure CLI
PYTHON_AVAILABLE=false
if command_exists python3; then
  PYTHON_VERSION=$(python3 --version 2>&1 | grep -oP '(?<=Python )\d+\.\d+' || echo "0.0")
  PYTHON_MAJOR=$(echo "$PYTHON_VERSION" | cut -d. -f1)
  PYTHON_MINOR=$(echo "$PYTHON_VERSION" | cut -d. -f2)
  if [[ "$PYTHON_MAJOR" -ge 3 ]] && [[ "$PYTHON_MINOR" -ge 10 ]]; then
    PYTHON_AVAILABLE=true
  fi
fi

# AWS CLI - user-local install to avoid sudo (C-5 security compliance)
print_status "Installing AWS CLI..."
if command_exists aws; then
  print_warning "AWS CLI already installed: $(aws --version)"
else
  print_status "Installing AWS CLI v${AWS_VERSION}..."
  # Ensure user-local bin directory exists and is in PATH
  mkdir -p "$HOME/.local/bin" "$HOME/.local/aws-cli"
  if curl -fsSL "https://awscli.amazonaws.com/awscli-exe-linux-${AWS_ARCH}-${AWS_VERSION}.zip" -o "/tmp/awscliv2.zip"; then
    (cd /tmp && unzip -o -q awscliv2.zip && bash aws/install --install-dir "$HOME/.local/aws-cli" --bin-dir "$HOME/.local/bin" --update 2>/dev/null)
    rm -rf /tmp/aws /tmp/awscliv2.zip
    print_success "AWS CLI v${AWS_VERSION} installed to ~/.local/bin"
  else
    print_warning "Failed to download AWS CLI v${AWS_VERSION} installer"
    rm -f /tmp/awscliv2.zip
  fi
fi

# Azure CLI - user-local install via pip (no sudo required)
print_status "Installing Azure CLI..."
if command_exists az; then
  print_warning "Azure CLI already installed"
else
  # Azure CLI requires Python 3.10+ (pip install azure-cli)
  if [[ "$PYTHON_AVAILABLE" == "true" ]]; then
    if python3 -m pip install --user "azure-cli==${AZURE_CLI_VERSION}" 2>/dev/null; then
      # Ensure ~/.local/bin is in PATH
      export PATH="$HOME/.local/bin:$PATH"
      print_success "Azure CLI ${AZURE_CLI_VERSION} installed to ~/.local/bin"
    else
      print_warning "Failed to install Azure CLI via pip"
    fi
  else
    print_warning "Skipping Azure CLI (requires Python 3.10+, found $PYTHON_VERSION)"
  fi
fi

# Google Cloud SDK - tarball install to user directory (no sudo required)
print_status "Installing Google Cloud CLI..."
if command_exists gcloud; then
  print_warning "Google Cloud CLI already installed"
else
  print_status "Installing Google Cloud CLI v${GCLOUD_VERSION}..."
  # Download and extract Google Cloud SDK to user directory
  # Google uses "x86_64" and "arm" (not "aarch64")
  case "$ARCH" in
    x86_64|amd64) GCLOUD_ARCH="x86_64" ;;
    aarch64|arm64) GCLOUD_ARCH="arm" ;;
    *) print_warning "Unsupported architecture for Google Cloud SDK: $ARCH"; GCLOUD_ARCH="x86_64" ;;
  esac

  if curl -fsSL "https://dl.google.com/dl/cloudsdk/channels/rapid/downloads/google-cloud-cli-${GCLOUD_VERSION}-linux-${GCLOUD_ARCH}.tar.gz" -o "/tmp/google-cloud-sdk.tar.gz"; then
    tar -xzf /tmp/google-cloud-sdk.tar.gz -C "$HOME"
    "$HOME/google-cloud-sdk/install.sh" --quiet --usage-reporting=false --path-update=false --command-completion=false 2>/dev/null
    export PATH="$HOME/google-cloud-sdk/bin:$PATH"
    rm -f /tmp/google-cloud-sdk.tar.gz
    print_success "Google Cloud CLI v${GCLOUD_VERSION} installed to ~/google-cloud-sdk"
  else
    print_warning "Failed to download Google Cloud CLI v${GCLOUD_VERSION}"
    rm -f /tmp/google-cloud-sdk.tar.gz
  fi
fi

# Fly.io CLI
print_status "Installing Fly.io CLI (flyctl)..."
if command_exists flyctl; then
  print_warning "Fly.io CLI already installed: $(flyctl version 2>/dev/null | head -1)"
else
  print_status "Installing flyctl version: v${FLYCTL_VERSION}..."

  # Detect architecture
  FLY_ARCH=""
  case "$(uname -m)" in
    x86_64|amd64) FLY_ARCH="x86_64" ;;
    aarch64|arm64) FLY_ARCH="arm64" ;;
    *) print_warning "Unsupported architecture"; FLY_ARCH="x86_64" ;;
  esac

  # Download specific version from GitHub releases
  mkdir -p "$HOME/.fly/bin"
  if curl -fsSL "https://github.com/superfly/flyctl/releases/download/v${FLYCTL_VERSION}/flyctl_${FLYCTL_VERSION}_Linux_${FLY_ARCH}.tar.gz" -o "/tmp/flyctl.tar.gz"; then
    tar -xzf /tmp/flyctl.tar.gz -C /tmp
    mv /tmp/flyctl "$HOME/.fly/bin/"
    chmod +x "$HOME/.fly/bin/flyctl"
    rm -f /tmp/flyctl.tar.gz
    export FLYCTL_INSTALL="$HOME/.fly"
    export PATH="$FLYCTL_INSTALL/bin:$PATH"
    print_success "Fly.io CLI v${FLYCTL_VERSION} installed"
  else
    print_warning "Failed to download Fly.io CLI v${FLYCTL_VERSION}"
    rm -f /tmp/flyctl.tar.gz
  fi
fi

# Check if running in CI mode
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  print_status "CI mode detected - skipping optional cloud CLIs (Oracle, Alibaba, DigitalOcean, IBM)"
  exit 0
fi

# Oracle Cloud Infrastructure CLI
print_status "Installing Oracle Cloud CLI..."
if command_exists oci; then
  print_warning "Oracle Cloud CLI already installed"
else
  if bash -c "$(curl -L https://raw.githubusercontent.com/oracle/oci-cli/master/scripts/install/install.sh)" -- --accept-all-defaults 2>/dev/null; then
    export PATH=$PATH:$HOME/bin
    print_success "Oracle Cloud CLI installed"
  else
    print_warning "Failed to install Oracle Cloud CLI"
  fi
fi

# Alibaba Cloud CLI - user-local install (C-5 security compliance)
print_status "Installing Alibaba Cloud CLI..."
if command_exists aliyun; then
  print_warning "Alibaba Cloud CLI already installed"
else
  print_status "Installing Alibaba Cloud CLI version: v${ALIYUN_VERSION}"
  # Note: Alibaba CLI uses 'amd64' for x86_64 and 'arm64' for aarch64
  mkdir -p "$HOME/.local/bin"
  if wget -q -O "/tmp/aliyun-cli-linux-${ALIYUN_VERSION}-${ALI_ARCH}.tgz" "https://github.com/aliyun/aliyun-cli/releases/download/v${ALIYUN_VERSION}/aliyun-cli-linux-${ALIYUN_VERSION}-${ALI_ARCH}.tgz"; then
    tar xzf "/tmp/aliyun-cli-linux-${ALIYUN_VERSION}-${ALI_ARCH}.tgz" -C /tmp
    mv /tmp/aliyun "$HOME/.local/bin/"
    rm -f "/tmp/aliyun-cli-linux-${ALIYUN_VERSION}-${ALI_ARCH}.tgz"
    print_success "Alibaba Cloud CLI v${ALIYUN_VERSION} installed to ~/.local/bin"
  else
    print_warning "Failed to download Alibaba Cloud CLI"
    rm -f "/tmp/aliyun-cli-linux-${ALIYUN_VERSION}-${ALI_ARCH}.tgz"
  fi
fi

# DigitalOcean CLI - user-local install (C-5 security compliance)
print_status "Installing DigitalOcean CLI (doctl)..."
if command_exists doctl; then
  print_warning "DigitalOcean CLI already installed"
else
  print_status "Installing doctl version: v${DOCTL_VERSION}"
    mkdir -p "$HOME/.local/bin"
    if wget -q -O "/tmp/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz" "https://github.com/digitalocean/doctl/releases/download/v${DOCTL_VERSION}/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz"; then
      tar xzf "/tmp/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz" -C /tmp
      mv /tmp/doctl "$HOME/.local/bin/"
      rm -f "/tmp/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz"
      print_success "DigitalOcean CLI v${DOCTL_VERSION} installed to ~/.local/bin"
    else
      print_warning "Failed to download DigitalOcean CLI"
      rm -f "/tmp/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz" /tmp/doctl
    fi
fi

# IBM Cloud CLI - tarball install to user directory (no sudo required)
print_status "Installing IBM Cloud CLI..."
if command_exists ibmcloud; then
  print_warning "IBM Cloud CLI already installed"
else
  # Determine architecture for download
  case "$ARCH" in
    x86_64|amd64) IBM_ARCH="amd64" ;;
    aarch64|arm64) IBM_ARCH="arm64" ;;
    *) print_warning "Unsupported architecture for IBM Cloud CLI: $ARCH"; IBM_ARCH="amd64" ;;
  esac

  print_status "Installing IBM Cloud CLI version: v${IBM_VERSION}"
    mkdir -p "$HOME/.local/ibmcloud" "$HOME/.local/bin"
    if curl -fsSL "https://download.clis.cloud.ibm.com/ibm-cloud-cli/${IBM_VERSION}/binaries/IBM_Cloud_CLI_${IBM_VERSION}_linux_${IBM_ARCH}.tgz" -o "/tmp/ibmcloud.tgz"; then
      tar -xzf /tmp/ibmcloud.tgz -C "$HOME/.local/ibmcloud" --strip-components=1
      ln -sf "$HOME/.local/ibmcloud/ibmcloud" "$HOME/.local/bin/ibmcloud"
      rm -f /tmp/ibmcloud.tgz
      print_success "IBM Cloud CLI v${IBM_VERSION} installed to ~/.local/bin"
    else
      print_warning "Failed to download IBM Cloud CLI"
      rm -f /tmp/ibmcloud.tgz
    fi
fi

# Clean up APT caches and temporary files (critical for disk-constrained environments)
cleanup_apt_cache

print_success "Cloud provider CLI tools installation complete"
