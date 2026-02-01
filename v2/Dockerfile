# Sindri Development Environment
# Provider-agnostic cloud dev environment with extensible tooling
# Optimized with multi-stage builds and aggressive cache cleanup

# =============================================================================
# Stage 1: Builder - Download and prepare external tools
# =============================================================================
FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Install minimal build tools in builder stage
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    wget \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Download yq with pinned version for reproducibility and security tracking
# NOTE: Version must use Go 1.25.6+ to fix CVE-2025-61728 (archive/zip DoS)
ARG YQ_VERSION=4.52.2
RUN ARCH=$(dpkg --print-architecture) && \
    wget -qO /tmp/yq "https://github.com/mikefarah/yq/releases/download/v${YQ_VERSION}/yq_linux_${ARCH}" && \
    chmod +x /tmp/yq

# =============================================================================
# Stage 2: Final - Production image
# =============================================================================
FROM ubuntu:24.04

LABEL org.opencontainers.image.title="Sindri Development Environment"
LABEL org.opencontainers.image.description="Provider-agnostic cloud dev environment with extensible tooling and pre-configured runtimes"
LABEL org.opencontainers.image.vendor="Sindri"

# Define the alternate home path for volume mounting
# This allows the entire home directory to be on a persistent volume
ARG ALT_HOME=/alt/home/developer

# Set environment variables
# Note: HOME will be reset to ALT_HOME at runtime via entrypoint
# Note: MISE_* vars are set at runtime to user's home (on persistent volume)
# MISE_YES=1 and MISE_TRUSTED_CONFIG_PATHS are baked in to ensure docker exec bash -c works
ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    ALT_HOME=${ALT_HOME} \
    WORKSPACE=${ALT_HOME}/workspace \
    DOCKER_LIB=/docker/lib \
    SSH_PORT=2222 \
    MISE_YES=1 \
    MISE_TRUSTED_CONFIG_PATHS="${ALT_HOME}/.config/mise:${ALT_HOME}/.config/mise/conf.d" \
    PATH="/docker/cli:${ALT_HOME}/workspace/bin:${ALT_HOME}/.local/share/mise/shims:/usr/local/bin:$PATH"

# Create developer user (without home directory - it's on the volume)
# Use -M to skip home directory creation during build
# Use -d to set home to ALT_HOME (critical for DevPod/devcontainer compatibility)
# Home directory will be initialized at runtime on persistent volume
RUN useradd -M -d ${ALT_HOME} -s /bin/bash -u 1001 -G sudo developer && \
    mkdir -p ${ALT_HOME} && \
    chown developer:developer ${ALT_HOME}

