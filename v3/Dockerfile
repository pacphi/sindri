# syntax=docker/dockerfile:1.4
# ==============================================================================
# Sindri v3 Dockerfile (Production) — Multi-Distro
# ==============================================================================
# Multi-stage build for Sindri v3 with pre-compiled binary
#
# Supported distros:
#   - ubuntu   (default) — apt-get based
#   - fedora             — dnf based
#   - opensuse           — zypper based
#
# Build modes (priority order):
#   1. Local binary from build context (v3/bin/sindri)
#      - Used by CI workflows that pre-compile the binary
#      - Fastest builds (~2 min)
#
#   2. Pre-compiled binary from releases (default)
#      - Downloads binary from GitHub releases (~5 min build)
#      - Smaller build context, faster builds
#      - Production-ready binaries
#
# Key differences from Dockerfile.dev:
#   - Uses pre-compiled binaries (no source build)
#   - NO bundled extensions (installed at runtime via sindri CLI)
#   - Smaller image (~800MB vs dev's ~1.2GB)
#   - Faster builds (2-5 min vs dev's ~8 min)
#   - Extensions installed to ${HOME}/.sindri/extensions (respects ALT_HOME)
#
# Layer optimizations:
#   - Layers ordered by change frequency (least → most frequent)
#   - Consolidated COPY + RUN operations for scripts
#   - Multi-stage GH CLI + Cosign download (parallel build, isolated caching)
#   - Builder/downloader stages stay on Ubuntu (distro-independent file ops)
#   - Only the runtime stage is multi-distro
#
# Build:
#   docker build -f v3/Dockerfile -t sindri:v3.0.0 .
#   docker build -f v3/Dockerfile --build-arg DISTRO=fedora -t sindri:v3.0.0-fedora .
#   docker build -f v3/Dockerfile --build-arg DISTRO=opensuse -t sindri:v3.0.0-opensuse .
# ==============================================================================

# ------------------------------------------------------------------------------
# Build Arguments
# ------------------------------------------------------------------------------
ARG DISTRO=ubuntu
ARG UBUNTU_VERSION=24.04
ARG FEDORA_VERSION=41
ARG OPENSUSE_VERSION=15.6
ARG SINDRI_VERSION=3.0.0
ARG GH_VERSION=2.88.1
ARG COSIGN_VERSION=3.0.5

# ==============================================================================
# Runtime Distro Selection
# ==============================================================================
# Define one stage per supported distro, then select via DISTRO build arg.
# BuildKit resolves the variable in FROM base-${DISTRO}.
# Only the selected distro image is pulled/built (BuildKit optimization).
FROM ubuntu:${UBUNTU_VERSION} AS base-ubuntu
FROM fedora:${FEDORA_VERSION} AS base-fedora
FROM opensuse/leap:${OPENSUSE_VERSION} AS base-opensuse

# ==============================================================================
# Stage 1a: Local Binary from Build Context (CI workflows)
# ==============================================================================
# This stage checks for a pre-built binary in the build context (v3/bin/sindri)
# which is provided by CI workflows that compile the binary in a previous job.
# The v3/bin/.gitkeep ensures the directory always exists for COPY to work.
# Stays on Ubuntu — only does file operations (distro-independent).
FROM ubuntu:${UBUNTU_VERSION} AS builder-local

# Create output directory
RUN mkdir -p /output && touch /output/.skip

# Copy entire v3/bin directory contents (always succeeds due to .gitkeep)
COPY v3/bin/ /tmp/local-bin/

# Check if binary was copied and set appropriate marker
RUN if [ -f /tmp/local-bin/sindri ]; then \
        cp /tmp/local-bin/sindri /output/sindri && \
        chmod +x /output/sindri && \
        rm /output/.skip && \
        echo "LOCAL_BINARY=true: Using local binary from build context"; \
    else \
        echo "LOCAL_BINARY=false: No local binary found, will use alternative source"; \
    fi

# ==============================================================================
# Stage 1b: Download Pre-compiled Binary (fallback)
# ==============================================================================
# This stage attempts to download a pre-compiled binary from GitHub releases.
# If the release doesn't exist (e.g., during CI before any release is published),
# it gracefully creates a .skip marker to allow fallback to local binary.
# Stays on Ubuntu — only does downloads (distro-independent).
FROM ubuntu:${UBUNTU_VERSION} AS builder-binary

