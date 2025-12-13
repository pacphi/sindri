#!/usr/bin/env bash
set -euo pipefail

# Install script for supabase-cli
# Supabase CLI for local development, migrations, and edge functions

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/supabase-cli"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/supabase-cli/resources"

print_status "Installing Supabase CLI..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Install supabase npm package as a local dev dependency
# Note: Global npm install is not supported by Supabase
print_status "Installing supabase npm package..."
cd "${EXTENSION_DIR}"

# Initialize a minimal package.json if not present
if [[ ! -f "package.json" ]]; then
    cat > package.json << 'EOF'
{
  "name": "supabase-cli-wrapper",
  "version": "1.0.0",
  "private": true,
  "description": "Supabase CLI wrapper for Sindri",
  "scripts": {
    "supabase": "supabase"
  }
}
EOF
fi

# Install supabase as dev dependency
npm install supabase --save-dev

# Create a wrapper script for easy global-like access
cat > "${HOME}/workspace/bin/supabase" << 'EOF'
#!/usr/bin/env bash
# Supabase CLI wrapper
# Delegates to npx supabase

exec npx supabase "$@"
EOF
chmod +x "${HOME}/workspace/bin/supabase"

# Create an alternative wrapper in the extension directory
cat > "${EXTENSION_DIR}/run-supabase.sh" << 'EOF'
#!/usr/bin/env bash
# Run Supabase CLI
exec npx supabase "$@"
EOF
chmod +x "${EXTENSION_DIR}/run-supabase.sh"

# Verify installation
if npx supabase --version &>/dev/null; then
    VERSION=$(npx supabase --version 2>/dev/null || echo "unknown")
    print_success "Supabase CLI v${VERSION} installed successfully"
else
    print_warning "Supabase CLI installed but version check failed"
fi

print_status "Usage: supabase <command> or npx supabase <command>"
print_status "Run 'supabase init' to initialize a new project"
print_status "Run 'supabase start' to start local Supabase services (requires Docker)"

if [[ -n "${SUPABASE_ACCESS_TOKEN:-}" ]]; then
    print_success "SUPABASE_ACCESS_TOKEN is configured"
else
    print_warning "SUPABASE_ACCESS_TOKEN not set - some features may be limited"
    print_status "Get your token from: https://supabase.com/dashboard/account/tokens"
fi
