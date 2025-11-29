#!/bin/bash
# Guacamole installation script
# Sindri Extension API v2.0
#
# This script installs Apache Guacamole, a clientless remote desktop gateway
# providing browser-based access to SSH, RDP, and VNC sessions.

set -euo pipefail

# Source common functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
source "$(dirname "$(dirname "$SCRIPT_DIR")")/common.sh" 2>/dev/null || {
    echo "Error: Cannot source common.sh"
    exit 1
}

# ============================================================================
# CONFIGURATION
# ============================================================================

GUACAMOLE_VERSION="${1:-1.5.4}"
TEMP_DIR="/tmp/guacamole-install-$$"

# ============================================================================
# PREREQUISITES CHECK
# ============================================================================

check_prerequisites() {
    print_status "Checking prerequisites..."

    # Check for apt-get (Debian/Ubuntu only)
    if ! command -v apt-get >/dev/null 2>&1; then
        print_error "apt-get is required (Debian/Ubuntu only)"
        return 1
    fi

    # Check disk space (need ~1GB)
    local available_mb
    available_mb=$(df /tmp | tail -1 | awk '{print int($4/1024)}')
    if [[ $available_mb -lt 1000 ]]; then
        print_error "Insufficient disk space: ${available_mb}MB available, need 1000MB"
        return 1
    fi

    # Check RAM (warn if less than 1GB)
    if command -v free >/dev/null 2>&1; then
        local total_ram_mb
        total_ram_mb=$(free -m | awk '/^Mem:/ {print $2}')
        if [[ $total_ram_mb -lt 1024 ]]; then
            print_warning "Low RAM: ${total_ram_mb}MB (Tomcat may struggle)"
            print_warning "Recommended: 1GB+ RAM"
        fi
    fi

    print_success "Prerequisites met"
    return 0
}

# ============================================================================
# INSTALL BUILD DEPENDENCIES
# ============================================================================

install_build_dependencies() {
    print_status "Installing build dependencies..."

    local build_deps=(
        build-essential
        libcairo2-dev
        libjpeg-turbo8-dev
        libpng-dev
        libtool-bin
        libossp-uuid-dev
        libavcodec-dev
        libavformat-dev
        libavutil-dev
        libswscale-dev
        freerdp2-dev
        libpango1.0-dev
        libssh2-1-dev
        libtelnet-dev
        libvncserver-dev
        libwebsockets-dev
        libpulse-dev
        libssl-dev
        libvorbis-dev
        libwebp-dev
    )

    # Update package lists
    sudo apt-get update -qq || {
        print_error "Failed to update package lists"
        return 1
    }

    # Install dependencies
    # shellcheck disable=SC2068
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq ${build_deps[@]} || {
        print_error "Failed to install build dependencies"
        return 1
    }

    print_success "Build dependencies installed"
    return 0
}

# ============================================================================
# INSTALL TOMCAT
# ============================================================================

