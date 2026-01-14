#!/bin/bash
#
# project-core.sh - Core project operations shared by new-project and clone-project
#
# This library provides shared functionality for project creation and repository
# cloning operations, including dependency installation, Claude tools initialization,
# Git configuration, and project enhancement orchestration.
#

# Note: set -euo pipefail is set by the calling script

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Only source if not already loaded
[[ "${COMMON_SH_LOADED:-}" != "true" ]] && source "${SCRIPT_DIR}/common.sh"
[[ "${GIT_SH_LOADED:-}" != "true" ]] && source "${SCRIPT_DIR}/git.sh"
source "${SCRIPT_DIR}/project-templates.sh"

# shellcheck disable=SC2120
install_project_dependencies() {
    local skip_build=false
    local template=""

    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-build)
                skip_build=true
                shift
                ;;
            --template)
                template="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    print_status "Detecting and installing project dependencies..."

    local deps_installed=false

    # If template is specified, use its dependencies config
    if [[ -n "$template" ]]; then
        _install_template_dependencies "$template" "$skip_build" && deps_installed=true
    else
        # Fallback: scan all templates for matching dependency files
        _scan_and_install_dependencies "$skip_build" && deps_installed=true
    fi

    [[ "$deps_installed" == "false" ]] && print_debug "No dependency files detected"
    return 0
}

_install_template_dependencies() {
    local template="$1"
    local skip_build="$2"

    local deps_config
    if ! deps_config=$(get_template_dependencies "$template" 2>/dev/null); then
        print_debug "No dependencies config for template: $template"
        return 1
    fi

    _execute_dependency_config "$deps_config" "$skip_build"
}

_scan_and_install_dependencies() {
    local skip_build="$1"
    local found_any=false

    local all_configs
    if ! all_configs=$(get_all_dependencies_configs 2>/dev/null); then
        print_debug "Could not load dependency configs"
        return 1
    fi

    local count
    count=$(echo "$all_configs" | jq 'length')

    for ((i=0; i<count; i++)); do
        local config
        config=$(echo "$all_configs" | jq ".[$i]")

        local detect
        detect=$(echo "$config" | jq -r '.detect')

        # Handle both string and array detect patterns
        if [[ "$detect" == "["* ]]; then
            # Array of patterns - check each with glob
            local pattern_count
            pattern_count=$(echo "$config" | jq -r '.detect | length')
            for ((j=0; j<pattern_count; j++)); do
                local pattern
                pattern=$(echo "$config" | jq -r ".detect[$j]")
                # shellcheck disable=SC2086
                if compgen -G $pattern > /dev/null 2>&1; then
                    if _execute_dependency_config "$config" "$skip_build"; then
                        found_any=true
                    fi
                    break
                fi
            done
        else
            # Single file pattern
            # shellcheck disable=SC2086
            if compgen -G $detect > /dev/null 2>&1; then
                if _execute_dependency_config "$config" "$skip_build"; then
                    found_any=true
                fi
            fi
        fi
    done

    [[ "$found_any" == "true" ]]
}

_execute_dependency_config() {
    local config="$1"
    local skip_build="$2"

    local requires command description fetch_command
    requires=$(echo "$config" | jq -r '.requires')
    command=$(echo "$config" | jq -r '.command')
    description=$(echo "$config" | jq -r '.description // "dependencies"')
    fetch_command=$(echo "$config" | jq -r '.fetch_command // ""')

    # Check if required tool exists
    if ! command_exists "$requires"; then
        print_debug "Skipping $description ($requires not available)"
        return 1
    fi

    # Choose command based on skip_build flag
    local cmd_to_run="$command"
    if [[ "$skip_build" == "true" ]] && [[ -n "$fetch_command" ]]; then
        cmd_to_run="$fetch_command"
        print_status "Fetching $description..."
    else
        print_status "Installing $description..."
    fi

    if eval "$cmd_to_run"; then
        print_success "$description installed"
        return 0
    else
        print_error "Failed to install $description"
        return 1
    fi
}

