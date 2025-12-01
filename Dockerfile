# Sindri Development Environment
# Provider-agnostic cloud dev environment with extensible tooling
FROM ubuntu:24.04

LABEL org.opencontainers.image.title="Sindri Development Environment"
LABEL org.opencontainers.image.description="Provider-agnostic cloud dev environment with extensible tooling and pre-configured runtimes"
LABEL org.opencontainers.image.vendor="Sindri"

# Define the alternate home path for volume mounting
# This allows the entire home directory to be on a persistent volume
ARG ALT_HOME=/alt/home/developer

# Set environment variables
# Note: HOME will be reset to ALT_HOME at runtime via entrypoint
ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    ALT_HOME=${ALT_HOME} \
    WORKSPACE=${ALT_HOME}/workspace \
    DOCKER_LIB=/docker/lib \
    SSH_PORT=2222 \
    PATH="/docker/cli:${ALT_HOME}/workspace/bin:${ALT_HOME}/.local/share/mise/shims:/usr/local/bin:$PATH"

# Create developer user (without home directory - it's on the volume)
# Use -M to skip home directory creation during build
# Home directory will be initialized at runtime on persistent volume
RUN useradd -M -s /bin/bash -u 1001 -G sudo developer && \
    mkdir -p ${ALT_HOME} && \
    chown developer:developer ${ALT_HOME}

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

# Install GitHub CLI
RUN mkdir -p -m 755 /etc/apt/keyrings && \
    wget -qO- https://cli.github.com/packages/githubcli-archive-keyring.gpg | tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null && \
    chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null && \
    apt-get update && \
    apt-get install -y gh

# Clean up to reduce layer size
RUN apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Copy scripts first so we can use them for installation
COPY docker/scripts/install-mise.sh /docker/scripts/install-mise.sh
COPY docker/scripts/install-claude.sh /docker/scripts/install-claude.sh
RUN chmod +x /docker/scripts/install-mise.sh /docker/scripts/install-claude.sh

# Install mise (tool version manager) system-wide with default tools
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN /docker/scripts/install-mise.sh --with-tools

# Install Claude Code CLI system-wide
RUN /docker/scripts/install-claude.sh

# Copy extension system, CLI tools, and configurations
COPY docker/ /docker/
COPY cli /docker/cli
COPY deploy /docker/deploy

# Set permissions for scripts and CLI tools
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

# Add mise activation to /etc/skel/.bashrc (template for new home directories)
RUN echo '' >> /etc/skel/.bashrc && \
    echo '# mise - unified tool version manager' >> /etc/skel/.bashrc && \
    echo 'if command -v mise >/dev/null 2>&1; then' >> /etc/skel/.bashrc && \
    echo '    eval "$(mise activate bash)"' >> /etc/skel/.bashrc && \
    echo 'fi' >> /etc/skel/.bashrc

# Create welcome script in /etc/skel for first-login message
RUN /docker/scripts/create-welcome.sh

# Expose SSH port (internal port 2222)
EXPOSE 2222

# Health check for SSH service (CI_MODE aware)
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD /docker/scripts/health-check.sh

# Set working directory (will be created on volume at runtime)
WORKDIR ${ALT_HOME}/workspace

# Entrypoint runs as root to:
# 1. Initialize home directory on volume
# 2. Set proper permissions
# 3. Start SSH daemon (requires root)
# Note: SSH sessions run as developer user
CMD ["/docker/scripts/entrypoint.sh"]
