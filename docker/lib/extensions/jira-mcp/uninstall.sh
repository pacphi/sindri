#!/usr/bin/env bash
set -euo pipefail

# Uninstall script for jira-mcp

source "${DOCKER_LIB:-/docker/lib}/common.sh"

print_status "Removing Atlassian MCP server..."

# Remove Docker image
if docker image inspect ghcr.io/sooperset/mcp-atlassian:latest &>/dev/null; then
    print_status "Removing Docker image..."
    docker rmi ghcr.io/sooperset/mcp-atlassian:latest || true
fi

print_success "jira-mcp uninstalled"
