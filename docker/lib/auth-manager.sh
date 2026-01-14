#!/usr/bin/env bash

# auth-manager.sh - Multi-provider authentication validation
#
# This module provides a generalized authentication system supporting:
# - Anthropic (Claude API)
# - OpenAI (GPT API)
# - GitHub (gh CLI)
# - Custom validators (defined in extension.yaml)
#
# Replaces hardcoded verify_claude_auth() with pluggable provider system

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

# Source capability manager for extension capability queries
if [[ -f "${SCRIPT_DIR}/capability-manager.sh" ]]; then
    source "${SCRIPT_DIR}/capability-manager.sh"
fi

###############################################################################
# Provider-Specific Validation Functions
###############################################################################

# Validate Anthropic API authentication
# Checks: ANTHROPIC_API_KEY environment variable, claude CLI availability
# Returns: 0 if valid, 1 if invalid
validate_anthropic_auth() {
    # Check if ANTHROPIC_API_KEY is set
    if [[ -z "${ANTHROPIC_API_KEY:-}" ]]; then
        print_warning "ANTHROPIC_API_KEY environment variable is not set"
        return 1
    fi

    # Verify Claude CLI is available and working
    if ! command_exists claude; then
        print_warning "Claude Code CLI not found in PATH"
        return 1
    fi

    # Simple validation: check if claude CLI responds
    if ! claude --version &>/dev/null; then
        print_warning "Claude Code CLI not responding or API key invalid"
        return 1
    fi

    return 0
}

# Validate OpenAI API authentication
# Checks: OPENAI_API_KEY environment variable
# Returns: 0 if valid, 1 if invalid
validate_openai_auth() {
    # Check if OPENAI_API_KEY is set
    if [[ -z "${OPENAI_API_KEY:-}" ]]; then
        print_warning "OPENAI_API_KEY environment variable is not set"
        return 1
    fi

    # Note: We don't validate the key itself as that would require an API call
    # Extensions can define custom validators in their extension.yaml if needed
    return 0
}

# Validate GitHub CLI authentication
# Checks: gh CLI availability and authentication status
# Returns: 0 if valid, 1 if invalid
validate_github_auth() {
    # Check if gh CLI is installed
    if ! command_exists gh; then
        print_warning "GitHub CLI (gh) not found in PATH"
        return 1
    fi

    # Check authentication status
    if ! gh auth status &>/dev/null; then
        print_warning "Not authenticated with GitHub CLI"
        print_info "Run 'gh auth login' to authenticate"
        return 1
    fi

    return 0
}

# Validate custom authentication using extension-defined command
# Usage: validate_custom_auth <extension_name>
# Returns: 0 if valid, 1 if invalid
validate_custom_auth() {
    local ext="$1"

    # Get custom validator command
    local validator_command
    local expected_exit_code

    validator_command=$(get_extension_capability "$ext" "auth.validator.command")
    expected_exit_code=$(get_extension_capability "$ext" "auth.validator.expectedExitCode")

    if [[ -z "$validator_command" || "$validator_command" == "null" ]]; then
        print_error "Custom auth validator command not defined for ${ext}"
        return 1
    fi

    # Set default expected exit code
    if [[ -z "$expected_exit_code" || "$expected_exit_code" == "null" ]]; then
        expected_exit_code=0
    fi

    # Execute validator command
    local exit_code=0
    eval "$validator_command" &>/dev/null || exit_code=$?

    if [[ "$exit_code" -ne "$expected_exit_code" ]]; then
        print_warning "Custom auth validation failed for ${ext} (exit code: ${exit_code}, expected: ${expected_exit_code})"
        return 1
    fi

    return 0
}

###############################################################################
# Generic Authentication Validation
###############################################################################

# Validate authentication for a given provider
# Usage: validate_auth <provider> [extension_name]
# Provider: anthropic, openai, github, custom, none
# Returns: 0 if valid, 1 if invalid
validate_auth() {
    local provider="$1"
    local ext="${2:-}"

    case "$provider" in
        anthropic)
            validate_anthropic_auth
            ;;
        openai)
            validate_openai_auth
            ;;
        github)
            validate_github_auth
            ;;
        custom)
            if [[ -z "$ext" ]]; then
                print_error "Extension name required for custom auth validation"
                return 1
            fi
            validate_custom_auth "$ext"
            ;;
        none)
            # No authentication required
            return 0
            ;;
        *)
            print_error "Unknown authentication provider: ${provider}"
            return 1
            ;;
    esac
}

