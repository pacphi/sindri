#!/bin/bash
# Install Nerd Fonts for starship prompt support
# Downloads and installs specified Nerd Fonts to /usr/share/fonts/truetype/nerd-fonts/

set -euo pipefail

# Source common utilities if available
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/../lib/common.sh" ]]; then
    source "$SCRIPT_DIR/../lib/common.sh"
else
    print_status() { echo "==> $1"; }
    print_success() { echo "[OK] $1"; }
    print_error() { echo "[ERROR] $1" >&2; }
fi

# Nerd Fonts version
NERD_FONTS_VERSION="${NERD_FONTS_VERSION:-v3.4.0}"
FONTS_DIR="/usr/share/fonts/truetype/nerd-fonts"
BASE_URL="https://github.com/ryanoasis/nerd-fonts/releases/download/${NERD_FONTS_VERSION}"

# List of fonts to install (as they appear in GitHub releases)
# These map to the .zip file names in the releases
FONTS=(
    "Arimo"
    "BlexMono"
    "CascadiaCode"    # CaskaydiaCove is the patched name, but downloads as CascadiaCode
    "CodeNewRoman"
    "Cousine"
    "DroidSansMono"   # DroidSansM downloads as DroidSansMono
    "FiraCode"
    "GeistMono"
    "Inconsolata"
    "Meslo"           # MesloLG downloads as Meslo
    "Noto"
    "RobotoMono"
    "Ubuntu"
)

print_status "Installing Nerd Fonts to $FONTS_DIR..."

# Ensure fontconfig is installed
if ! command -v fc-cache >/dev/null 2>&1; then
    print_error "fontconfig not installed. Please install it first:"
    print_error "  apt-get install -y fontconfig"
    exit 1
fi

# Create fonts directory
mkdir -p "$FONTS_DIR"

# Create temporary directory for downloads
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

# Download and install each font
for font in "${FONTS[@]}"; do
    print_status "Installing ${font} Nerd Font..."

    FONT_URL="${BASE_URL}/${font}.zip"
    FONT_ZIP="${TEMP_DIR}/${font}.zip"
    FONT_EXTRACT_DIR="${TEMP_DIR}/${font}"

    # Download font
    if curl -fsSL "$FONT_URL" -o "$FONT_ZIP"; then
        # Create extraction directory
        mkdir -p "$FONT_EXTRACT_DIR"

        # Extract font files
        if unzip -q "$FONT_ZIP" -d "$FONT_EXTRACT_DIR"; then
            # Copy only .ttf and .otf files to the fonts directory
            # Exclude Windows-specific fonts (ending in Windows Compatible.ttf)
            find "$FONT_EXTRACT_DIR" -type f \( -iname "*.ttf" -o -iname "*.otf" \) \
                ! -iname "*Windows Compatible.ttf" \
                -exec cp {} "$FONTS_DIR/" \;

            print_success "${font} Nerd Font installed"
        else
            print_error "Failed to extract ${font}.zip"
        fi

        # Clean up extraction directory
        rm -rf "$FONT_EXTRACT_DIR" "$FONT_ZIP"
    else
        print_error "Failed to download ${font} Nerd Font from $FONT_URL"
        print_status "Continuing with remaining fonts..."
    fi
done

# Rebuild font cache
print_status "Rebuilding font cache..."
fc-cache -fv "$FONTS_DIR" >/dev/null 2>&1

# Verify installation
INSTALLED_FONTS=$(fc-list : family | grep -i "nerd font" | wc -l)
print_success "Nerd Fonts installation complete"
print_status "Installed ${INSTALLED_FONTS} Nerd Font families"
print_status "Users can now use starship with full Unicode symbol support"

# List some installed fonts for verification
print_status "Sample installed fonts:"
fc-list : family | grep -i "nerd font" | head -n 5 || true