ARG SINDRI_VERSION
ARG TARGETARCH

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        wget \
        ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /output && \
    # Determine architecture for binary download
    case "${TARGETARCH}" in \
        amd64) ARCH_SUFFIX="x86_64" ;; \
        arm64) ARCH_SUFFIX="aarch64" ;; \
        *) echo "Unsupported architecture: ${TARGETARCH}" && exit 1 ;; \
    esac && \
    echo "Attempting to download sindri binary for linux-${ARCH_SUFFIX}..." && \
    if wget -qO /tmp/sindri.tar.gz \
        "https://github.com/pacphi/sindri/releases/download/v${SINDRI_VERSION}/sindri-v${SINDRI_VERSION}-linux-${ARCH_SUFFIX}.tar.gz" 2>/dev/null; then \
        tar -xzf /tmp/sindri.tar.gz -C /output && \
        chmod +x /output/sindri && \
        rm /tmp/sindri.tar.gz && \
        echo "Successfully downloaded binary from releases"; \
    else \
        echo "Release v${SINDRI_VERSION} not available, marking as skip (will use local build)"; \
        touch /output/.skip; \
    fi

# ==============================================================================
# Stage 2: Merge Binaries (choose appropriate binary based on priority)
# ==============================================================================
# Priority order:
#   1. Local binary from build context (CI workflows)
#   2. Downloaded binary from releases (default fallback)
# Stays on Ubuntu — only does file operations (distro-independent).
FROM ubuntu:${UBUNTU_VERSION} AS binary-merge

ARG SINDRI_VERSION

COPY --from=builder-local /output /tmp/local
COPY --from=builder-binary /output /tmp/binary

# Copy the appropriate binary to /output based on priority
RUN mkdir -p /output && \
    if [ -f /tmp/local/sindri ] && [ ! -f /tmp/local/.skip ]; then \
        echo "Using local binary from build context (CI artifact)"; \
        cp /tmp/local/sindri /output/sindri; \
    elif [ -f /tmp/binary/sindri ] && [ ! -f /tmp/binary/.skip ]; then \
        echo "Using downloaded binary from releases"; \
        cp /tmp/binary/sindri /output/sindri; \
    else \
        echo "ERROR: No binary available. Options:" && \
        echo "  1. Provide local binary in v3/bin/sindri (CI workflows)" && \
        echo "  2. Ensure GitHub release v${SINDRI_VERSION:-3.0.0} exists with binaries" && \
        echo "  3. Use Dockerfile.dev to build from source" && \
        exit 1; \
    fi && \
    chmod +x /output/sindri

# ==============================================================================
# Stage 3: Download GitHub CLI (parallel build, isolated caching)
# ==============================================================================
# Stays on Ubuntu — only does downloads (distro-independent).
FROM ubuntu:${UBUNTU_VERSION} AS gh-downloader

ARG GH_VERSION

RUN apt-get update && \
    apt-get install -y --no-install-recommends wget ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    ARCH=$(dpkg --print-architecture) && \
    wget -qO /tmp/gh.tar.gz "https://github.com/cli/cli/releases/download/v${GH_VERSION}/gh_${GH_VERSION}_linux_${ARCH}.tar.gz" && \
    tar -xzf /tmp/gh.tar.gz -C /tmp && \
    mv /tmp/gh_${GH_VERSION}_linux_${ARCH}/bin/gh /gh && \
    chmod +x /gh

# ==============================================================================
# Stage 3b: Download Cosign (parallel build, isolated caching)
# ==============================================================================
# Stays on Ubuntu — only does downloads (distro-independent).
FROM ubuntu:${UBUNTU_VERSION} AS cosign-downloader

ARG COSIGN_VERSION
ARG TARGETARCH

RUN apt-get update && \
    apt-get install -y --no-install-recommends wget ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    echo "Downloading cosign for linux-${TARGETARCH}..." && \
    wget -qO /cosign "https://github.com/sigstore/cosign/releases/download/v${COSIGN_VERSION}/cosign-linux-${TARGETARCH}" && \
    chmod +x /cosign && \
    echo "Cosign ${COSIGN_VERSION} installed for ${TARGETARCH}"

