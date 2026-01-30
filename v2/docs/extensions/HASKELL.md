# Haskell

Haskell development environment with GHC, Cabal, Stack, and HLS via ghcup.

## Overview

| Property         | Value          |
| ---------------- | -------------- |
| **Category**     | language       |
| **Version**      | 2.0.0          |
| **Installation** | script (ghcup) |
| **Disk Space**   | 6000 MB        |
| **Memory**       | 4096 MB        |
| **Dependencies** | None           |

## What's New in 2.0.0

- **Switched from mise to ghcup**: Uses the official Haskell toolchain installer
- **Full toolchain support**: GHC, Cabal, Stack, and HLS all managed by ghcup
- **Better version management**: Use `ghcup tui` or `ghcup list` to manage versions

## Description

Complete Haskell development environment providing the Glasgow Haskell Compiler (GHC), Cabal build system, Stack tool, and Haskell Language Server (HLS) for IDE integration. All tools are managed via ghcup, the official Haskell toolchain installer.

## Installed Tools

| Tool                      | Version  | Type            | Description                     | License      |
| ------------------------- | -------- | --------------- | ------------------------------- | ------------ |
| `ghcup`                   | 0.1.50.x | cli-tool        | Haskell toolchain installer     | LGPL-3.0     |
| `ghc`                     | 9.12.2   | compiler        | Glasgow Haskell Compiler        | BSD-3-Clause |
| `cabal`                   | 3.14.x   | package-manager | Haskell package manager         | BSD-3-Clause |
| `stack`                   | 3.3.x    | cli-tool        | Haskell build and project tool  | BSD-3-Clause |
| `haskell-language-server` | 2.13.x   | server          | Language server for IDE support | Apache-2.0   |

## Configuration

### Environment Variables

| Variable                    | Value    | Description               |
| --------------------------- | -------- | ------------------------- |
| `GHCUP_INSTALL_BASE_PREFIX` | ${HOME}  | ghcup installation prefix |
| `CABAL_DIR`                 | ~/.cabal | Cabal data directory      |
| `STACK_ROOT`                | ~/.stack | Stack root directory      |

## Network Requirements

- `haskell.org` - Main Haskell downloads
- `hackage.haskell.org` - Haskell package repository
- `stackage.org` - Curated package sets
- `downloads.haskell.org` - GHC downloads
- `get-ghcup.haskell.org` - ghcup installer

## Installation

```bash
extension-manager install haskell
```

## Validation

```bash
ghc --version                           # Expected: The Glorious Glasgow Haskell Compilation System
cabal --version                         # Expected: cabal-install version X.X.X
stack --version                         # Expected: Version X.X.X
haskell-language-server-wrapper --version  # Expected: haskell-language-server X.X.X
```

## Usage Examples

### Managing Versions with ghcup

```bash
# Interactive TUI
ghcup tui

# List all installed tools
ghcup list

# Install a different GHC version
ghcup install ghc 9.8.4
ghcup set ghc 9.8.4

# Upgrade ghcup itself
ghcup upgrade
```

### Create a New Project with Cabal

```bash
mkdir myproject && cd myproject
cabal init
cabal build
cabal run
```

### Create a New Project with Stack

```bash
stack new myproject
cd myproject
stack build
stack exec myproject
```

### Interactive REPL

```bash
ghci
```

## Removal

```bash
extension-manager remove haskell
```

Runs `ghcup nuke` to remove all Haskell tools and configuration.

## Related Extensions

- [jvm](JVM.md) - JVM languages (alternative FP with Scala/Clojure)
