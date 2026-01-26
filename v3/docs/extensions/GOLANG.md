# Go (Golang) Extension

> Version: 1.0.1 | Category: languages | Last Updated: 2026-01-26

## Overview

Go 1.25 via mise. Provides a complete Go development environment with proper workspace configuration.

## What It Provides

| Tool | Type     | License      | Description                                |
| ---- | -------- | ------------ | ------------------------------------------ |
| go   | compiler | BSD-3-Clause | Go programming language compiler and tools |

## Requirements

- **Disk Space**: 250 MB
- **Memory**: 256 MB
- **Install Time**: ~60 seconds
- **Dependencies**: mise-config

### Network Domains

- golang.org
- proxy.golang.org

## Installation

```bash
extension-manager install golang
```

## Configuration

### Environment Variables

| Variable     | Value                  | Description                 |
| ------------ | ---------------------- | --------------------------- |
| `GOPATH`     | ${HOME}/go             | Go workspace path           |
| `GOMODCACHE` | ${HOME}/go/pkg/mod     | Module cache location       |
| `GOBIN`      | ${HOME}/go/bin         | Go binary installation path |
| `PATH`       | ${HOME}/go/bin:${PATH} | Adds Go binaries to PATH    |

### Install Method

Uses mise for Go installation with automatic shim refresh.

## Usage Examples

### Basic Go Commands

```bash
# Check version
go version

# Initialize a new module
go mod init github.com/user/project

# Build a project
go build

# Run a Go file
go run main.go

# Run tests
go test ./...
```

### Module Management

```bash
# Add a dependency
go get github.com/gin-gonic/gin

# Update dependencies
go mod tidy

# Verify dependencies
go mod verify

# Download dependencies
go mod download
```

### Installing Go Tools

```bash
# Install a tool
go install golang.org/x/tools/gopls@latest

# Install commonly used tools
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest
```

### Building for Different Platforms

```bash
# Build for Linux
GOOS=linux GOARCH=amd64 go build -o app-linux

# Build for macOS
GOOS=darwin GOARCH=arm64 go build -o app-mac

# Build for Windows
GOOS=windows GOARCH=amd64 go build -o app.exe
```

## Validation

The extension validates the following commands:

- `go version` - Must match pattern `go version go\d+\.\d+`

## Removal

```bash
extension-manager remove golang
```

This removes the mise configuration and Go tools.

## Related Extensions

- [mise-config](MISE-CONFIG.md) - Required mise configuration
- [ai-toolkit](AI-TOOLKIT.md) - Uses Go for some AI tools
