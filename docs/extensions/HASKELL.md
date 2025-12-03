# Haskell

Haskell development environment with GHC, Cabal, Stack, and HLS via mise.

## Overview

| Property         | Value    |
| ---------------- | -------- |
| **Category**     | language |
| **Version**      | 1.0.0    |
| **Installation** | mise     |
| **Disk Space**   | 2000 MB  |
| **Memory**       | 4096 MB  |
| **Dependencies** | None     |

## Description

Complete Haskell development environment providing the Glasgow Haskell Compiler (GHC), Cabal build system, Stack tool, and Haskell Language Server (HLS) for IDE integration.

## Installed Tools

| Tool                      | Type            | Description                     | License      |
| ------------------------- | --------------- | ------------------------------- | ------------ |
| `ghc`                     | compiler        | Glasgow Haskell Compiler        | BSD-3-Clause |
| `cabal`                   | package-manager | Haskell package manager         | BSD-3-Clause |
| `stack`                   | cli-tool        | Haskell build and project tool  | BSD-3-Clause |
| `haskell-language-server` | server          | Language server for IDE support | Apache-2.0   |

## Configuration

### mise.toml

```toml
[tools]
haskell = "latest"
hls = "latest"
```

### Environment Variables

| Variable     | Value    | Description          |
| ------------ | -------- | -------------------- |
| `CABAL_DIR`  | ~/.cabal | Cabal data directory |
| `STACK_ROOT` | ~/.stack | Stack root directory |

## Network Requirements

- `haskell.org` - Main Haskell downloads
- `hackage.haskell.org` - Haskell package repository
- `stackage.org` - Curated package sets
- `downloads.haskell.org` - GHC downloads

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

Removes mise configuration and all Haskell tools.

## Related Extensions

- [jvm](JVM.md) - JVM languages (alternative FP with Scala/Clojure)
