#!/bin/bash
# Volume initialization for Kubernetes

set -e

# This script runs in init container to prepare volume
echo "Initializing volume..."

# Create directory structure
mkdir -p /workspace/{projects,config,scripts,bin,.local,.config}
mkdir -p /workspace/.system/{manifest,installed,logs}

# Set permissions
chown -R 1001:1001 /workspace

echo "Volume initialized"
