#!/bin/bash
# setup-ssh-environment.sh - Configure SSH for non-interactive sessions
set -e

echo "Configuring SSH environment..."

PROFILE_D_DIR="/etc/profile.d"
SSHD_CONFIG_D="/etc/ssh/sshd_config.d"
ENV_CONFIG_FILE="$SSHD_CONFIG_D/99-bash-env.conf"
SSH_ENV_FILE="$PROFILE_D_DIR/00-ssh-environment.sh"
CLI_PATH_FILE="$PROFILE_D_DIR/sindri-cli.sh"
MISE_ENV_FILE="$PROFILE_D_DIR/sindri-mise.sh"

# Get ALT_HOME from environment or use default
# This must match the Dockerfile ARG/ENV value
ALT_HOME="${ALT_HOME:-/alt/home/developer}"

# Create sshd_config.d directory if it doesn't exist
if [[ ! -d "$SSHD_CONFIG_D" ]]; then
    mkdir -p "$SSHD_CONFIG_D"
    echo "  Created $SSHD_CONFIG_D"
fi

# Create the SSH environment initialization script
echo "  Creating SSH environment initialization script..."
cat > "$SSH_ENV_FILE" << 'EOF'
#!/bin/bash
# SSH environment initialization for non-interactive sessions
# Used via BASH_ENV for non-interactive SSH commands
#
# This script is placed in /etc/profile.d/ but is primarily used via BASH_ENV
# for non-interactive SSH sessions. When sourced during a login shell,
# /etc/profile has already sourced bash.bashrc, so we skip redundant sourcing.

# Guard against re-entry (prevents infinite recursion)
[ -n "$__SSH_ENV_LOADED" ] && return 0
export __SSH_ENV_LOADED=1

# Skip if we're in a login shell (profile.d sourcing) - bash.bashrc already loaded
# Only source bash.bashrc when used as BASH_ENV for non-interactive sessions
if [ -z "$PS1" ] && [ -n "$BASH_ENV" ]; then
    [ -f /etc/bash.bashrc ] && . /etc/bash.bashrc
fi

# Source profile.d scripts (excluding this file to prevent recursion)
# Only needed for BASH_ENV usage; login shells already source profile.d via /etc/profile
if [ -z "$PS1" ]; then
    for script in /etc/profile.d/*.sh; do
        [ "$script" = "/etc/profile.d/00-ssh-environment.sh" ] && continue
        [ -r "$script" ] && . "$script"
    done
fi

# Source user's bashrc if it exists (for non-interactive BASH_ENV usage)
if [ -z "$PS1" ] && [ -n "$HOME" ] && [ -f "$HOME/.bashrc" ]; then
    . "$HOME/.bashrc"
fi
EOF

chmod +x "$SSH_ENV_FILE"
echo "  Created SSH environment script: $SSH_ENV_FILE"

# Add BASH_ENV configuration for non-interactive SSH sessions
echo "  Creating SSH daemon environment configuration..."
cat > "$ENV_CONFIG_FILE" << EOF
# Configure BASH_ENV for non-interactive SSH sessions
Match User *
    SetEnv BASH_ENV=$SSH_ENV_FILE
EOF

echo "  Created SSH daemon environment config: $ENV_CONFIG_FILE"

# Add CLI tools and mise shims to PATH for all login shells (SSH sessions)
# Dockerfile ENV PATH only works for docker exec, not SSH
# This ensures SSH sessions have the same PATH as docker exec
echo "  Creating CLI PATH configuration..."
cat > "$CLI_PATH_FILE" << EOF
# Sindri PATH configuration for SSH sessions
# Mirrors the Docker ENV PATH to ensure consistency
# Order matters: CLI tools, workspace bin, mise shims, then system paths
export PATH="/docker/cli:${ALT_HOME}/workspace/bin:${ALT_HOME}/.local/share/mise/shims:\$PATH"
EOF
chmod +x "$CLI_PATH_FILE"
echo "  Created CLI PATH config: $CLI_PATH_FILE"

# Create mise environment configuration for SSH sessions
# These variables are critical for mise to work correctly:
# - MISE_YES: auto-accept prompts (trust, install confirmations)
# - MISE_TRUSTED_CONFIG_PATHS: auto-trust extension config files in conf.d
# - MISE_DATA_DIR etc: point to persistent volume locations
echo "  Creating mise environment configuration..."
cat > "$MISE_ENV_FILE" << EOF
# Sindri mise environment for SSH sessions
# Mirrors the Docker ENV variables that SSH doesn't inherit
# Required for mise tool manager to function correctly after restarts

# Sindri home and workspace directories (on persistent volume)
# ALT_HOME is the volume mount point, WORKSPACE is the main working area
export ALT_HOME="${ALT_HOME}"
export WORKSPACE="${ALT_HOME}/workspace"
export DOCKER_LIB="/docker/lib"

# Auto-accept all mise prompts (trust, install confirmations)
# Without this, mise may hang waiting for input in non-interactive shells
export MISE_YES=1

# Auto-trust extension config files installed to conf.d
# Without this, mise ignores configs and tools aren't activated
export MISE_TRUSTED_CONFIG_PATHS="${ALT_HOME}/.config/mise:${ALT_HOME}/.config/mise/conf.d"

# XDG directories for mise data (all on persistent volume)
export MISE_DATA_DIR="${ALT_HOME}/.local/share/mise"
export MISE_CONFIG_DIR="${ALT_HOME}/.config/mise"
export MISE_CACHE_DIR="${ALT_HOME}/.cache/mise"
export MISE_STATE_DIR="${ALT_HOME}/.local/state/mise"
EOF
chmod +x "$MISE_ENV_FILE"
echo "  Created mise environment config: $MISE_ENV_FILE"

# Create TMPDIR configuration for Claude Code plugin compatibility
# Prevents EXDEV error by keeping temp files on same filesystem as ~/.claude
# See: https://github.com/anthropics/claude-code/issues/14799
TMPDIR_FILE="$PROFILE_D_DIR/sindri-tmpdir.sh"
echo "  Creating TMPDIR configuration for Claude Code..."
cat > "$TMPDIR_FILE" << EOF
# Claude Code plugin compatibility
# Set TMPDIR to persistent volume to avoid EXDEV cross-device link errors
# when installing plugins (fs.rename() cannot cross filesystem boundaries)
# See: https://github.com/anthropics/claude-code/issues/14799
export TMPDIR="${ALT_HOME}/.cache/tmp"
EOF
chmod +x "$TMPDIR_FILE"
echo "  Created TMPDIR config: $TMPDIR_FILE"

echo "SSH environment configured successfully"