# Install system dependencies in a single layer with aggressive cleanup
# This is the key optimization - all apt operations in one RUN command
RUN apt-get update && apt-get install -y --no-install-recommends \
    bind9-dnsutils \
    build-essential \
    ca-certificates \
    curl \
    direnv \
    fontconfig \
    gettext-base \
    git \
    gnupg \
    iputils-ping \
    jq \
    libreadline-dev \
    libssl-dev \
    libyaml-dev \
    nano \
    net-tools \
    netcat-openbsd \
    openssh-server \
    pkg-config \
    postgresql-client \
    python3-jsonschema \
    redis-tools \
    rsync \
    screen \
    software-properties-common \
    sqlite3 \
    sudo \
    telnet \
    tree \
    unzip \
    vim \
    wget \
    zip \
    zlib1g-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /var/cache/apt/archives/* \
    && rm -f /etc/ssh/ssh_host_*

# Copy yq from builder stage
COPY --from=builder /tmp/yq /usr/local/bin/yq

# Install GitHub CLI from official releases (avoids CVE vulnerabilities in apt version)
ARG GH_VERSION=2.86.0
RUN ARCH=$(dpkg --print-architecture) && \
    wget -qO /tmp/gh.tar.gz "https://github.com/cli/cli/releases/download/v${GH_VERSION}/gh_${GH_VERSION}_linux_${ARCH}.tar.gz" && \
    tar -xzf /tmp/gh.tar.gz -C /tmp && \
    mv /tmp/gh_${GH_VERSION}_linux_${ARCH}/bin/gh /usr/local/bin/ && \
    rm -rf /tmp/gh.tar.gz /tmp/gh_${GH_VERSION}_linux_${ARCH} && \
    gh --version

# Copy extension system, CLI tools, and configurations
# Note: Paths use v2/ prefix so build context can be repo root (consistent with v3)
COPY v2/docker/ /docker/
COPY v2/cli /docker/cli
COPY v2/deploy /docker/deploy

# Install mise (tool version manager) binary only
# Tools are installed by users via extensions at runtime (stored on persistent volume)
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN /docker/scripts/install-mise.sh && \
    rm -rf /tmp/* /var/tmp/*

# Install Claude Code CLI system-wide
RUN /docker/scripts/install-claude.sh && \
    rm -rf /tmp/* /var/tmp/*

# Install Nerd Fonts (prerequisite for starship)
RUN /docker/scripts/install-nerd-fonts.sh && \
    rm -rf /tmp/* /var/tmp/*

# Install starship (shell prompt) system-wide
RUN /docker/scripts/install-starship.sh && \
    rm -rf /tmp/* /var/tmp/*

# Set permissions for scripts and CLI tools
# Note: Scripts use 755 (not 750) to allow developer user execution
# See M-2 in security audit for rationale
RUN chmod -R +r /docker/lib && \
    find /docker/lib -type f -exec chmod 644 {} \; && \
    find /docker/lib -type d -exec chmod 755 {} \; && \
    find /docker/scripts -type f -name "*.sh" -exec chmod 755 {} \; && \
    find /docker/cli -type f -exec chmod 755 {} \; && \
    find /docker/config -type f -exec chmod 644 {} \;

# Configure SSH daemon
# - Copy secure sshd_config (port 2222 to avoid Fly.io hallpass conflicts)
# - Setup sudoers for developer user
# - Create sshd runtime directory
RUN cp /docker/config/sshd_config /etc/ssh/sshd_config && \
    cp /docker/config/developer-sudoers /etc/sudoers.d/developer && \
    chmod 440 /etc/sudoers.d/developer && \
    mkdir -p /var/run/sshd && \
    chmod 755 /var/run/sshd

# Configure SSH environment for non-interactive sessions (CI/CD support)
# This ensures BASH_ENV is set so SSH commands get full environment
RUN /docker/scripts/setup-ssh-environment.sh

# Create welcome script in /etc/skel for first-login message
RUN /docker/scripts/create-welcome.sh

# Create npm config in /etc/skel to suppress misleading registry notices
# See: https://github.com/npm/cli/issues/8816
RUN /docker/scripts/create-npmrc.sh

# Setup MOTD banner
RUN /docker/scripts/setup-motd.sh

# Setup installation status banner (shown on SSH login during extension install)
RUN /docker/scripts/setup-install-status-banner.sh

# Final cleanup to minimize image size
RUN apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /var/cache/apt/archives/* \
    /root/.cache /root/.npm /root/.local

# Expose SSH port (internal port 2222)
EXPOSE 2222

# Health check for SSH service (CI_MODE aware)
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD /docker/scripts/health-check.sh

# Working directory is set to /tmp initially
# The entrypoint will cd to workspace after creating it on the volume
# This avoids issues with WORKDIR pointing to a non-existent path on empty volumes
WORKDIR /tmp

# Entrypoint runs as root to:
# 1. Initialize home directory on volume
# 2. Set proper permissions
# 3. Start SSH daemon (requires root) OR execute passed command
# Note: SSH sessions run as developer user
ENTRYPOINT ["/docker/scripts/entrypoint.sh"]
CMD []
