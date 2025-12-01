#!/bin/bash
# ==============================================================================
# Sindri Container Entrypoint
# ==============================================================================
# Initializes and starts the Sindri development environment container.
# Runs as root to set up volumes, permissions, and start SSH daemon.
# SSH sessions run as the developer user.
#
# Environment Variables:
#   CI_MODE         - Set to "true" to skip SSH daemon (use flyctl ssh console)
#   AUTHORIZED_KEYS - SSH public keys for authentication
#   GIT_USER_NAME   - Git user.name configuration
#   GIT_USER_EMAIL  - Git user.email configuration
#   GITHUB_TOKEN    - GitHub token for git credential helper
#   SSH_PORT        - SSH daemon port (default: 2222)
# ==============================================================================

set -e

# ==============================================================================
# Environment Configuration
# ==============================================================================
DEVELOPER_USER="developer"
ALT_HOME="${ALT_HOME:-/alt/home/developer}"
WORKSPACE="${WORKSPACE:-${ALT_HOME}/workspace}"
SSH_PORT="${SSH_PORT:-2222}"
SKEL_DIR="/etc/skel"

# Source common functions if available
if [[ -f "/docker/lib/common.sh" ]]; then
    source /docker/lib/common.sh
else
    # Fallback print functions
    print_status() { echo "[INFO] $*"; }
    print_success() { echo "[OK] $*"; }
    print_warning() { echo "[WARN] $*"; }
    print_error() { echo "[ERROR] $*"; }
fi

# ==============================================================================
# Functions
# ==============================================================================

# ------------------------------------------------------------------------------
# setup_home_directory - Initialize developer home on persistent volume
# ------------------------------------------------------------------------------
setup_home_directory() {
    print_status "Setting up developer home directory..."

    # Ensure the volume mount point exists
    if [[ ! -d "$ALT_HOME" ]]; then
        mkdir -p "$ALT_HOME"
        print_status "Created home directory: $ALT_HOME"
    fi

    # Check if home is initialized (first boot detection)
    if [[ ! -f "${ALT_HOME}/.initialized" ]]; then
        print_status "Initializing home directory (first boot)..."

        # Create directory structure
        mkdir -p "${ALT_HOME}"/{.ssh,.config,.local/{share,state,bin},.cache}
        mkdir -p "${WORKSPACE}"/{projects,config,scripts,bin}
        mkdir -p "${WORKSPACE}/.system"/{manifest,installed,logs}

        # Copy skeleton files from /etc/skel
        if [[ -d "$SKEL_DIR" ]]; then
            cp -rn "$SKEL_DIR/." "${ALT_HOME}/" 2>/dev/null || true
            print_status "Copied skeleton files"
        fi

        # Create .bashrc if it doesn't exist
        if [[ ! -f "${ALT_HOME}/.bashrc" ]]; then
            cat > "${ALT_HOME}/.bashrc" << 'EOF'
# ~/.bashrc: executed by bash for non-login shells

# If not running interactively, don't do anything
case $- in
    *i*) ;;
      *) return;;
esac

# History settings
HISTCONTROL=ignoreboth
HISTSIZE=1000
HISTFILESIZE=2000
shopt -s histappend

# Check window size
shopt -s checkwinsize

# Make less more friendly
[ -x /usr/bin/lesspipe ] && eval "$(SHELL=/bin/sh lesspipe)"

# Color prompt
PS1='\[\033[01;32m\]\u@sindri\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ '

# Enable color support
if [ -x /usr/bin/dircolors ]; then
    test -r ~/.dircolors && eval "$(dircolors -b ~/.dircolors)" || eval "$(dircolors -b)"
    alias ls='ls --color=auto'
    alias grep='grep --color=auto'
fi

# Aliases
alias ll='ls -alF'
alias la='ls -A'
alias l='ls -CF'

# mise - unified tool version manager
if command -v mise >/dev/null 2>&1; then
    eval "$(mise activate bash)"
fi

# Add workspace bin to PATH
export PATH="${HOME}/workspace/bin:${PATH}"
EOF
            print_status "Created .bashrc"
        fi

        # Create .profile
        if [[ ! -f "${ALT_HOME}/.profile" ]]; then
            cat > "${ALT_HOME}/.profile" << 'EOF'
