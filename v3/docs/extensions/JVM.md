# JVM Extension

> Version: 2.0.0 | Category: languages | Last Updated: 2026-01-26

## Overview

JVM languages (Java, Kotlin, Scala) with SDKMAN and Clojure/Leiningen with mise. Provides a complete JVM development environment with multiple language support.

## What It Provides

| Tool    | Type     | License                          | Description                      |
| ------- | -------- | -------------------------------- | -------------------------------- |
| java    | runtime  | GPL-2.0-with-classpath-exception | OpenJDK runtime                  |
| mvn     | cli-tool | Apache-2.0                       | Apache Maven build tool          |
| gradle  | cli-tool | Apache-2.0                       | Gradle build system              |
| kotlin  | compiler | Apache-2.0                       | Kotlin compiler                  |
| scala   | compiler | Apache-2.0                       | Scala compiler                   |
| clojure | runtime  | EPL-1.0                          | Clojure language                 |
| lein    | cli-tool | EPL-1.0                          | Leiningen build tool for Clojure |

## Requirements

- **Disk Space**: 2000 MB
- **Memory**: 4096 MB
- **Install Time**: ~180 seconds
- **Validation Timeout**: 60 seconds (JVM tools have slow cold start)
- **Dependencies**: None

### Network Domains

- sdkman.io, get.sdkman.io, api.sdkman.io
- downloads.gradle-dn.com, services.gradle.org, plugins.gradle.org
- adoptium.net, api.adoptium.net, download.eclipse.org
- bell-sw.com, download.bell-sw.com (Liberica JDK)
- repo.maven.apache.org, repo1.maven.org
- github.com, objects.githubusercontent.com

## Installation

```bash
extension-manager install jvm
```

## Configuration

### Templates

- bashrc.template - Shell configuration
- profile.template - Profile configuration

### Install Method

Uses a custom installation script with 900 second timeout.

### Upgrade Strategy

Manual - run upgrade.sh script.

## Usage Examples

### Java

```bash
# Check Java version
java -version

# Compile and run
javac Main.java
java Main

# Run a JAR
java -jar app.jar
```

### Maven

```bash
# Create a project
mvn archetype:generate -DgroupId=com.example -DartifactId=my-app

# Build
mvn clean install

# Run tests
mvn test

# Package
mvn package
```

### Gradle

```bash
# Initialize a project
gradle init

# Build
gradle build

# Run
gradle run

# Run tests
gradle test
```

### Kotlin

```bash
# Check version
kotlin -version

# Run a script
kotlin script.kts

# Compile
kotlinc main.kt -include-runtime -d app.jar
```

### Scala

```bash
# Check version
scala -version

# Interactive REPL
scala

# Run a file
scala app.scala

# Compile
scalac app.scala
```

### Clojure

```bash
# Check version
clojure --version

# Start REPL
clj

# Run a file
clojure -M app.clj
```

### Leiningen

```bash
# Create a project
lein new app my-app

# Run REPL
lein repl

# Run tests
lein test

# Build uberjar
lein uberjar
```

### SDKMAN

```bash
# List available JDKs
sdk list java

# Install a specific JDK
sdk install java 21.0.1-tem

# Switch JDK version
sdk use java 17.0.9-tem

# Set default version
sdk default java 21.0.1-tem
```

## Validation

The extension validates the following commands:

- `java -version` - Must match pattern `version "\d+\.\d+\.\d+"`
- `mvn` - Must be available
- `gradle` - Must be available
- `kotlin -version` - Must be available
- `scala -version` - Must be available
- `clojure --version` - Must be available
- `lein version` - Must be available

## Removal

```bash
extension-manager remove jvm
```

This removes:

- ~/.sdkman
- ~/.m2
- ~/.gradle

## Related Extensions

None - JVM is a comprehensive language extension.
