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

# AWS CLI
print_status "Installing AWS CLI..."
if command_exists aws; then
  print_warning "AWS CLI already installed: $(aws --version)"
else
  if curl -fsSL "https://awscli.amazonaws.com/awscli-exe-linux-${AWS_ARCH}.zip" -o "/tmp/awscliv2.zip"; then
    (cd /tmp && unzip -q awscliv2.zip && sudo ./aws/install 2>/dev/null)
    rm -rf /tmp/aws /tmp/awscliv2.zip
    print_success "AWS CLI installed"
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
  echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | sudo tee -a /etc/apt/sources.list.d/google-cloud-sdk.list > /dev/null
  if curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo apt-key --keyring /usr/share/keyrings/cloud.google.gpg add - 2>/dev/null; then
    sudo apt-get update && sudo apt-get install -y google-cloud-cli
    print_success "Google Cloud CLI installed"
  else
    print_warning "Failed to install Google Cloud CLI"
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

# Alibaba Cloud CLI
print_status "Installing Alibaba Cloud CLI..."
if command_exists aliyun; then
  print_warning "Alibaba Cloud CLI already installed"
else
  # Note: Alibaba CLI uses 'amd64' for x86_64 and 'arm64' for aarch64
  if wget -q -O "/tmp/aliyun-cli-linux-latest-${ALI_ARCH}.tgz" "https://aliyuncli.alicdn.com/aliyun-cli-linux-latest-${ALI_ARCH}.tgz"; then
    tar xzf "/tmp/aliyun-cli-linux-latest-${ALI_ARCH}.tgz" -C /tmp
    sudo mv /tmp/aliyun /usr/local/bin/
    rm -f "/tmp/aliyun-cli-linux-latest-${ALI_ARCH}.tgz"
    print_success "Alibaba Cloud CLI installed"
  else
    print_warning "Failed to download Alibaba Cloud CLI"
    rm -f "/tmp/aliyun-cli-linux-latest-${ALI_ARCH}.tgz"
  fi
fi

# DigitalOcean CLI
print_status "Installing DigitalOcean CLI (doctl)..."
if command_exists doctl; then
  print_warning "DigitalOcean CLI already installed"
else
  DOCTL_VERSION=$(curl -s https://api.github.com/repos/digitalocean/doctl/releases/latest 2>/dev/null | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
  if [[ -n "$DOCTL_VERSION" ]] && wget -q -O "/tmp/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz" "https://github.com/digitalocean/doctl/releases/download/v${DOCTL_VERSION}/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz"; then
    tar xzf "/tmp/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz" -C /tmp
    sudo mv /tmp/doctl /usr/local/bin/
    rm -f "/tmp/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz"
    print_success "DigitalOcean CLI installed"
  else
    print_warning "Failed to download DigitalOcean CLI"
    rm -f "/tmp/doctl-${DOCTL_VERSION}-linux-${DO_ARCH}.tar.gz" /tmp/doctl
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

print_success "Cloud provider CLI tools installation complete"
