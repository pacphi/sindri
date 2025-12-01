#!/bin/bash
# setup-ssh-environment.sh - Configure SSH for non-interactive sessions
# Ensures environment variables and tool paths are available in SSH commands

set -e

echo "Configuring SSH environment for non-interactive sessions..."

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

# Source system-wide bash configuration
[ -f /etc/bashrc ] && . /etc/bashrc
[ -f /etc/bash.bashrc ] && . /etc/bash.bashrc

# Source all profile.d scripts
for script in /etc/profile.d/*.sh; do
    [ -r "$script" ] && . "$script"
done

# Source user's bashrc if it exists
if [ -n "$HOME" ] && [ -f "$HOME/.bashrc" ]; then
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
echo "SSH environment configured successfully"
