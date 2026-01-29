# SDKMAN Extension

**Category:** Package Manager
**Version:** 1.0.0
**Author:** SDKMAN Team

## Overview

SDKMAN is a tool for managing parallel versions of multiple Software Development Kits (SDKs) on Unix-based systems. It provides a convenient CLI interface to install, switch, remove and list candidate SDKs.

This extension installs SDKMAN and configures your shell to use it automatically.

## Installation

```bash
sindri extension install sdkman
```

## Features

- **Multiple SDK Versions**: Install and switch between multiple versions of JDK, Maven, Gradle, etc.
- **Easy Version Switching**: Use `sdk use` to switch between versions
- **Auto-updates**: SDKMAN keeps itself up to date
- **Offline Mode**: Works with cached SDKs when offline

## Supported SDKs

SDKMAN supports a wide range of JVM-related tools including:

- **Java**: OpenJDK, Temurin, Liberica, GraalVM, Corretto, Zulu, and more
- **Build Tools**: Maven, Gradle, Ant, SBT
- **Languages**: Kotlin, Scala, Groovy, Ceylon
- **Frameworks**: Spring Boot, Micronaut, Quarkus
- **Tools**: JBang, VisualVM, JMeter, and many more

## Usage

After installation, open a new terminal or run:

```bash
source ~/.bashrc
```

Then you can use SDKMAN:

```bash
# List available SDKs
sdk list

# List available Java versions
sdk list java

# Install a specific Java version
sdk install java 21.0.2-tem

# Set default Java version
sdk default java 21.0.2-tem

# Use a specific version in current shell
sdk use java 17.0.10-tem

# Show current versions
sdk current
```

## Dependent Extensions

The following extensions depend on SDKMAN:

- `jvm` - JVM development environment (Java, Kotlin, Scala)

## Configuration

SDKMAN is installed at `~/.sdkman` and configured in your shell profile files:

- `~/.bashrc`
- `~/.profile`

## Upgrade

```bash
# Upgrade via extension manager
sindri extension upgrade sdkman

# Or directly via SDKMAN
sdk selfupdate force
```

## Removal

```bash
sindri extension remove sdkman
```

**Note:** Removing SDKMAN will also remove all SDKs installed via SDKMAN.

## Links

- [SDKMAN Documentation](https://sdkman.io/usage)
- [SDKMAN GitHub](https://github.com/sdkman/sdkman-cli)
- [Available SDKs](https://sdkman.io/sdks)
