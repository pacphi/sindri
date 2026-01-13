# Rust

Rust stable toolchain via mise.

## Overview

| Property         | Value                           |
| ---------------- | ------------------------------- |
| **Category**     | language                        |
| **Version**      | 1.0.1                           |
| **Installation** | mise                            |
| **Disk Space**   | 800 MB                          |
| **Dependencies** | [mise-config](MISE-CONFIG.md)   |

## Description

Rust stable via mise - provides the Rust compiler, cargo package manager, and associated tooling for systems programming.

## Installed Tools

| Tool    | Type            | Description                         |
| ------- | --------------- | ----------------------------------- |
| `rustc` | compiler        | Rust compiler                       |
| `cargo` | package-manager | Rust package manager and build tool |

## Configuration

### mise.toml

```toml
[tools]
rust = "stable"
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

Removes mise configuration and Rust toolchain.

## Related Extensions

- [infra-tools](INFRA-TOOLS.md) - Infrastructure tools
