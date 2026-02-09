#!/bin/bash
set -eo pipefail
# Note: We don't use 'set -u' (nounset) because SDKMAN scripts have unbound variables

# jvm install script - Simplified for YAML-driven architecture
# Uses SDKMAN for most JVM tools + mise for Clojure/Leiningen
# REQUIRES: sdkman extension to be installed first

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing JVM development environment..."

# Set SDKMAN directory
export SDKMAN_DIR="${SDKMAN_DIR:-$HOME/.sdkman}"

# Verify SDKMAN is installed (should be via sdkman extension dependency)
if [[ ! -f "$SDKMAN_DIR/bin/sdkman-init.sh" ]]; then
    print_error "SDKMAN not found. Please install the 'sdkman' extension first."
    print_error "Run: sindri extension install sdkman"
    exit 1
fi

# Source SDKMAN
# shellcheck source=/dev/null
# Note: In some environments (e.g., Fly.io), sdkman-init.sh may return non-zero
# even when successful. We rely on the subsequent sdk command check instead.
source "$SDKMAN_DIR/bin/sdkman-init.sh" 2>/dev/null || true

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
    # Note: Use { grep || true; } to prevent pipefail from causing script exit when no match
    local available_version
    available_version=$(echo "$java_list" | { grep "${major_version}\\..*${primary_vendor}" || true; } | head -1 | awk '{print $NF}')

    if [[ -n "$available_version" && "$available_version" != "|" ]]; then
        print_status "Installing Java $available_version (Liberica - ARM optimized)..."
        # Answer 'n' to "set as default" prompt, allow output for progress
        if echo "n" | sdk install java "$available_version"; then
            print_success "Java $available_version installed"
            return 0
        fi
    fi

    # Try fallback vendor (Temurin)
    available_version=$(echo "$java_list" | { grep "${major_version}\\..*${fallback_vendor}" || true; } | head -1 | awk '{print $NF}')
    if [[ -n "$available_version" && "$available_version" != "|" ]]; then
        print_status "Fallback: Installing Java $available_version (Temurin)..."
        if echo "n" | sdk install java "$available_version"; then
            print_success "Java $available_version installed"
            return 0
        fi
    fi

    # Last resort: any vendor with this major version
    available_version=$(echo "$java_list" | { grep "${major_version}\\." || true; } | head -1 | awk '{print $NF}')
    if [[ -n "$available_version" && "$available_version" != "|" ]]; then
        print_status "Last resort: Installing Java $available_version..."
        echo "n" | sdk install java "$available_version" || true
        return 0
    else
        print_warning "No Java $major_version version found for this platform"
        return 0  # Don't fail the entire script if a Java version isn't available
    fi
}

# Install Java LTS versions (Liberica preferred for ARM optimization)
install_java "25" "librca" "tem"   # Java 25 LTS (current)
install_java "21" "librca" "tem"   # Java 21 LTS (previous)
install_java "17" "librca" "tem"   # Java 17 LTS
install_java "11" "librca" "tem"   # Java 11 LTS (legacy support)

# Set default Java version
print_status "Setting default Java version..."
default_java=$(sdk list java 2>/dev/null | { grep "installed" || true; } | head -1 | awk '{print $NF}')
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
    local version="${3:-}"  # Optional version parameter

    if command_exists "$check_cmd"; then
        print_status "$tool already installed"
        return 0
    fi

    if [[ -n "$version" ]]; then
        print_status "Installing $tool $version..."
        if sdk install "$tool" "$version"; then
            print_success "$tool $version installed"
        else
            print_warning "Failed to install $tool $version (may not be available for this platform)"
        fi
    else
        print_status "Installing $tool (latest)..."
        if sdk install "$tool"; then
            print_success "$tool installed"
        else
            print_warning "Failed to install $tool (may not be available for this platform)"
        fi
    fi
}

print_status "Installing build tools..."
# Pinned versions for consistency (researched 2026-02-09)
install_sdk_tool maven mvn 3.9.12
install_sdk_tool gradle "" 9.3.1

# Install languages
print_status "Installing JVM languages..."
# Pinned versions for consistency (researched 2026-02-09)
install_sdk_tool kotlin "" 2.3.10
install_sdk_tool scala "" 3.8.1
install_sdk_tool sbt

# Clojure via mise (more reliable than SDKMAN for Clojure)
if ! command_exists clojure; then
    if command_exists mise; then
        print_status "Installing Clojure via mise..."
        if mise use -g clojure@1.12 2>/dev/null && mise install clojure 2>/dev/null; then
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
        if mise use -g leiningen@2.12 2>/dev/null && mise install leiningen 2>/dev/null; then
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
