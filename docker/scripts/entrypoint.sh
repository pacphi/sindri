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
        mkdir -p "$ALT_HOME" || {
            print_error "Failed to create home directory: $ALT_HOME"
            exit 1
        }
        print_status "Created home directory: $ALT_HOME"
    fi

    # Verify volume is writable (critical for Fly.io volume mounts)
    if ! touch "${ALT_HOME}/.write_test" 2>/dev/null; then
        print_error "Volume at $ALT_HOME is not writable - check volume mount"
        exit 1
    fi
    rm -f "${ALT_HOME}/.write_test"

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

# starship - cross-shell prompt
if command -v starship >/dev/null 2>&1; then
    eval "$(starship init bash)"
fi

# Add CLI tools and workspace bin to PATH
export PATH="/docker/cli:${HOME}/workspace/bin:${PATH}"
EOF
            print_status "Created .bashrc"
        fi

        # Ensure starship init is in .bashrc (append snippet if not present)
        # This follows the same pattern as JVM extension (mode: append)
        if [[ -f "${ALT_HOME}/.bashrc" ]] && ! grep -q "starship init" "${ALT_HOME}/.bashrc" 2>/dev/null; then
            if [[ -f "/etc/skel/.bashrc-starship" ]]; then
                cat "/etc/skel/.bashrc-starship" >> "${ALT_HOME}/.bashrc"
                print_status "Added starship initialization to .bashrc"
            fi
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

    # Update user's home directory in passwd (only if different)
    local current_home
    current_home=$(getent passwd "$DEVELOPER_USER" | cut -d: -f6) || true
    if [[ "$current_home" != "$ALT_HOME" ]]; then
        usermod -d "$ALT_HOME" "$DEVELOPER_USER" 2>/dev/null || true
    fi

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

        # Disable password authentication while allowing SSH key authentication
        # usermod -p '*' sets impossible password (cannot be matched by any input)
        usermod -p '*' "${DEVELOPER_USER}" 2>/dev/null || true

        # Security logging (H-12)
        local key_type
        key_type=$(echo "$AUTHORIZED_KEYS" | awk '{print $1}')
        security_log_auth "ssh_keys_configured" "success" "SSH keys configured: $key_type"

        print_success "SSH keys configured"
    else
        print_warning "No SSH keys found in AUTHORIZED_KEYS environment variable"
        security_log_auth "ssh_keys_missing" "failure" "No AUTHORIZED_KEYS provided"
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
        # Validate input to prevent command injection
        if [[ ! "$GIT_USER_NAME" =~ ^[a-zA-Z0-9._\ -]+$ ]]; then
            print_error "Invalid GIT_USER_NAME: contains unsafe characters"
            print_status "GIT_USER_NAME must contain only alphanumeric, dots, spaces, underscores, or hyphens"
            security_log_config "git_user_name" "denied" "GIT_USER_NAME" "Invalid characters detected"
            return 1
        fi
        # Use printf %q for safe shell quoting
        su - "$DEVELOPER_USER" -c "$(printf 'git config --global user.name %q' "$GIT_USER_NAME")"
        print_status "Git user name configured: $GIT_USER_NAME"
        security_log_config "git_user_name" "success" "GIT_USER_NAME" "$GIT_USER_NAME"
        configured=true
    fi

    if [[ -n "${GIT_USER_EMAIL:-}" ]]; then
        # Validate email format to prevent command injection
        if [[ ! "$GIT_USER_EMAIL" =~ ^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$ ]]; then
            print_error "Invalid GIT_USER_EMAIL: must be a valid email address"
            security_log_config "git_user_email" "denied" "GIT_USER_EMAIL" "Invalid email format"
            return 1
        fi
        # Use printf %q for safe shell quoting
        su - "$DEVELOPER_USER" -c "$(printf 'git config --global user.email %q' "$GIT_USER_EMAIL")"
        print_status "Git user email configured: $GIT_USER_EMAIL"
        security_log_config "git_user_email" "success" "GIT_USER_EMAIL" "$GIT_USER_EMAIL"
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
        # Use printf %q for safe path quoting
        local safe_helper_path
        safe_helper_path=$(printf '%q' "${ALT_HOME}/.git-credential-helper.sh")
        su - "$DEVELOPER_USER" -c "git config --global credential.helper $safe_helper_path"
        print_status "Git credential helper configured"
        configured=true
    fi

    if [[ "$configured" == "false" ]]; then
        print_status "No Git configuration provided (skipping)"
    fi
}