# ==============================================================================
# Stage 4: Runtime Image (Multi-Distro)
# ==============================================================================
# Uses the selected distro via BuildKit variable FROM resolution.
# Only the chosen base-${DISTRO} image is pulled (BuildKit optimization).
FROM base-${DISTRO} AS runtime

# Re-declare DISTRO after FROM (ARGs are scoped per stage)
ARG DISTRO

# ------------------------------------------------------------------------------
# Runtime Build Arguments
# ------------------------------------------------------------------------------
ARG DEVELOPER_USER=developer
ARG DEVELOPER_UID=1001
ARG DEVELOPER_GID=1001
ARG ALT_HOME=/alt/home/developer
ARG WORKSPACE=${ALT_HOME}/workspace
ARG SSH_PORT=2222

# ------------------------------------------------------------------------------
# Environment Variables (rarely change)
# ------------------------------------------------------------------------------
ENV DEVELOPER_USER=${DEVELOPER_USER} \
    ALT_HOME=${ALT_HOME} \
    WORKSPACE=${WORKSPACE} \
    SSH_PORT=${SSH_PORT} \
    TERM=xterm-256color \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

# ------------------------------------------------------------------------------
# Install System Packages via pkg-manager.sh (multi-distro aware)
# ------------------------------------------------------------------------------
# Uses pkg-manager.sh to install the correct packages for the selected distro.
# Replaces the inline apt-get block — supports Ubuntu, Fedora, and openSUSE.
# ------------------------------------------------------------------------------
COPY v3/docker/lib/pkg-manager.sh /docker/lib/pkg-manager.sh
SHELL ["/bin/bash", "-c"]
RUN chmod +x /docker/lib/pkg-manager.sh && \
    export SINDRI_DISTRO=${DISTRO} && \
    . /docker/lib/pkg-manager.sh && \
    pkg_install_sindri_base

# ------------------------------------------------------------------------------
# Create Developer User via pkg-manager.sh (multi-distro aware)
# ------------------------------------------------------------------------------
RUN export SINDRI_DISTRO=${DISTRO} && \
    . /docker/lib/pkg-manager.sh && \
    create_developer_user "${DEVELOPER_USER}" "${DEVELOPER_UID}" "${DEVELOPER_GID}" "${ALT_HOME}"
SHELL ["/bin/sh", "-c"]

# ------------------------------------------------------------------------------
# Setup SSH Server and Configuration (changes infrequently)
# ------------------------------------------------------------------------------
COPY v3/docker/config/sshd_config /etc/ssh/sshd_config
COPY v3/docker/config/developer-sudoers /etc/sudoers.d/developer

RUN mkdir -p /run/sshd /etc/ssh/sshd_config.d && \
    chmod 0440 /etc/sudoers.d/developer && \
    chmod 644 /etc/ssh/sshd_config

# ------------------------------------------------------------------------------
# Copy GitHub CLI Binary (cached independently via multi-stage)
# ------------------------------------------------------------------------------
COPY --from=gh-downloader /gh /usr/local/bin/gh
RUN gh --version

# ------------------------------------------------------------------------------
# Copy Cosign Binary (cached independently via multi-stage)
# ------------------------------------------------------------------------------
COPY --from=cosign-downloader /cosign /usr/local/bin/cosign
RUN cosign version

# ------------------------------------------------------------------------------
# Install Tools: All setup scripts in single layer (changes occasionally)
# ------------------------------------------------------------------------------
# Consolidates 12 layers → 2 layers for mise, fonts, starship, SSH env, MOTD
COPY v3/docker/scripts/setup-ssh-environment.sh \
     v3/docker/scripts/install-mise.sh \
     v3/docker/scripts/install-nerd-fonts.sh \
     v3/docker/scripts/install-starship.sh \
     v3/docker/scripts/setup-motd.sh \
     v3/docker/scripts/check-extension-status.sh \
     v3/docker/scripts/show-motd.sh \
     /tmp/scripts/

RUN --mount=type=secret,id=github_token <<EOF
export GITHUB_TOKEN=$(cat /run/secrets/github_token 2>/dev/null || true)
bash /tmp/scripts/setup-ssh-environment.sh
bash /tmp/scripts/install-mise.sh
bash /tmp/scripts/install-nerd-fonts.sh
bash /tmp/scripts/install-starship.sh
bash /tmp/scripts/setup-motd.sh
mkdir -p /etc/profile.d
cp /tmp/scripts/show-motd.sh /etc/profile.d/00-show-motd.sh
chmod +x /etc/profile.d/00-show-motd.sh
cp /tmp/scripts/check-extension-status.sh /etc/profile.d/sindri-extension-status.sh
chmod +x /etc/profile.d/sindri-extension-status.sh
rm -rf /tmp/scripts
EOF

