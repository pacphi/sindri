# ==============================================================================
# Sindri v3 Dockerfile
# ==============================================================================
# Multi-stage build for Sindri v3 with Rust CLI
#
# Build modes (priority order):
#   1. Local binary from build context (v3/bin/sindri)
#      - Used by CI workflows that pre-compile the binary
#      - Fastest builds (~2 min)
#
#   2. Pre-compiled binary from releases (BUILD_FROM_SOURCE=false, default)
#      - Downloads binary from GitHub releases (~5 min build)
#      - Smaller build context, faster builds
#      - Production-ready binaries
#
#   3. Build from source (BUILD_FROM_SOURCE=true)
#      - Builds from Rust source code (~8 min build)
#      - Supports custom features, architectures
#      - Development and testing
#
# Key differences from v2:
#   - No bundled extensions (all installed at runtime via sindri CLI)
#   - Removed jq, yq, python3-jsonschema (replaced by Rust CLI)
#   - Smaller base image (~800MB vs v2's 2.5GB)
#   - Faster builds (5-8 min vs v2's 15-20 min)
# ==============================================================================

# ------------------------------------------------------------------------------
# Build Arguments
# ------------------------------------------------------------------------------
ARG UBUNTU_VERSION=24.04
ARG RUST_VERSION=1.75
ARG SINDRI_VERSION=3.0.0
ARG SINDRI_REPO=https://github.com/pacphi/sindri
ARG BUILD_FROM_SOURCE=false

# ==============================================================================
# Stage 1a: Local Binary from Build Context (CI workflows)
# ==============================================================================
# This stage checks for a pre-built binary in the build context (v3/bin/sindri)
# which is provided by CI workflows that compile the binary in a previous job.
# The v3/bin/.gitkeep ensures the directory always exists for COPY to work.
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
# Stage 1b: Build from Source (conditional)
# ==============================================================================
FROM rust:${RUST_VERSION}-slim-bookworm AS builder-source

ARG SINDRI_VERSION
ARG SINDRI_REPO
ARG BUILD_FROM_SOURCE
ARG TARGETARCH

