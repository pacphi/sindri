#!/bin/bash
set -euo pipefail

# mise-config install script - Simplified for YAML-driven architecture
# Creates global mise configuration

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

# Ensure mise auto-accepts prompts (even for trust commands)
# These are set here as explicit exports to ensure they're available in all contexts
export MISE_YES=1
export MISE_TRUSTED_CONFIG_PATHS="${HOME}/.config/mise:${HOME}/.config/mise/conf.d"

print_status "Configuring mise..."

# Create mise config directory
mkdir -p ~/.config/mise/conf.d

# Get home directory for trusted_config_paths
MISE_HOME="${HOME:-/alt/home/developer}"

# Create global mise config
# - yes = true: auto-accept all prompts (trust, install confirmations)
# - trusted_config_paths: auto-trust extension configs in conf.d
cat > ~/.config/mise/config.toml << EOF
[settings]
experimental = true
always_keep_download = false
always_keep_install = false
plugin_autoupdate_last_check_duration = "7d"
jobs = 4
# Auto-accept prompts for CI/automated environments
yes = true
# Auto-trust extension config files
trusted_config_paths = ["${MISE_HOME}/.config/mise/conf.d"]

[env]
MISE_USE_TOML = "1"
EOF

# Trust the config file so mise can read it (required even for global configs in some scenarios)
# This resolves chicken-and-egg problem: config has yes=true but mise won't read untrusted config
mise trust ~/.config/mise/config.toml 2>/dev/null || true

# Also trust the conf.d directory for extension configs
mise trust ~/.config/mise/conf.d 2>/dev/null || true

print_success "mise configuration complete"
