#!/usr/bin/env bash
set -euo pipefail

# RTK (Rust Token Killer) installer
# Downloads pre-built binary from GitHub releases
# Supports: Linux (x86_64, aarch64), macOS (x86_64, aarch64)

LOG_DIR="${SINDRI_LOG_DIR:-/tmp}"
LOG_FILE="${LOG_DIR}/rtk-install.log"
mkdir -p "$LOG_DIR"

log() { echo "[rtk-install] $*" | tee -a "$LOG_FILE"; }

INSTALL_DIR="${RTK_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  os="unknown-linux-musl" ;;
  Darwin) os="apple-darwin" ;;
  *)      log "ERROR: Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64)          arch="x86_64" ;;
  aarch64|arm64)   arch="aarch64" ;;
  *)               log "ERROR: Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Override musl vs gnu for aarch64 Linux
if [ "$OS" = "Linux" ] && [ "$arch" = "aarch64" ]; then
  os="unknown-linux-gnu"
fi

TARGET="${arch}-${os}"

# Fetch latest version from GitHub API
log "Fetching latest RTK release..."
VERSION="$(curl -fsSL https://api.github.com/repos/rtk-ai/rtk/releases/latest | grep '"tag_name"' | sed 's/.*"tag_name": *"//;s/".*//')"

if [ -z "$VERSION" ]; then
  log "ERROR: Could not determine latest RTK version"
  exit 1
fi

log "Latest version: $VERSION"

TARBALL="rtk-${TARGET}.tar.gz"
URL="https://github.com/rtk-ai/rtk/releases/download/${VERSION}/${TARBALL}"

log "Downloading ${URL}..."
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$URL" -o "$TMPDIR/$TARBALL"
tar -xzf "$TMPDIR/$TARBALL" -C "$TMPDIR"

# Move binary to install dir
if [ -f "$TMPDIR/rtk" ]; then
  mv "$TMPDIR/rtk" "$INSTALL_DIR/rtk"
elif [ -f "$TMPDIR/rtk-${TARGET}/rtk" ]; then
  mv "$TMPDIR/rtk-${TARGET}/rtk" "$INSTALL_DIR/rtk"
else
  # Find the binary wherever it was extracted
  RTK_BIN="$(find "$TMPDIR" -name rtk -type f | head -1)"
  if [ -z "$RTK_BIN" ]; then
    log "ERROR: Could not find rtk binary in archive"
    exit 1
  fi
  mv "$RTK_BIN" "$INSTALL_DIR/rtk"
fi

chmod +x "$INSTALL_DIR/rtk"

log "RTK ${VERSION} installed to ${INSTALL_DIR}/rtk"

# Verify
if command -v rtk &>/dev/null; then
  log "Verification: $(rtk --version)"
else
  log "NOTE: ${INSTALL_DIR} may not be in PATH. Add it to your shell profile."
fi
