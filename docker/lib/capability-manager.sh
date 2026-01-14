#!/usr/bin/env bash

# capability-manager.sh - Extension capability discovery and execution
#
# This module provides the core capability system for extensions, enabling:
# - Dynamic discovery of extensions with project initialization capabilities
# - Execution of project-init commands
# - State marker verification (idempotent initialization)
# - Validation of successful initialization
# - Project context file merging (e.g., CLAUDE.md)

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

# Source dependency module for install state checking
if [[ -f "${SCRIPT_DIR}/../cli/extension-manager-modules/dependency.sh" ]]; then
    source "${SCRIPT_DIR}/../cli/extension-manager-modules/dependency.sh"
fi

# Constants
REGISTRY_FILE="${SCRIPT_DIR}/registry.yaml"
EXTENSIONS_DIR="${SCRIPT_DIR}/extensions"

###############################################################################
# Core Capability Functions
###############################################################################

# Get a specific capability definition for an extension
# Usage: get_extension_capability <extension_name> <capability_path>
# Example: get_extension_capability "claude-flow" "project-init"
# Returns: YAML/JSON representation of the capability, or empty string if not found
get_extension_capability() {
    local ext="$1"
    local capability_path="$2"
    local extension_file="${EXTENSIONS_DIR}/${ext}/extension.yaml"

    if [[ ! -f "$extension_file" ]]; then
        return 1
    fi

    # Extract capability using yq
    # capability_path examples: "project-init", "auth", "hooks.pre-install", "mcp.enabled"
    local capability
    capability=$(yq eval ".capabilities.${capability_path} // \"\"" "$extension_file" 2>/dev/null || echo "")

    echo "$capability"
}

# Discover all extensions with a specific capability type
# Usage: discover_project_capabilities <capability_type>
# Example: discover_project_capabilities "project-init"
# Returns: Space-separated list of extension names that have this capability enabled
discover_project_capabilities() {
    local capability_type="$1"
    local extensions
    local result=()

    # Get all registered extensions
    extensions=$(yq eval '.extensions[].name' "${REGISTRY_FILE}" 2>/dev/null || echo "")

    if [[ -z "$extensions" ]]; then
        return 0
    fi

    # Check each extension for the capability
    while IFS= read -r ext; do
        if [[ -z "$ext" ]]; then
            continue
        fi

        # ADDED: Check if extension is actually installed
        if ! is_extension_installed "$ext" 2>/dev/null; then
            continue  # Skip uninstalled extensions
        fi

        local extension_file="${EXTENSIONS_DIR}/${ext}/extension.yaml"
        if [[ ! -f "$extension_file" ]]; then
            continue
        fi

        # Check if capability is enabled
        local enabled
        enabled=$(yq eval ".capabilities.${capability_type}.enabled // false" "$extension_file" 2>/dev/null || echo "false")

        if [[ "$enabled" == "true" ]]; then
            result+=("$ext")
        fi
    done <<< "$extensions"

    # Return space-separated list
    echo "${result[@]}"
}

# Check if extension's state markers exist (indicates already initialized)
# Usage: check_state_markers <extension_name>
# Returns: 0 if all state markers exist, 1 if any are missing
check_state_markers() {
    local ext="$1"
    local state_markers
    local project_dir="${PWD}"

    # Get state-markers definition
    state_markers=$(get_extension_capability "$ext" "project-init.state-markers")

    if [[ -z "$state_markers" || "$state_markers" == "null" ]]; then
        # No state markers defined - assume not initialized
        return 1
    fi

    # Parse state markers YAML (it's an array)
    local marker_count
    marker_count=$(echo "$state_markers" | yq eval 'length' - 2>/dev/null || echo "0")

    if [[ "$marker_count" -eq 0 ]]; then
        return 1
    fi

    # Check each state marker
    for ((i=0; i<marker_count; i++)); do
        local marker_path
        local marker_type

        marker_path=$(echo "$state_markers" | yq eval ".[$i].path" - 2>/dev/null || echo "")
        marker_type=$(echo "$state_markers" | yq eval ".[$i].type" - 2>/dev/null || echo "")

        if [[ -z "$marker_path" ]]; then
            continue
        fi

        local full_path="${project_dir}/${marker_path}"

        case "$marker_type" in
            directory)
                if [[ ! -d "$full_path" ]]; then
                    return 1
                fi
                ;;
            file)
                if [[ ! -f "$full_path" ]]; then
                    return 1
                fi
                ;;
            symlink)
                if [[ ! -L "$full_path" ]]; then
                    return 1
                fi
                ;;
            *)
                # Unknown type - check if it exists as anything
                if [[ ! -e "$full_path" ]]; then
                    return 1
                fi
                ;;
        esac
    done

    # All state markers exist
    return 0
}

