#!/bin/bash
# ==============================================================================
# Sindri v3 Container Entrypoint
# ==============================================================================
# Initializes and starts the Sindri v3 development environment container.
# Runs as root to set up volumes, permissions, and start SSH daemon.
# SSH sessions run as the developer user.
#
# Key differences from v2:
# - Uses sindri Rust CLI for extension management (not bash scripts)
# - Simplified directory structure (~/.sindri vs workspace/.system)
# - No bundled extensions (all installed at runtime)
#
# Environment Variables:
#   CI_MODE              - Set to "true" to skip SSH daemon (use flyctl ssh console)
#   AUTHORIZED_KEYS      - SSH public keys for authentication
#   GIT_USER_NAME        - Git user.name configuration
#   GIT_USER_EMAIL       - Git user.email configuration
#   GITHUB_TOKEN         - GitHub token for git credential helper
#   SSH_PORT             - SSH daemon port (default: 2222)
#   INSTALL_PROFILE      - Profile to install (no default)
#   ADDITIONAL_EXTENSIONS - Comma-separated list of extensions to install on top of profile
#   CUSTOM_EXTENSIONS    - Comma-separated list of extensions to install (without profile)
#   SKIP_AUTO_INSTALL    - Set to "true" to skip automatic extension installation
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
SINDRI_HOME="${ALT_HOME}/.sindri"

# Fallback print functions
print_status() { echo "[INFO] $*"; }
print_success() { echo "[OK] $*"; }
print_warning() { echo "[WARN] $*"; }
print_error() { echo "[ERROR] $*"; }

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
        mkdir -p "${SINDRI_HOME}"/{extensions,cache,state,logs}

        # Optionally upgrade support files from GitHub in background (non-blocking)
        # Note: Bundled support files (common.sh, etc.) are now copied on every boot
        # (see the always-run block after this if/else) to handle volume reuse correctly.
        # This fetches version-matched files that may be newer than the bundled ones
        if command -v sindri >/dev/null 2>&1; then
            (
                su - "${DEVELOPER_USER}" -c "sindri extension update-support-files --quiet" 2>&1 | \
                    tee -a "${SINDRI_HOME}/logs/support-files-init.log" || true
            ) &
        fi

        # Create temp directory on persistent volume (Claude Code plugin compatibility)
        # Required to prevent EXDEV cross-device link error during plugin installation
        # fs.rename() cannot cross filesystem boundaries (/tmp is tmpfs, ~/.claude is on volume)
        # See: https://github.com/anthropics/claude-code/issues/14799
        mkdir -p "${ALT_HOME}/.cache/tmp"

        # Copy skeleton files from /etc/skel
        if [[ -d "$SKEL_DIR" ]]; then
            cp -rn "$SKEL_DIR/." "${ALT_HOME}/" 2>/dev/null || true
            print_status "Copied skeleton files"
        fi

        # Create .bashrc if it doesn't exist
        if [[ ! -f "${ALT_HOME}/.bashrc" ]]; then
            cp /docker/templates/bashrc "${ALT_HOME}/.bashrc"
            print_status "Created .bashrc from template"
        fi

        # Install optimized Starship config for fast shell startup
        if command -v starship &> /dev/null; then
            mkdir -p "${ALT_HOME}/.config"
            if [[ ! -f "${ALT_HOME}/.config/starship.toml" ]]; then
                cp /docker/templates/starship.toml "${ALT_HOME}/.config/starship.toml"
                print_status "Installed optimized Starship config"
            fi
        fi

        # Mark home as initialized
        touch "${ALT_HOME}/.initialized"
        print_success "Home directory initialized"
    else
        print_status "Home directory already initialized"
    fi

    # Always ensure Sindri support files are present (not just on first boot)
    # Extensions may be reinstalled after volume reuse, and common.sh must be
    # available for script-based extensions that source it at startup.
    if [[ -d "/docker/config/sindri" ]]; then
        mkdir -p "${SINDRI_HOME}/extensions"
        cp -f /docker/config/sindri/common.sh "${SINDRI_HOME}/extensions/" 2>/dev/null || true
        cp -f /docker/config/sindri/compatibility-matrix.yaml "${SINDRI_HOME}/" 2>/dev/null || true
        cp -f /docker/config/sindri/extension-source.yaml "${SINDRI_HOME}/" 2>/dev/null || true
    fi

    # Ensure correct ownership
    chown -R "${DEVELOPER_USER}:${DEVELOPER_USER}" "${ALT_HOME}"
    print_success "Home directory setup complete"
}

