#!/bin/bash
set -euo pipefail

# dotnet install script - Simplified for YAML-driven architecture
# Installs .NET SDK 10.0 and 8.0 with ASP.NET Core and dev tools

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

# Determine if we need sudo (handle no-new-privileges containers)
if [[ $(id -u) -eq 0 ]]; then
  # Already root, no sudo needed
  SUDO=""
elif sudo -n true 2>/dev/null; then
  # Sudo works without password
  SUDO="sudo"
else
  # Try without sudo (might fail, but worth attempting)
  print_warning "sudo not available, attempting commands as current user"
  SUDO=""
fi

print_status "Installing .NET development environment..."

# Check if already installed
if command_exists dotnet; then
  dotnet_version=$(dotnet --version 2>/dev/null)
  print_warning ".NET already installed: $dotnet_version"
  exit 0
fi

# Add .NET backports PPA if requested (for .NET 9 or older versions on Ubuntu 24.04)
if [[ "${EXT_USE_DOTNET_BACKPORTS:-false}" == "true" ]]; then
  print_status "Adding Ubuntu .NET backports PPA..."
  $SUDO add-apt-repository -y ppa:dotnet/backports 2>/dev/null || print_warning "Backports PPA failed"
fi

# Try apt-based installation first (requires root)
apt_install_succeeded=false

if [[ -n "$SUDO" ]] || [[ $(id -u) -eq 0 ]]; then
  # Update package lists
  print_status "Updating package lists..."
  if $SUDO apt-get update 2>/dev/null; then
    # Show available .NET packages for debugging
    print_status "Checking available .NET packages..."
    apt-cache search dotnet-sdk 2>/dev/null | head -10 || true

    # Install .NET SDKs - try newest first, fall back to older versions
    print_status "Installing .NET SDKs via apt..."

    # Try .NET 10 first (LTS as of Nov 2025)
    if $SUDO apt-get install -y dotnet-sdk-10.0 2>/dev/null; then
      print_success "Installed dotnet-sdk-10.0"
      apt_install_succeeded=true
    else
      print_warning "dotnet-sdk-10.0 not available via apt, trying .NET 8..."
    fi

    # Try .NET 8 as fallback (LTS)
    if ! $apt_install_succeeded; then
      if $SUDO apt-get install -y dotnet-sdk-8.0 2>/dev/null; then
        print_success "Installed dotnet-sdk-8.0"
        apt_install_succeeded=true
      fi
    fi

    # Install ASP.NET Core Runtimes (optional)
    if $apt_install_succeeded; then
      print_status "Installing ASP.NET Core Runtimes..."
      $SUDO apt-get install -y aspnetcore-runtime-10.0 2>/dev/null || true
      $SUDO apt-get install -y aspnetcore-runtime-8.0 2>/dev/null || true
    fi
  fi
fi

# If apt failed, use Microsoft's dotnet-install.sh (works without root)
if ! $apt_install_succeeded; then
  print_warning "apt installation not available (requires root), using dotnet-install.sh..."

  # Set up .NET installation directory in user space
  export DOTNET_ROOT="${DOTNET_ROOT:-$HOME/.dotnet}"
  mkdir -p "$DOTNET_ROOT"

  # Download and run Microsoft's official install script
  print_status "Downloading dotnet-install.sh from Microsoft..."
  INSTALL_SCRIPT="/tmp/dotnet-install.sh"

  if curl -fsSL https://dot.net/v1/dotnet-install.sh -o "$INSTALL_SCRIPT"; then
    chmod +x "$INSTALL_SCRIPT"

    # Install .NET 10 LTS
    print_status "Installing .NET 10 SDK..."
    if "$INSTALL_SCRIPT" --channel 10.0 --install-dir "$DOTNET_ROOT"; then
      print_success "Installed .NET 10 SDK to $DOTNET_ROOT"
    else
      print_warning ".NET 10 installation failed, trying .NET 8..."
      # Try .NET 8 as fallback
      if "$INSTALL_SCRIPT" --channel 8.0 --install-dir "$DOTNET_ROOT"; then
        print_success "Installed .NET 8 SDK to $DOTNET_ROOT"
      else
        print_error "Failed to install .NET SDK"
        rm -f "$INSTALL_SCRIPT"
        exit 1
      fi
    fi

    rm -f "$INSTALL_SCRIPT"

    # Add to PATH for this session
    export PATH="$DOTNET_ROOT:$PATH"

    # Add to .bashrc for future sessions
    if ! grep -q "DOTNET_ROOT" "$HOME/.bashrc" 2>/dev/null; then
      cat >> "$HOME/.bashrc" << 'DOTNET_ENV'

# .NET SDK
export DOTNET_ROOT="$HOME/.dotnet"
export PATH="$DOTNET_ROOT:$PATH"
DOTNET_ENV
      print_status "Added .NET to ~/.bashrc"
    fi
  else
    print_error "Failed to download dotnet-install.sh"
    exit 1
  fi
fi

# Verify installation
if ! command_exists dotnet; then
  # Try with explicit path
  if [[ -x "${DOTNET_ROOT:-$HOME/.dotnet}/dotnet" ]]; then
    export PATH="${DOTNET_ROOT:-$HOME/.dotnet}:$PATH"
  else
    print_error ".NET installation failed - dotnet command not found"
    print_status "Checked PATH: $PATH"
    print_status "Checked DOTNET_ROOT: ${DOTNET_ROOT:-not set}"
    exit 1
  fi
fi

print_success ".NET installed: $(dotnet --version)"

# Check CI mode
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  print_status "CI mode - skipping .NET global tools"
  exit 0
fi

# Set environment for tool installation
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_ROOT=/usr/share/dotnet
export PATH=$PATH:$HOME/.dotnet/tools

# Install .NET global tools
print_status "Installing .NET global tools..."
dotnet_tools=(
  "dotnet-ef"
  "dotnet-aspnet-codegenerator"
  "dotnet-format"
  "dotnet-outdated-tool"
  "dotnet-script"
  "dotnet-serve"
  "Microsoft.dotnet-interactive"
  "Microsoft.Web.LibraryManager.Cli"
  "dotnet-reportgenerator-globaltool"
  "dotnet-sonarscanner"
  "BenchmarkDotNet.Tool"
  "dotnet-counters"
  "dotnet-trace"
  "dotnet-dump"
  "Swashbuckle.AspNetCore.Cli"
  "coverlet.console"
)

for tool in "${dotnet_tools[@]}"; do
  dotnet tool install --global "$tool" 2>/dev/null || \
    dotnet tool update --global "$tool" 2>/dev/null || \
    print_warning "Failed to install $tool"
done

# Install NuGet CLI
print_status "Installing NuGet CLI..."
if ! command_exists nuget; then
  $SUDO wget -O /usr/local/bin/nuget.exe https://dist.nuget.org/win-x86-commandline/latest/nuget.exe 2>/dev/null
  $SUDO cp "$(dirname "${BASH_SOURCE[0]}")/nuget-wrapper.template" /usr/local/bin/nuget
  $SUDO chmod +x /usr/local/bin/nuget
  $SUDO apt-get install -y mono-complete 2>/dev/null && print_success "NuGet CLI installed"
fi

print_success ".NET development environment installation complete"
