#!/bin/bash
set -euo pipefail

# mise-config install script - Simplified for YAML-driven architecture
# Creates global mise configuration

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Configuring mise..."

# Create mise config directory
mkdir -p ~/.config/mise/conf.d

# Create global mise config
cat > ~/.config/mise/config.toml << 'EOF'
[settings]
experimental = true
always_keep_download = false
always_keep_install = false
plugin_autoupdate_last_check_duration = "7d"
jobs = 4

[env]
MISE_USE_TOML = "1"
EOF

print_success "mise configuration complete"