# ------------------------------------------------------------------------------
# propagate_secrets - Write provider-injected secrets to /etc/profile.d/
# ------------------------------------------------------------------------------
# Providers (Fly.io, Docker, Kubernetes) inject secrets as environment variables
# at container startup. However, SSH and `su - user` create login shells that
# don't inherit these variables. This function writes secrets to /etc/profile.d/
# which is sourced by all login shells, making secrets available to users.
#
# Pattern matching automatically detects secrets (*_API_KEY, *_TOKEN, *_SECRET)
# so new secrets don't require code changes.
propagate_secrets() {
    local secrets_file="/etc/profile.d/sindri-secrets.sh"
    local secrets_written=0

    # Create script header
    cat > "$secrets_file" << 'HEADER'
#!/bin/bash
# Sindri secrets - auto-generated by entrypoint.sh
# Propagates provider-injected secrets to login shells (SSH, su - user)
# DO NOT EDIT - regenerated on container startup
HEADER

    # Find and export all secret-like environment variables
    # Patterns: *_API_KEY, *_TOKEN, *_SECRET, and AUTHORIZED_KEYS
    while IFS='=' read -r name value; do
        # Skip empty names or values
        [[ -z "$name" ]] && continue
        [[ -z "$value" ]] && continue

        # Match secret patterns
        if [[ "$name" =~ _API_KEY$ ]] || \
           [[ "$name" =~ _TOKEN$ ]] || \
           [[ "$name" =~ _SECRET$ ]] || \
           [[ "$name" == "AUTHORIZED_KEYS" ]]; then
            # Use printf %q for safe shell quoting - handles ALL special characters
            local safe_value
            safe_value=$(printf '%q' "$value")
            echo "export ${name}=${safe_value}" >> "$secrets_file"
            ((secrets_written++)) || true
        fi
    done < <(env)

    chmod 644 "$secrets_file"

    if [[ $secrets_written -gt 0 ]]; then
        print_status "Propagated $secrets_written secret(s) to login shells"
        security_log_config "secrets_propagated" "success" "/etc/profile.d/sindri-secrets.sh" "$secrets_written secrets"
    fi
}

# ------------------------------------------------------------------------------
# start_ssh_daemon - Start SSH daemon (if not in CI mode)
# ------------------------------------------------------------------------------
# Follows Fly.io OpenSSH blueprint pattern:
# https://fly.io/docs/blueprints/opensshd/
start_ssh_daemon() {
    if [[ "${CI_MODE:-}" == "true" ]]; then
        print_status "CI Mode: Skipping SSH daemon startup"
        print_success "Sindri is ready (CI Mode)!"
        print_status "SSH access available via: flyctl ssh console"
        print_status "Home directory: $ALT_HOME"
        print_status "Workspace: $WORKSPACE"
    else
        print_status "Starting SSH daemon on port ${SSH_PORT}..."

        # Persist/restore SSH host keys for stable fingerprints
        persist_ssh_host_keys

        # Ensure sshd runtime directory exists
        mkdir -p /var/run/sshd

        # Start SSH daemon in foreground mode, then background it
        # This matches the legacy approach for better process control
        /usr/sbin/sshd -D &

        print_success "Sindri is ready!"
        print_status "SSH server listening on port ${SSH_PORT}"
        print_status "Home directory: $ALT_HOME"
        print_status "Workspace: $WORKSPACE"
    fi
}

