# Sindri - Optimized Multi-Stage Build
FROM ubuntu:24.04 AS base

# Define the alternate home path for volume mounting
# This allows the entire home directory to be on a persistent volume
ARG ALT_HOME=/alt/home/developer

# Set environment
ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    ALT_HOME=${ALT_HOME} \
    HOME=${ALT_HOME} \
    WORKSPACE=${ALT_HOME}/workspace \
    DOCKER_LIB=/docker/lib \
    MISE_DATA_DIR="${ALT_HOME}/.local/share/mise" \
    MISE_CONFIG_DIR="${ALT_HOME}/.config/mise" \
    MISE_CACHE_DIR="${ALT_HOME}/.cache/mise" \
    MISE_STATE_DIR="${ALT_HOME}/.local/state/mise" \
    PATH="/docker/cli:${ALT_HOME}/workspace/bin:${ALT_HOME}/.local/share/mise/shims:$PATH"

# Create developer user with alternate home directory
RUN useradd -m -s /bin/bash -u 1001 -d ${ALT_HOME} developer && \
    mkdir -p ${ALT_HOME}/workspace && \
    chown -R developer:developer ${ALT_HOME}

# Install system dependencies
RUN apt-get update && apt-get install -y \
    bind9-dnsutils \
    build-essential \
    ca-certificates \
    curl \
    direnv \
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
WORKDIR ${WORKSPACE}

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/bin/bash"]
