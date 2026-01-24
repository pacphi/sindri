#!/bin/bash
# setup-ssh-environment.sh - Configure SSH environment for Sindri v3
#
# Creates /etc/profile.d/ scripts for SSH session environment configuration.
# This ensures environment variables are set for all login shells (SSH, su - user).
set -e

echo "Configuring SSH environment..."

PROFILE_D_DIR="/etc/profile.d"

# Get ALT_HOME from environment or use default
# This must match the Dockerfile ARG/ENV value
ALT_HOME="${ALT_HOME:-/alt/home/developer}"

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
