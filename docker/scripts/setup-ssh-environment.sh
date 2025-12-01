#!/bin/bash
# setup-ssh-environment.sh - Configure SSH for non-interactive sessions
# This ensures environment variables and tool paths are available in SSH commands
# Critical for CI/CD pipelines that execute commands via SSH

set -e

echo "Configuring SSH environment for non-interactive sessions..."

# ==============================================================================
# Environment Configuration
# ==============================================================================
PROFILE_D_DIR="/etc/profile.d"
SSHD_CONFIG_D="/etc/ssh/sshd_config.d"
ENV_CONFIG_FILE="$SSHD_CONFIG_D/99-bash-env.conf"
SSH_ENV_FILE="$PROFILE_D_DIR/00-ssh-environment.sh"

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
# This script is sourced via BASH_ENV for SSH commands

# Source system-wide bash configuration
[ -f /etc/bashrc ] && . /etc/bashrc
[ -f /etc/bash.bashrc ] && . /etc/bash.bashrc

# Source user's bashrc if it exists
if [ -n "$HOME" ] && [ -f "$HOME/.bashrc" ]; then
    . "$HOME/.bashrc"
fi

# Activate mise if available
if command -v mise >/dev/null 2>&1; then
    eval "$(mise activate bash 2>/dev/null)" || true
fi

# Ensure PATH includes common locations
export PATH="/docker/cli:$HOME/workspace/bin:$HOME/.local/share/mise/shims:/usr/local/bin:$PATH"
EOF

chmod +x "$SSH_ENV_FILE"
echo "  Created SSH environment script: $SSH_ENV_FILE"

# Add BASH_ENV configuration for non-interactive SSH sessions
echo "  Creating SSH daemon environment configuration..."
cat > "$ENV_CONFIG_FILE" << EOF
# Configure BASH_ENV for non-interactive SSH sessions
# This allows environment setup for commands executed via SSH
Match User *
    SetEnv BASH_ENV=$SSH_ENV_FILE
EOF

echo "  Created SSH daemon environment config: $ENV_CONFIG_FILE"

# Create mise activation for profile.d (login shells)
MISE_PROFILE="/etc/profile.d/01-mise-activation.sh"
cat > "$MISE_PROFILE" << 'EOF'
#!/bin/bash
# mise activation for interactive and non-interactive sessions
# Ensures mise-managed tools are available in CI/CD pipelines

# Activate mise if available (binary is in /usr/local/bin)
if command -v mise >/dev/null 2>&1; then
    eval "$(mise activate bash 2>/dev/null)" || true
fi
EOF
chmod +x "$MISE_PROFILE"
echo "  Created mise activation profile: $MISE_PROFILE"

echo "SSH environment configured successfully"
echo "  Non-interactive SSH sessions will have full environment"
echo "  Tools installed via mise will be available in CI/CD"
