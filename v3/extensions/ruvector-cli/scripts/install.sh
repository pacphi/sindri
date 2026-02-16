#!/usr/bin/env bash
set -euo pipefail

echo "Installing ruvector-cli via cargo..."

# Ensure cargo is available
if ! command -v cargo &>/dev/null; then
    echo "ERROR: cargo not found. Please install the rust extension first." >&2
    exit 1
fi

# Use home directory for cargo build artifacts to avoid /tmp noexec issues
export CARGO_TARGET_DIR="${HOME}/.cache/cargo-build"
mkdir -p "$CARGO_TARGET_DIR"

# ruvector-cli v2.0.3 has a bug: workspace Cargo.toml is missing tokio "io-std"
# and "io-util" features required by ruvector-mcp binary (uses tokio::io::stdout).
# Workaround: clone the repo, patch Cargo.toml, and build from source.
RUVECTOR_SRC="${HOME}/.cache/ruvector-src"
rm -rf "$RUVECTOR_SRC"
git clone --depth 1 https://github.com/ruvnet/ruvector.git "$RUVECTOR_SRC"

# Patch workspace Cargo.toml to add missing tokio features
sed -i.bak 's/tokio = { version = "1.41", features = \["rt-multi-thread", "sync", "macros"\] }/tokio = { version = "1.41", features = ["rt-multi-thread", "sync", "macros", "io-std", "io-util"] }/' "$RUVECTOR_SRC/Cargo.toml"

# Build and install from patched source
cargo install --path "$RUVECTOR_SRC/crates/ruvector-cli"

# Clean up source and build artifacts to reclaim disk space
rm -rf "$RUVECTOR_SRC" "$CARGO_TARGET_DIR"

echo "ruvector-cli installed successfully."