install_tomcat() {
    print_status "Installing Tomcat 9 via mise..."

    # Install Java (required for Tomcat)
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq default-jdk || {
        print_error "Failed to install Java"
        return 1
    }

    # Install Tomcat 9.x via mise (latest 9.x version)
    if ! mise use -g tomcat@9; then
        print_error "Failed to install Tomcat via mise"
        return 1
    fi

    # Get the mise tomcat installation path
    local TOMCAT_DIR
    TOMCAT_DIR="$(mise where tomcat)"

    if [[ -z "$TOMCAT_DIR" || ! -d "$TOMCAT_DIR" ]]; then
        print_error "Tomcat installation directory not found"
        return 1
    fi

    print_status "Tomcat installed at: $TOMCAT_DIR"

    # Create tomcat user and group for service management
    if ! getent group tomcat > /dev/null 2>&1; then
        sudo groupadd tomcat
    fi
    if ! getent passwd tomcat > /dev/null 2>&1; then
        sudo useradd -s /bin/false -g tomcat -d "$TOMCAT_DIR" tomcat
    fi

    # Set ownership for webapps directory
    sudo chown -R tomcat:tomcat "$TOMCAT_DIR/webapps"
    sudo chmod +x "$TOMCAT_DIR"/bin/*.sh

    # Create symlinks for compatibility
    sudo mkdir -p /var/lib/tomcat9
    sudo ln -sf "$TOMCAT_DIR/webapps" /var/lib/tomcat9/webapps
    sudo mkdir -p /usr/share/tomcat9

    # Create systemd service for Tomcat
    sudo tee /etc/systemd/system/tomcat9.service > /dev/null << EOF
[Unit]
Description=Apache Tomcat 9 Web Application Container
After=network.target

[Service]
Type=forking
User=tomcat
Group=tomcat
Environment="JAVA_HOME=/usr/lib/jvm/default-java"
Environment="CATALINA_PID=$TOMCAT_DIR/temp/catalina.pid"
Environment="CATALINA_HOME=$TOMCAT_DIR"
Environment="CATALINA_BASE=$TOMCAT_DIR"
ExecStart=$TOMCAT_DIR/bin/startup.sh
ExecStop=$TOMCAT_DIR/bin/shutdown.sh
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload

    # Export for use by other functions
    export TOMCAT_HOME="$TOMCAT_DIR"

    print_success "Tomcat 9 installed via mise"
    return 0
}

# ============================================================================
# BUILD GUACAMOLE SERVER
# ============================================================================

build_guacamole_server() {
    print_status "Building guacamole-server ${GUACAMOLE_VERSION}..."

    mkdir -p "$TEMP_DIR"
    cd "$TEMP_DIR" || return 1

    # Download source
    local server_url="https://downloads.apache.org/guacamole/${GUACAMOLE_VERSION}/source/guacamole-server-${GUACAMOLE_VERSION}.tar.gz"
    print_status "Downloading from ${server_url}..."

    if ! wget -q "$server_url" -O guacamole-server.tar.gz; then
        print_error "Failed to download guacamole-server"
        return 1
    fi

    # Extract
    if ! tar -xzf guacamole-server.tar.gz; then
        print_error "Failed to extract guacamole-server"
        return 1
    fi

    cd "guacamole-server-${GUACAMOLE_VERSION}" || return 1

    # Configure
    print_status "Configuring (this may take a few minutes)..."
    if ! ./configure \
        --with-init-dir=/etc/init.d \
        --enable-allow-freerdp-snapshots \
        --disable-guaclog \
        > /tmp/guac-configure.log 2>&1; then
        print_error "Failed to configure (see /tmp/guac-configure.log)"
        return 1
    fi

    # Build
    print_status "Compiling (this may take 5-10 minutes)..."
    if ! make -j"$(nproc)" > /tmp/guac-make.log 2>&1; then
        print_error "Failed to compile (see /tmp/guac-make.log)"
        return 1
    fi

    # Install
    print_status "Installing..."
    # shellcheck disable=SC2024  # /tmp is world-writable, redirect is safe
    if ! sudo make install > /tmp/guac-install.log 2>&1; then
        print_error "Failed to install (see /tmp/guac-install.log)"
        return 1
    fi

    sudo ldconfig
    print_success "guacamole-server built and installed"

    return 0
}

# ============================================================================
# INSTALL GUACAMOLE CLIENT
# ============================================================================

install_guacamole_client() {
    print_status "Installing guacamole-client ${GUACAMOLE_VERSION}..."

    cd "$TEMP_DIR" || return 1

    # Download WAR file
    local client_url="https://downloads.apache.org/guacamole/${GUACAMOLE_VERSION}/binary/guacamole-${GUACAMOLE_VERSION}.war"
    print_status "Downloading from ${client_url}..."

    if ! wget -q "$client_url" -O guacamole.war; then
        print_error "Failed to download guacamole-client"
        return 1
    fi

    # Get Tomcat directory from mise (use exported TOMCAT_HOME or query mise)
    local TOMCAT_DIR="${TOMCAT_HOME:-$(mise where tomcat)}"
    if [[ -z "$TOMCAT_DIR" || ! -d "$TOMCAT_DIR" ]]; then
        print_error "Tomcat installation directory not found"
        return 1
    fi

    # Deploy to Tomcat webapps directory
    print_status "Deploying to Tomcat at ${TOMCAT_DIR}..."
    sudo mkdir -p "$TOMCAT_DIR/webapps"
    sudo cp guacamole.war "$TOMCAT_DIR/webapps/"
    sudo chown tomcat:tomcat "$TOMCAT_DIR/webapps/guacamole.war"

    print_success "guacamole-client deployed"
    return 0
}

# ============================================================================
# CREATE CONFIGURATION DIRECTORIES
# ============================================================================

create_config_dirs() {
    print_status "Creating configuration directories..."

    # Get Tomcat directory from mise (use exported TOMCAT_HOME or query mise)
    local TOMCAT_DIR="${TOMCAT_HOME:-$(mise where tomcat)}"
    if [[ -z "$TOMCAT_DIR" || ! -d "$TOMCAT_DIR" ]]; then
        print_error "Tomcat installation directory not found"
        return 1
    fi

    sudo mkdir -p /etc/guacamole
    sudo mkdir -p /etc/guacamole/extensions
    sudo mkdir -p /etc/guacamole/lib
    sudo mkdir -p "$TOMCAT_DIR/.guacamole"

    # Link configuration to Tomcat
    if [[ ! -L "$TOMCAT_DIR/.guacamole/guacamole.properties" ]]; then
        sudo ln -sf /etc/guacamole/guacamole.properties "$TOMCAT_DIR/.guacamole/"
    fi
    sudo chown -R tomcat:tomcat "$TOMCAT_DIR/.guacamole"

    # Also create backward-compatible symlink
    sudo mkdir -p /usr/share/tomcat9
    if [[ ! -L /usr/share/tomcat9/.guacamole ]]; then
        sudo ln -sf "$TOMCAT_DIR/.guacamole" /usr/share/tomcat9/.guacamole
    fi

    print_success "Configuration directories created"
    return 0
}

# ============================================================================
# CREATE SYSTEMD SERVICE
# ============================================================================

create_systemd_service() {
    print_status "Creating guacd systemd service..."

    sudo tee /etc/systemd/system/guacd.service > /dev/null << 'EOF'
[Unit]
Description=Guacamole proxy daemon (guacd)
Documentation=man:guacd(8)
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/sbin/guacd -f
Restart=on-failure
User=daemon
Group=daemon

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    print_success "guacd service created"

    return 0
}

# ============================================================================
# ENABLE AND START SERVICES
# ============================================================================

start_services() {
    print_status "Enabling and starting services..."

    # Enable services
    sudo systemctl enable guacd 2>/dev/null || true
    sudo systemctl enable tomcat9 2>/dev/null || true

    # Start guacd
    if sudo systemctl start guacd 2>/dev/null; then
        print_success "guacd started"
    else
        print_warning "Failed to start guacd (will retry)"
    fi

    # Start Tomcat
    if sudo systemctl restart tomcat9 2>/dev/null; then
        print_success "Tomcat started"
        print_status "Waiting for Guacamole to deploy (30 seconds)..."
        sleep 30
    else
        print_warning "Failed to start Tomcat (check logs)"
    fi

    return 0
}

# ============================================================================
# CLEANUP
# ============================================================================

cleanup() {
    if [[ -d "$TEMP_DIR" ]]; then
        print_status "Cleaning up temporary files..."
        rm -rf "$TEMP_DIR"
    fi
}

# ============================================================================
# MAIN INSTALLATION
# ============================================================================

main() {
    print_status "Installing Apache Guacamole ${GUACAMOLE_VERSION}..."
    print_warning "This will take 10-15 minutes and requires multiple downloads"
    echo ""

    # Trap cleanup on exit
    trap cleanup EXIT

    # Run installation steps
    check_prerequisites || exit 1
    install_build_dependencies || exit 1
    install_tomcat || exit 1
    build_guacamole_server || exit 1
    install_guacamole_client || exit 1
    create_config_dirs || exit 1
    create_systemd_service || exit 1
    start_services || exit 1

    print_success "Apache Guacamole installation complete!"
    echo ""
    print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_status "NEXT STEPS:"
    print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "1. Configuration files will be created via templates"
    echo "2. Default credentials: guacadmin/guacadmin"
    echo "3. Access via: http://localhost:8080/guacamole"
    echo "4. See extension documentation for deployment setup"
    echo ""
    print_warning "⚠ SECURITY: Change default password after first login!"
    echo ""

    return 0
}

# Run main installation
main "$@"
