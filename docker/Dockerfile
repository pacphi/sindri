# Sindri - Optimized Multi-Stage Build
FROM ubuntu:24.04 AS base

# Set environment
ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    WORKSPACE=/workspace \
    DOCKER_LIB=/docker/lib \
    MISE_DATA_DIR="/workspace/.local/share/mise" \
    MISE_CONFIG_DIR="/workspace/.config/mise" \
    MISE_CACHE_DIR="/workspace/.cache/mise" \
    MISE_STATE_DIR="/workspace/.local/state/mise" \
    PATH="/docker/cli:/workspace/bin:/workspace/.local/share/mise/shims:$PATH"

# Create developer user
RUN useradd -m -s /bin/bash -u 1001 developer && \
    mkdir -p /workspace && \
    chown -R developer:developer /workspace

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    wget \
    git \
    ca-certificates \
    gnupg \
    lsb-release \
    sudo \
    jq \
    unzip \
    zip \
    && rm -rf /var/lib/apt/lists/*

# Install yq for YAML parsing
RUN wget -qO /usr/local/bin/yq https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 && \
    chmod +x /usr/local/bin/yq

# Install mise (tool version manager) and pre-install runtimes
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN curl https://mise.run | sh && \
    mv ~/.local/bin/mise /usr/local/bin/mise && \
    chmod +x /usr/local/bin/mise && \
    mise use -g node@lts python@3.13

# Copy extension system and CLI tools
COPY docker/lib /docker/lib
COPY docker/scripts /docker/scripts
COPY cli /docker/cli

# Set up entrypoint and CLI tools
COPY docker/scripts/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh && \
    chmod -R +r /docker/lib && \
    find /docker/lib -type f -exec chmod 644 {} \; && \
    find /docker/lib -type d -exec chmod 755 {} \; && \
    find /docker/scripts -type f -name "*.sh" -exec chmod 755 {} \; && \
    find /docker/cli -type f -exec chmod 755 {} \;

# Allow developer sudo access (for apt installations)
RUN echo "developer ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Switch to developer user
USER developer
WORKDIR /workspace

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/bin/bash"]