# ------------------------------------------------------------------------------
# wait_for_shutdown - Handle graceful shutdown and keep container alive
# ------------------------------------------------------------------------------
# Uses a while loop with signal handling for robust container lifecycle
# This avoids potential segfaults with 'exec sleep infinity' on some platforms
wait_for_shutdown() {
    # Handle shutdown gracefully
    shutdown_handler() {
        echo "Shutting down Sindri..."
        pkill sshd 2>/dev/null || true
        exit 0
    }
    trap shutdown_handler SIGTERM SIGINT SIGHUP

    # Keep container alive with a simple loop
    # This is more reliable than 'exec sleep infinity' which can segfault
    # on some container runtimes (observed on Fly.io with Ubuntu 24.04)
    while true; do
        sleep 60 &
        wait $! || true
    done
}

# ------------------------------------------------------------------------------
# show_welcome - Display welcome message on first boot only
# ------------------------------------------------------------------------------
show_welcome() {
    local welcome_marker="${ALT_HOME}/.welcome_shown"

    # Only show welcome on first boot (marker doesn't exist yet)
    if [[ ! -f "$welcome_marker" ]] && [[ -x "${ALT_HOME}/welcome.sh" ]]; then
        su - "$DEVELOPER_USER" -c "${ALT_HOME}/welcome.sh" 2>/dev/null || true
        # Create marker so welcome isn't shown on container restarts
        touch "$welcome_marker"
        chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$welcome_marker"
    fi
}

# ------------------------------------------------------------------------------
# Fly.io Lease Management - Prevents auto-suspend during installation
# ------------------------------------------------------------------------------
# Fly.io's auto-suspend feature monitors traffic through Fly Proxy and suspends
# machines when idle. During extension installation, the machine may appear idle
# (no SSH connections through the proxy) even though installation is in progress.
#
# Machine leases provide an exclusive lock that prevents Fly Proxy from
# stopping or suspending the machine. We acquire a lease before installation
# and release it when complete.
#
# Requirements:
#   - FLY_API_TOKEN must be set as a secret (not auto-injected by Fly.io)
#   - Set with: flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app>
#
# References:
#   - Machines API: https://fly.io/docs/machines/api/machines-resource/
#   - Auto-suspend: https://fly.io/docs/launch/autostop-autostart/
# ------------------------------------------------------------------------------

# Fly.io internal API endpoint (accessible from within machines)
FLY_API_BASE="http://_api.internal:4280/v1/apps"