# Execute project initialization commands for an extension
# Usage: execute_project_init <extension_name>
# Returns: 0 on success, 1 on failure
execute_project_init() {
    local ext="$1"
    local project_dir="${PWD}"

    # Get project-init commands
    local commands
    commands=$(get_extension_capability "$ext" "project-init.commands")

    if [[ -z "$commands" || "$commands" == "null" ]]; then
        print_warning "No project-init commands defined for ${ext}"
        return 1
    fi

    # Parse commands array
    local command_count
    command_count=$(echo "$commands" | yq eval 'length' - 2>/dev/null || echo "0")

    if [[ "$command_count" -eq 0 ]]; then
        print_warning "No project-init commands found for ${ext}"
        return 1
    fi

    # Execute each command
    for ((i=0; i<command_count; i++)); do
        local command
        local description
        local requires_auth
        local conditional

        command=$(echo "$commands" | yq eval ".[$i].command" - 2>/dev/null || echo "")
        description=$(echo "$commands" | yq eval ".[$i].description // \"\"" - 2>/dev/null || echo "")
        requires_auth=$(echo "$commands" | yq eval ".[$i].requiresAuth // \"none\"" - 2>/dev/null || echo "none")
        conditional=$(echo "$commands" | yq eval ".[$i].conditional // false" - 2>/dev/null || echo "false")

        if [[ -z "$command" ]]; then
            continue
        fi

        # Print description if available
        if [[ -n "$description" ]]; then
            print_info "${description}"
        fi

        # Check auth requirement (delegated to auth-manager.sh if loaded)
        if [[ "$requires_auth" != "none" ]]; then
            if command -v validate_auth &>/dev/null; then
                if ! validate_auth "$requires_auth"; then
                    if [[ "$conditional" == "true" ]]; then
                        print_warning "Skipping conditional command due to missing ${requires_auth} auth"
                        continue
                    else
                        print_error "Required ${requires_auth} authentication not available"
                        return 1
                    fi
                fi
            else
                print_warning "Auth manager not loaded, cannot validate ${requires_auth} auth"
                if [[ "$conditional" != "true" ]]; then
                    return 1
                fi
            fi
        fi

        # Resolve relative script paths from extension directory
        local resolved_command="$command"
        local extension_dir="${EXTENSIONS_DIR}/${ext}"

        # If command starts with "bash scripts/" or "sh scripts/", resolve to absolute path
        if [[ "$command" =~ ^(bash|sh)[[:space:]]+scripts/ ]]; then
            # Extract the script path after "bash " or "sh "
            local script_path
            script_path=$(echo "$command" | sed -E 's/^(bash|sh)[[:space:]]+//')
            resolved_command="${command%% scripts/*} ${extension_dir}/${script_path}"
        fi

        # Execute command in project directory
        if ! (cd "$project_dir" && eval "$resolved_command"); then
            if [[ "$conditional" == "true" ]]; then
                print_warning "Conditional command failed (continuing): ${command}"
            else
                print_error "Project init command failed: ${command}"
                return 1
            fi
        fi
    done

    return 0
}

# Validate project capability after initialization
# Usage: validate_project_capability <extension_name>
# Returns: 0 if validation succeeds, 1 if validation fails or not defined
validate_project_capability() {
    local ext="$1"

    # Get validation definition
    local validation_command
    local expected_pattern
    local expected_exit_code

    validation_command=$(get_extension_capability "$ext" "project-init.validation.command")
    expected_pattern=$(get_extension_capability "$ext" "project-init.validation.expectedPattern")
    expected_exit_code=$(get_extension_capability "$ext" "project-init.validation.expectedExitCode")

    if [[ -z "$validation_command" || "$validation_command" == "null" ]]; then
        # No validation defined - assume success
        return 0
    fi

    # Set default expected exit code
    if [[ -z "$expected_exit_code" || "$expected_exit_code" == "null" ]]; then
        expected_exit_code=0
    fi

    # Execute validation command
    local output
    local exit_code=0
    output=$(eval "$validation_command" 2>&1) || exit_code=$?

    # Check exit code
    if [[ "$exit_code" -ne "$expected_exit_code" ]]; then
        print_error "Validation failed for ${ext}: expected exit code ${expected_exit_code}, got ${exit_code}"
        return 1
    fi

    # Check pattern if defined
    if [[ -n "$expected_pattern" && "$expected_pattern" != "null" ]]; then
        if ! echo "$output" | grep -qE "$expected_pattern"; then
            print_error "Validation failed for ${ext}: output doesn't match pattern '${expected_pattern}'"
            print_error "Output was: ${output}"
            return 1
        fi
    fi

    return 0
}

