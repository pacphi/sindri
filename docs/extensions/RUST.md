# Rust

Rust stable toolchain via rustup.

## Overview

| Property         | Value    |
| ---------------- | -------- |
| **Category**     | language |
| **Version**      | 1.0.2    |
| **Installation** | rustup   |
| **Disk Space**   | 1200 MB  |
| **Dependencies** | None     |

## Description

Rust stable via rustup - provides the Rust compiler, cargo package manager, and associated tooling for systems programming. Uses a custom installation script that works around `/tmp` noexec restrictions common in Docker environments.

## Installed Tools

| Tool    | Type            | Description                         |
| ------- | --------------- | ----------------------------------- |
| `rustc` | compiler        | Rust compiler                       |
| `cargo` | package-manager | Rust package manager and build tool |

## Configuration

### Installation Method

This extension uses a custom script-based installation rather than mise to work around `/tmp` noexec restrictions. The installation script:

- Downloads rustup-init to `$HOME/.cache/tmp` (executable location on persistent volume)
- Sets `TMPDIR=$HOME/.cache/tmp` to avoid `/tmp` noexec issues
- Installs Rust stable toolchain via rustup
- Configures `RUSTUP_HOME` and `CARGO_HOME` in `$HOME/.rustup` and `$HOME/.cargo`
- Adds cargo bin directory to PATH in `.profile`

### Environment Variables

The following environment variables are automatically configured:

```bash
export RUSTUP_HOME="${HOME}/.rustup"
export CARGO_HOME="${HOME}/.cargo"
export PATH="${HOME}/.cargo/bin:${PATH}"
```

## Network Requirements

- `crates.io` - Rust package registry
- `rust-lang.org` - Rust downloads

## Installation

```bash
extension-manager install rust
```

## Validation

```bash
rustc --version    # Expected: rustc X.X.X
cargo --version
```

## Removal

```bash
extension-manager remove rust
```

Removes rustup and the Rust toolchain, including all cargo and rustup directories.

## Troubleshooting

### `/tmp` noexec Issue

**Problem:** Rust installation fails with error: `Cannot execute /tmp/tmp.XXXXXXX/rustup-init (likely because of mounting /tmp as noexec)`

**Cause:** For security hardening, Sindri mounts `/tmp` with the `noexec` flag in Docker containers. Standard rustup installation downloads rustup-init to `/tmp` and attempts to execute it, which fails.

**Solution:** This extension automatically works around the issue by:

1. Creating an executable temporary directory at `$HOME/.cache/tmp`
2. Setting `TMPDIR=$HOME/.cache/tmp` before running rustup
3. Installing from the executable location on the persistent volume

This approach maintains security (noexec on `/tmp`) while enabling Rust installation.

**OrbStack Note:** This issue is particularly prevalent on OrbStack (macOS Docker runtime) which enforces mount flags more strictly than Docker Desktop.

### PATH Configuration

If rust commands are not found, ensure your shell sources the profile:

```bash
source ~/.profile
rustc --version
```

Or source the cargo environment directly:

```bash
source "$HOME/.cargo/env"
```

## Related Extensions

- [infra-tools](INFRA-TOOLS.md) - Infrastructure tools
- [nodejs](NODEJS.md) - Node.js runtime (also uses custom installation)
