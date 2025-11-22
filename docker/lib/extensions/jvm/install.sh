#!/bin/bash
set -euo pipefail

# jvm install script - Simplified for YAML-driven architecture
# Uses SDKMAN for most JVM tools + mise for Clojure/Leiningen

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing JVM development environment..."

# Install SDKMAN
if [[ -d "$HOME/.sdkman" ]]; then
  print_warning "SDKMAN already installed"
  source "$HOME/.sdkman/bin/sdkman-init.sh"
else
  print_status "Installing SDKMAN..."
  if curl -s "https://get.sdkman.io" | bash; then
    source "$HOME/.sdkman/bin/sdkman-init.sh"
    print_success "SDKMAN installed"
  else
    print_error "Failed to install SDKMAN"
    exit 1
  fi
fi

# Install Java versions
print_status "Installing Java SDKs..."
java_versions=(
  "25.0.1-librca"    # Current LTS
  "21.0.7-librca"    # LTS
  "17.0.16-librca"   # Previous LTS
  "11.0.28-librca"   # Extended LTS
)

for version in "${java_versions[@]}"; do
  sdk install java "$version" <<< "n" 2>/dev/null || true
done

sdk default java 25.0.1-librca 2>/dev/null

# Install build tools
print_status "Installing build tools..."
command_exists mvn || sdk install maven 2>/dev/null
command_exists gradle || sdk install gradle 2>/dev/null

# Install languages
print_status "Installing JVM languages..."
command_exists kotlin || sdk install kotlin 2>/dev/null
command_exists scala || sdk install scala 2>/dev/null
command_exists sbt || sdk install sbt 2>/dev/null

# Clojure via mise
if ! command_exists clojure; then
  if command_exists mise; then
    mise use -g clojure@latest 2>/dev/null && mise install clojure 2>/dev/null
  else
    print_warning "mise not available - skipping Clojure"
  fi
fi

# Leiningen via mise
if ! command_exists lein; then
  if command_exists mise; then
    mise use -g leiningen@latest 2>/dev/null && mise install leiningen 2>/dev/null
  else
    print_warning "mise not available - skipping Leiningen"
  fi
fi

# Additional JVM tools
print_status "Installing additional tools..."
command_exists jbang || sdk install jbang 2>/dev/null
command_exists spring || sdk install springboot 2>/dev/null
command_exists mn || sdk install micronaut 2>/dev/null

print_success "JVM development environment installation complete"