# ------------------------------------------------------------------------------
# Setup Directories and Permissions
# ------------------------------------------------------------------------------
RUN mkdir -p /docker/scripts /docker/config && \
    mkdir -p ${ALT_HOME} ${WORKSPACE} && \
    chown -R ${DEVELOPER_USER}:${DEVELOPER_USER} ${ALT_HOME} && \
    chmod 755 /docker /docker/scripts /docker/config

# ------------------------------------------------------------------------------
# Copy Sindri CLI Binary (changes frequently - placed late for better caching)
# ------------------------------------------------------------------------------
COPY --from=binary-merge /output/sindri /usr/local/bin/sindri
RUN sindri --version

# ------------------------------------------------------------------------------
# Extension Configuration (Production Mode)
# ------------------------------------------------------------------------------
# In production mode, extensions are NOT bundled with the image.
# Instead, they are installed at runtime to ${HOME}/.sindri/extensions
# which resolves to /alt/home/developer/.sindri/extensions in the container.
#
# This approach:
# - Reduces image size (~800MB vs ~1.2GB for dev image)
# - Allows extensions to be updated independently of the image
# - Respects volume-mounted home directory (ALT_HOME=/alt/home/developer)
# - Extensions installed via: sindri extension install <name>
#
# Set extension home to the volume-mounted path (absolute, not ${HOME} which varies)
ENV SINDRI_EXT_HOME=/alt/home/developer/.sindri/extensions

# Store Sindri support files outside volume mount (/docker/config/sindri)
# When /alt/home/developer is volume-mounted (Fly.io, K8s, Docker volumes),
# files copied here during build are hidden by the mount. Store in /docker/config/sindri
# for entrypoint.sh to copy to the volume on first boot, or for CLI to fetch from GitHub.
#
# This enables:
# 1. First-boot initialization from bundled files (offline fallback)
# 2. Runtime updates from GitHub (zero-downtime upgrades)
# 3. Version-matched support files (automatically updated when CLI version changes)
RUN mkdir -p /docker/config/sindri
COPY v3/compatibility-matrix.yaml /docker/config/sindri/compatibility-matrix.yaml
COPY v3/extension-source.yaml /docker/config/sindri/extension-source.yaml
COPY v3/common.sh /docker/config/sindri/common.sh
# ------------------------------------------------------------------------------

# ------------------------------------------------------------------------------
# Copy Entrypoint and Support Scripts (changes occasionally)
# ------------------------------------------------------------------------------
COPY v3/docker/scripts/entrypoint.sh /docker/scripts/
COPY v3/docker/scripts/healthcheck.sh /docker/scripts/
COPY v3/docker/templates/ /docker/templates/

RUN chmod +x /docker/scripts/entrypoint.sh /docker/scripts/healthcheck.sh

# ------------------------------------------------------------------------------
# Labels
# ------------------------------------------------------------------------------
LABEL sindri.distro="${DISTRO}"

# ------------------------------------------------------------------------------
# Runtime Configuration
# ------------------------------------------------------------------------------
# Use /tmp as WORKDIR (like v2) instead of ${WORKSPACE} which is on the volume.
# On first boot, the volume isn't formatted yet, so /alt/home/developer/workspace
# doesn't exist and causes container crashes. The entrypoint.sh will cd to
# $WORKSPACE after the volume is mounted and ready.
WORKDIR /tmp
EXPOSE ${SSH_PORT}

# Health check - verifies SSH daemon is running
# - Simplified approach using pgrep (v2 pattern)
# - Increased start-period to 120s for first boot on Fly.io with volume initialization
# - Increased retries to 5 for better tolerance during startup
HEALTHCHECK --interval=30s --timeout=10s --start-period=120s --retries=5 \
    CMD /docker/scripts/healthcheck.sh > /dev/null 2>&1 || exit 1

# Run entrypoint as root (it will start SSH daemon and manage permissions)
ENTRYPOINT ["/docker/scripts/entrypoint.sh"]
