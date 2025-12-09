#!/bin/bash
# Create .npmrc in /etc/skel so it gets copied to the persistent home
# This configures npm defaults for all users
set -e

cat > /etc/skel/.npmrc << 'EOF'
# npm configuration for Sindri development environment
#
# Suppress misleading "Access token expired" notices from npm registry
# These notices appear on 404 responses due to npm registry server-side changes
# and are not actual authentication failures.
#
# Tracking issues:
#   - https://github.com/npm/cli/issues/8816
#   - https://github.com/rollup/rollup/issues/6204
#
loglevel=warn

# Security settings
audit-level=moderate

# Disable funding messages
fund=false

# Performance - prefer cached packages when possible
prefer-offline=true

# Default registry
registry=https://registry.npmjs.org/
EOF

chmod 644 /etc/skel/.npmrc
