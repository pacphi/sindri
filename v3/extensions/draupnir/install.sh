#!/usr/bin/env bash
set -euo pipefail

# Install draupnir - Sindri instance agent for mimir fleet management
curl -fsSL https://raw.githubusercontent.com/pacphi/draupnir/main/extension/install.sh | bash
