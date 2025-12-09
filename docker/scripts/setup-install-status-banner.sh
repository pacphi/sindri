#!/bin/bash
# ==============================================================================
# Setup Installation Status Banner
# ==============================================================================
# Creates a profile.d script that displays installation status when users
# log in via SSH. This informs users if extensions are still being installed.
#
# The banner checks $WORKSPACE/.system/install-status for:
#   - "installing" - Installation in progress
#   - "complete"   - Installation finished successfully
#   - "failed"     - Installation encountered errors
# ==============================================================================

set -e

PROFILE_SCRIPT="/etc/profile.d/sindri-install-status.sh"

echo "Setting up installation status banner..."

cat > "$PROFILE_SCRIPT" << 'PROFILE_EOF'
#!/bin/bash
# Sindri Installation Status Banner
# Displays installation status on SSH login

# Only run for interactive shells
[[ $- == *i* ]] || return 0

# Define paths
INSTALL_STATUS_FILE="${WORKSPACE:-/alt/home/developer/workspace}/.system/install-status"
INSTALL_LOG_FILE="${WORKSPACE:-/alt/home/developer/workspace}/.system/logs/install.log"

# Color codes
YELLOW='\033[1;33m'
RED='\033[1;31m'
RESET='\033[0m'

# Check installation status
if [[ -f "$INSTALL_STATUS_FILE" ]]; then
    status=$(cat "$INSTALL_STATUS_FILE" 2>/dev/null)

    case "$status" in
        installing)
            echo ""
            echo -e "${YELLOW}╔════════════════════════════════════════════════════════════════════╗${RESET}"
            echo -e "${YELLOW}║                                                                    ║${RESET}"
            echo -e "${YELLOW}║  ⏳ EXTENSION INSTALLATION IN PROGRESS                             ║${RESET}"
            echo -e "${YELLOW}║                                                                    ║${RESET}"
            echo -e "${YELLOW}╠════════════════════════════════════════════════════════════════════╣${RESET}"
            echo -e "${YELLOW}║                                                                    ║${RESET}"
            echo -e "${YELLOW}║  Extensions are being installed in the background.                ║${RESET}"
            echo -e "${YELLOW}║  Some tools may not be available until installation completes.    ║${RESET}"
            echo -e "${YELLOW}║                                                                    ║${RESET}"
            echo -e "${YELLOW}║  Monitor progress:                                                ║${RESET}"
            echo -e "${YELLOW}║    tail -f \$WORKSPACE/.system/logs/install.log                    ║${RESET}"
            echo -e "${YELLOW}║                                                                    ║${RESET}"
            echo -e "${YELLOW}║  Check status:                                                    ║${RESET}"
            echo -e "${YELLOW}║    cat \$WORKSPACE/.system/install-status                          ║${RESET}"
            echo -e "${YELLOW}║                                                                    ║${RESET}"
            echo -e "${YELLOW}╚════════════════════════════════════════════════════════════════════╝${RESET}"
            echo ""
            ;;
        failed)
            echo ""
            echo -e "${RED}╔════════════════════════════════════════════════════════════════════╗${RESET}"
            echo -e "${RED}║                                                                    ║${RESET}"
            echo -e "${RED}║  ❌ EXTENSION INSTALLATION FAILED                                  ║${RESET}"
            echo -e "${RED}║                                                                    ║${RESET}"
            echo -e "${RED}╠════════════════════════════════════════════════════════════════════╣${RESET}"
            echo -e "${RED}║                                                                    ║${RESET}"
            echo -e "${RED}║  Check the installation log for details:                          ║${RESET}"
            echo -e "${RED}║    cat \$WORKSPACE/.system/logs/install.log                        ║${RESET}"
            echo -e "${RED}║                                                                    ║${RESET}"
            echo -e "${RED}║  To retry installation:                                           ║${RESET}"
            echo -e "${RED}║    extension-manager install-profile \$INSTALL_PROFILE            ║${RESET}"
            echo -e "${RED}║                                                                    ║${RESET}"
            echo -e "${RED}╚════════════════════════════════════════════════════════════════════╝${RESET}"
            echo ""
            ;;
        complete)
            # Installation complete - no banner needed
            ;;
        *)
            # Unknown status - ignore
            ;;
    esac
fi
PROFILE_EOF

chmod 644 "$PROFILE_SCRIPT"

echo "Installation status banner configured successfully"