# Acquire a lease to prevent Fly.io from auto-suspending during installation
# Args: $1 - TTL in seconds (default: 3600 = 1 hour)
# Returns: 0 on success, 1 on failure (installation proceeds without protection)
acquire_install_lease() {
    local ttl="${1:-3600}"
    local lease_nonce_file="${WORKSPACE}/.system/.lease_nonce"
    local lease_renewal_pid_file="${WORKSPACE}/.system/.lease_renewal_pid"

    # Skip if not running on Fly.io
    if [[ -z "${FLY_MACHINE_ID:-}" ]]; then
        print_status "Not running on Fly.io, skipping lease acquisition"
        return 0
    fi

    # Check if FLY_API_TOKEN is available
    if [[ -z "${FLY_API_TOKEN:-}" ]]; then
        print_warning "FLY_API_TOKEN not set - machine may suspend during installation"
        print_status "To enable installation protection, set the token:"
        print_status "  flyctl secrets set FLY_API_TOKEN=\"\$(fly tokens deploy)\" -a ${FLY_APP_NAME}"
        return 1
    fi

    print_status "Acquiring installation protection lease (TTL: ${ttl}s)..."

    local response http_code nonce
    local max_retries=3
    local retry_delay=2
    local attempt=1

    while [[ $attempt -le $max_retries ]]; do
        print_status "  Attempt $attempt/$max_retries: ${FLY_API_BASE}/${FLY_APP_NAME}/machines/${FLY_MACHINE_ID}/lease"

        response=$(curl -s -w "\n%{http_code}" \
            -X POST \
            -H "Authorization: Bearer ${FLY_API_TOKEN}" \
            -H "Content-Type: application/json" \
            -d "{\"description\": \"sindri-extension-install\", \"ttl\": ${ttl}}" \
            "${FLY_API_BASE}/${FLY_APP_NAME}/machines/${FLY_MACHINE_ID}/lease" 2>&1)

        # Extract HTTP code from last line
        http_code=$(echo "$response" | tail -1)
        response=$(echo "$response" | sed '$d')

        nonce=$(echo "$response" | jq -r '.data.nonce // empty' 2>/dev/null)

        if [[ -n "$nonce" ]] && [[ "$http_code" =~ ^2 ]]; then
            break
        fi

        if [[ $attempt -lt $max_retries ]]; then
            print_status "  Lease request failed (HTTP ${http_code}), retrying in ${retry_delay}s..."
            sleep "$retry_delay"
            retry_delay=$((retry_delay * 2))
        fi
        ((attempt++))
    done

    if [[ -n "$nonce" ]] && [[ "$http_code" =~ ^2 ]]; then
        # Save nonce for later release
        echo "$nonce" > "$lease_nonce_file"
        chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$lease_nonce_file" 2>/dev/null || true

        print_success "Installation lease acquired (nonce: ${nonce:0:8}...)"

        # Start background lease renewal (every 30 minutes)
        (
            while [[ -f "$lease_nonce_file" ]]; do
                sleep 1800  # 30 minutes
                if [[ -f "$lease_nonce_file" ]]; then
                    curl -sf \
                        -X POST \
                        -H "Authorization: Bearer ${FLY_API_TOKEN}" \
                        -H "Content-Type: application/json" \
                        -H "fly-machine-lease-nonce: $nonce" \
                        -d "{\"description\": \"sindri-extension-install\", \"ttl\": ${ttl}}" \
                        "${FLY_API_BASE}/${FLY_APP_NAME}/machines/${FLY_MACHINE_ID}/lease" >/dev/null 2>&1
                fi
            done
        ) &
        echo $! > "$lease_renewal_pid_file"

        return 0
    else
        print_warning "Could not acquire lease (HTTP ${http_code}): ${response}"
        print_status "Installation will proceed without auto-suspend protection"
        return 1
    fi
}

# Release the installation lease, allowing normal auto-suspend behavior
release_install_lease() {
    local lease_nonce_file="${WORKSPACE}/.system/.lease_nonce"
    local lease_renewal_pid_file="${WORKSPACE}/.system/.lease_renewal_pid"

    # Skip if not running on Fly.io or no lease was acquired
    if [[ -z "${FLY_MACHINE_ID:-}" ]] || [[ ! -f "$lease_nonce_file" ]]; then
        return 0
    fi

    local nonce
    nonce=$(cat "$lease_nonce_file" 2>/dev/null)

    if [[ -n "$nonce" ]]; then
        print_status "Releasing installation protection lease..."

        # Stop the lease renewal background process
        if [[ -f "$lease_renewal_pid_file" ]]; then
            local renewal_pid
            renewal_pid=$(cat "$lease_renewal_pid_file")
            kill "$renewal_pid" 2>/dev/null || true
            rm -f "$lease_renewal_pid_file"
        fi

        # Release the lease
        curl -sf \
            -X DELETE \
            -H "Authorization: Bearer ${FLY_API_TOKEN}" \
            -H "fly-machine-lease-nonce: $nonce" \
            "${FLY_API_BASE}/${FLY_APP_NAME}/machines/${FLY_MACHINE_ID}/lease" >/dev/null 2>&1

        rm -f "$lease_nonce_file"
        print_success "Installation lease released - machine can now auto-suspend"
    fi
}

