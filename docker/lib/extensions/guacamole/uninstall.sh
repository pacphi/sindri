#!/bin/bash
# Guacamole uninstallation script
# Sindri Extension API v2.0

set -euo pipefail

# Source common functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
source "$(dirname "$(dirname "$SCRIPT_DIR")")/common.sh" 2>/dev/null || {
    echo "Error: Cannot source common.sh"
    exit 1
}

# ============================================================================
# STOP SERVICES
# ============================================================================

stop_services() {
    print_status "Stopping Guacamole services..."

    # Stop guacd
    if sudo systemctl is-active --quiet guacd 2>/dev/null; then
        sudo systemctl stop guacd || true
    fi

    # Stop tomcat
    if sudo systemctl is-active --quiet tomcat9 2>/dev/null; then
        sudo systemctl stop tomcat9 || true
    fi

    # Disable services
    sudo systemctl disable guacd 2>/dev/null || true
    sudo systemctl disable tomcat9 2>/dev/null || true

    print_success "Services stopped"
}

# ============================================================================
# REMOVE GUACD
# ============================================================================

remove_guacd() {
    print_status "Removing guacd..."

    # Remove guacd binary
    sudo rm -f /usr/local/sbin/guacd
    sudo rm -f /usr/local/bin/guacd

    # Remove shared libraries
    sudo rm -rf /usr/local/lib/freerdp2/
    sudo rm -f /usr/local/lib/libguac*.so*

    # Remove init script and systemd service
    sudo rm -f /etc/init.d/guacd
    sudo rm -f /etc/systemd/system/guacd.service

    # Update library cache
    sudo ldconfig

    print_success "guacd removed"
}

# ============================================================================
# REMOVE TOMCAT (via mise)
# ============================================================================

remove_tomcat() {
    print_status "Removing Tomcat..."

    # Remove systemd service
    sudo rm -f /etc/systemd/system/tomcat9.service
    sudo systemctl daemon-reload

    # Remove compatibility symlinks
    sudo rm -rf /var/lib/tomcat9
    sudo rm -rf /usr/share/tomcat9

    # Remove tomcat user/group (optional, may be needed by other things)
    # sudo userdel tomcat 2>/dev/null || true
    # sudo groupdel tomcat 2>/dev/null || true

    # Note: Tomcat itself is removed via mise (handled by extension manager)
    print_status "Tomcat symlinks and service removed"
    print_status "Tomcat binary will be removed by mise"

    print_success "Tomcat cleanup complete"
}

# ============================================================================
# REMOVE CONFIGURATION
# ============================================================================

remove_config() {
    print_status "Removing Guacamole configuration..."

    # Remove guacamole config directory
    sudo rm -rf /etc/guacamole

    print_success "Configuration removed"
}

# ============================================================================
# REMOVE ENVIRONMENT VARIABLES
# ============================================================================

remove_environment() {
    print_status "Cleaning up environment variables..."

    # Remove from bashrc if present
    if [[ -f "$HOME/.bashrc" ]]; then
        sed -i '/GUACAMOLE_HOME/d' "$HOME/.bashrc" 2>/dev/null || true
        sed -i '/CATALINA_HOME/d' "$HOME/.bashrc" 2>/dev/null || true
    fi

    print_success "Environment cleaned"
}

# ============================================================================
# MAIN
# ============================================================================

main() {
    print_status "Uninstalling Apache Guacamole..."
    echo ""

    stop_services
    remove_guacd
    remove_tomcat
    remove_config
    remove_environment

    print_success "Apache Guacamole uninstallation complete!"
    echo ""
    print_warning "Note: Java (default-jdk) was not removed as it may be used by other applications"
    print_warning "Note: Build dependencies were not removed to avoid breaking other packages"
    echo ""
}

main "$@"
