#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-import-to-ontology
# VisionFlow capability: Document to ontology import

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-import-to-ontology"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-import-to-ontology/resources"

print_status "Installing Import to Ontology tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python dependencies
pip install --user \
    rdflib \
    pyyaml \
    beautifulsoup4 \
    pdfplumber \
    python-docx

# Make scripts executable
if [[ -d "${EXTENSION_DIR}/scripts" ]]; then
    chmod +x "${EXTENSION_DIR}/scripts"/*.py 2>/dev/null || true
fi

print_success "vf-import-to-ontology installed successfully"
