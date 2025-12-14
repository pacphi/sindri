#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-latex-documents
# VisionFlow capability: TeX Live with BibTeX and Beamer

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/vf-latex-documents"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-latex-documents/resources"

print_status "Installing TeX Live (this may take a while)..."

# Install TeX Live base with essential packages
sudo apt-get update -qq
sudo apt-get install -y -qq \
    texlive-base \
    texlive-latex-base \
    texlive-latex-recommended \
    texlive-latex-extra \
    texlive-fonts-recommended \
    texlive-bibtex-extra \
    biber \
    latexmk

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (templates, themes, examples)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

print_success "vf-latex-documents installed successfully"
print_status "Available: pdflatex, bibtex, biber, latexmk"
print_status "Templates available in: ${EXTENSION_DIR}/templates/"
