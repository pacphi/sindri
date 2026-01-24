#!/bin/bash
set -euo pipefail

# dotnet install script - Simplified for YAML-driven architecture
# Installs .NET SDK 10.0 and 8.0 with ASP.NET Core and dev tools

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing .NET development environment..."

# Check if already installed
if command_exists dotnet; then
  dotnet_version=$(dotnet --version 2>/dev/null)
  print_warning ".NET already installed: $dotnet_version"
  exit 0
fi

# Add .NET backports PPA if requested
if [[ "${EXT_USE_DOTNET_BACKPORTS:-false}" == "true" ]]; then
  print_status "Adding Ubuntu .NET backports PPA..."
  sudo add-apt-repository -y ppa:dotnet/backports 2>/dev/null || print_warning "Backports PPA failed"
fi

# Update package lists
sudo apt-get update || exit 1

# Install .NET SDKs
print_status "Installing .NET SDKs..."
dotnet_sdks=(
  "dotnet-sdk-10.0"   # Current LTS
  "dotnet-sdk-8.0"    # Previous version
)

for sdk in "${dotnet_sdks[@]}"; do
  sudo apt-get install -y "$sdk" 2>/dev/null || print_warning "Failed to install $sdk"
done

# Install ASP.NET Core Runtimes
print_status "Installing ASP.NET Core Runtimes..."
for runtime in aspnetcore-runtime-10.0 aspnetcore-runtime-8.0; do
  sudo apt-get install -y "$runtime" 2>/dev/null || print_warning "Failed to install $runtime"
done

# Verify installation
if ! command_exists dotnet; then
  print_error ".NET installation failed"
  exit 1
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
  sudo wget -O /usr/local/bin/nuget.exe https://dist.nuget.org/win-x86-commandline/latest/nuget.exe 2>/dev/null
  sudo cp "$(dirname "${BASH_SOURCE[0]}")/nuget-wrapper.template" /usr/local/bin/nuget
  sudo chmod +x /usr/local/bin/nuget
  sudo apt-get install -y mono-complete 2>/dev/null && print_success "NuGet CLI installed"
fi

print_success ".NET development environment installation complete"
