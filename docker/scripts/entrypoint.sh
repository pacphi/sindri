#!/bin/bash
set -e

echo "🚀 Starting Sindri..."

# Ensure workspace exists and has correct permissions
echo "📁 Setting up workspace and developer home..."
if [ ! -d "/workspace" ]; then
    mkdir -p /workspace
fi

# Create developer home directory on persistent volume if it doesn't exist
if [ ! -d "/workspace/developer" ]; then
    echo "🏠 Creating developer home directory on persistent volume..."
    mkdir -p /workspace/developer
    # Copy skeleton files from /etc/skel
    if [ -d "/etc/skel" ]; then
        cp -r /etc/skel/. /workspace/developer/
    fi
    chown -R developer:developer /workspace/developer
    chmod 755 /workspace/developer
    echo "✅ Developer home directory created at /workspace/developer"
fi

# Update the user's home directory to point to persistent volume
echo "🔧 Updating user home directory..."
usermod -d /workspace/developer developer

# Ensure correct ownership of workspace
chown developer:developer /workspace
chmod 755 /workspace

# Create essential directories in workspace if they don't exist
sudo -u developer mkdir -p /workspace/projects
sudo -u developer mkdir -p /workspace/scripts
sudo -u developer mkdir -p /workspace/backups

# Configure SSH keys from environment variable
if [ -n "$AUTHORIZED_KEYS" ]; then
    echo "🔑 Configuring SSH keys..."
    mkdir -p /workspace/developer/.ssh
    echo "$AUTHORIZED_KEYS" > /workspace/developer/.ssh/authorized_keys
    chown -R developer:developer /workspace/developer/.ssh
    chmod 700 /workspace/developer/.ssh
    chmod 600 /workspace/developer/.ssh/authorized_keys
    echo "✅ SSH keys configured"
else
    echo "⚠️  No SSH keys found in AUTHORIZED_KEYS environment variable"
fi

# Copy lib directory if it doesn't exist
if [ ! -d "/workspace/scripts/lib" ]; then
    cp -r /docker/lib /workspace/scripts/
    chown -R developer:developer /workspace/scripts/lib
    chmod +x /workspace/scripts/lib/*.sh
fi

# Create /workspace/bin directory and symlink extension-manager
if [ ! -d "/workspace/bin" ]; then
    mkdir -p /workspace/bin
    chown developer:developer /workspace/bin
fi

# Create symlink for extension-manager if script exists and symlink doesn't
if [ -f "/workspace/scripts/lib/extension-manager.sh" ] && [ ! -L "/workspace/bin/extension-manager" ]; then
    ln -sf /workspace/scripts/lib/extension-manager.sh /workspace/bin/extension-manager
    chown -h developer:developer /workspace/bin/extension-manager
fi

# Set up environment variables for developer user
if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo "export ANTHROPIC_API_KEY='$ANTHROPIC_API_KEY'" >> /workspace/developer/.bashrc
fi

# Configure GitHub token if provided
if [ -n "$GITHUB_TOKEN" ]; then
    echo "🔐 Configuring GitHub authentication..."
    echo "export GITHUB_TOKEN='$GITHUB_TOKEN'" >> /workspace/developer/.bashrc

    # Create GitHub CLI config for gh commands
    sudo -u developer mkdir -p /workspace/developer/.config/gh
    echo "github.com:" > /workspace/developer/.config/gh/hosts.yml
    echo "    oauth_token: $GITHUB_TOKEN" >> /workspace/developer/.config/gh/hosts.yml
    echo "    user: $GITHUB_USER" >> /workspace/developer/.config/gh/hosts.yml
    echo "    git_protocol: https" >> /workspace/developer/.config/gh/hosts.yml
    chown -R developer:developer /workspace/developer/.config/gh
    chmod 600 /workspace/developer/.config/gh/hosts.yml
fi

# Configure Git credentials if provided
if [ -n "$GIT_USER_NAME" ]; then
    sudo -u developer git config --global user.name "$GIT_USER_NAME"
    echo "✅ Git user name configured: $GIT_USER_NAME"
fi

if [ -n "$GIT_USER_EMAIL" ]; then
    sudo -u developer git config --global user.email "$GIT_USER_EMAIL"
    echo "✅ Git user email configured: $GIT_USER_EMAIL"
fi

# Setup Git credential helper for GitHub token
if [ -n "$GITHUB_TOKEN" ]; then
    # Create credential helper script
    cat > /workspace/developer/.git-credential-helper.sh << 'EOF'
#!/bin/bash
# Git credential helper for GitHub token authentication

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
            host=*)
                # For non-GitHub hosts, exit without providing credentials
                exit 0
                ;;
        esac
    done
fi
EOF

    chmod +x /workspace/developer/.git-credential-helper.sh
    chown developer:developer /workspace/developer/.git-credential-helper.sh

    # Configure Git to use the credential helper
    sudo -u developer git config --global credential.helper "/workspace/developer/.git-credential-helper.sh"
    echo "✅ GitHub token authentication configured"
fi

# Setup Message of the Day (MOTD)
if [ -f "/docker/scripts/setup-motd.sh" ]; then
    echo "📋 Setting up MOTD banner..."
    bash /docker/scripts/setup-motd.sh
fi

# Start SSH daemon (check for CI mode)
if [ "$CI_MODE" = "true" ]; then
    echo "🔌 CI Mode: Skipping SSH daemon startup (using Fly.io hallpass)"
    echo "🎯 Sindri is ready (CI Mode)!"
    echo "📡 SSH access available via flyctl ssh console"
    echo "🏠 Workspace mounted at /workspace"
else
    echo "🔌 Starting SSH daemon on port ${SSH_PORT:-2222}..."
    mkdir -p /var/run/sshd
    /usr/sbin/sshd -D &

    echo "🎯 Sindri is ready!"
    echo "📡 SSH server listening on port ${SSH_PORT:-2222}"
    echo "🏠 Workspace mounted at /workspace"
fi

# Handle shutdown gracefully
trap 'echo "📴 Shutting down..."; kill $(jobs -p); exit 0' SIGTERM SIGINT

# Wait for SSH daemon (only if running)
if [ "$CI_MODE" != "true" ]; then
    wait $!
else
    # In CI mode, just keep container running
    sleep infinity
fi