# Check if Claude Code is authenticated
# Returns 0 if authenticated, 1 if not
check_claude_auth() {
    # Check if claude command exists first
    if ! command_exists claude; then
        return 1
    fi

    # Check for authentication files
    # macOS: Credentials stored in Keychain (no file to check directly)
    # Linux/Container: Check for auth files
    if [[ -f "$HOME/.claude.json" ]] || [[ -f "$HOME/.config/claude-code/auth.json" ]]; then
        return 0
    fi

    # If no auth files found, try running a simple claude command to check auth
    # This will fail if not authenticated
    if claude /doctor &>/dev/null; then
        return 0
    fi

    return 1
}

# Check Claude Code authentication status (non-blocking)
# Returns 0 if authenticated, 1 if not
# LEGACY FUNCTIONS REMOVED - Replaced by capability-manager.sh, auth-manager.sh
# See docs/EXTENSION_CAPABILITIES_ARCHITECTURE.md for details on the new architecture
#
# Removed functions:
# - verify_claude_auth() → auth-manager.sh:validate_anthropic_auth()
# - _is_claude_flow_initialized() → capability-manager.sh:check_state_markers()
# - _is_aqe_initialized() → capability-manager.sh:check_state_markers()
# - _is_claude_flow_agentdb_initialized() → capability-manager.sh:check_state_markers()
# - _initialize_claude_flow() → capability-manager.sh:execute_project_init()
# - _initialize_claude_flow_agentdb() → capability-manager.sh:execute_project_init()

init_project_tools() {
    local skip_tools="${1:-false}"
    local tools_initialized=false

    # Skip tools if --skip-tools flag is set
    if [[ "$skip_tools" == "true" ]]; then
        print_debug "Skipping tool initialization (--skip-tools)"
        [[ "$tools_initialized" == "false" ]] && print_warning "No tools were initialized"
        return 0
    fi

    # Source capability management modules
    source "${DOCKER_LIB}/capability-manager.sh"
    source "${DOCKER_LIB}/auth-manager.sh"
    source "${DOCKER_LIB}/hooks-manager.sh"
    source "${DOCKER_LIB}/mcp-manager.sh"

    # NOTE: spec-kit is now a proper extension with project-init capability
    # No hardcoded initialization needed - capability-manager handles it automatically

    # Check Claude Code availability (non-extension tool)
    if command_exists claude; then
        print_success "Claude Code is available"
    fi

    # Check agentic-flow availability (extension without project-init)
    if command_exists agentic-flow; then
        print_success "agentic-flow is available"
    fi

    # Ensure mise is activated if available
    if command_exists mise && [[ -z "${MISE_ACTIVATED:-}" ]]; then
        eval "$(mise activate bash)" 2>/dev/null || true
        export MISE_ACTIVATED=1
    fi

    # Discover extensions with project-init capabilities
    local extensions
    extensions=$(discover_project_capabilities "project-init")

    if [[ -z "$extensions" ]]; then
        print_debug "No extensions with project-init capabilities found"
        [[ "$tools_initialized" == "false" ]] && print_warning "No tools were initialized"
        return 0
    fi

    print_debug "Found extensions with project-init: ${extensions}"

    # Initialize each extension with project-init capability
    for ext in $extensions; do
        print_status "Initializing ${ext}..."

        # Execute pre-project-init hook
        execute_hook "$ext" "pre-project-init"

        # Check authentication requirements FIRST
        if ! check_extension_auth "$ext"; then
            print_warning "Skipping ${ext} due to missing authentication"
            continue
        fi

        # Check for collision with existing installation
        local ext_version
        ext_version=$(yq eval ".metadata.version" "${DOCKER_LIB}/extensions/${ext}/extension.yaml" 2>/dev/null || echo "unknown")

        if ! handle_collision "$ext" "$ext_version"; then
            print_debug "Skipping ${ext} initialization due to collision"
            tools_initialized=true  # Mark as initialized to avoid warning
            continue
        fi

        # Check if already initialized (via state markers)
        if check_state_markers "$ext"; then
            print_debug "${ext} already initialized (state markers found)"
            tools_initialized=true

            # Still execute post-project-init hook for already-initialized extensions
            execute_hook "$ext" "post-project-init"
            continue
        fi

        # Execute project initialization
        if execute_project_init "$ext"; then
            # Validate initialization
            if validate_project_capability "$ext"; then
                print_success "${ext} initialized successfully"
                tools_initialized=true

                # Merge project context files if capability is enabled
                merge_project_context "$ext"

                # Register MCP server if capability is enabled
                register_mcp_server "$ext"
            else
                print_warning "${ext} initialization succeeded but validation failed"
            fi
        else
            print_warning "${ext} initialization failed"
        fi

        # Execute post-project-init hook
        execute_hook "$ext" "post-project-init"
    done

    [[ "$tools_initialized" == "false" ]] && print_warning "No tools were initialized"
    return 0
}

