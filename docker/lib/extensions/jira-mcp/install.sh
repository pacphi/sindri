#!/usr/bin/env bash
set -euo pipefail

# Install script for jira-mcp
# Atlassian Jira and Confluence MCP server for AI-powered issue tracking

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/jira-mcp"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/jira-mcp/resources"

print_status "Installing Atlassian MCP server (Jira/Confluence)..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources (SKILL.md and other files)
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"
fi

# Pull the mcp-atlassian Docker image
print_status "Pulling mcp-atlassian Docker image..."
if docker pull ghcr.io/sooperset/mcp-atlassian:latest; then
    print_success "Docker image pulled successfully"
else
    print_warning "Failed to pull Docker image - will be pulled on first use"
fi

# Create a wrapper script for running the MCP server
cat > "${EXTENSION_DIR}/run-jira-mcp.sh" << 'EOF'
#!/usr/bin/env bash
# Run Atlassian MCP server (Jira/Confluence)
# Requires: JIRA_URL, JIRA_USERNAME, JIRA_API_TOKEN environment variables

set -euo pipefail

# Check required environment variables
if [[ -z "${JIRA_URL:-}" ]]; then
    echo "Error: JIRA_URL environment variable is not set"
    echo "Example: https://your-company.atlassian.net"
    exit 1
fi

if [[ -z "${JIRA_USERNAME:-}" ]]; then
    echo "Error: JIRA_USERNAME environment variable is not set"
    echo "This should be your email address"
    exit 1
fi

if [[ -z "${JIRA_API_TOKEN:-}" ]]; then
    echo "Error: JIRA_API_TOKEN environment variable is not set"
    echo "Get your token from: https://id.atlassian.com/manage-profile/security/api-tokens"
    exit 1
fi

# Run the MCP server
exec docker run -i --rm \
    -e JIRA_URL="${JIRA_URL}" \
    -e JIRA_USERNAME="${JIRA_USERNAME}" \
    -e JIRA_API_TOKEN="${JIRA_API_TOKEN}" \
    ${CONFLUENCE_URL:+-e CONFLUENCE_URL="${CONFLUENCE_URL}"} \
    ${CONFLUENCE_USERNAME:+-e CONFLUENCE_USERNAME="${CONFLUENCE_USERNAME}"} \
    ${CONFLUENCE_API_TOKEN:+-e CONFLUENCE_API_TOKEN="${CONFLUENCE_API_TOKEN}"} \
    ${JIRA_PROJECTS_FILTER:+-e JIRA_PROJECTS_FILTER="${JIRA_PROJECTS_FILTER}"} \
    ${READ_ONLY_MODE:+-e READ_ONLY_MODE="${READ_ONLY_MODE}"} \
    ghcr.io/sooperset/mcp-atlassian:latest "$@"
EOF
chmod +x "${EXTENSION_DIR}/run-jira-mcp.sh"

# Create Claude Code MCP configuration snippet (Docker method)
cat > "${EXTENSION_DIR}/claude-mcp-config-docker.json" << 'EOF'
{
  "mcpServers": {
    "atlassian": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-e", "JIRA_URL",
        "-e", "JIRA_USERNAME",
        "-e", "JIRA_API_TOKEN",
        "ghcr.io/sooperset/mcp-atlassian:latest"
      ],
      "env": {
        "JIRA_URL": "https://your-company.atlassian.net",
        "JIRA_USERNAME": "your-email@company.com",
        "JIRA_API_TOKEN": "your_api_token"
      }
    }
  }
}
EOF

# Create Claude Code MCP configuration snippet (Official Atlassian remote method)
cat > "${EXTENSION_DIR}/claude-mcp-config-official.json" << 'EOF'
{
  "mcpServers": {
    "atlassian": {
      "command": "claude",
      "args": ["mcp", "add", "--transport", "sse", "atlassian", "https://mcp.atlassian.com/v1/sse"]
    }
  }
}
EOF

# Create .env template
cat > "${EXTENSION_DIR}/.env.template" << 'EOF'
# Atlassian MCP Server Configuration
# Copy this to .env and fill in your values

# Required: Jira Cloud configuration
JIRA_URL=https://your-company.atlassian.net
JIRA_USERNAME=your-email@company.com
JIRA_API_TOKEN=your_api_token_here

# Optional: Confluence configuration
#CONFLUENCE_URL=https://your-company.atlassian.net/wiki
#CONFLUENCE_USERNAME=your-email@company.com
#CONFLUENCE_API_TOKEN=your_api_token_here

# Optional: Filtering
#JIRA_PROJECTS_FILTER=PROJ1,PROJ2
#CONFLUENCE_SPACES_FILTER=DEV,TEAM

# Optional: Read-only mode (set to true to disable write operations)
#READ_ONLY_MODE=false
EOF

print_success "jira-mcp installed successfully"
print_status ""
print_status "Configuration required:"
print_status "  1. Get API token: https://id.atlassian.com/manage-profile/security/api-tokens"
print_status "  2. Set environment variables:"
print_status "     export JIRA_URL='https://your-company.atlassian.net'"
print_status "     export JIRA_USERNAME='your-email@company.com'"
print_status "     export JIRA_API_TOKEN='your_token'"
print_status ""
print_status "Two MCP options available:"
print_status "  - Docker (self-hosted): See ${EXTENSION_DIR}/claude-mcp-config-docker.json"
print_status "  - Official Atlassian (cloud): claude mcp add --transport sse atlassian https://mcp.atlassian.com/v1/sse"
