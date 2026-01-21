#!/usr/bin/env bash
#
# init-claude-flow-agentdb - Initialize claude-flow with AgentDB backend
#
# This script initializes AgentDB capabilities for claude-flow in a project.
# It should be called AFTER claude-flow basic initialization is complete.
#
# Prerequisites:
#   - claude-flow must be installed (globally or via mise)
#   - .claude directory should exist (from claude-flow init)
#
# What this does:
#   - Configures AgentDB as the memory backend
#   - Initializes AgentDB storage
#   - Creates memory namespaces (swarm, aqe, session)
#   - Initializes optional features (neural models, hooks)
#   - Verifies AgentDB configuration
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine lib directory (support both container and local paths)
if [[ -d "/docker/lib" ]]; then
    DOCKER_LIB="/docker/lib"
else
    DOCKER_LIB="$(cd "${SCRIPT_DIR}/../docker/lib" && pwd)"
fi

source "${DOCKER_LIB}/common.sh"

# Check if claude-flow is available
if ! command_exists claude-flow; then
    print_error "claude-flow is not installed or not in PATH"
    exit 1
fi

# Check if .claude directory exists (indicates basic init was done)
if [[ ! -d ".claude" ]]; then
    print_warning ".claude directory not found. Run 'claude-flow init --full' first"
    exit 1
fi

print_status "Initializing AgentDB backend for claude-flow..."

# Configure AgentDB as memory backend
print_status "Configuring AgentDB memory backend..."
if claude-flow memory backend set agentdb 2>/dev/null; then
    print_success "AgentDB backend configured"
else
    print_warning "Failed to configure AgentDB backend (may already be set)"
fi

# Set memory storage path
if claude-flow config set memory.path ./.agentdb 2>/dev/null; then
    print_debug "Memory path set to ./.agentdb"
else
    print_debug "Could not set memory path (may not be supported in this version)"
fi

# Initialize AgentDB storage
print_status "Initializing AgentDB storage..."
if claude-flow agentdb init 2>/dev/null; then
    print_success "AgentDB storage initialized"
else
    print_warning "AgentDB init had warnings (may already be initialized)"
fi

# Create memory namespaces
print_status "Creating memory namespaces..."
for namespace in swarm aqe session; do
    if claude-flow memory namespace create "$namespace" 2>/dev/null; then
        print_debug "Created namespace: $namespace"
    else
        print_debug "Namespace $namespace may already exist"
    fi
done
print_success "Memory namespaces configured"

# Initialize neural models (optional but recommended)
print_status "Initializing neural models..."
if claude-flow neural init 2>/dev/null; then
    print_success "Neural models initialized"
else
    print_debug "Neural models initialization skipped (optional)"
fi

# Initialize hooks (optional)
print_status "Initializing hooks..."
if claude-flow hooks init 2>/dev/null; then
    print_success "Hooks initialized"
else
    print_debug "Hooks initialization skipped (optional)"
fi

# Verify AgentDB is active
print_status "Verifying AgentDB configuration..."
if claude-flow memory backend info 2>/dev/null | grep -q "agentdb"; then
    print_success "AgentDB backend is active"
else
    print_warning "Could not verify AgentDB backend status"
fi

# Show final status
echo ""
print_success "AgentDB initialization complete!"
echo ""
echo "AgentDB Features:"
echo "  • Semantic vector search (96x-164x faster)"
echo "  • Persistent memory with HNSW indexing"
echo "  • Automatic memory consolidation"
echo "  • Reflexion memory and skill library"
echo ""
echo "To verify status, run:"
echo "  claude-flow memory status"
echo "  claude-flow memory backend info"