# Only build if BUILD_FROM_SOURCE=true
RUN if [ "$BUILD_FROM_SOURCE" = "true" ]; then \
        apt-get update && \
        apt-get install -y --no-install-recommends \
            git \
            pkg-config \
            libssl-dev \
            ca-certificates && \
        rm -rf /var/lib/apt/lists/* && \
        echo "Building sindri from source for ${TARGETARCH}..." && \
        git clone --depth 1 --branch "${SINDRI_VERSION}" "${SINDRI_REPO}" /tmp/sindri && \
        cd /tmp/sindri/v3 && \
        cargo build --release --bin sindri && \
        mkdir -p /output && \
        cp target/release/sindri /output/sindri && \
        chmod +x /output/sindri; \
    else \
        mkdir -p /output && \
        touch /output/.skip; \
    fi

# ==============================================================================
# Stage 1c: Download Pre-compiled Binary (fallback)
# ==============================================================================
# This stage attempts to download a pre-compiled binary from GitHub releases.
# If the release doesn't exist (e.g., during CI before any release is published),
# it gracefully creates a .skip marker to allow fallback to other build methods.
FROM ubuntu:${UBUNTU_VERSION} AS builder-binary

ARG SINDRI_VERSION
ARG BUILD_FROM_SOURCE
ARG TARGETARCH

# Only download if BUILD_FROM_SOURCE=false
RUN if [ "$BUILD_FROM_SOURCE" != "true" ]; then \
        apt-get update && \
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
            echo "Release v${SINDRI_VERSION} not available, marking as skip (will use local or source build)"; \
            touch /output/.skip; \
        fi; \
    else \
        mkdir -p /output && \
        touch /output/.skip; \
    fi

# ==============================================================================
# Stage 2: Merge Binaries (choose appropriate binary based on priority)
# ==============================================================================
# Priority order:
#   1. Local binary from build context (CI workflows)
#   2. Source build (if BUILD_FROM_SOURCE=true)
#   3. Downloaded binary from releases (default fallback)
FROM ubuntu:${UBUNTU_VERSION} AS binary-merge

ARG BUILD_FROM_SOURCE

COPY --from=builder-local /output /tmp/local
COPY --from=builder-source /output /tmp/source
COPY --from=builder-binary /output /tmp/binary

# Copy the appropriate binary to /output based on priority
RUN mkdir -p /output && \
    if [ -f /tmp/local/sindri ] && [ ! -f /tmp/local/.skip ]; then \
        echo "Using local binary from build context (CI artifact)"; \
        cp /tmp/local/sindri /output/sindri; \
    elif [ "$BUILD_FROM_SOURCE" = "true" ]; then \
        if [ -f /tmp/source/sindri ]; then \
            echo "Using source-built binary"; \
            cp /tmp/source/sindri /output/sindri; \
        else \
            echo "ERROR: Source build failed - binary not found" && exit 1; \
        fi; \
    elif [ -f /tmp/binary/sindri ] && [ ! -f /tmp/binary/.skip ]; then \
        echo "Using downloaded binary from releases"; \
        cp /tmp/binary/sindri /output/sindri; \
    else \
        echo "ERROR: No binary available. Options:" && \
        echo "  1. Provide local binary in v3/bin/sindri (CI workflows)" && \
        echo "  2. Set BUILD_FROM_SOURCE=true to build from source" && \
        echo "  3. Ensure GitHub release v${SINDRI_VERSION:-3.0.0} exists with binaries" && \
        exit 1; \
    fi && \
    chmod +x /output/sindri

# ==============================================================================
# Stage 4: Runtime Image
# ==============================================================================
FROM ubuntu:${UBUNTU_VERSION}

# ------------------------------------------------------------------------------
# Runtime Build Arguments
# ------------------------------------------------------------------------------
ARG DEBIAN_FRONTEND=noninteractive
ARG DEVELOPER_USER=developer
ARG DEVELOPER_UID=1001
ARG DEVELOPER_GID=1001
ARG ALT_HOME=/alt/home/developer
ARG WORKSPACE=${ALT_HOME}/workspace
ARG SSH_PORT=2222

# Set environment variables
ENV DEVELOPER_USER=${DEVELOPER_USER} \
    ALT_HOME=${ALT_HOME} \
    WORKSPACE=${WORKSPACE} \
    SSH_PORT=${SSH_PORT} \
    DEBIAN_FRONTEND=noninteractive \
    TERM=xterm-256color \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

# ------------------------------------------------------------------------------
# Install System Packages
# ------------------------------------------------------------------------------
# Key changes from v2:
# - REMOVED: jq, yq, python3-jsonschema (replaced by Rust CLI)
# - KEPT: git, curl, wget (still needed for cloning, downloads)
# - KEPT: build-essential, libssl-dev (for native extension compilation)
# - KEPT: openssh-server (SSH access)
# - KEPT: gh (GitHub CLI - may become extension in future)
# ------------------------------------------------------------------------------
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        # Core utilities
        ca-certificates \
        curl \
        wget \
        git \
        openssh-server \
        sudo \
        locales \
        tzdata \
        fontconfig \
        # Build tools (for native extensions)
        build-essential \
        pkg-config \
        libssl-dev && \
    # Configure locales
    locale-gen en_US.UTF-8 && \
    update-locale LANG=en_US.UTF-8 && \
    # Cleanup apt cache and remove SSH host keys generated during installation
    # SSH keys will be generated at runtime by entrypoint.sh for security
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
    rm -f /etc/ssh/ssh_host_*

# ------------------------------------------------------------------------------
# Copy Sindri CLI Binary
# ------------------------------------------------------------------------------
COPY --from=binary-merge /output/sindri /usr/local/bin/sindri

# Verify binary is executable and functional
RUN sindri --version

# ------------------------------------------------------------------------------
# Install GitHub CLI from official releases (avoids CVE vulnerabilities)
# ------------------------------------------------------------------------------
ARG GH_VERSION=2.86.0
RUN ARCH=$(dpkg --print-architecture) && \
    wget -qO /tmp/gh.tar.gz "https://github.com/cli/cli/releases/download/v${GH_VERSION}/gh_${GH_VERSION}_linux_${ARCH}.tar.gz" && \
    tar -xzf /tmp/gh.tar.gz -C /tmp && \
    mv /tmp/gh_${GH_VERSION}_linux_${ARCH}/bin/gh /usr/local/bin/ && \
    rm -rf /tmp/gh.tar.gz /tmp/gh_${GH_VERSION}_linux_${ARCH} && \
    gh --version

# ------------------------------------------------------------------------------
# Create Developer User
# ------------------------------------------------------------------------------
RUN groupadd -g ${DEVELOPER_GID} ${DEVELOPER_USER} && \
    useradd -m -u ${DEVELOPER_UID} -g ${DEVELOPER_GID} -s /bin/bash ${DEVELOPER_USER} && \
    echo "${DEVELOPER_USER} ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/${DEVELOPER_USER} && \
    chmod 0440 /etc/sudoers.d/${DEVELOPER_USER}

# ------------------------------------------------------------------------------
# Setup SSH Server
# ------------------------------------------------------------------------------
RUN mkdir -p /run/sshd && \
    mkdir -p /etc/ssh/sshd_config.d

# Copy SSH configuration
COPY v3/docker/config/sshd_config /etc/ssh/sshd_config
COPY v3/docker/config/developer-sudoers /etc/sudoers.d/${DEVELOPER_USER}

RUN chmod 0440 /etc/sudoers.d/${DEVELOPER_USER} && \
    chmod 644 /etc/ssh/sshd_config

# Configure SSH environment for login shells (TMPDIR for Claude Code plugins)
COPY v3/docker/scripts/setup-ssh-environment.sh /tmp/
RUN bash /tmp/setup-ssh-environment.sh && rm /tmp/setup-ssh-environment.sh

# ------------------------------------------------------------------------------
# Install mise (Tool Version Manager)
# ------------------------------------------------------------------------------
# mise is critical for managing language runtimes installed via extensions
# (node, python, ruby, go, etc.)
COPY v3/docker/scripts/install-mise.sh /tmp/
RUN bash /tmp/install-mise.sh && rm /tmp/install-mise.sh

# ------------------------------------------------------------------------------
# Install Nerd Fonts (for Starship icons/symbols)
# ------------------------------------------------------------------------------
COPY v3/docker/scripts/install-nerd-fonts.sh /tmp/
RUN bash /tmp/install-nerd-fonts.sh && rm /tmp/install-nerd-fonts.sh

# ------------------------------------------------------------------------------
# Install Starship (Optional Shell Prompt)
# ------------------------------------------------------------------------------
COPY v3/docker/scripts/install-starship.sh /tmp/
RUN bash /tmp/install-starship.sh && rm /tmp/install-starship.sh

# ------------------------------------------------------------------------------
# Setup Directories and Permissions
# ------------------------------------------------------------------------------
RUN mkdir -p /docker/scripts /docker/config && \
    mkdir -p ${ALT_HOME} ${WORKSPACE} && \
    chown -R ${DEVELOPER_USER}:${DEVELOPER_USER} ${ALT_HOME} && \
    chmod 755 /docker /docker/scripts /docker/config

# ------------------------------------------------------------------------------
# Copy Entrypoint and Support Scripts
# ------------------------------------------------------------------------------
COPY v3/docker/scripts/entrypoint.sh /docker/scripts/
COPY v3/docker/scripts/healthcheck.sh /docker/scripts/
COPY v3/docker/scripts/setup-motd.sh /tmp/

RUN chmod +x /docker/scripts/entrypoint.sh && \
    chmod +x /docker/scripts/healthcheck.sh && \
    bash /tmp/setup-motd.sh && \
    rm /tmp/setup-motd.sh

# ------------------------------------------------------------------------------
# Runtime Configuration
# ------------------------------------------------------------------------------
WORKDIR ${WORKSPACE}
EXPOSE ${SSH_PORT}

# Comprehensive health check
# - Checks SSH daemon
# - Checks extension installation status
# - Checks critical directories
# - Checks Sindri CLI functionality
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD /docker/scripts/healthcheck.sh > /dev/null 2>&1 || exit 1

# Run entrypoint as root (it will start SSH daemon and manage permissions)
ENTRYPOINT ["/docker/scripts/entrypoint.sh"]
