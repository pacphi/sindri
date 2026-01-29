# .NET

.NET SDK 10.0 and 8.0 with ASP.NET Core and development tools.

## Overview

| Property         | Value    |
| ---------------- | -------- |
| **Category**     | language |
| **Version**      | 2.1.0    |
| **Installation** | script   |
| **Disk Space**   | 2500 MB  |
| **Dependencies** | None     |

## Description

.NET SDK 10.0 and 8.0 with ASP.NET Core and development tools - provides the complete .NET development environment for cross-platform application development.

## Installed Tools

| Tool     | Type    | Description          |
| -------- | ------- | -------------------- |
| `dotnet` | runtime | .NET CLI and runtime |

### SDK Versions

- **.NET 10.0** - Current release
- **.NET 8.0** - LTS release

## Configuration

### Templates

| Template                         | Destination                                  | Description         |
| -------------------------------- | -------------------------------------------- | ------------------- |
| `bashrc-aliases.template`        | `~/.bashrc`                                  | .NET aliases        |
| `directory-build-props.template` | `/workspace/templates/Directory.Build.props` | MSBuild properties  |
| `editorconfig.template`          | `/workspace/templates/.editorconfig`         | Editor settings     |
| `global-json.template`           | `/workspace/templates/global.json`           | SDK version pinning |
| `nuget-config.template`          | `/workspace/templates/nuget.config`          | NuGet configuration |

## Network Requirements

- `dist.nuget.org` - NuGet packages
- `archive.ubuntu.com` - Ubuntu packages
- `ppa.launchpadcontent.net` - Microsoft PPA

## Installation

```bash
extension-manager install dotnet
```

## Validation

```bash
dotnet --version    # Expected: X.X.X
dotnet --list-sdks
```

## Upgrade

**Strategy:** automatic

Automatically updates via apt:

```yaml
apt:
  packages:
    - dotnet-sdk-10.0
    - dotnet-sdk-8.0
```

## Removal

```bash
extension-manager remove dotnet
```

Removes:

- `~/.dotnet`
- `~/.nuget`
- Template files
