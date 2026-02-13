#!/bin/bash
set -euo pipefail

# ruby install script - Simplified for YAML-driven architecture
# Uses mise for Ruby version management with gem installation

# Auto-accept mise trust prompts. The script runs with cwd set to the extension
# directory where mise.toml lives; mise discovers it and refuses to parse
# untrusted configs. MISE_YES=1 follows the same convention as mise-config.
export MISE_YES=1

print_status "Installing Ruby development environment via mise..."

# Install Ruby via mise
if ! command_exists mise; then
  print_error "mise is required but not found"
  exit 1
fi

# Copy mise.toml to conf.d (trusted path) and install from there
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
MISE_CONF_DIR="$HOME/.config/mise/conf.d"
mkdir -p "$MISE_CONF_DIR"
TOML_FILE="$MISE_CONF_DIR/ruby.toml"

if [[ -f "$SCRIPT_DIR/mise.toml" ]]; then
  cp "$SCRIPT_DIR/mise.toml" "$TOML_FILE"
  print_success "Created mise config: $TOML_FILE"
else
  print_error "mise.toml not found in extension directory"
  exit 1
fi

# Install Ruby - use the trusted config in conf.d, not the untrusted source
# Global configs (~/.config/mise/conf.d/*) are implicitly trusted by mise
# CRITICAL: Change to home directory to avoid mise discovering the untrusted
# mise.toml in the extension directory (current working directory)
cd "$HOME" || exit 1
if mise install 2>&1; then
  print_success "Ruby installed via mise"
else
  print_error "mise install failed"
  exit 1
fi

# Activate mise for current session
eval "$(mise activate bash)"

# Verify Ruby is available
if ! command_exists ruby; then
  print_error "ruby not found after mise install"
  exit 1
fi

print_success "Ruby $(ruby -v | awk '{print $2}') installed via mise"

# Install Bundler
print_status "Installing Bundler..."
gem install bundler 2>/dev/null || {
  print_error "Failed to install Bundler"
  exit 1
}
print_success "Bundler installed: $(bundle -v 2>&1 | head -n1)"

# Check if running in CI mode
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  print_status "CI mode detected - skipping Rails and development gems"
  exit 0
fi

# Install Rails
print_status "Installing Ruby on Rails..."
if gem install rails 2>/dev/null; then
  print_success "Rails installed: $(rails -v 2>&1 | head -n1)"
else
  print_warning "Failed to install Rails"
fi

# Install Sinatra
print_status "Installing Sinatra..."
gem install sinatra sinatra-contrib 2>/dev/null || print_warning "Failed to install Sinatra"

# Install Ruby development gems
print_status "Installing Ruby development gems..."
ruby_gems=(
  pry pry-byebug rubocop rubocop-rails rubocop-performance
  reek brakeman bundler-audit solargraph rufo
  rspec minitest factory_bot faker database_cleaner simplecov
)

for gem_name in "${ruby_gems[@]}"; do
  gem install "$gem_name" 2>/dev/null || print_warning "Failed to install $gem_name"
done

print_success "Ruby development environment installation complete"
