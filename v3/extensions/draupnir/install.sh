#!/usr/bin/env bash
set -euo pipefail

# Install draupnir - Sindri instance agent for mimir fleet management
# $1 = version (from extension.yaml args)

if [ -n "${1:-}" ]; then
  export DRAUPNIR_VERSION="$1"
fi

curl -fsSL https://raw.githubusercontent.com/pacphi/draupnir/main/extension/install.sh | bash
