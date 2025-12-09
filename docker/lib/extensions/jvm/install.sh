#!/bin/bash
set -euo pipefail

# jvm install script - Simplified for YAML-driven architecture
# Uses SDKMAN for most JVM tools + mise for Clojure/Leiningen

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing JVM development environment..."

# Set SDKMAN directory
export SDKMAN_DIR="${SDKMAN_DIR:-$HOME/.sdkman}"

# Install SDKMAN
if [[ -d "$SDKMAN_DIR" ]] && [[ -f "$SDKMAN_DIR/bin/sdkman-init.sh" ]]; then
  print_warning "SDKMAN already installed"
else
  print_status "Installing SDKMAN..."
  if curl -s "https://get.sdkman.io" | bash; then
    print_success "SDKMAN installed"
  else
    print_error "Failed to install SDKMAN"
    exit 1
  fi
fi

# Source SDKMAN (with error handling for unbound variables)
set +u  # Temporarily disable unbound variable check for SDKMAN
# shellcheck source=/dev/null
source "$SDKMAN_DIR/bin/sdkman-init.sh" || {
  print_error "Failed to source SDKMAN init script"
  exit 1
}
set -u  # Re-enable unbound variable check

# Verify SDKMAN is working
if ! command -v sdk &>/dev/null; then
  print_error "SDKMAN 'sdk' command not available after sourcing"
  exit 1
fi

print_success "SDKMAN initialized: $(sdk version 2>/dev/null | head -1)"

# Detect architecture for appropriate Java distributions
ARCH=$(uname -m)
print_status "Detected architecture: $ARCH"

# Use Liberica JDK as primary (ARM-optimized by BellSoft, recommended by Spring)
# Falls back to Temurin if Liberica not available
# See: https://whichjdk.com/ and https://bell-sw.com/
print_status "Installing Java SDKs (this may take several minutes)..."

# Function to install Java version with vendor fallback
install_java() {
  local major_version="$1"
  local primary_vendor="${2:-librca}"    # Default to Liberica (ARM-optimized)
  local fallback_vendor="${3:-tem}"      # Fallback to Temurin

  print_status "Looking for Java $major_version..."

  # Get list of available Java versions (format: "| 21.0.9 | librca | | 21.0.9-librca")
  local java_list
  java_list=$(sdk list java 2>/dev/null || true)

  # Try primary vendor first (Liberica)
  local available_version
  available_version=$(echo "$java_list" | grep "${major_version}\\..*${primary_vendor}" | head -1 | awk '{print $NF}' || true)

  if [[ -n "$available_version" && "$available_version" != "|" ]]; then
    print_status "Installing Java $available_version (Liberica - ARM optimized)..."
    # Answer 'n' to "set as default" prompt, allow output for progress
    if echo "n" | sdk install java "$available_version"; then
      print_success "Java $available_version installed"
      return 0
    fi
  fi

  # Try fallback vendor (Temurin)
  available_version=$(echo "$java_list" | grep "${major_version}\\..*${fallback_vendor}" | head -1 | awk '{print $NF}' || true)
  if [[ -n "$available_version" && "$available_version" != "|" ]]; then
    print_status "Fallback: Installing Java $available_version (Temurin)..."
    if echo "n" | sdk install java "$available_version"; then
      print_success "Java $available_version installed"
      return 0
    fi
  fi

  # Last resort: any vendor with this major version
  available_version=$(echo "$java_list" | grep "${major_version}\\." | head -1 | awk '{print $NF}' || true)
  if [[ -n "$available_version" && "$available_version" != "|" ]]; then
    print_status "Last resort: Installing Java $available_version..."
    echo "n" | sdk install java "$available_version" || true
  else
    print_warning "No Java $major_version version found for this platform"
  fi
}

# Install Java LTS versions (Liberica preferred for ARM optimization)
install_java "21" "librca" "tem"   # Java 21 LTS (current)
install_java "17" "librca" "tem"   # Java 17 LTS
install_java "11" "librca" "tem"   # Java 11 LTS (legacy support)

# Set default Java version
print_status "Setting default Java version..."
default_java=$(sdk list java 2>/dev/null | grep "installed" | head -1 | awk '{print $NF}' || true)
if [[ -n "$default_java" ]]; then
  sdk default java "$default_java" 2>/dev/null || true
  print_success "Default Java: $default_java"
else
  print_warning "No Java version installed to set as default"
fi

# Install build tools with proper error handling
install_sdk_tool() {
  local tool="$1"
  local check_cmd="${2:-$tool}"

  if command_exists "$check_cmd"; then
    print_status "$tool already installed"
    return 0
  fi

  print_status "Installing $tool..."
  if sdk install "$tool"; then
    print_success "$tool installed"
  else
    print_warning "Failed to install $tool (may not be available for this platform)"
  fi
}

print_status "Installing build tools..."
install_sdk_tool maven mvn
install_sdk_tool gradle

# Install languages
print_status "Installing JVM languages..."
install_sdk_tool kotlin
install_sdk_tool scala
install_sdk_tool sbt

# Clojure via mise (more reliable than SDKMAN for Clojure)
if ! command_exists clojure; then
  if command_exists mise; then
    print_status "Installing Clojure via mise..."
    if mise use -g clojure@latest 2>/dev/null && mise install clojure 2>/dev/null; then
      print_success "Clojure installed via mise"
    else
      print_warning "Failed to install Clojure via mise"
    fi
  else
    print_warning "mise not available - skipping Clojure"
  fi
fi

# Leiningen via mise
if ! command_exists lein; then
  if command_exists mise; then
    print_status "Installing Leiningen via mise..."
    if mise use -g leiningen@latest 2>/dev/null && mise install leiningen 2>/dev/null; then
      print_success "Leiningen installed via mise"
    else
      print_warning "Failed to install Leiningen via mise"
    fi
  else
    print_warning "mise not available - skipping Leiningen"
  fi
fi

# Additional JVM tools (optional - skip failures silently)
print_status "Installing additional tools (optional)..."
install_sdk_tool jbang
install_sdk_tool springboot spring
install_sdk_tool micronaut mn

# Summary
print_status "=== JVM Installation Summary ==="
print_status "SDKMAN directory: $SDKMAN_DIR"
sdk current 2>/dev/null || print_warning "No SDK tools currently active"

print_success "JVM development environment installation complete"
print_status "Note: Run 'source ~/.bashrc' or start a new shell to use JVM tools"