# ------------------------------------------------------------------------------
# setup_ssh_keys - Configure SSH authorized keys for developer user
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
        usermod -p '*' "${DEVELOPER_USER}" 2>/dev/null || true

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
# setup_sindri_environment - Write SINDRI_* variables to profile.d for SSH sessions
# ------------------------------------------------------------------------------
setup_sindri_environment() {
    # Capture all SINDRI_* environment variables and write to /etc/profile.d/
    # This makes docker-compose environment variables (like SINDRI_EXT_HOME)
    # available in SSH login shells

    local sindri_profile="/etc/profile.d/sindri-runtime.sh"

    print_status "Configuring SINDRI environment variables for SSH sessions..."

    # Start the profile script
    cat > "$sindri_profile" << 'PROFILE_HEADER'
# Sindri runtime environment variables
# Auto-generated from docker-compose environment at container startup
PROFILE_HEADER

    # Capture all SINDRI_* variables currently set
    local sindri_vars
    sindri_vars=$(env | grep -E '^SINDRI_' || true)

    if [[ -n "$sindri_vars" ]]; then
        while IFS= read -r line; do
            # Export each variable
            echo "export $line" >> "$sindri_profile"
        done <<< "$sindri_vars"

        chmod +x "$sindri_profile"
        print_success "SINDRI environment variables configured for SSH sessions"
    else
        print_status "No SINDRI_* environment variables found"
    fi
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
            return 1
        fi
        su - "$DEVELOPER_USER" -c "$(printf 'git config --global user.name %q' "$GIT_USER_NAME")"
        print_status "Git user name configured: $GIT_USER_NAME"
        configured=true
    fi

    if [[ -n "${GIT_USER_EMAIL:-}" ]]; then
        # Validate email format to prevent command injection
        if [[ ! "$GIT_USER_EMAIL" =~ ^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$ ]]; then
            print_error "Invalid GIT_USER_EMAIL: must be a valid email address"
            return 1
        fi
        su - "$DEVELOPER_USER" -c "$(printf 'git config --global user.email %q' "$GIT_USER_EMAIL")"
        print_status "Git user email configured: $GIT_USER_EMAIL"
        configured=true
    fi

    # Setup Git credential helper for GitHub token
    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        cat > "${ALT_HOME}/.git-credential-helper.sh" << 'GITCRED'
#!/bin/bash
if [ "$1" = "get" ]; then
    echo "protocol=https"
    echo "host=github.com"
    echo "username=git"
    echo "password=${GITHUB_TOKEN}"
fi
GITCRED
        chmod +x "${ALT_HOME}/.git-credential-helper.sh"
        chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "${ALT_HOME}/.git-credential-helper.sh"

        su - "$DEVELOPER_USER" -c 'git config --global credential.helper "!${HOME}/.git-credential-helper.sh"'
        print_status "GitHub token credential helper configured"
        configured=true
    fi

    if $configured; then
        print_success "Git configuration complete"
    else
        print_status "No Git configuration provided"
    fi
}

