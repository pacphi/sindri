#!/bin/bash
set -euo pipefail

# Create temporary directory in home (not /tmp) to avoid noexec
RUST_TMP_DIR="${HOME}/.cache/tmp"
mkdir -p "$RUST_TMP_DIR"

# Set environment variables for rustup installation
export TMPDIR="$RUST_TMP_DIR"
export RUSTUP_HOME="${HOME}/.rustup"
export CARGO_HOME="${HOME}/.cargo"

# Download and run rustup-init from executable location
RUSTUP_INIT="${RUST_TMP_DIR}/rustup-init"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$RUSTUP_INIT"
chmod +x "$RUSTUP_INIT"

# Install Rust with rustup
RUSTUP_HOME="${RUSTUP_HOME}" CARGO_HOME="${CARGO_HOME}" \
  "$RUSTUP_INIT" -y --default-toolchain stable --no-modify-path

# Clean up
rm -f "$RUSTUP_INIT"

# Add rust environment variables and PATH to profile
cat >> "${HOME}/.profile" << 'EOF'
# Rust environment
export RUSTUP_HOME="${HOME}/.rustup"
export CARGO_HOME="${HOME}/.cargo"
export PATH="${HOME}/.cargo/bin:${PATH}"
EOF

# Set default toolchain explicitly
export PATH="${CARGO_HOME}/bin:${PATH}"
rustup default stable

echo "Rust installed successfully"
