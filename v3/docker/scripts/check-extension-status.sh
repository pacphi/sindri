#!/bin/bash
# ==============================================================================
# Extension Installation Status Check
# ==============================================================================
# Displays extension installation status when user logs in.
# Integrated into /etc/profile.d/ for automatic execution on shell startup.
# ==============================================================================

# Only run for interactive shells
case $- in
    *i*) ;;
      *) return 0;;
esac

# CRITICAL: Use ALT_HOME if set (volume-mounted location where extensions are installed)
# On login shell, HOME=/home/developer but extensions are at /alt/home/developer
# This ensures we check the correct location where auto-install wrote the data
SINDRI_HOME="${ALT_HOME:-${HOME}}/.sindri"
BOOTSTRAP_MARKER="${SINDRI_HOME}/bootstrap-complete"
INSTALL_LOG="${SINDRI_HOME}/logs/install.log"
LEDGER="${SINDRI_HOME}/status_ledger.jsonl"

# Skip if extensions are disabled
if [[ "${SKIP_AUTO_INSTALL:-false}" == "true" ]]; then
    return 0
fi

# Skip if already shown in this session (prevent duplicate on nested shells)
if [[ -n "${SINDRI_STATUS_SHOWN:-}" ]]; then
    return 0
fi
export SINDRI_STATUS_SHOWN=1

# Check extension installation status
if [[ -f "$BOOTSTRAP_MARKER" ]]; then
    # Bootstrap marker exists - verify extensions are actually installed
    if [[ -f "$LEDGER" ]]; then
        # Check if ledger has any install_completed events
        # shellcheck disable=SC2312
        if grep -q '"install_completed"' "$LEDGER" 2>/dev/null; then
            # Extensions are installed (at least one install_completed event)
            if [[ ! -f "${SINDRI_HOME}/.login-notified" ]]; then
                echo ""
                echo "✅ Extensions ready! Run 'sindri extension list --installed' to see all installed tools."
                echo ""
                touch "${SINDRI_HOME}/.login-notified"
            fi
        else
            # Ledger exists but no completed installations
            # Check if installation is in progress
            if pgrep -f "sindri.*install" > /dev/null 2>&1; then
                echo ""
                echo "⏳ Extension installation in progress..."
                echo "   Monitor: tail -f ~/.sindri/logs/install.log"
                echo ""
            else
                # No installation running and no extensions installed
                echo ""
                echo "ℹ️  No extensions installed yet"
                echo "   Install a profile: sindri profile install <profile> --yes"
                echo "   Or install individual: sindri extension install <name>"
                echo ""
            fi
        fi
    else
        # Marker exists but no ledger
        echo ""
        echo "⚠️  Extension marker found but status ledger is missing"
        echo "   Retry installation: sindri profile install <profile> --yes"
        echo ""
    fi
elif [[ -f "$INSTALL_LOG" ]]; then
    # Check if installation is in progress
    if pgrep -f "sindri.*install" > /dev/null 2>&1; then
        echo ""
        echo "⏳ Extension installation in progress..."
        echo "   Monitor: tail -f ~/.sindri/logs/install.log"
        echo "   Status:  sindri extension status"
        echo ""
    else
        # Installation finished but marker not created - check for errors
        # shellcheck disable=SC2312
        if tail -20 "$INSTALL_LOG" 2>/dev/null | grep -qi "error\|failed"; then
            echo ""
            echo "❌ Extension installation failed!"
            echo "   Check log: tail ~/.sindri/logs/install.log"
            echo "   Retry:     sindri extension install --from-config sindri.yaml --yes"
            echo ""
        else
            # Likely completed but marker pending (race condition)
            echo ""
            echo "✅ Extension installation appears complete"
            echo "   Verify: sindri extension list --installed"
            echo ""
        fi
    fi
else
    # No log file - installation hasn't started yet
    echo ""
    echo "⏳ Extension installation starting..."
    echo "   Monitor: tail -f ~/.sindri/logs/install.log"
    echo ""
fi
