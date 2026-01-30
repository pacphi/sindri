# Haskell Extension

> Version: 1.0.1 | Category: languages | Last Updated: 2026-01-26

## Overview

Haskell development environment with GHC, Cabal, Stack, and HLS. Provides a complete functional programming environment with IDE support.

## What It Provides

| Tool                    | Type            | License      | Description              |
| ----------------------- | --------------- | ------------ | ------------------------ |
| ghc                     | compiler        | BSD-3-Clause | Glasgow Haskell Compiler |
| cabal                   | package-manager | BSD-3-Clause | Cabal build system       |
| stack                   | cli-tool        | BSD-3-Clause | Stack build tool         |
| haskell-language-server | server          | Apache-2.0   | IDE support for Haskell  |

## Requirements

- **Disk Space**: 6000 MB (6 GB)
- **Memory**: 4096 MB
- **Install Time**: ~180 seconds
- **Dependencies**: mise-config

### Network Domains

- haskell.org
- hackage.haskell.org
- stackage.org
- downloads.haskell.org

## Installation

```bash
extension-manager install haskell
```

## Configuration

### Environment Variables

| Variable     | Value    | Description                   |
| ------------ | -------- | ----------------------------- |
| `CABAL_DIR`  | ~/.cabal | Cabal configuration directory |
| `STACK_ROOT` | ~/.stack | Stack root directory          |

### Install Method

Uses mise for tool management with automatic shim refresh.

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

## Validation

The extension validates the following commands:

- `ghc` - Must match pattern `The Glorious Glasgow Haskell Compilation System`
- `cabal` - Must match pattern `cabal-install version`
- `stack` - Must match pattern `Version`
- `haskell-language-server-wrapper` - Must match pattern `haskell-language-server`

## Removal

```bash
extension-manager remove haskell
```

This removes mise Haskell tools (haskell, hls).

## Related Extensions

- [mise-config](MISE-CONFIG.md) - Required mise configuration
