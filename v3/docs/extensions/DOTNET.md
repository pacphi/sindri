# .NET Extension

> Version: 2.1.0 | Category: languages | Last Updated: 2026-01-26

## Overview

.NET SDK 10.0 and 8.0 with ASP.NET Core and development tools. Provides a complete .NET development environment with templates and configuration.

## What It Provides

| Tool   | Type    | License | Description          |
| ------ | ------- | ------- | -------------------- |
| dotnet | runtime | MIT     | .NET SDK and runtime |

## Requirements

- **Disk Space**: 1000 MB
- **Memory**: 512 MB
- **Install Time**: ~120 seconds
- **Dependencies**: None

### Network Domains

- nuget.org
- dist.nuget.org
- archive.ubuntu.com
- ppa.launchpadcontent.net

## Installation

```bash
extension-manager install dotnet
```

## Configuration

### Templates Installed

| Template                       | Destination                       | Description              |
| ------------------------------ | --------------------------------- | ------------------------ |
| bashrc-aliases.template        | ~/.bashrc                         | Shell aliases for .NET   |
| directory-build-props.template | ~/templates/Directory.Build.props | MSBuild properties       |
| editorconfig.template          | ~/templates/.editorconfig         | Code style configuration |
| global-json.template           | ~/templates/global.json           | SDK version pinning      |
| nuget-config.template          | ~/templates/nuget.config          | NuGet package sources    |

### Install Method

Uses a custom installation script with 900 second timeout.

### Upgrade Strategy

Automatic via apt packages.

## Usage Examples

### Creating Projects

```bash
# Create a console app
dotnet new console -n MyApp

# Create a web API
dotnet new webapi -n MyApi

# Create a Blazor app
dotnet new blazor -n MyBlazorApp

# Create a class library
dotnet new classlib -n MyLibrary
```

### Building and Running

```bash
# Build a project
dotnet build

# Run a project
dotnet run

# Run with watch mode
dotnet watch run

# Publish for deployment
dotnet publish -c Release
```

### Package Management

```bash
# Add a package
dotnet add package Newtonsoft.Json

# Remove a package
dotnet remove package Newtonsoft.Json

# Restore packages
dotnet restore

# List packages
dotnet list package
```

### Testing

```bash
# Create a test project
dotnet new xunit -n MyTests

# Run tests
dotnet test

# Run with coverage
dotnet test --collect:"XPlat Code Coverage"
```

### Entity Framework Core

```bash
# Add EF tools
dotnet tool install --global dotnet-ef

# Create a migration
dotnet ef migrations add InitialCreate

# Update database
dotnet ef database update
```

### SDK Management

```bash
# List installed SDKs
dotnet --list-sdks

# List installed runtimes
dotnet --list-runtimes

# Check version
dotnet --version
```

## Validation

The extension validates the following commands:

- `dotnet` - Must match pattern `\d+\.\d+\.\d+`

## Removal

```bash
extension-manager remove dotnet
```

This removes:

- ~/.dotnet
- ~/.nuget
- ~/templates/Directory.Build.props
- ~/templates/.editorconfig
- ~/templates/global.json
- ~/templates/nuget.config

## Related Extensions

None - .NET is a standalone language extension.