# ~/.profile: executed by the command interpreter for login shells

# Set PATH so it includes user's private bin if it exists
if [ -d "$HOME/bin" ] ; then
    PATH="$HOME/bin:$PATH"
fi

# Set PATH so it includes user's private bin if it exists
if [ -d "$HOME/.local/bin" ] ; then
    PATH="$HOME/.local/bin:$PATH"
fi

# Include .bashrc if it exists
if [ -n "$BASH_VERSION" ]; then
    if [ -f "$HOME/.bashrc" ]; then
        . "$HOME/.bashrc"
    fi
fi
EOF
            print_status "Created .profile"
        fi

        # Mark as initialized
        touch "${ALT_HOME}/.initialized"
        print_success "Home directory initialized"
    else
        print_status "Home directory already initialized"
    fi

    # Always ensure correct ownership (critical for volume mounts)
    chown -R "${DEVELOPER_USER}:${DEVELOPER_USER}" "$ALT_HOME"
    chmod 755 "$ALT_HOME"

    # Ensure .ssh directory has correct permissions
    if [[ -d "${ALT_HOME}/.ssh" ]]; then
        chmod 700 "${ALT_HOME}/.ssh"
        [[ -f "${ALT_HOME}/.ssh/authorized_keys" ]] && chmod 600 "${ALT_HOME}/.ssh/authorized_keys"
    fi

    # Update user's home directory in passwd
    usermod -d "$ALT_HOME" "$DEVELOPER_USER" 2>/dev/null || true

    print_success "Home directory configured"
}

# ------------------------------------------------------------------------------
# setup_ssh_keys - Configure SSH authorized keys from environment
# ------------------------------------------------------------------------------
setup_ssh_keys() {
    if [[ -n "${AUTHORIZED_KEYS:-}" ]]; then
        print_status "Configuring SSH authorized keys..."

        mkdir -p "${ALT_HOME}/.ssh"
        echo "$AUTHORIZED_KEYS" > "${ALT_HOME}/.ssh/authorized_keys"
        chown -R "${DEVELOPER_USER}:${DEVELOPER_USER}" "${ALT_HOME}/.ssh"
        chmod 700 "${ALT_HOME}/.ssh"
        chmod 600 "${ALT_HOME}/.ssh/authorized_keys"

        print_success "SSH keys configured"
    else
        print_warning "No SSH keys found in AUTHORIZED_KEYS environment variable"
    fi
}

