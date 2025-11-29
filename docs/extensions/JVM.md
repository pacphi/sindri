# JVM Languages

Java, Kotlin, Scala via SDKMAN and Clojure/Leiningen via mise.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | language               |
| **Version**      | 2.0.0                  |
| **Installation** | script (SDKMAN + mise) |
| **Disk Space**   | 2000 MB                |
| **Dependencies** | None                   |

## Description

JVM languages (Java, Kotlin, Scala) with SDKMAN and Clojure/Leiningen with mise - provides a comprehensive JVM development environment supporting multiple languages and build tools.

## Installed Tools

| Tool      | Type     | Source | Description             |
| --------- | -------- | ------ | ----------------------- |
| `java`    | runtime  | SDKMAN | Java 21 LTS (OpenJDK)   |
| `mvn`     | cli-tool | SDKMAN | Apache Maven build tool |
| `gradle`  | cli-tool | SDKMAN | Gradle build tool       |
| `kotlin`  | compiler | SDKMAN | Kotlin compiler         |
| `scala`   | compiler | SDKMAN | Scala compiler          |
| `clojure` | runtime  | mise   | Clojure runtime         |
| `lein`    | cli-tool | mise   | Leiningen build tool    |

## Configuration

### Templates

| Template          | Destination | Description           |
| ----------------- | ----------- | --------------------- |
| `bashrc.template` | `~/.bashrc` | SDKMAN initialization |

## Network Requirements

- `get.sdkman.io` - SDKMAN installer
- `api.sdkman.io` - SDKMAN API
- `github.com` - mise tools

## Installation

```bash
extension-manager install jvm
```

## Validation

```bash
java -version     # Expected: version "X.X.X"
mvn --version
gradle --version
kotlin -version
scala -version
clojure --version
lein --version
```

## Upgrade

**Strategy:** manual

Use the upgrade script to update all JVM tools:

```bash
extension-manager upgrade jvm
```

## Removal

```bash
extension-manager remove jvm
```

Removes:

- `~/.sdkman`
- `~/.m2`
- `~/.gradle`
