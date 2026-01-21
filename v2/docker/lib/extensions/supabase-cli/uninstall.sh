#!/usr/bin/env bash
set -euo pipefail

# Uninstall script for supabase-cli
# Removes the Supabase CLI .deb package

source "${DOCKER_LIB:-/docker/lib}/common.sh"

print_status "Uninstalling Supabase CLI..."

# Remove the dpkg package if installed
if dpkg -l supabase &>/dev/null; then
    print_status "Removing supabase package..."
    sudo DEBIAN_FRONTEND=noninteractive dpkg -r supabase || true
    sudo DEBIAN_FRONTEND=noninteractive apt-get autoremove -y -qq || true
    print_success "Supabase CLI package removed"
else
    print_warning "Supabase CLI package not found in dpkg"
fi

# Clean up any remaining files
if [[ -f "/usr/bin/supabase" ]]; then
    sudo rm -f /usr/bin/supabase
fi

if [[ -f "/usr/local/bin/supabase" ]]; then
    sudo rm -f /usr/local/bin/supabase
fi

print_success "Supabase CLI uninstallation complete"
