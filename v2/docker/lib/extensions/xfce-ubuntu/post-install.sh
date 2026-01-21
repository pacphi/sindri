#!/bin/bash
# Post-installation script for xfce-ubuntu extension
# Configures xRDP and XFCE for remote desktop access

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$(dirname "$(dirname "$SCRIPT_DIR")")/common.sh"

# Configure xRDP
configure_xrdp() {
    print_status "Configuring xRDP for XFCE..."

    # Create .xsession file if template was not processed
    if [[ ! -f "$HOME/.xsession" ]]; then
        cat > "$HOME/.xsession" << 'EOF'
#!/bin/sh
unset SESSION_MANAGER
unset DBUS_SESSION_BUS_ADDRESS
exec startxfce4
EOF
        chmod +x "$HOME/.xsession"
        print_success "Created $HOME/.xsession"
    fi

    # Add user to xrdp group
    if ! groups | grep -q xrdp; then
        sudo usermod -aG xrdp "$USER" 2>/dev/null || true
        print_success "Added user to xrdp group"
    fi

    # Enable and start xRDP service
    if command -v systemctl > /dev/null 2>&1; then
        sudo systemctl enable xrdp 2>/dev/null || true
        sudo systemctl start xrdp 2>/dev/null || true
        print_success "xRDP service configured"
    fi
}

# Create desktop directories
setup_desktop_dirs() {
    print_status "Setting up desktop directories..."
    mkdir -p "$HOME/Desktop" "$HOME/Documents" "$HOME/Downloads"
    print_success "Desktop directories created"
}

# Configure firewall for RDP
configure_firewall() {
    if command -v ufw > /dev/null 2>&1; then
        if sudo ufw status | grep -q "Status: active"; then
            print_status "Configuring firewall for RDP..."
            sudo ufw allow 3389/tcp 2>/dev/null || true
            print_success "Firewall configured for RDP (port 3389)"
        fi
    fi
}

# Print connection instructions
print_connection_info() {
    echo ""
    print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_status "RDP CONNECTION SETUP"
    print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "For Fly.io deployment, add to fly.toml:"
    echo ""
    echo "  [[services]]"
    echo "    internal_port = 3389"
    echo "    protocol = \"tcp\""
    echo ""
    echo "    [[services.ports]]"
    echo "      port = 3389"
    echo ""
    echo "Connect with RDP client:"
    echo "  Address: your-app.fly.dev:3389"
    echo "  Username: $USER"
    echo ""
    print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Main execution
main() {
    print_status "Running post-installation setup for XFCE desktop..."

    configure_xrdp
    setup_desktop_dirs
    configure_firewall
    print_connection_info

    print_success "XFCE desktop post-installation complete"
}

main "$@"