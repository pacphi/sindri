#!/usr/bin/env bash
set -euo pipefail

# Shannon installation script for Sindri V3

SHANNON_HOME="${HOME}/.shannon"
REPO_URL="https://github.com/KeygraphHQ/shannon.git"

echo "Installing Shannon autonomous pentester..."

# Check Docker is installed
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is required but not installed."
    echo "Please install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

# Check Docker daemon is running
if ! docker info &> /dev/null; then
    echo "Error: Docker daemon is not running."
    echo "Please start Docker and try again."
    exit 1
fi

# Create Shannon home directory
mkdir -p "${SHANNON_HOME}"

# Clone Shannon repository
echo "Cloning Shannon repository..."
if [ -d "${SHANNON_HOME}/shannon" ]; then
    echo "Shannon directory already exists, updating..."
    cd "${SHANNON_HOME}/shannon"
    git pull origin main || git pull origin master
else
    git clone "${REPO_URL}" "${SHANNON_HOME}/shannon"
fi

# Create wrapper script
echo "Creating Shannon wrapper script..."
cat > "${SHANNON_HOME}/shannon" << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

SHANNON_REPO="${HOME}/.shannon/shannon"

if [ ! -d "${SHANNON_REPO}" ]; then
    echo "Error: Shannon repository not found at ${SHANNON_REPO}"
    exit 1
fi

cd "${SHANNON_REPO}"
exec ./shannon "$@"
EOF

chmod +x "${SHANNON_HOME}/shannon"

# Pull initial Docker images (non-blocking, to speed up first run)
echo "Pre-pulling Docker images (this may take a moment)..."
cd "${SHANNON_HOME}/shannon"
docker compose pull 2>/dev/null || true

echo ""
echo "Shannon installation complete!"
echo ""
echo "Next steps:"
echo "1. Set your Anthropic API key:"
echo "   export ANTHROPIC_API_KEY='your-api-key'"
echo "   export CLAUDE_CODE_MAX_OUTPUT_TOKENS=64000"
echo ""
echo "2. Run a pentest:"
echo "   shannon start URL=https://your-app.com REPO=/path/to/repo"
echo ""
echo "Location: ${SHANNON_HOME}"
