#!/usr/bin/env bash
set -euo pipefail

LOG_DIR="${SINDRI_LOG_DIR:-/tmp}"
LOG_FILE="${LOG_DIR}/rtk-uninstall.log"
mkdir -p "$LOG_DIR"

log() { echo "[rtk-uninstall] $*" | tee -a "$LOG_FILE"; }

INSTALL_DIR="${RTK_INSTALL_DIR:-$HOME/.local/bin}"

if [ -f "$INSTALL_DIR/rtk" ]; then
  rm -f "$INSTALL_DIR/rtk"
  log "Removed ${INSTALL_DIR}/rtk"
else
  log "RTK binary not found at ${INSTALL_DIR}/rtk"
fi

# Remove config and data
if [ -d "$HOME/.config/rtk" ]; then
  rm -rf "$HOME/.config/rtk"
  log "Removed ~/.config/rtk"
fi

log "RTK uninstalled"
