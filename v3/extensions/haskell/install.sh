#!/bin/bash
set -euo pipefail

# Haskell install script - Uses ghcup for GHC, Cabal, Stack, and HLS
# ghcup is the official Haskell toolchain installer

# Tool versions - update these when upgrading
GHC_VERSION="9.12.2"           # Latest stable (9.14.1 is preliminary)
CABAL_VERSION="3.14.1.1"       # Latest stable compatible with GHC 9.12
STACK_VERSION="3.3.1"          # Latest stable
HLS_VERSION="2.13.0.0"         # Latest with GHC 9.12 support

# Create temporary directory in home (not /tmp) to avoid noexec
HASKELL_TMP_DIR="${HOME}/.cache/tmp"
mkdir -p "$HASKELL_TMP_DIR"

# Set environment variables for ghcup installation
export TMPDIR="$HASKELL_TMP_DIR"
export GHCUP_INSTALL_BASE_PREFIX="${HOME}"
export GHCUP_USE_XDG_DIRS=1
export CABAL_DIR="${HOME}/.cabal"
export STACK_ROOT="${HOME}/.stack"

# Download and run ghcup installer
GHCUP_INSTALL="${HASKELL_TMP_DIR}/ghcup-install"
curl --proto '=https' --tlsv1.2 -sSf https://get-ghcup.haskell.org -o "$GHCUP_INSTALL"

# Make it executable and run with non-interactive options
chmod +x "$GHCUP_INSTALL"

# Run ghcup bootstrap (non-interactive)
# BOOTSTRAP_HASKELL_NONINTERACTIVE=1 skips prompts
# BOOTSTRAP_HASKELL_MINIMAL=1 installs only ghcup, we'll install tools separately
export BOOTSTRAP_HASKELL_NONINTERACTIVE=1
export BOOTSTRAP_HASKELL_MINIMAL=1
export BOOTSTRAP_HASKELL_NO_UPGRADE=1
export BOOTSTRAP_HASKELL_ADJUST_BASHRC=0

# Run the installer
bash "$GHCUP_INSTALL" || {
    echo "ghcup bootstrap failed, trying alternative method..."
    # Alternative: direct binary download
    GHCUP_BIN="${HOME}/.ghcup/bin"
    mkdir -p "$GHCUP_BIN"
    ARCH=$(uname -m)
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')

    if [[ "$ARCH" == "x86_64" ]]; then
        GHCUP_ARCH="x86_64"
    elif [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]]; then
        GHCUP_ARCH="aarch64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi

    GHCUP_URL="https://downloads.haskell.org/~ghcup/${GHCUP_ARCH}-${OS}-ghcup"
    curl -L "$GHCUP_URL" -o "${GHCUP_BIN}/ghcup"
    chmod +x "${GHCUP_BIN}/ghcup"
}

# Clean up installer
rm -f "$GHCUP_INSTALL"

# Set up PATH for ghcup
export PATH="${HOME}/.ghcup/bin:${HOME}/.cabal/bin:${PATH}"

# Verify ghcup is available
if ! command -v ghcup &>/dev/null; then
    echo "ERROR: ghcup not found in PATH after installation"
    exit 1
fi

echo "ghcup installed: $(ghcup --version)"

# Install GHC
echo "Installing GHC ${GHC_VERSION}..."
ghcup install ghc "$GHC_VERSION" --set || {
    echo "Failed to install GHC ${GHC_VERSION}, trying recommended..."
    ghcup install ghc recommended --set
}

# Install Cabal
echo "Installing Cabal ${CABAL_VERSION}..."
ghcup install cabal "$CABAL_VERSION" --set || {
    echo "Failed to install Cabal ${CABAL_VERSION}, trying recommended..."
    ghcup install cabal recommended --set
}

# Install Stack
echo "Installing Stack ${STACK_VERSION}..."
ghcup install stack "$STACK_VERSION" --set || {
    echo "Failed to install Stack ${STACK_VERSION}, trying recommended..."
    ghcup install stack recommended --set
}

# Install HLS
echo "Installing HLS ${HLS_VERSION}..."
ghcup install hls "$HLS_VERSION" --set || {
    echo "Failed to install HLS ${HLS_VERSION}, trying recommended..."
    ghcup install hls recommended --set || {
        echo "WARNING: HLS installation failed - IDE support may be limited"
    }
}

# Update cabal package index
echo "Updating cabal package index..."
cabal update || echo "WARNING: cabal update failed"

# Add Haskell environment to profile
cat >> "${HOME}/.profile" << 'EOF'

# Haskell environment (ghcup)
export GHCUP_INSTALL_BASE_PREFIX="${HOME}"
export GHCUP_USE_XDG_DIRS=1
export CABAL_DIR="${HOME}/.cabal"
export STACK_ROOT="${HOME}/.stack"
export PATH="${HOME}/.ghcup/bin:${HOME}/.cabal/bin:${HOME}/.local/bin:${PATH}"
EOF

# Summary
echo ""
echo "=== Haskell Installation Summary ==="
ghcup list 2>/dev/null | grep -E '(ghc|cabal|stack|hls).*installed' || echo "See ghcup list for installed tools"
echo ""
echo "Haskell development environment installed successfully"
echo "Note: Run 'source ~/.profile' or start a new shell to use Haskell tools"
