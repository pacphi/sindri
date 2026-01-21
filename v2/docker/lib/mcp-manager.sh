#!/usr/bin/env bash

# mcp-manager.sh - MCP (Model Context Protocol) server registration
#
# This module provides MCP server registration capabilities for extensions:
# - Register extensions as MCP servers with Claude Code
# - Update Claude Code configuration (~/.claude/config.json)
# - List MCP-capable extensions
# - Unregister MCP servers
#
# Extensions with MCP capabilities can expose tools, resources, and prompts
# that become available in Claude Code's tool use system

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

# Source capability manager for extension capability queries
if [[ -f "${SCRIPT_DIR}/capability-manager.sh" ]]; then
    source "${SCRIPT_DIR}/capability-manager.sh"
fi

# Constants
CLAUDE_CONFIG_FILE="${HOME}/.claude/config.json"
CLAUDE_CONFIG_DIR="${HOME}/.claude"

###############################################################################
# MCP Server Registration Functions
###############################################################################

# Register an extension as an MCP server in Claude Code config
# Usage: register_mcp_server <extension_name>
# Returns: 0 on success, 1 on failure
register_mcp_server() {
    local ext="$1"

    # Check if MCP capability is enabled
    local mcp_enabled
    mcp_enabled=$(get_extension_capability "$ext" "mcp.enabled")

    if [[ "$mcp_enabled" != "true" ]]; then
        print_info "${ext} does not have MCP capability enabled"
        return 0
    fi

    # Get MCP server configuration
    local server_command
    local server_args
    local server_env

    server_command=$(get_extension_capability "$ext" "mcp.server.command")

    if [[ -z "$server_command" || "$server_command" == "null" ]]; then
        print_error "${ext} MCP capability enabled but server command not defined"
        return 1
    fi

    server_args=$(get_extension_capability "$ext" "mcp.server.args")
    server_env=$(get_extension_capability "$ext" "mcp.server.env")

    print_info "Registering ${ext} as MCP server..."

    # Ensure Claude config directory exists
    if [[ ! -d "$CLAUDE_CONFIG_DIR" ]]; then
        mkdir -p "$CLAUDE_CONFIG_DIR"
    fi

    # Ensure config file exists
    if [[ ! -f "$CLAUDE_CONFIG_FILE" ]]; then
        echo '{"mcpServers":{}}' > "$CLAUDE_CONFIG_FILE"
    fi

    # Check if jq is available
    if ! command_exists jq; then
        print_error "jq not available - required for MCP server registration"
        print_info "Install jq: apt install jq (Debian/Ubuntu) or brew install jq (macOS)"
        return 1
    fi

    # Build MCP server configuration JSON
    local server_config='{"command":"'"$server_command"'"'

    # Add args if defined
    if [[ -n "$server_args" && "$server_args" != "null" ]]; then
        # Parse args array and build JSON array
        local args_json
        args_json=$(echo "$server_args" | yq eval -o=json - 2>/dev/null || echo "[]")
        server_config+=', "args":'"$args_json"
    fi

    # Add env if defined
    if [[ -n "$server_env" && "$server_env" != "null" ]]; then
        # Parse env object and build JSON object
        local env_json
        env_json=$(echo "$server_env" | yq eval -o=json - 2>/dev/null || echo "{}")
        server_config+=', "env":'"$env_json"
    fi

    server_config+='}'

    # Update Claude config with jq
    local temp_file
    temp_file=$(mktemp)

    if jq ".mcpServers.\"${ext}\" = ${server_config}" "$CLAUDE_CONFIG_FILE" > "$temp_file"; then
        mv "$temp_file" "$CLAUDE_CONFIG_FILE"
        print_success "${ext} registered as MCP server"
    else
        rm "$temp_file"
        print_error "Failed to register ${ext} as MCP server"
        return 1
    fi

    return 0
}

# Unregister an extension from MCP servers
# Usage: unregister_mcp_server <extension_name>
# Returns: 0 on success, 1 on failure
unregister_mcp_server() {
    local ext="$1"

    if [[ ! -f "$CLAUDE_CONFIG_FILE" ]]; then
        print_info "Claude config file not found - nothing to unregister"
        return 0
    fi

    if ! command_exists jq; then
        print_error "jq not available - required for MCP server management"
        return 1
    fi

    print_info "Unregistering ${ext} from MCP servers..."

    local temp_file
    temp_file=$(mktemp)

    if jq "del(.mcpServers.\"${ext}\")" "$CLAUDE_CONFIG_FILE" > "$temp_file"; then
        mv "$temp_file" "$CLAUDE_CONFIG_FILE"
        print_success "${ext} unregistered from MCP servers"
    else
        rm "$temp_file"
        print_error "Failed to unregister ${ext}"
        return 1
    fi

    return 0
}