# ------------------------------------------------------------------------------
# install_extensions_background - Run extension installation via sindri CLI
# ------------------------------------------------------------------------------
# Uses sindri CLI to install extensions based on priority:
# 1. sindri.yaml config file in workspace (if present) - handles profile + additional
# 2. INSTALL_PROFILE + ADDITIONAL_EXTENSIONS environment variables (profile-based)
# 3. CUSTOM_EXTENSIONS environment variable (explicit list, no profile)
#
# Note: ADDITIONAL_EXTENSIONS only applies when using INSTALL_PROFILE, not sindri.yaml
# (sindri.yaml already contains its own additional extensions list)
install_extensions_background() {
    local bootstrap_marker="${SINDRI_HOME}/bootstrap-complete"
    local install_log="${SINDRI_HOME}/logs/install.log"

    # Skip if already bootstrapped
    if [[ -f "$bootstrap_marker" ]]; then
        print_status "Extensions already installed (bootstrap complete)"
        return 0
    fi

    # Skip if auto-install disabled
    if [[ "${SKIP_AUTO_INSTALL:-false}" == "true" ]]; then
        print_status "Skipping automatic extension installation (SKIP_AUTO_INSTALL=true)"
        return 0
    fi

    # Ensure log directory exists
    mkdir -p "$(dirname "$install_log")"
    chown -R "${DEVELOPER_USER}:${DEVELOPER_USER}" "${SINDRI_HOME}"

    print_status "Starting background extension installation..."

    # Run installation in background
    (
        cd "$WORKSPACE"

        # Determine installation method (priority order)
        # Set SINDRI_EXT_HOME at runtime using ALT_HOME (the volume-mounted home)
        # Preserves existing value if already set (e.g., /opt/sindri/extensions from Dockerfile.dev)
        # Falls back to ${ALT_HOME}/.sindri/extensions for production builds (Dockerfile)
        local ext_home="${SINDRI_EXT_HOME:-${ALT_HOME}/.sindri/extensions}"

        # CRITICAL: Export variables BEFORE sudo (v2 pattern)
        # sudo --preserve-env requires variables to be exported first
        # This ensures mise installs to the correct location from the start
        export HOME="${ALT_HOME}"
        export PATH="${ALT_HOME}/.local/share/mise/shims:${PATH}"
        export WORKSPACE="${WORKSPACE}"
        export SINDRI_EXT_HOME="${ext_home}"
        export SINDRI_SOURCE_REF="${SINDRI_SOURCE_REF:-}"
        export MISE_DATA_DIR="${ALT_HOME}/.local/share/mise"
        export MISE_CONFIG_DIR="${ALT_HOME}/.config/mise"
        export MISE_CACHE_DIR="${ALT_HOME}/.cache/mise"
        export MISE_STATE_DIR="${ALT_HOME}/.local/state/mise"

        # Build preserve list dynamically from environment (prevents staleness)
        # Auto-discovers all relevant variables instead of hardcoding
        local preserve_list="HOME,PATH,WORKSPACE"

        # Add all SINDRI_* variables
        local sindri_vars
        sindri_vars=$(env | grep -E '^SINDRI_' | cut -d= -f1 | tr '\n' ',' | sed 's/,$//')
        [[ -n "$sindri_vars" ]] && preserve_list="${preserve_list},${sindri_vars}"

        # Add all MISE_* variables
        local mise_vars
        mise_vars=$(env | grep -E '^MISE_' | cut -d= -f1 | tr '\n' ',' | sed 's/,$//')
        [[ -n "$mise_vars" ]] && preserve_list="${preserve_list},${mise_vars}"

        # Add all GIT_* variables
        local git_vars
        git_vars=$(env | grep -E '^GIT_' | cut -d= -f1 | tr '\n' ',' | sed 's/,$//')
        [[ -n "$git_vars" ]] && preserve_list="${preserve_list},${git_vars}"

        # Add all credential/secret variables (comprehensive pattern)
        # Matches: *_TOKEN, *_API_KEY, *_KEY, *_KEYS, *_PASSWORD, *_PASS, *_USERNAME, *_USER, *_URL, *_SECRET
        local credential_vars
        credential_vars=$(env | grep -E '_(TOKEN|API_KEY|KEY|KEYS|PASSWORD|PASS|USERNAME|USER|URL|SECRET)$' | cut -d= -f1 | tr '\n' ',' | sed 's/,$//')
        [[ -n "$credential_vars" ]] && preserve_list="${preserve_list},${credential_vars}"

        local env_vars="$preserve_list"

        if [[ -f "sindri.yaml" ]]; then
            # Priority 1: Install from sindri.yaml if present in workspace
            print_status "Installing extensions from sindri.yaml..." 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
            sudo -u "$DEVELOPER_USER" --preserve-env="${env_vars}" bash -c "cd '$WORKSPACE' && sindri extension install --from-config sindri.yaml --yes" 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
        elif [[ -n "${INSTALL_PROFILE:-}" ]]; then
            # Priority 2: Install from INSTALL_PROFILE environment variable
            print_status "Installing profile: ${INSTALL_PROFILE}..." 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
            sudo -u "$DEVELOPER_USER" --preserve-env="${env_vars}" sindri profile install "${INSTALL_PROFILE}" --yes 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null

            # Install additional extensions on top of profile if specified
            if [[ -n "${ADDITIONAL_EXTENSIONS:-}" ]]; then
                print_status "Installing additional extensions: ${ADDITIONAL_EXTENSIONS}..." 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
                # Split comma-separated list and install each extension
                IFS=',' read -ra EXTENSIONS <<< "$ADDITIONAL_EXTENSIONS"
                for ext in "${EXTENSIONS[@]}"; do
                    # Trim whitespace
                    ext=$(echo "$ext" | xargs)
                    if [[ -n "$ext" ]]; then
                        print_status "Installing additional extension: ${ext}..." 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
                        sudo -u "$DEVELOPER_USER" --preserve-env="${env_vars}" sindri extension install "$ext" --yes 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
                    fi
                done
            fi
        elif [[ -n "${CUSTOM_EXTENSIONS:-}" ]]; then
            # Priority 3: Install from CUSTOM_EXTENSIONS environment variable (explicit list, no profile)
            print_status "Installing custom extensions: ${CUSTOM_EXTENSIONS}..." 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
            # Split comma-separated list and install each extension
            IFS=',' read -ra EXTENSIONS <<< "$CUSTOM_EXTENSIONS"
            for ext in "${EXTENSIONS[@]}"; do
                # Trim whitespace
                ext=$(echo "$ext" | xargs)
                if [[ -n "$ext" ]]; then
                    print_status "Installing extension: ${ext}..." 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
                    sudo -u "$DEVELOPER_USER" --preserve-env="${env_vars}" sindri extension install "$ext" --yes 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
                fi
            done
        else
            # No profile specified - this is a valid state
            print_status "No extensions profile configured." 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
            print_status "Run 'sindri profile list' to see available profiles." 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
            touch "$bootstrap_marker"
            chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$bootstrap_marker"
            return 0
        fi

        # Capture exit code before checking it
        local exit_code=$?

        # Mark as complete if successful
        if [[ $exit_code -eq 0 ]]; then
            touch "$bootstrap_marker"
            chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$bootstrap_marker"
            print_success "Extension installation complete" 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
            echo "✅ Extension installation complete. View log: sindri extension log"
        else
            print_error "Extension installation failed (exit code: ${exit_code})" 2>&1 | sudo -u "$DEVELOPER_USER" tee -a "$install_log" > /dev/null
            echo "❌ Extension installation failed. View errors: sindri extension log -l error"
        fi
    ) &

    print_status "Extension installation running in background (PID: $!)"
    print_status "Monitor progress: sindri extension log -f"
}