# Check authentication requirements for an extension
# Usage: check_extension_auth <extension_name>
# Returns: 0 if auth satisfied or not required, 1 if auth required but missing
check_extension_auth() {
    local ext="$1"

    # Check if extension has auth capability defined
    local auth_provider
    local auth_required
    local env_vars

    auth_provider=$(get_extension_capability "$ext" "auth.provider")

    if [[ -z "$auth_provider" || "$auth_provider" == "null" ]]; then
        # No auth capability defined - proceed without auth
        return 0
    fi

    auth_required=$(get_extension_capability "$ext" "auth.required")
    if [[ -z "$auth_required" || "$auth_required" == "null" ]]; then
        auth_required="false"
    fi

    # Check required environment variables (if defined)
    env_vars=$(get_extension_capability "$ext" "auth.envVars")
    if [[ -n "$env_vars" && "$env_vars" != "null" ]]; then
        # Parse env vars array
        local env_var_count
        env_var_count=$(echo "$env_vars" | yq eval 'length' - 2>/dev/null || echo "0")

        for ((i=0; i<env_var_count; i++)); do
            local env_var
            env_var=$(echo "$env_vars" | yq eval ".[$i]" - 2>/dev/null || echo "")

            if [[ -n "$env_var" ]]; then
                if [[ -z "${!env_var:-}" ]]; then
                    if [[ "$auth_required" == "true" ]]; then
                        print_error "${ext} requires ${env_var} environment variable"
                        return 1
                    else
                        print_warning "${ext} recommends ${env_var} environment variable (continuing without)"
                    fi
                fi
            fi
        done
    fi

    # Validate using provider-specific validator
    if [[ "$auth_provider" == "custom" ]]; then
        if ! validate_auth "custom" "$ext"; then
            if [[ "$auth_required" == "true" ]]; then
                print_error "${ext} requires custom authentication"
                return 1
            else
                print_warning "${ext} recommends custom authentication (continuing without)"
            fi
        fi
    else
        if ! validate_auth "$auth_provider"; then
            if [[ "$auth_required" == "true" ]]; then
                print_error "${ext} requires ${auth_provider} authentication"
                return 1
            else
                print_warning "${ext} recommends ${auth_provider} authentication (continuing without)"
            fi
        fi
    fi

    return 0
}

###############################################################################
# Utility Functions
###############################################################################

# Get authentication status for all providers
# Usage: get_auth_status
# Outputs: Status for each provider
get_auth_status() {
    echo "Authentication Status:"
    echo "====================="

    if validate_anthropic_auth &>/dev/null; then
        echo "  ✓ Anthropic (Claude API)"
    else
        echo "  ✗ Anthropic (Claude API) - not configured"
    fi

    if validate_openai_auth &>/dev/null; then
        echo "  ✓ OpenAI (GPT API)"
    else
        echo "  ✗ OpenAI (GPT API) - not configured"
    fi

    if validate_github_auth &>/dev/null; then
        echo "  ✓ GitHub CLI"
    else
        echo "  ✗ GitHub CLI - not authenticated"
    fi
}

###############################################################################
# Main Entry Point (for testing)
###############################################################################

# If script is executed directly (not sourced), run tests
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "Auth Manager - Test Mode"
    echo "========================"
    echo ""

    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 <command> [args]"
        echo ""
        echo "Commands:"
        echo "  status                       Show authentication status for all providers"
        echo "  validate <provider>          Validate specific provider (anthropic, openai, github)"
        echo "  check <extension>            Check auth requirements for extension"
        echo ""
        exit 0
    fi

    case "$1" in
        status)
            get_auth_status
            ;;
        validate)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 validate <provider>"
                exit 1
            fi
            if validate_auth "$2"; then
                echo "✓ $2 authentication valid"
            else
                echo "✗ $2 authentication invalid"
                exit 1
            fi
            ;;
        check)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 check <extension>"
                exit 1
            fi
            if check_extension_auth "$2"; then
                echo "✓ $2 authentication requirements satisfied"
            else
                echo "✗ $2 authentication requirements NOT satisfied"
                exit 1
            fi
            ;;
        *)
            echo "Unknown command: $1"
            exit 1
            ;;
    esac
fi
