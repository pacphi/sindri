# SDKMAN Extension

> Version: 1.0.0 | Category: package-manager | Last Updated: 2026-01-29

## Overview

SDKMAN - The Software Development Kit Manager for JVM tools. Provides a comprehensive CLI for managing parallel versions of multiple JVM-related SDKs on Unix-based systems.

## What It Provides

| Tool   | Type            | License    | Description                                    |
| ------ | --------------- | ---------- | ---------------------------------------------- |
| sdkman | package-manager | Apache-2.0 | Software Development Kit Manager for JVM tools |

## Requirements

- **Disk Space**: 100 MB
- **Memory**: 128 MB
- **Install Time**: ~60 seconds
- **Validation Timeout**: 30 seconds
- **Dependencies**: None

### Network Domains

- sdkman.io
- get.sdkman.io
- api.sdkman.io

## Installation

```bash
sindri extension install sdkman
```

## Configuration

### Templates

- bashrc.template - Shell configuration
- profile.template - Profile configuration

### Environment Variables

- `SDKMAN_DIR` - Installation directory (`$HOME/.sdkman`)

### Install Method

Uses a custom installation script with 300 second timeout.

### Upgrade Strategy

Script-based - runs upgrade.sh script.

## About SDKMAN

SDKMAN is a tool for managing parallel versions of multiple Software Development Kits (SDKs) on Unix-based systems. It provides a convenient CLI interface to install, switch, remove and list candidate SDKs.

### Supported SDKs

SDKMAN supports a wide range of JVM-related tools including:

- **Java**: OpenJDK, Temurin, Liberica, GraalVM, Corretto, Zulu, and more
- **Build Tools**: Maven, Gradle, Ant, SBT
- **Languages**: Kotlin, Scala, Groovy, Ceylon
- **Frameworks**: Spring Boot, Micronaut, Quarkus
- **Tools**: JBang, VisualVM, JMeter, and many more

## Usage Examples

### Basic Commands

```bash
# List available SDKs
sdk list

# List available Java versions
sdk list java

# Show current versions
sdk current

# Get SDKMAN version
sdk version
```

### Installing SDKs

```bash
# Install a specific Java version
sdk install java 21.0.2-tem

# Install Liberica JDK (ARM-optimized)
sdk install java 21.0.2-librca

# Install Gradle
sdk install gradle 8.5

# Install Maven
sdk install maven 3.9.6

# Install Kotlin
sdk install kotlin 1.9.22
```

### Version Management

```bash
# Set default Java version (persists across shells)
sdk default java 21.0.2-tem

# Use a specific version in current shell only
sdk use java 17.0.10-tem

# Uninstall a version
sdk uninstall java 11.0.21-tem
```

### Multiple Versions

```bash
# Install multiple Java versions
sdk install java 21.0.2-tem
sdk install java 17.0.10-tem
sdk install java 11.0.21-tem

# Switch between versions
sdk use java 21  # Use Java 21 in current shell
sdk use java 17  # Use Java 17 in current shell

# Set global default
sdk default java 21.0.2-tem
```

### Offline Mode

```bash
# Enable offline mode (uses cached SDKs)
sdk offline enable

# Disable offline mode
sdk offline disable
```

### Updates

```bash
# Update SDKMAN itself
sdk selfupdate force

# Update SDK lists
sdk update
```

## Validation

The extension validates the following commands:

- `sdk version` - Must match pattern `SDKMAN!`

## Removal

```bash
sindri extension remove sdkman
```

This removes:

- ~/.sdkman

**Note:** Removing SDKMAN will also remove all SDKs installed via SDKMAN.

## Extensions That Depend on SDKMAN

The following extensions depend on SDKMAN:

- [jvm](JVM.md) - JVM development environment (Java, Kotlin, Scala)

## Related Extensions

- [mise-config](MISE-CONFIG.md) - Alternative polyglot tool version manager
- [jvm](JVM.md) - Uses SDKMAN for Java, Kotlin, and Scala

## Links

- [SDKMAN Documentation](https://sdkman.io/usage)
- [SDKMAN GitHub](https://github.com/sdkman/sdkman-cli)
- [Available SDKs](https://sdkman.io/sdks)
