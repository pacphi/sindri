#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-jupyter-notebooks
# VisionFlow capability: Jupyter notebook execution MCP

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-jupyter-notebooks"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-jupyter-notebooks/resources"

print_status "Installing Jupyter Notebooks MCP server..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Jupyter and MCP dependencies
pip install --user jupyter jupyterlab notebook ipykernel mcp pydantic

# Install kernel
python3 -m ipykernel install --user --name python3

print_success "vf-jupyter-notebooks installed successfully"