# List all extensions with MCP capabilities
# Usage: list_mcp_extensions
# Outputs: List of extensions with MCP capability enabled
list_mcp_extensions() {
    local extensions
    local result=()

    # Get all extensions from registry
    extensions=$(yq eval '.extensions[].name' "${SCRIPT_DIR}/registry.yaml" 2>/dev/null || echo "")

    if [[ -z "$extensions" ]]; then
        echo "No extensions found in registry"
        return 0
    fi

    # Check each extension for MCP capability
    while IFS= read -r ext; do
        if [[ -z "$ext" ]]; then
            continue
        fi

        local mcp_enabled
        mcp_enabled=$(get_extension_capability "$ext" "mcp.enabled")

        if [[ "$mcp_enabled" == "true" ]]; then
            result+=("$ext")
        fi
    done <<< "$extensions"

    # Output results
    if [[ ${#result[@]} -eq 0 ]]; then
        echo "No extensions with MCP capabilities found"
    else
        echo "Extensions with MCP capabilities:"
        for ext in "${result[@]}"; do
            # Get tools count if available
            local tools_count
            tools_count=$(get_extension_capability "$ext" "mcp.tools" | yq eval 'length' - 2>/dev/null || echo "0")

            echo "  - ${ext} (${tools_count} tools)"
        done
    fi
}

# List registered MCP servers from Claude config
# Usage: list_registered_mcp_servers
# Outputs: List of currently registered MCP servers
list_registered_mcp_servers() {
    if [[ ! -f "$CLAUDE_CONFIG_FILE" ]]; then
        echo "No Claude config file found at ${CLAUDE_CONFIG_FILE}"
        return 0
    fi

    if ! command_exists jq; then
        print_error "jq not available - required for reading Claude config"
        return 1
    fi

    echo "Registered MCP servers in Claude Code:"
    jq -r '.mcpServers | keys[]' "$CLAUDE_CONFIG_FILE" 2>/dev/null | while read -r server; do
        local command
        command=$(jq -r ".mcpServers.\"${server}\".command" "$CLAUDE_CONFIG_FILE" 2>/dev/null)
        echo "  - ${server}: ${command}"
    done
}

# Show MCP server details for an extension
# Usage: show_mcp_server_details <extension_name>
show_mcp_server_details() {
    local ext="$1"

    # Check if MCP capability is enabled
    local mcp_enabled
    mcp_enabled=$(get_extension_capability "$ext" "mcp.enabled")

    if [[ "$mcp_enabled" != "true" ]]; then
        echo "${ext} does not have MCP capability enabled"
        return 0
    fi

    echo "MCP Server Details for ${ext}:"
    echo "=============================="

    # Server configuration
    local server_command
    local server_args
    local server_env

    server_command=$(get_extension_capability "$ext" "mcp.server.command")
    server_args=$(get_extension_capability "$ext" "mcp.server.args")
    server_env=$(get_extension_capability "$ext" "mcp.server.env")

    echo "Command: ${server_command}"

    if [[ -n "$server_args" && "$server_args" != "null" ]]; then
        echo "Args:"
        echo "$server_args" | yq eval '.[]' - 2>/dev/null | while read -r arg; do
            echo "  - ${arg}"
        done
    fi

    if [[ -n "$server_env" && "$server_env" != "null" ]]; then
        echo "Environment:"
        echo "$server_env" | yq eval 'to_entries | .[] | "  - " + .key + "=" + .value' - 2>/dev/null
    fi

    # Tools
    local tools
    tools=$(get_extension_capability "$ext" "mcp.tools")

    if [[ -n "$tools" && "$tools" != "null" ]]; then
        local tools_count
        tools_count=$(echo "$tools" | yq eval 'length' - 2>/dev/null || echo "0")

        echo ""
        echo "Tools (${tools_count}):"

        for ((i=0; i<tools_count; i++)); do
            local tool_name
            local tool_description

            tool_name=$(echo "$tools" | yq eval ".[$i].name" - 2>/dev/null || echo "")
            tool_description=$(echo "$tools" | yq eval ".[$i].description" - 2>/dev/null || echo "")

            echo "  - ${tool_name}: ${tool_description}"
        done
    fi

    # Check registration status
    echo ""
    if [[ -f "$CLAUDE_CONFIG_FILE" ]] && command_exists jq; then
        if jq -e ".mcpServers.\"${ext}\"" "$CLAUDE_CONFIG_FILE" &>/dev/null; then
            echo "Status: ✓ Registered in Claude Code"
        else
            echo "Status: ✗ Not registered in Claude Code"
        fi
    fi
}

###############################################################################
# Main Entry Point (for testing)
###############################################################################

# If script is executed directly (not sourced), run tests
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "MCP Manager - Test Mode"
    echo "======================="
    echo ""

    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 <command> [args]"
        echo ""
        echo "Commands:"
        echo "  list                        List extensions with MCP capabilities"
        echo "  registered                  List registered MCP servers in Claude Code"
        echo "  register <extension>        Register extension as MCP server"
        echo "  unregister <extension>      Unregister extension from MCP servers"
        echo "  show <extension>            Show MCP server details for extension"
        echo ""
        exit 0
    fi

    case "$1" in
        list)
            list_mcp_extensions
            ;;
        registered)
            list_registered_mcp_servers
            ;;
        register)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 register <extension>"
                exit 1
            fi
            if register_mcp_server "$2"; then
                exit 0
            else
                exit 1
            fi
            ;;
        unregister)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 unregister <extension>"
                exit 1
            fi
            if unregister_mcp_server "$2"; then
                exit 0
            else
                exit 1
            fi
            ;;
        show)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 show <extension>"
                exit 1
            fi
            show_mcp_server_details "$2"
            ;;
        *)
            echo "Unknown command: $1"
            exit 1
            ;;
    esac
fi