create_project_claude_md() {
    local template_content=""
    local use_cli=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --template)
                template_content="$2"
                shift 2
                ;;
            --from-cli)
                use_cli=true
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                return 1
                ;;
        esac
    done

    if [[ -f "CLAUDE.md" ]]; then
        print_success "CLAUDE.md already exists"
        return 0
    fi

    print_status "Creating CLAUDE.md..."

    if [[ "$use_cli" == "true" ]] && command_exists claude; then
        if claude /init 2>/dev/null; then
            print_success "CLAUDE.md created via claude CLI"
            return 0
        fi
    fi

    if [[ -n "$template_content" ]]; then
        echo "$template_content" > CLAUDE.md
        print_success "CLAUDE.md created from template"
        return 0
    fi

    local project_name
    project_name=$(basename "$(pwd)")

    cat > CLAUDE.md << EOF
# ${project_name}

## Project Overview
This project was created with Sindri.

## Setup Instructions
[Add setup instructions here]

## Development Commands
[Add common commands here]

## Architecture Notes
[Add architectural decisions and patterns]

## Important Files
[List key files and their purposes]
EOF

    print_success "CLAUDE.md created with basic template"
    return 0
}

setup_project_enhancements() {
    local skip_deps=false
    local skip_tools=false
    local git_name=""
    local git_email=""

    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-deps)
                skip_deps=true
                shift
                ;;
            --skip-tools)
                skip_tools=true
                shift
                ;;
            --git-name)
                git_name="$2"
                shift 2
                ;;
            --git-email)
                git_email="$2"
                shift 2
                ;;
            *)
                print_error "Unknown option: $1"
                return 1
                ;;
        esac
    done

    print_status "Setting up project enhancements..."

    if [[ -n "$git_name" ]] || [[ -n "$git_email" ]]; then
        apply_git_config_overrides ${git_name:+--name "$git_name"} ${git_email:+--email "$git_email"} || return 1
    fi

    if [[ "$skip_deps" == "false" ]]; then
        # shellcheck disable=SC2119
        install_project_dependencies || print_warning "Dependency installation failed, continuing..."
    fi

    if [[ "$skip_tools" == "false" ]]; then
        init_project_tools "$skip_tools" || print_warning "Project tools initialization failed, continuing..."
    else
        init_project_tools "true" || print_warning "Project tools initialization failed, continuing..."
    fi

    print_success "Project enhancements complete"
    return 0
}

export -f install_project_dependencies
export -f _install_template_dependencies
export -f _scan_and_install_dependencies
export -f _execute_dependency_config
export -f check_claude_auth
export -f verify_claude_auth
export -f _is_claude_flow_initialized
export -f _is_aqe_initialized
export -f _is_claude_flow_agentdb_initialized
export -f _initialize_claude_flow
export -f _initialize_claude_flow_agentdb
export -f init_project_tools
export -f create_project_claude_md
export -f setup_project_enhancements
