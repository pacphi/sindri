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

SINDRI_HOME="${HOME}/.sindri"
BOOTSTRAP_MARKER="${SINDRI_HOME}/bootstrap-complete"
INSTALL_LOG="${SINDRI_HOME}/logs/install.log"
STATUS_SHOWN_MARKER="${SINDRI_HOME}/.status-shown-${RANDOM}"

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
    # Installation complete - show brief success message (only once per session)
    if [[ ! -f "${SINDRI_HOME}/.login-notified" ]]; then
        echo ""
        echo "✅ Extensions ready! Run 'sindri extension list --installed' to see all installed tools."
        echo ""
        touch "${SINDRI_HOME}/.login-notified"
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
