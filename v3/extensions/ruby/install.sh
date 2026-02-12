#!/bin/bash
set -euo pipefail

# ruby install script - Simplified for YAML-driven architecture
# Uses mise for Ruby version management with gem installation

print_status "Installing Ruby development environment via mise..."

# Install Ruby via mise
if ! command_exists mise; then
  print_error "mise is required but not found"
  exit 1
fi

# Use mise.toml for configuration
MISE_CONFIG="$(dirname "${BASH_SOURCE[0]}")/mise.toml"
if [[ ! -f "$MISE_CONFIG" ]]; then
  print_error "mise.toml not found"
  exit 1
fi

# Install Ruby
mise install -C "$(dirname "${BASH_SOURCE[0]}")"

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
