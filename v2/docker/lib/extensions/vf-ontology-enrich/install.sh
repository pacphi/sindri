#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-ontology-enrich
# VisionFlow capability: AI-powered ontology enrichment

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-ontology-enrich"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-ontology-enrich/resources"

print_status "Installing Ontology Enrichment tools..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install Python dependencies for OWL2/RDF
pip install --user \
    rdflib \
    owlrl \
    requests \
    httpx \
    pydantic

# Make scripts executable
if [[ -d "${EXTENSION_DIR}/scripts" ]]; then
    chmod +x "${EXTENSION_DIR}/scripts"/*.py 2>/dev/null || true
fi

print_success "vf-ontology-enrich installed successfully"
print_warning "Requires PERPLEXITY_API_KEY for enrichment"
