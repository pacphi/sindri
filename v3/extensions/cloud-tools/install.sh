#!/bin/bash
set -euo pipefail

# cloud-tools install script - Simplified for YAML-driven architecture
# This script focuses on installation logic only. All metadata, validation,
# and configuration is handled by extension.yaml

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing cloud provider CLI tools..."

# Detect architecture for binary downloads
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64) AWS_ARCH="x86_64"; ALI_ARCH="amd64"; DO_ARCH="amd64" ;;
  aarch64|arm64) AWS_ARCH="aarch64"; ALI_ARCH="arm64"; DO_ARCH="arm64" ;;
  *) print_warning "Unsupported architecture: $ARCH"; AWS_ARCH="x86_64"; ALI_ARCH="amd64"; DO_ARCH="amd64" ;;
esac

# AWS CLI - user-local install to avoid sudo (C-5 security compliance)
print_status "Installing AWS CLI..."
if command_exists aws; then
  print_warning "AWS CLI already installed: $(aws --version)"
else
  # Ensure user-local bin directory exists and is in PATH
  mkdir -p "$HOME/.local/bin" "$HOME/.local/aws-cli"
  if curl -fsSL "https://awscli.amazonaws.com/awscli-exe-linux-${AWS_ARCH}.zip" -o "/tmp/awscliv2.zip"; then
    (cd /tmp && unzip -o -q awscliv2.zip && bash aws/install --install-dir "$HOME/.local/aws-cli" --bin-dir "$HOME/.local/bin" --update 2>/dev/null)
    rm -rf /tmp/aws /tmp/awscliv2.zip
    print_success "AWS CLI installed to ~/.local/bin"
  else
    print_warning "Failed to download AWS CLI installer"
    rm -f /tmp/awscliv2.zip
  fi
fi

# Azure CLI
print_status "Installing Azure CLI..."
if command_exists az; then
  print_warning "Azure CLI already installed"
else
  if curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash 2>/dev/null; then
    print_success "Azure CLI installed"
  else
    print_warning "Failed to install Azure CLI"
  fi
fi

# Google Cloud CLI
print_status "Installing Google Cloud CLI..."
if command_exists gcloud; then
  print_warning "Google Cloud CLI already installed"
else
  if echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | sudo tee -a /etc/apt/sources.list.d/google-cloud-sdk.list > /dev/null 2>&1 && \
     curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo apt-key --keyring /usr/share/keyrings/cloud.google.gpg add - 2>/dev/null && \
     sudo apt-get update 2>/dev/null && sudo apt-get install -y google-cloud-cli 2>/dev/null; then
    print_success "Google Cloud CLI installed"
  else
    print_warning "Failed to install Google Cloud CLI (sudo may be restricted)"
  fi
fi

# Fly.io CLI
print_status "Installing Fly.io CLI (flyctl)..."
if command_exists flyctl; then
  print_warning "Fly.io CLI already installed: $(flyctl version 2>/dev/null | head -1)"
else
  if curl -L https://fly.io/install.sh | sh 2>/dev/null; then
    # Add to PATH for current session
    export FLYCTL_INSTALL="${FLYCTL_INSTALL:-$HOME/.fly}"
    export PATH="$FLYCTL_INSTALL/bin:$PATH"
    print_success "Fly.io CLI installed"
  else
    print_warning "Failed to install Fly.io CLI"
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
  # Note: Alibaba CLI uses 'amd64' for x86_64 and 'arm64' for aarch64
  mkdir -p "$HOME/.local/bin"
  if wget -q -O "/tmp/aliyun-cli-linux-latest-${ALI_ARCH}.tgz" "https://aliyuncli.alicdn.com/aliyun-cli-linux-latest-${ALI_ARCH}.tgz"; then
    tar xzf "/tmp/aliyun-cli-linux-latest-${ALI_ARCH}.tgz" -C /tmp
    mv /tmp/aliyun "$HOME/.local/bin/"
    rm -f "/tmp/aliyun-cli-linux-latest-${ALI_ARCH}.tgz"
    print_success "Alibaba Cloud CLI installed to ~/.local/bin"
  else
    print_warning "Failed to download Alibaba Cloud CLI"
    rm -f "/tmp/aliyun-cli-linux-latest-${ALI_ARCH}.tgz"
  fi
fi

# DigitalOcean CLI - user-local install (C-5 security compliance)
print_status "Installing DigitalOcean CLI (doctl)..."
if command_exists doctl; then
  print_warning "DigitalOcean CLI already installed"
else
  # Use standardized GitHub release version detection (gh CLI with curl fallback)
  DOCTL_VERSION=$(get_github_release_version "digitalocean/doctl" false)
  if [[ -n "$DOCTL_VERSION" ]]; then
    print_status "Latest doctl version: v${DOCTL_VERSION}"
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
  else
    print_warning "Failed to fetch DigitalOcean CLI version from GitHub"
  fi
fi

# IBM Cloud CLI
print_status "Installing IBM Cloud CLI..."
if command_exists ibmcloud; then
  print_warning "IBM Cloud CLI already installed"
else
  if curl -fsSL https://clis.cloud.ibm.com/install/linux | sh 2>/dev/null; then
    print_success "IBM Cloud CLI installed"
  else
    print_warning "Failed to install IBM Cloud CLI"
  fi
fi

# Clean up APT caches and temporary files (critical for disk-constrained environments)
cleanup_apt_cache

print_success "Cloud provider CLI tools installation complete"