# ------------------------------------------------------------------------------
# install_extensions_background - Run extension installation in background
# ------------------------------------------------------------------------------
# Runs extension installation asynchronously so SSH can accept connections
# immediately. Users connecting during installation see a status banner.
#
# On Fly.io, this function also manages a machine lease to prevent auto-suspend
# during installation. The lease is acquired before installation starts and
# released when installation completes (success or failure).
install_extensions_background() {
    local install_status_file="${WORKSPACE}/.system/install-status"
    local install_log_file="${WORKSPACE}/.system/logs/install.log"

    # Ensure directories exist
    mkdir -p "$(dirname "$install_status_file")"
    mkdir -p "$(dirname "$install_log_file")"

    # Create marker indicating installation is in progress
    echo "installing" > "$install_status_file"
    chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$install_status_file"

    # Source and run extension installation in background
    if [[ -f "/docker/scripts/auto-install-extensions.sh" ]]; then
        source /docker/scripts/auto-install-extensions.sh

        # Run installation in background, logging output
        (
            # Acquire lease to prevent Fly.io auto-suspend during installation
            # This is a no-op on non-Fly.io environments or if FLY_API_TOKEN is not set
            acquire_install_lease 3600  # 1 hour initial TTL

            print_status "Starting background extension installation..."
            # Use set -o pipefail to capture install_extensions exit code through tee
            set -o pipefail
            if install_extensions 2>&1 | tee -a "$install_log_file"; then
                echo "complete" > "$install_status_file"
                print_success "Extension installation complete"
            else
                local exit_code=$?
                echo "failed" > "$install_status_file"
                print_error "Extension installation failed (exit code: $exit_code)"
            fi
            chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$install_status_file"
            chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$install_log_file" 2>/dev/null || true

            # Release lease to allow normal auto-suspend behavior
            release_install_lease
        ) &

        print_status "Extension installation running in background (PID: $!)"
        print_status "Monitor progress: tail -f ${install_log_file}"
    else
        # No extensions to install
        echo "complete" > "$install_status_file"
        chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$install_status_file"
    fi
}

# ------------------------------------------------------------------------------
# main - Entry point that orchestrates container startup
# ------------------------------------------------------------------------------
main() {
    # Early startup logging - output to stderr to ensure it's visible
    # This helps debug container startup issues on Fly.io
    echo "========================================"  >&2
    echo "Sindri Container Starting"  >&2
    local timestamp
    timestamp=$(date -Iseconds 2>/dev/null) || timestamp=$(date) || true
    echo "Time: $timestamp"  >&2
    echo "CI_MODE: ${CI_MODE:-false}"  >&2
    echo "ALT_HOME: ${ALT_HOME}"  >&2
    echo "========================================"  >&2

    # Display MOTD banner
    if [[ -f /etc/motd ]]; then
        cat /etc/motd
    fi

    # Always initialize the environment
    setup_home_directory
    setup_ssh_keys
    setup_git_config

    # Propagate provider-injected secrets to login shells (SSH, su - user)
    propagate_secrets

    # Check if a command was passed (interactive mode)
    if [[ $# -gt 0 ]]; then
        # Interactive mode: execute command as developer user
        print_status "Interactive mode: executing command as $DEVELOPER_USER"

        # Show welcome message for interactive shells
        if [[ "$1" == "/bin/bash" || "$1" == "bash" || "$1" == "/bin/sh" || "$1" == "sh" ]]; then
            show_welcome
        fi

        # Execute the command as the developer user
        export HOME="$ALT_HOME"
        export PATH="${ALT_HOME}/.local/share/mise/shims:/docker/cli:${ALT_HOME}/workspace/bin:/usr/local/bin:$PATH"
        export MISE_DATA_DIR="${ALT_HOME}/.local/share/mise"
        export MISE_CONFIG_DIR="${ALT_HOME}/.config/mise"
        cd "$WORKSPACE"
        exec sudo -u "$DEVELOPER_USER" --preserve-env=HOME,PATH,WORKSPACE,ALT_HOME,MISE_DATA_DIR,MISE_CONFIG_DIR "$@"
    else
        # Server mode: Start SSH daemon FIRST to pass health checks immediately
        # This prevents Fly.io auto-suspend from killing the machine during
        # extension installation. Extensions install in background.
        start_ssh_daemon

        # Start extension installation in background
        # Users connecting during installation see a status banner via /etc/profile.d/
        install_extensions_background

        wait_for_shutdown
    fi
}

# ==============================================================================
# Execute main function
# ==============================================================================
main "$@"
