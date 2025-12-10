#!/bin/bash
# setup-ssh-environment.sh - Configure SSH for non-interactive sessions
set -e

echo "Configuring SSH environment..."

PROFILE_D_DIR="/etc/profile.d"
SSHD_CONFIG_D="/etc/ssh/sshd_config.d"
ENV_CONFIG_FILE="$SSHD_CONFIG_D/99-bash-env.conf"
SSH_ENV_FILE="$PROFILE_D_DIR/00-ssh-environment.sh"
CLI_PATH_FILE="$PROFILE_D_DIR/sindri-cli.sh"

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

# Add CLI tools to PATH for all login shells (SSH sessions)
# Dockerfile ENV PATH only works for docker exec, not SSH
echo "  Creating CLI PATH configuration..."
cat > "$CLI_PATH_FILE" << 'EOF'
export PATH="/docker/cli:$PATH"
EOF
chmod +x "$CLI_PATH_FILE"
echo "  Created CLI PATH config: $CLI_PATH_FILE"

echo "SSH environment configured successfully"
