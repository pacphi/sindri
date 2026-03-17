#!/bin/bash
set -euo pipefail

# mise-config install script - Simplified for YAML-driven architecture
# Creates global mise configuration

# Fallback print functions if BASH_ENV didn't load common.sh
# (e.g., common.sh not found at expected path on first boot)
if ! declare -f print_status >/dev/null 2>&1; then
    print_status() { echo "[INFO] $*"; }
    print_success() { echo "[OK] $*"; }
    print_error() { echo "[ERROR] $*" >&2; }
fi

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
# - npm section: uses npm (default) for mise's npm: backend (pnpm v10 blocks native build scripts)
cat > ~/.config/mise/config.toml << EOF
[settings]
experimental = true
always_keep_download = false
always_keep_install = false
plugin_autoupdate_last_check_duration = "7d"
# Serial installation to avoid npm registry rate limits
jobs = 1
# Increase timeouts for npm registry (defaults: http_timeout=30s, fetch_remote_versions_timeout=20s)
# 180s handles slow npm registry responses and rate limiting
http_timeout = "180s"
fetch_remote_versions_timeout = "180s"
# Auto-accept prompts for CI/automated environments
yes = true
# Auto-trust extension config files
trusted_config_paths = ["${MISE_HOME}/.config/mise/conf.d"]

[settings.npm]
# Use npm (default) for mise's npm: backend package installations.
# pnpm v10 blocks native build scripts (better-sqlite3, hnswlib-node, etc.)
# with no viable workaround for mise's global install context.
# npm runs all build scripts and is the most compatible choice.
# Note: project-level pnpm is unaffected — corepack provides it via PATH.

[settings.python]
# Force precompiled binaries only (no compilation)
# Required for environments with /tmp mounted noexec (security hardening)
# Falls back to closest precompiled version if exact version unavailable
compile = false

[env]
MISE_USE_TOML = "1"
# Enable corepack — makes pnpm/yarn available for project-level use
MISE_NODE_COREPACK = "1"
# npm timeout configuration (in milliseconds)
# These apply globally to all npm-based tool installations
npm_config_fetch_timeout = "300000"
npm_config_fetch_retries = "2"
npm_config_fetch_retry_mintimeout = "10000"
npm_config_fetch_retry_maxtimeout = "60000"
npm_config_maxsockets = "10"
npm_config_prefer_offline = "true"
EOF

# Trust the config file so mise can read it (required even for global configs in some scenarios)
# This resolves chicken-and-egg problem: config has yes=true but mise won't read untrusted config
mise trust ~/.config/mise/config.toml 2>/dev/null || true

# Also trust the conf.d directory for extension configs
mise trust ~/.config/mise/conf.d 2>/dev/null || true

print_success "mise configuration complete"
