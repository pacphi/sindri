# Haskell Extension

> Version: 2.0.0 | Category: languages | Last Updated: 2026-01-30

## Overview

Haskell development environment with GHC, Cabal, Stack, and HLS via ghcup. Provides a complete functional programming environment with IDE support.

## What's New in 2.0.0

- **Switched from mise to ghcup**: Uses the official Haskell toolchain installer for reliable installation
- **Full toolchain support**: GHC, Cabal, Stack, and HLS all managed by ghcup
- **Better version management**: Use `ghcup tui` or `ghcup list` to manage installed versions

## What It Provides

| Tool                    | Version  | Type            | License      | Description                 |
| ----------------------- | -------- | --------------- | ------------ | --------------------------- |
| ghcup                   | 0.1.50.x | cli-tool        | LGPL-3.0     | Haskell toolchain installer |
| ghc                     | 9.12.2   | compiler        | BSD-3-Clause | Glasgow Haskell Compiler    |
| cabal                   | 3.14.x   | package-manager | BSD-3-Clause | Cabal build system          |
| stack                   | 3.3.x    | cli-tool        | BSD-3-Clause | Stack build tool            |
| haskell-language-server | 2.13.x   | server          | Apache-2.0   | IDE support for Haskell     |

## Requirements

- **Disk Space**: 6000 MB (6 GB)
- **Memory**: 4096 MB
- **Install Time**: ~300 seconds
- **Dependencies**: None (standalone installation)

### Network Domains

- haskell.org
- hackage.haskell.org
- stackage.org
- downloads.haskell.org
- get-ghcup.haskell.org

## Installation

```bash
sindri extension install haskell
```

## Configuration

### Environment Variables

| Variable                    | Value    | Description                   |
| --------------------------- | -------- | ----------------------------- |
| `GHCUP_INSTALL_BASE_PREFIX` | ${HOME}  | ghcup installation prefix     |
| `CABAL_DIR`                 | ~/.cabal | Cabal configuration directory |
| `STACK_ROOT`                | ~/.stack | Stack root directory          |

### Install Method

Uses ghcup for tool management. After installation, use ghcup to manage versions:

```bash
# Interactive TUI
ghcup tui

# List installed tools
ghcup list

# Install a different GHC version
ghcup install ghc 9.8.4
ghcup set ghc 9.8.4

# Upgrade tools
ghcup upgrade
```

## Usage Examples

### GHC (Compiler)

```bash
# Check version
ghc --version

# Compile a file
ghc Main.hs -o main

# Interactive REPL
ghci

# Load a file in GHCi
ghci Main.hs
```

### Cabal

```bash
# Initialize a project
cabal init

# Build
cabal build

# Run
cabal run

# Install dependencies
cabal install --only-dependencies

# Run tests
cabal test

# Generate documentation
cabal haddock
```

### Stack

```bash
# Create a new project
stack new my-project

# Build
stack build

# Run
stack exec my-project-exe

# Run tests
stack test

# Install globally
stack install

# Enter GHCi with project
stack ghci
```

### Package Management

```bash
# Cabal: Add dependency
# Edit .cabal file, then:
cabal update
cabal build

# Stack: Add dependency
# Edit package.yaml, then:
stack build
```

### Haskell Language Server

```bash
# HLS starts automatically with compatible editors
# Verify installation:
haskell-language-server-wrapper --version

# Generate HIE files (for better IDE support)
gen-hie > hie.yaml
```

### Common Patterns

```haskell
-- Main.hs
module Main where

main :: IO ()
main = putStrLn "Hello, Haskell!"

-- With imports
import Data.List (sort)
import qualified Data.Map as M

-- Example function
fibonacci :: Int -> Int
fibonacci n
  | n <= 1    = n
  | otherwise = fibonacci (n-1) + fibonacci (n-2)
```

## Version Management with ghcup

```bash
# See all available GHC versions
ghcup list -t ghc

# Install specific GHC version
ghcup install ghc 9.8.4

# Set active GHC version
ghcup set ghc 9.8.4

# Install HLS for a specific GHC
ghcup install hls --set

# Remove an old version
ghcup rm ghc 9.6.5
```

## Validation

The extension validates the following commands:

- `ghc` - Must match pattern `The Glorious Glasgow Haskell Compilation System`
- `cabal` - Must match pattern `cabal-install version`
- `stack` - Must match pattern `Version`
- `haskell-language-server-wrapper` - Must match pattern `haskell-language-server`

## Removal

```bash
sindri extension remove haskell
```

This runs `ghcup nuke` to remove all Haskell tools and cleans up environment configuration.

## Troubleshooting

### PATH Issues

If tools aren't found after installation:

```bash
source ~/.profile
# or start a new shell
```

### HLS Not Working

HLS requires a compatible GHC version. Check compatibility:

```bash
ghcup list -t hls
```

### Switching GHC Versions

```bash
# See installed versions
ghcup list -t ghc -c installed

# Switch version
ghcup set ghc <version>

# You may need to reinstall HLS for the new GHC
ghcup install hls --set
```

## Related Extensions

- [jvm](JVM.md) - JVM languages (alternative FP with Scala/Clojure)
