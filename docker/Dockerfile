# Sindri - Optimized Multi-Stage Build
FROM ubuntu:24.04 AS base

# Set environment
ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    WORKSPACE=/workspace \
    DOCKER_LIB=/docker/lib

# Create developer user
RUN useradd -m -s /bin/bash -u 1000 developer && \
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

# Install mise (tool version manager)
RUN curl https://mise.run | sh && \
    mv ~/.local/bin/mise /usr/local/bin/mise && \
    chmod +x /usr/local/bin/mise

# Pre-install common runtimes (as root, will be activated for developer)
RUN mise use -g node@lts python@3.13

# Copy extension system
COPY docker/lib /docker/lib
COPY docker/scripts /docker/scripts

# Set up entrypoint
COPY docker/scripts/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh /docker/scripts/*.sh

# Allow developer sudo access (for apt installations)
RUN echo "developer ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Switch to developer user
USER developer
WORKDIR /workspace

# Activate mise for developer
RUN echo 'eval "$(mise activate bash)"' >> ~/.bashrc && \
    echo 'export PATH="${HOME}/.local/bin:${PATH}"' >> ~/.bashrc

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/bin/bash"]
