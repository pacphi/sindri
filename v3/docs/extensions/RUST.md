# Rust Extension

> Version: 1.0.2 | Category: languages | Last Updated: 2026-01-26

## Overview

Rust stable via rustup. Provides the Rust programming language toolchain including the compiler and Cargo package manager.

## What It Provides

| Tool  | Type            | License           | Description                         |
| ----- | --------------- | ----------------- | ----------------------------------- |
| rustc | compiler        | MIT OR Apache-2.0 | Rust compiler                       |
| cargo | package-manager | MIT OR Apache-2.0 | Rust package manager and build tool |

## Requirements

- **Disk Space**: 1200 MB
- **Memory**: 2048 MB
- **Install Time**: ~120 seconds
- **Dependencies**: None

### Network Domains

- crates.io
- rust-lang.org

## Installation

```bash
extension-manager install rust
```

## Configuration

### Install Method

Uses a custom installation script (rustup-based).

## Usage Examples

### Basic Rust Commands

```bash
# Check version
rustc --version
cargo --version

# Create a new project
cargo new my-project
cargo new my-lib --lib

# Build a project
cargo build
cargo build --release
```

### Running and Testing

```bash
# Run a project
cargo run

# Run tests
cargo test

# Run benchmarks
cargo bench

# Check code without building
cargo check
```

### Dependency Management

```bash
# Add a dependency
cargo add serde
cargo add tokio --features full

# Update dependencies
cargo update

# Show dependency tree
cargo tree
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Generate documentation
cargo doc --open
```

### Building for Release

```bash
# Optimized release build
cargo build --release

# Cross-compilation (with targets installed)
rustup target add x86_64-unknown-linux-musl
cargo build --target x86_64-unknown-linux-musl --release
```

### Using Rustup

```bash
# Update Rust
rustup update

# Install a component
rustup component add rustfmt clippy

# Install a specific toolchain
rustup install nightly

# Switch default toolchain
rustup default stable
```

## Validation

The extension validates the following commands:

- `rustc` - Must match pattern `rustc \d+\.\d+\.\d+`
- `cargo` - Must be available

## Removal

```bash
extension-manager remove rust
```

This runs the removal script to uninstall rustup and associated tools.

## Related Extensions

None - Rust is a standalone language extension.
