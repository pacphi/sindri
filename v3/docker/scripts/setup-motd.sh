#!/bin/bash
# Setup Message of the Day (MOTD) for Sindri v3
# Displays ASCII art banner when users interact with the development environment

set -e

MOTD_FILE="/etc/motd"

echo "Setting up Sindri v3 MOTD banner..."

cat > "$MOTD_FILE" << 'EOF'

   ███████╗██╗███╗   ██╗██████╗ ██████╗ ██╗
   ██╔════╝██║████╗  ██║██╔══██╗██╔══██╗██║
   ███████╗██║██╔██╗ ██║██║  ██║██████╔╝██║
   ╚════██║██║██║╚██╗██║██║  ██║██╔══██╗██║
   ███████║██║██║ ╚████║██████╔╝██║  ██║██║
   ╚══════╝╚═╝╚═╝  ╚═══╝╚═════╝ ╚═╝  ╚═╝╚═╝

   🚀 Declarative Cloud Development Environments (v3)
   📦 https://github.com/pacphi/sindri
   🦀 Powered by Rust CLI

EOF

chmod 644 "$MOTD_FILE"

echo "✅ MOTD banner configured successfully"
