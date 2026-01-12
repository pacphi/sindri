# Go (Golang)

Go 1.25 compiler and toolchain via mise.

## Overview

| Property         | Value    |
| ---------------- | -------- |
| **Category**     | language |
| **Version**      | 1.0.0    |
| **Installation** | mise     |
| **Disk Space**   | 500 MB   |
| **Dependencies** | None     |

## Description

Go 1.25 via mise - provides the Go compiler and development toolchain for systems programming.

## Installed Tools

| Tool | Type     | Description               |
| ---- | -------- | ------------------------- |
| `go` | compiler | Go compiler and toolchain |

## Configuration

### Environment Variables

| Variable | Value  | Scope  |
| -------- | ------ | ------ |
| `GOPATH` | `~/go` | bashrc |

### mise.toml

```toml
[tools]
go = "1.25"
```

## Network Requirements

- `golang.org` - Go downloads
- `proxy.golang.org` - Go module proxy

## Installation

```bash
extension-manager install golang
```

## Validation

```bash
go version    # Expected: go version goX.X
```

## Removal

```bash
extension-manager remove golang
```

Removes mise configuration and Go installation.

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools (requires golang)
- [infra-tools](INFRA-TOOLS.md) - Infrastructure tools