# Merge project context files (e.g., CLAUDE.md)
# Usage: merge_project_context <extension_name>
# Returns: 0 on success, 1 on failure
merge_project_context() {
    local ext="$1"
    local project_dir="${PWD}"
    local extension_dir="${EXTENSIONS_DIR}/${ext}"

    # Check if project-context capability is enabled
    local enabled
    enabled=$(get_extension_capability "$ext" "project-context.enabled")

    if [[ "$enabled" != "true" ]]; then
        return 0
    fi

    # Get merge file configuration
    local source_file
    local target_file
    local strategy

    source_file=$(get_extension_capability "$ext" "project-context.mergeFile.source")
    target_file=$(get_extension_capability "$ext" "project-context.mergeFile.target")
    strategy=$(get_extension_capability "$ext" "project-context.mergeFile.strategy")

    if [[ -z "$source_file" || -z "$target_file" || -z "$strategy" ]]; then
        print_warning "Incomplete project-context merge configuration for ${ext}"
        return 1
    fi

    local full_source="${extension_dir}/${source_file}"
    local full_target="${project_dir}/${target_file}"

    if [[ ! -f "$full_source" ]]; then
        print_warning "Source file not found: ${full_source}"
        return 1
    fi

    # Execute merge strategy
    case "$strategy" in
        append)
            cat "$full_source" >> "$full_target"
            print_success "Appended ${source_file} to ${target_file}"
            ;;
        prepend)
            local temp_file
            temp_file=$(mktemp)
            cat "$full_source" > "$temp_file"
            if [[ -f "$full_target" ]]; then
                cat "$full_target" >> "$temp_file"
            fi
            mv "$temp_file" "$full_target"
            print_success "Prepended ${source_file} to ${target_file}"
            ;;
        replace)
            cp "$full_source" "$full_target"
            print_success "Replaced ${target_file} with ${source_file}"
            ;;
        append-if-missing)
            if [[ ! -f "$full_target" ]]; then
                cp "$full_source" "$full_target"
                print_success "Created ${target_file} from ${source_file}"
            else
                # Check if source content already exists in target
                if ! grep -qF "$(cat "$full_source")" "$full_target"; then
                    cat "$full_source" >> "$full_target"
                    print_success "Appended ${source_file} to ${target_file} (content was missing)"
                else
                    print_info "${target_file} already contains content from ${source_file}"
                fi
            fi
            ;;
        merge)
            # Simple line-based merge: add lines from source that don't exist in target
            if [[ ! -f "$full_target" ]]; then
                cp "$full_source" "$full_target"
                print_success "Created ${target_file} from ${source_file}"
            else
                local temp_file
                temp_file=$(mktemp)
                while IFS= read -r line; do
                    if ! grep -qF "$line" "$full_target"; then
                        echo "$line" >> "$temp_file"
                    fi
                done < "$full_source"

                if [[ -s "$temp_file" ]]; then
                    cat "$temp_file" >> "$full_target"
                    print_success "Merged new lines from ${source_file} to ${target_file}"
                else
                    print_info "${target_file} already contains all lines from ${source_file}"
                fi
                rm "$temp_file"
            fi
            ;;
        *)
            print_error "Unknown merge strategy: ${strategy}"
            return 1
            ;;
    esac

    return 0
}

# Report initialized extensions (for CLI status display)
# Usage: report_initialized_extensions
# Outputs: List of initialized extensions with checkmarks
report_initialized_extensions() {
    local extensions
    extensions=$(discover_project_capabilities "project-init")

    if [[ -z "$extensions" ]]; then
        return 0
    fi

    for ext in $extensions; do
        if check_state_markers "$ext"; then
            echo "  ✓ ${ext}"
        fi
    done
}

###############################################################################
# Main Entry Point (for testing)
###############################################################################

# If script is executed directly (not sourced), run tests
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "Capability Manager - Test Mode"
    echo "=============================="
    echo ""

    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 <command> [args]"
        echo ""
        echo "Commands:"
        echo "  discover <capability-type>   Discover extensions with capability"
        echo "  check <extension>            Check if extension is initialized"
        echo "  report                       Report all initialized extensions"
        echo ""
        exit 0
    fi

    case "$1" in
        discover)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 discover <capability-type>"
                exit 1
            fi
            discover_project_capabilities "$2"
            ;;
        check)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 check <extension>"
                exit 1
            fi
            if check_state_markers "$2"; then
                echo "✓ $2 is initialized"
            else
                echo "✗ $2 is not initialized"
            fi
            ;;
        report)
            report_initialized_extensions
            ;;
        *)
            echo "Unknown command: $1"
            exit 1
            ;;
    esac
fi