# ------------------------------------------------------------------------------
# start_ssh_daemon - Start OpenSSH server
# ------------------------------------------------------------------------------
start_ssh_daemon() {
    print_status "Starting SSH daemon on port ${SSH_PORT}..."

    # Ensure SSH directory exists
    mkdir -p /run/sshd

    # Start SSH daemon
    /usr/sbin/sshd -D -p "${SSH_PORT}" -e &
    local sshd_pid=$!

    # Verify SSH is running
    sleep 1
    if kill -0 $sshd_pid 2>/dev/null; then
        print_success "SSH daemon started (PID: $sshd_pid, Port: ${SSH_PORT})"
    else
        print_error "Failed to start SSH daemon"
        exit 1
    fi
}

# ==============================================================================
# Main Execution
# ==============================================================================

print_status "========================================="
print_status "Sindri v3 Container Initialization"
print_status "========================================="

# Step 1: Setup home directory and user environment
setup_home_directory

# Step 2: Setup SINDRI environment variables for SSH sessions
setup_sindri_environment

# Step 3: Configure SSH keys
setup_ssh_keys

# Step 4: Persist SSH host keys for stable fingerprints
persist_ssh_host_keys

# Step 5: Configure Git
setup_git_config

# Step 6: Install extensions in background (non-blocking)
install_extensions_background

# Step 7: Start SSH daemon (foreground if not CI mode)
if [[ "${CI_MODE:-false}" != "true" ]]; then
    start_ssh_daemon

    print_success "========================================="
    print_success "Sindri v3 initialization complete!"
    print_success "SSH available on port ${SSH_PORT}"
    print_success "========================================="

    # Keep container running
    wait
else
    print_status "Running in CI mode - SSH daemon not started"
    print_status "Use 'flyctl ssh console' to access the shell"

    # Keep container running in CI mode
    tail -f /dev/null
fi
