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

init_claude_tools() {
    local tools_initialized=false

    # Initialize Claude Code project context
    if command_exists claude; then
        print_status "Checking for Claude Code..."
        tools_initialized=true
        print_success "Claude Code is available"
    fi

    # Initialize GitHub spec-kit if uv is available
    if command_exists uvx || command_exists uv; then
        print_status "Initializing GitHub spec-kit..."
        if uvx --from git+https://github.com/github/spec-kit.git specify init --here 2>/dev/null; then
            print_success "GitHub spec-kit initialized"
            tools_initialized=true

            if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
                git add . 2>/dev/null
                git commit -m "feat: add GitHub spec-kit configuration" 2>/dev/null || true
            fi
        else
            print_debug "GitHub spec-kit initialization skipped"
        fi
    fi

    # Initialize Claude Flow if npx is available
    if command_exists npx; then
        print_status "Initializing Claude Flow..."
        if npx claude-flow@alpha init --force 2>/dev/null; then
            print_success "Claude Flow initialized"
            tools_initialized=true
        else
            print_debug "Claude Flow initialization skipped"
        fi
    else
        print_debug "Skipping Claude Flow initialization (npx not available)"
    fi

    # Check for agentic-flow availability
    if command_exists npx; then
        print_status "Checking agent-flow availability..."
        if npx --yes agentic-flow --help >/dev/null 2>&1; then
            print_success "agent-flow available"
            tools_initialized=true
        else
            print_debug "agent-flow initialization skipped"
        fi
    fi

    [[ "$tools_initialized" == "false" ]] && print_warning "No Claude tools were initialized"
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
            --skip-claude-tools)
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
        init_claude_tools || print_warning "Claude tools initialization failed, continuing..."
    fi

    print_success "Project enhancements complete"
    return 0
}

export -f install_project_dependencies
export -f _install_template_dependencies
export -f _scan_and_install_dependencies
export -f _execute_dependency_config
export -f init_claude_tools
export -f create_project_claude_md
export -f setup_project_enhancements
