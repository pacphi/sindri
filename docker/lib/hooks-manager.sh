#!/usr/bin/env bash

# hooks-manager.sh - Lifecycle hooks execution for extensions
#
# This module provides lifecycle hook execution for extensions:
# - pre-install: Before extension installation
# - post-install: After extension installation
# - pre-project-init: Before project initialization
# - post-project-init: After project initialization
#
# Hooks enable extensions to perform custom setup, validation, or cleanup
# at specific points in the installation and initialization workflow

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

# Source capability manager for extension capability queries
if [[ -f "${SCRIPT_DIR}/capability-manager.sh" ]]; then
    source "${SCRIPT_DIR}/capability-manager.sh"
fi

###############################################################################
# Hook Execution Functions
###############################################################################

# Execute a specific hook for an extension
# Usage: execute_hook <extension_name> <hook_type>
# Hook types: pre-install, post-install, pre-project-init, post-project-init
# Returns: 0 on success or if hook not defined, 1 on failure
execute_hook() {
    local ext="$1"
    local hook_type="$2"

    # Get hook definition
    local hook_command
    local hook_description

    hook_command=$(get_extension_capability "$ext" "hooks.${hook_type}.command")
    hook_description=$(get_extension_capability "$ext" "hooks.${hook_type}.description")

    if [[ -z "$hook_command" || "$hook_command" == "null" ]]; then
        # Hook not defined - silently succeed
        return 0
    fi

    # Print hook description if available
    if [[ -n "$hook_description" && "$hook_description" != "null" ]]; then
        print_info "${hook_description}"
    else
        print_info "Running ${hook_type} hook for ${ext}"
    fi

    # Execute hook command
    local exit_code=0
    eval "$hook_command" || exit_code=$?

    if [[ "$exit_code" -ne 0 ]]; then
        print_warning "${ext} ${hook_type} hook failed with exit code ${exit_code}"
        return 1
    fi

    return 0
}

# Execute pre-install hook
# Usage: execute_pre_install_hook <extension_name>
execute_pre_install_hook() {
    execute_hook "$1" "pre-install"
}

# Execute post-install hook
# Usage: execute_post_install_hook <extension_name>
execute_post_install_hook() {
    execute_hook "$1" "post-install"
}

# Execute pre-project-init hook
# Usage: execute_pre_project_init_hook <extension_name>
execute_pre_project_init_hook() {
    execute_hook "$1" "pre-project-init"
}

# Execute post-project-init hook
# Usage: execute_post_project_init_hook <extension_name>
execute_post_project_init_hook() {
    execute_hook "$1" "post-project-init"
}

###############################################################################
# Hook Discovery Functions
###############################################################################

# Check if extension has a specific hook defined
# Usage: has_hook <extension_name> <hook_type>
# Returns: 0 if hook exists, 1 if not
has_hook() {
    local ext="$1"
    local hook_type="$2"

    local hook_command
    hook_command=$(get_extension_capability "$ext" "hooks.${hook_type}.command")

    if [[ -n "$hook_command" && "$hook_command" != "null" ]]; then
        return 0
    else
        return 1
    fi
}

# List all hooks defined for an extension
# Usage: list_extension_hooks <extension_name>
# Outputs: List of defined hooks
list_extension_hooks() {
    local ext="$1"
    local hooks=()

    if has_hook "$ext" "pre-install"; then
        hooks+=("pre-install")
    fi

    if has_hook "$ext" "post-install"; then
        hooks+=("post-install")
    fi

    if has_hook "$ext" "pre-project-init"; then
        hooks+=("pre-project-init")
    fi

    if has_hook "$ext" "post-project-init"; then
        hooks+=("post-project-init")
    fi

    if [[ ${#hooks[@]} -eq 0 ]]; then
        echo "No hooks defined for ${ext}"
    else
        echo "Hooks for ${ext}:"
        for hook in "${hooks[@]}"; do
            local description
            description=$(get_extension_capability "$ext" "hooks.${hook}.description")
            if [[ -n "$description" && "$description" != "null" ]]; then
                echo "  - ${hook}: ${description}"
            else
                echo "  - ${hook}"
            fi
        done
    fi
}

###############################################################################
# Main Entry Point (for testing)
###############################################################################

# If script is executed directly (not sourced), run tests
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "Hooks Manager - Test Mode"
    echo "========================="
    echo ""

    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 <command> [args]"
        echo ""
        echo "Commands:"
        echo "  execute <extension> <hook-type>   Execute a specific hook"
        echo "  list <extension>                   List hooks for extension"
        echo "  has <extension> <hook-type>        Check if hook exists"
        echo ""
        echo "Hook types:"
        echo "  - pre-install"
        echo "  - post-install"
        echo "  - pre-project-init"
        echo "  - post-project-init"
        echo ""
        exit 0
    fi

    case "$1" in
        execute)
            if [[ $# -lt 3 ]]; then
                echo "Usage: $0 execute <extension> <hook-type>"
                exit 1
            fi
            if execute_hook "$2" "$3"; then
                echo "✓ Hook executed successfully"
            else
                echo "✗ Hook execution failed"
                exit 1
            fi
            ;;
        list)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 list <extension>"
                exit 1
            fi
            list_extension_hooks "$2"
            ;;
        has)
            if [[ $# -lt 3 ]]; then
                echo "Usage: $0 has <extension> <hook-type>"
                exit 1
            fi
            if has_hook "$2" "$3"; then
                echo "✓ Hook exists"
            else
                echo "✗ Hook does not exist"
                exit 1
            fi
            ;;
        *)
            echo "Unknown command: $1"
            exit 1
            ;;
    esac
fi
