#!/bin/bash
# generate-bom-report.sh - Generate comprehensive Bill of Materials report
#
# This script generates a detailed BOM report showing:
# - System information (OS, disk, memory)
# - mise-managed tools and versions
# - Extension installation status
#
# Usage:
#   ./generate-bom-report.sh
#
# Output: Text report to stdout

set -euo pipefail

# ============================================================================
# HEADER SECTION
# ============================================================================

echo "=== Sindri VM Bill of Materials ==="
echo "Generated: $(date)"
echo "Hostname: $(hostname)"
echo ""

# ============================================================================
# SYSTEM INFORMATION
# ============================================================================

echo "=== System Information ==="
echo ""

# Operating system and kernel
echo "OS/Kernel:"
uname -a
echo ""

# Disk space
echo "Disk Space:"
df -h / | tail -1
echo ""

# Memory
echo "Memory:"
free -h | grep "Mem:"
echo ""

# ============================================================================
# MISE-MANAGED TOOLS
# ============================================================================

echo "=== mise-Managed Tools ==="
echo ""

if command -v mise >/dev/null 2>&1; then
    mise ls
else
    echo "mise not installed"
fi
echo ""

# ============================================================================
# EXTENSION STATUS REPORT
# ============================================================================

echo "=== Extension Status Report ==="
echo ""

# Determine the correct path to extension-manager.sh
if [[ -f "/workspace/scripts/lib/extension-manager.sh" ]]; then
    # On VM
    EXTENSION_MANAGER="/workspace/scripts/lib/extension-manager.sh"
elif [[ -f "$(dirname "$0")/lib/extension-manager.sh" ]]; then
    # In repository (relative to this script)
    EXTENSION_MANAGER="$(dirname "$0")/lib/extension-manager.sh"
else
    echo "ERROR: extension-manager.sh not found"
    echo "Searched locations:"
    echo "  - /workspace/scripts/lib/extension-manager.sh"
    echo "  - $(dirname "$0")/lib/extension-manager.sh"
    exit 1
fi

# Run extension status report
bash "$EXTENSION_MANAGER" status-all