# ------------------------------------------------------------------------------
# persist_ssh_host_keys - Persist SSH host keys for stable fingerprints
# ------------------------------------------------------------------------------
persist_ssh_host_keys() {
    local host_keys_dir="${ALT_HOME}/.ssh/host_keys"

    # Create host keys directory in persistent volume
    mkdir -p "$host_keys_dir" 2>/dev/null || true

    # Check if we have persisted keys in the volume
    if ls "$host_keys_dir"/*_key >/dev/null 2>&1; then
        # Restore persisted keys to /etc/ssh
        print_status "Restoring persisted SSH host keys..."
        cp "$host_keys_dir"/*_key /etc/ssh/ 2>/dev/null || true
        cp "$host_keys_dir"/*_key.pub /etc/ssh/ 2>/dev/null || true
        chmod 600 /etc/ssh/*_key 2>/dev/null || true
        chmod 644 /etc/ssh/*_key.pub 2>/dev/null || true
    else
        # Generate new host keys if they don't exist
        if [[ ! -f /etc/ssh/ssh_host_rsa_key ]]; then
            print_status "Generating new SSH host keys..."
            ssh-keygen -A 2>/dev/null || true
        fi

        # Persist the keys to the volume for future deploys
        print_status "Persisting SSH host keys to volume..."
        mkdir -p "$host_keys_dir"
        cp /etc/ssh/ssh_host_*_key "$host_keys_dir/" 2>/dev/null || true
        cp /etc/ssh/ssh_host_*_key.pub "$host_keys_dir/" 2>/dev/null || true
    fi

    # Ensure correct ownership
    chown -R "${DEVELOPER_USER}:${DEVELOPER_USER}" "$host_keys_dir" 2>/dev/null || true
}

# ------------------------------------------------------------------------------
# setup_git_config - Configure Git user credentials
# ------------------------------------------------------------------------------
setup_git_config() {
    local configured=false

    if [[ -n "${GIT_USER_NAME:-}" ]]; then
        su - "$DEVELOPER_USER" -c "git config --global user.name '$GIT_USER_NAME'"
        print_status "Git user name configured: $GIT_USER_NAME"
        configured=true
    fi

    if [[ -n "${GIT_USER_EMAIL:-}" ]]; then
        su - "$DEVELOPER_USER" -c "git config --global user.email '$GIT_USER_EMAIL'"
        print_status "Git user email configured: $GIT_USER_EMAIL"
        configured=true
    fi

    # Setup Git credential helper for GitHub token
    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        cat > "${ALT_HOME}/.git-credential-helper.sh" << 'GITCRED'
#!/bin/bash
if [ "$1" = "get" ]; then
    while IFS= read -r line; do
        case "$line" in
            host=github.com)
                echo "protocol=https"
                echo "host=github.com"
                echo "username=token"
                echo "password=$GITHUB_TOKEN"
                break
                ;;
        esac
    done
fi
GITCRED
        chmod +x "${ALT_HOME}/.git-credential-helper.sh"
        chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "${ALT_HOME}/.git-credential-helper.sh"
        su - "$DEVELOPER_USER" -c "git config --global credential.helper '${ALT_HOME}/.git-credential-helper.sh'"
        print_status "Git credential helper configured"
        configured=true
    fi

    if [[ "$configured" == "false" ]]; then
        print_status "No Git configuration provided (skipping)"
    fi
}

# ------------------------------------------------------------------------------
# setup_motd - Configure Message of the Day banner
# ------------------------------------------------------------------------------
setup_motd() {
    if [[ -f "/docker/scripts/setup-motd.sh" ]]; then
        print_status "Setting up MOTD banner..."
        bash /docker/scripts/setup-motd.sh 2>/dev/null || true
    fi
}

# ------------------------------------------------------------------------------
# start_ssh_daemon - Start SSH daemon (if not in CI mode)
# ------------------------------------------------------------------------------
start_ssh_daemon() {
    if [[ "${CI_MODE:-}" == "true" ]]; then
        print_status "CI Mode: Skipping SSH daemon startup"
        print_success "Sindri is ready (CI Mode)!"
        print_status "SSH access available via: flyctl ssh console"
        print_status "Home directory: $ALT_HOME"
    else
        print_status "Starting SSH daemon on port ${SSH_PORT}..."

        # Persist/restore SSH host keys for stable fingerprints
        persist_ssh_host_keys

        # Ensure sshd runtime directory exists
        mkdir -p /var/run/sshd

        # Start SSH daemon in foreground (-D) with logging to stderr (-e)
        # This keeps the container alive and accepts SSH connections
        /usr/sbin/sshd -D -e &

        print_success "Sindri is ready!"
        print_status "SSH server listening on port ${SSH_PORT}"
        print_status "Home directory: $ALT_HOME"
    fi
}

# ------------------------------------------------------------------------------
# wait_for_shutdown - Handle graceful shutdown
# ------------------------------------------------------------------------------
wait_for_shutdown() {
    # Handle shutdown gracefully
    trap 'echo "Shutting down..."; kill $(jobs -p) 2>/dev/null; exit 0' SIGTERM SIGINT

    if [[ "${CI_MODE:-}" == "true" ]]; then
        # In CI mode, keep container running without SSH
        sleep infinity
    else
        # Wait for SSH daemon
        wait $!
    fi
}

# ------------------------------------------------------------------------------
# main - Entry point that orchestrates container startup
# ------------------------------------------------------------------------------
main() {
    echo "========================================"
    echo "Starting Sindri Development Environment"
    echo "========================================"

    setup_home_directory
    setup_ssh_keys
    setup_git_config
    setup_motd
    start_ssh_daemon
    wait_for_shutdown
}

# ==============================================================================
# Execute main function
# ==============================================================================
main "$@"
