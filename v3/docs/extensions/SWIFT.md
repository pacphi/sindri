# Swift Extension

> Version: 1.0.0 | Category: languages | Last Updated: 2026-02-03

## Overview

Swift 6.2.3 via mise. Provides the Swift programming language for iOS, macOS, Linux, and server-side development.

## What It Provides

| Tool   | Type     | License    | Description                             |
| ------ | -------- | ---------- | --------------------------------------- |
| swift  | compiler | Apache-2.0 | Swift compiler and REPL                 |
| swiftc | compiler | Apache-2.0 | Swift compiler for building executables |

## Requirements

- **Disk Space**: 1500 MB
- **Memory**: 512 MB
- **Install Time**: ~120 seconds
- **Dependencies**: mise-config

### Network Domains

- swift.org
- download.swift.org

## Installation

```bash
sindri extension install swift
```

## Configuration

### Environment Variables

| Variable                      | Value | Description                                          |
| ----------------------------- | ----- | ---------------------------------------------------- |
| `SWIFT_DETERMINISTIC_HASHING` | 1     | Enable deterministic hashing for reproducible builds |

### Install Method

Uses mise for Swift installation with automatic shim refresh and GPG signature verification.

## Usage Examples

### Basic Swift Commands

```bash
# Check version
swift --version

# Start REPL
swift

# Run a Swift file
swift run.swift

# Build a project
swift build

# Run tests
swift test
```

### Swift Package Manager

```bash
# Initialize a new package
swift package init --type executable

# Add a dependency (edit Package.swift then run)
swift package update

# Generate Xcode project
swift package generate-xcodeproj

# Build release version
swift build -c release
```

### Building for Different Configurations

```bash
# Debug build
swift build

# Release build
swift build -c release

# Build with verbose output
swift build -v
```

## Validation

The extension validates the following commands:

- `swift --version` - Must match pattern `Swift version \d+\.\d+\.\d+`
- `swiftc --version` - Must match pattern `Swift version \d+\.\d+\.\d+`

## Removal

```bash
sindri extension remove swift
```

This removes the mise configuration and Swift toolchain.

## Related Extensions

- [mise-config](MISE-CONFIG.md) - Required mise configuration
- [nodejs](NODEJS.md) - Often used together for React Native development
