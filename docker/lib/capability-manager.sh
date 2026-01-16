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
# Container path: /docker/lib -> /docker/cli/extension-manager-modules/dependency.sh
# Local dev path: /Users/.../sindri/docker/lib -> /Users/.../sindri/cli/extension-manager-modules/dependency.sh

DEPENDENCY_SOURCED=false

# Try container path pattern (up one level to /docker, then into cli/)
if [[ -f "${SCRIPT_DIR}/../cli/extension-manager-modules/dependency.sh" ]]; then
    if source "${SCRIPT_DIR}/../cli/extension-manager-modules/dependency.sh" 2>/dev/null; then
        DEPENDENCY_SOURCED=true
    fi
# Try local development path pattern (up two levels to project root, then into cli/)
elif [[ -f "${SCRIPT_DIR}/../../cli/extension-manager-modules/dependency.sh" ]]; then
    if source "${SCRIPT_DIR}/../../cli/extension-manager-modules/dependency.sh" 2>/dev/null; then
        DEPENDENCY_SOURCED=true
    fi
fi

# Debug logging to help diagnose sourcing issues
if [[ "${DEPENDENCY_SOURCED}" == "true" ]]; then
    # Only log in debug mode to avoid noise
    [[ "${DEBUG:-}" == "1" ]] && echo "[DEBUG] dependency.sh sourced successfully" >&2
else
    # Always warn if dependency.sh couldn't be sourced
    echo "[WARN] dependency.sh not found or failed to source - extension installation checks disabled" >&2
    echo "[WARN] Attempted paths:" >&2
    echo "[WARN]   ${SCRIPT_DIR}/../cli/extension-manager-modules/dependency.sh" >&2
    echo "[WARN]   ${SCRIPT_DIR}/../../cli/extension-manager-modules/dependency.sh" >&2
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
# Returns: Space-separated list of extension names sorted by priority (lower = earlier)
discover_project_capabilities() {
    local capability_type="$1"
    local extensions
    local extensions_with_priority=""

    # Get all registered extensions (extension names are keys in the YAML)
    extensions=$(yq eval '.extensions | keys | .[]' "${REGISTRY_FILE}" 2>/dev/null || echo "")

    if [[ -z "$extensions" ]]; then
        return 0
    fi

    # Check each extension for the capability and collect priority
    while IFS= read -r ext; do
        if [[ -z "$ext" ]]; then
            continue
        fi

        # Check if extension is actually installed (defensive check)
        # This check is OPTIONAL - if dependency.sh wasn't sourced, we skip this check
        # and rely on registry.yaml which should only list actually installed extensions.
        if command -v is_extension_installed &>/dev/null; then
            if ! is_extension_installed "$ext" 2>/dev/null; then
                [[ "${DEBUG:-}" == "1" ]] && echo "[DEBUG] Skipping $ext - not installed" >&2
                continue  # Skip uninstalled extensions
            fi
        else
            # Function not available, proceed with all registered extensions
            [[ "${DEBUG:-}" == "1" ]] && echo "[DEBUG] Skipping installation check for $ext (is_extension_installed not available)" >&2
        fi

        local extension_file="${EXTENSIONS_DIR}/${ext}/extension.yaml"
        if [[ ! -f "$extension_file" ]]; then
            continue
        fi

        # Check if capability is enabled
        local enabled
        enabled=$(yq eval ".capabilities.${capability_type}.enabled // false" "$extension_file" 2>/dev/null || echo "false")

        if [[ "$enabled" != "true" ]]; then
            continue
        fi

        # Get priority (default: 100)
        local priority
        priority=$(yq eval ".capabilities.${capability_type}.priority // 100" "$extension_file" 2>/dev/null || echo "100")

        # Format: "priority:extension_name"
        extensions_with_priority="${extensions_with_priority}${priority}:${ext}
"
    done <<< "$extensions"

    # Sort by priority (numeric), extract extension names, return space-separated
    if [[ -n "$extensions_with_priority" ]]; then
        echo "$extensions_with_priority" | sort -t: -k1 -n | cut -d: -f2 | tr '\n' ' '
    fi
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
            print_status "${description}"
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
                    print_status "${target_file} already contains content from ${source_file}"
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
                    print_status "${target_file} already contains all lines from ${source_file}"
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
# Collision Handling (Generic, No Extension-Specific Logic)
###############################################################################

# Detect installed version from collision-handling markers
# Usage: detect_collision_version <extension_name>
# Returns: version string (e.g., "v2", "v3", "installed", "unknown") or "none"
detect_collision_version() {
    local ext="$1"
    local project_dir="${PWD}"

    # Check if collision handling is enabled
    local enabled
    enabled=$(get_extension_capability "$ext" "collision-handling.enabled")

    if [[ "$enabled" != "true" ]]; then
        echo "none"
        return 0
    fi

    # Get version-markers from extension
    local version_markers
    version_markers=$(get_extension_capability "$ext" "collision-handling.version-markers")

    if [[ -z "$version_markers" || "$version_markers" == "null" ]]; then
        echo "none"
        return 0
    fi

    # Parse and check each version marker (in order)
    local marker_count
    marker_count=$(echo "$version_markers" | yq eval 'length' - 2>/dev/null || echo "0")

    for ((i=0; i<marker_count; i++)); do
        local path version method
        path=$(echo "$version_markers" | yq eval ".[$i].path" - 2>/dev/null)
        version=$(echo "$version_markers" | yq eval ".[$i].version" - 2>/dev/null)
        method=$(echo "$version_markers" | yq eval ".[$i].detection.method" - 2>/dev/null)

        if [[ -z "$path" || -z "$version" || -z "$method" ]]; then
            continue
        fi

        local full_path="${project_dir}/${path}"

        case "$method" in
            file-exists)
                if [[ -f "$full_path" ]]; then
                    echo "$version"
                    return 0
                fi
                ;;

            directory-exists)
                if [[ -d "$full_path" ]]; then
                    # Check exclude-if conditions
                    local excludes
                    excludes=$(echo "$version_markers" | yq eval ".[$i].detection.exclude-if" - 2>/dev/null)

                    if [[ "$excludes" != "null" && -n "$excludes" ]]; then
                        local has_excludes=false
                        local exclude_count
                        exclude_count=$(echo "$excludes" | yq eval 'length' - 2>/dev/null || echo "0")

                        for ((j=0; j<exclude_count; j++)); do
                            local exclude_path
                            exclude_path=$(echo "$excludes" | yq eval ".[$j]" - 2>/dev/null)
                            if [[ -e "${project_dir}/${exclude_path}" ]]; then
                                has_excludes=true
                                break
                            fi
                        done

                        # Only return version if NO excludes found
                        if [[ "$has_excludes" == "false" ]]; then
                            echo "$version"
                            return 0
                        fi
                    else
                        # No excludes, directory exists = match
                        echo "$version"
                        return 0
                    fi
                fi
                ;;

            content-match)
                if [[ -f "$full_path" ]]; then
                    local patterns match_any
                    patterns=$(echo "$version_markers" | yq eval ".[$i].detection.patterns" - 2>/dev/null)
                    match_any=$(echo "$version_markers" | yq eval ".[$i].detection.match-any" - 2>/dev/null)

                    if [[ "$patterns" == "null" ]]; then
                        continue
                    fi

                    local pattern_count
                    pattern_count=$(echo "$patterns" | yq eval 'length' - 2>/dev/null || echo "0")

                    local all_matched=true
                    local any_matched=false

                    for ((j=0; j<pattern_count; j++)); do
                        local pattern
                        pattern=$(echo "$patterns" | yq eval ".[$j]" - 2>/dev/null)

                        if grep -q "$pattern" "$full_path" 2>/dev/null; then
                            any_matched=true
                        else
                            all_matched=false
                        fi
                    done

                    # Check match condition
                    if [[ "$match_any" == "true" && "$any_matched" == "true" ]]; then
                        echo "$version"
                        return 0
                    elif [[ "$match_any" != "true" && "$all_matched" == "true" ]]; then
                        echo "$version"
                        return 0
                    fi
                fi
                ;;
        esac
    done

    echo "none"
    return 0
}

# Backup state markers with timestamp
# Usage: backup_state_markers <extension_name>
backup_state_markers() {
    local ext="$1"
    local timestamp
    timestamp=$(date +%Y%m%d_%H%M%S)

    # Get state markers from project-init
    local state_markers
    state_markers=$(get_extension_capability "$ext" "project-init.state-markers")

    if [[ -z "$state_markers" || "$state_markers" == "null" ]]; then
        return 0
    fi

    local marker_count
    marker_count=$(echo "$state_markers" | yq eval 'length' - 2>/dev/null || echo "0")

    for ((i=0; i<marker_count; i++)); do
        local path type
        path=$(echo "$state_markers" | yq eval ".[$i].path" - 2>/dev/null)
        type=$(echo "$state_markers" | yq eval ".[$i].type" - 2>/dev/null)

        if [[ -z "$path" ]]; then
            continue
        fi

        local full_path="${PWD}/${path}"

        if [[ "$type" == "directory" && -d "$full_path" ]]; then
            local backup_path="${full_path}.backup.${timestamp}"
            mv "$full_path" "$backup_path"
            print_status "Backed up: ${path} → ${path}.backup.${timestamp}"
        elif [[ "$type" == "file" && -f "$full_path" ]]; then
            local backup_path="${full_path}.backup.${timestamp}"
            mv "$full_path" "$backup_path"
            print_status "Backed up: ${path} → ${path}.backup.${timestamp}"
        fi
    done
}

# Generic collision handling (no extension-specific logic)
# Usage: handle_collision <extension_name> <installing_version>
# Returns: 0 = proceed with init, 1 = skip init
handle_collision() {
    local ext="$1"
    local installing_version="$2"

    # Check if collision handling is enabled
    local enabled
    enabled=$(get_extension_capability "$ext" "collision-handling.enabled")

    if [[ "$enabled" != "true" ]]; then
        return 0  # No collision handling, proceed normally
    fi

    # Detect installed version
    local detected_version
    detected_version=$(detect_collision_version "$ext")

    if [[ "$detected_version" == "none" ]]; then
        return 0  # No collision detected, proceed
    fi

    # Find matching scenario
    local scenarios
    scenarios=$(get_extension_capability "$ext" "collision-handling.scenarios")

    if [[ -z "$scenarios" || "$scenarios" == "null" ]]; then
        return 0  # No scenarios defined, proceed
    fi

    local scenario_count
    scenario_count=$(echo "$scenarios" | yq eval 'length' - 2>/dev/null || echo "0")

    for ((i=0; i<scenario_count; i++)); do
        local detected_check installing_check action message
        detected_check=$(echo "$scenarios" | yq eval ".[$i].detected-version" - 2>/dev/null)
        installing_check=$(echo "$scenarios" | yq eval ".[$i].installing-version" - 2>/dev/null)

        if [[ "$detected_check" == "$detected_version" ]] && [[ "$installing_check" == "$installing_version" ]]; then
            action=$(echo "$scenarios" | yq eval ".[$i].action" - 2>/dev/null)
            message=$(echo "$scenarios" | yq eval ".[$i].message" - 2>/dev/null)

            case "$action" in
                stop|skip)
                    print_warning "$(echo "$message" | sed 's/^/  /')"
                    return 1  # Stop/skip initialization
                    ;;
                proceed)
                    if [[ -n "$message" ]]; then
                        print_status "$(echo "$message" | sed 's/^/  /')"
                    fi
                    return 0  # Proceed with initialization
                    ;;
                backup)
                    print_status "$(echo "$message" | sed 's/^/  /')"
                    backup_state_markers "$ext"
                    return 0  # Proceed after backup
                    ;;
                prompt)
                    # Interactive prompt (future enhancement)
                    print_warning "$(echo "$message" | sed 's/^/  /')"
                    print_status "  Interactive prompts not yet implemented. Skipping for safety."
                    return 1  # Default: skip for safety
                    ;;
            esac
        fi
    done

    # No matching scenario found, proceed
    return 0
}

###############################################################################
# Conflict Resolution (Generic File/Directory Handling)
###############################################################################

# Main conflict resolution entry point
# Called AFTER extension init completes
# Usage: resolve_conflicts_post_init <extension_name> <snapshot_marker_file>
# Returns: 0 on success
resolve_conflicts_post_init() {
    local ext="$1"
    local snapshot_marker="$2"
    local project_dir="${PWD}"

    # Get conflict rules from extension YAML
    local conflict_rules
    conflict_rules=$(get_extension_capability "$ext" "collision-handling.conflict-rules")

    if [[ -z "$conflict_rules" || "$conflict_rules" == "null" ]]; then
        return 0  # No conflict rules defined
    fi

    # Get list of files modified after snapshot
    local modified_files
    if [[ -f "$snapshot_marker" ]]; then
        modified_files=$(find "$project_dir" -type f -newer "$snapshot_marker" 2>/dev/null || echo "")
    else
        return 0  # No snapshot marker, skip conflict resolution
    fi

    # Process each conflict rule
    local rule_count
    rule_count=$(echo "$conflict_rules" | yq eval 'length' - 2>/dev/null || echo "0")

    for ((i=0; i<rule_count; i++)); do
        local path type rule
        path=$(echo "$conflict_rules" | yq eval ".[$i].path" - 2>/dev/null)
        type=$(echo "$conflict_rules" | yq eval ".[$i].type" - 2>/dev/null)

        if [[ -z "$path" || -z "$type" ]]; then
            continue
        fi

        local full_path="${project_dir}/${path}"

        # Check if this path was modified by extension
        if echo "$modified_files" | grep -q "^${full_path}$" || [[ -e "$full_path" ]]; then
            print_debug "Checking conflict rule for: ${path} (type: ${type})"

            # Extract the full rule for this item
            rule=$(echo "$conflict_rules" | yq eval ".[$i]" - 2>/dev/null)

            if [[ "$type" == "file" ]]; then
                handle_file_conflict "$ext" "$path" "$rule" || true
            elif [[ "$type" == "directory" ]]; then
                handle_directory_conflict "$ext" "$path" "$rule" || true
            fi
        fi
    done

    return 0
}

# Handle file-level conflict
# Called AFTER extension created/modified the file
# Usage: handle_file_conflict <extension_name> <file_path> <rule_yaml>
handle_file_conflict() {
    local ext="$1"
    local file_path="$2"
    local rule="$3"
    local project_dir="${PWD}"
    local full_path="${project_dir}/${file_path}"

    # Check if file exists
    if [[ ! -f "$full_path" ]]; then
        return 0  # File doesn't exist, no conflict
    fi

    # Extract conflict action and separator
    local action separator
    action=$(echo "$rule" | yq eval '.on-conflict.action' - 2>/dev/null)
    separator=$(echo "$rule" | yq eval '.on-conflict.separator // ""' - 2>/dev/null)

    # Environment variable override: EXTENSION_CONFLICT_STRATEGY
    # Takes precedence over extension-defined action
    if [[ -n "${EXTENSION_CONFLICT_STRATEGY:-}" ]]; then
        print_debug "Using EXTENSION_CONFLICT_STRATEGY=${EXTENSION_CONFLICT_STRATEGY} for ${file_path}"
        action="${EXTENSION_CONFLICT_STRATEGY}"
    fi

    # Environment variable override: EXTENSION_CONFLICT_PROMPT
    # If false, replace 'prompt' action with 'skip' (safe default)
    if [[ "${EXTENSION_CONFLICT_PROMPT:-true}" == "false" && "$action" == "prompt" ]]; then
        print_debug "EXTENSION_CONFLICT_PROMPT=false, changing action from 'prompt' to 'skip' for ${file_path}"
        action="skip"
    fi

    # Check if this is the FIRST extension to write this file
    # If .original doesn't exist, this is the first write, no conflict
    local original_file="${full_path}.original-before-${ext}"
    local preserved_original="${full_path}.original"

    if [[ ! -f "$preserved_original" ]]; then
        # First extension to write this file, no conflict
        print_debug "No conflict for ${file_path} - first extension to write it"
        return 0
    fi

    print_info "Resolving conflict: ${file_path} (action: ${action})"

    # Backup current state
    cp "$full_path" "${full_path}.new-from-${ext}"
    cp "$preserved_original" "$original_file"

    local new_file="${full_path}.new-from-${ext}"

    case "$action" in
        overwrite)
            print_info "Overwriting ${file_path} with ${ext} content"
            # New content already in place, nothing to do
            ;;

        append)
            print_info "Appending ${ext} content to ${file_path}"
            {
                cat "$original_file"
                if [[ -n "$separator" ]]; then
                    echo -e "$separator"
                fi
                cat "$new_file"
            } > "${full_path}.tmp"
            mv "${full_path}.tmp" "$full_path"
            ;;

        prepend)
            print_info "Prepending ${ext} content to ${file_path}"
            {
                cat "$new_file"
                if [[ -n "$separator" ]]; then
                    echo -e "$separator"
                fi
                cat "$original_file"
            } > "${full_path}.tmp"
            mv "${full_path}.tmp" "$full_path"
            ;;

        merge-json)
            if command -v jq &>/dev/null; then
                print_info "Merging JSON: ${file_path}"
                jq -s '.[0] * .[1]' "$original_file" "$new_file" > "${full_path}.tmp" 2>/dev/null
                if [[ $? -eq 0 ]]; then
                    mv "${full_path}.tmp" "$full_path"
                else
                    print_warning "JSON merge failed for ${file_path}, keeping new content"
                fi
            else
                print_warning "jq not available, cannot merge JSON for ${file_path}"
            fi
            ;;

        merge-yaml)
            if command -v yq &>/dev/null; then
                print_info "Merging YAML: ${file_path}"
                yq eval-all '. as $item ireduce ({}; . * $item)' "$original_file" "$new_file" > "${full_path}.tmp" 2>/dev/null
                if [[ $? -eq 0 ]]; then
                    mv "${full_path}.tmp" "$full_path"
                else
                    print_warning "YAML merge failed for ${file_path}, keeping new content"
                fi
            else
                print_warning "yq not available, cannot merge YAML for ${file_path}"
            fi
            ;;

        prompt)
            prompt_user_file_action "$ext" "$file_path" "$original_file" "$new_file"
            ;;

        skip)
            print_info "Skipping ${file_path} - keeping original"
            mv "$original_file" "$full_path"
            ;;

        *)
            print_warning "Unknown action '${action}' for ${file_path}, keeping new content"
            ;;
    esac

    # Cleanup temp files
    rm -f "$original_file" "$new_file" "${full_path}.tmp"

    return 0
}

# Handle directory-level conflict
# Usage: handle_directory_conflict <extension_name> <dir_path> <rule_yaml>
handle_directory_conflict() {
    local ext="$1"
    local dir_path="$2"
    local rule="$3"
    local project_dir="${PWD}"
    local full_path="${project_dir}/${dir_path}"

    # Check if directory exists
    if [[ ! -d "$full_path" ]]; then
        return 0  # Directory doesn't exist, no conflict
    fi

    # Extract action and options
    local action backup_enabled backup_suffix
    action=$(echo "$rule" | yq eval '.on-conflict.action' - 2>/dev/null)
    backup_enabled=$(echo "$rule" | yq eval '.on-conflict.backup // false' - 2>/dev/null)
    backup_suffix=$(echo "$rule" | yq eval '.on-conflict.backup-suffix // ".backup"' - 2>/dev/null)

    # Environment variable override: EXTENSION_CONFLICT_STRATEGY
    # Takes precedence over extension-defined action
    if [[ -n "${EXTENSION_CONFLICT_STRATEGY:-}" ]]; then
        print_debug "Using EXTENSION_CONFLICT_STRATEGY=${EXTENSION_CONFLICT_STRATEGY} for ${dir_path}"
        action="${EXTENSION_CONFLICT_STRATEGY}"
    fi

    # Environment variable override: EXTENSION_CONFLICT_PROMPT
    # If false, replace 'prompt' and 'prompt-per-file' actions with 'skip' (safe default)
    if [[ "${EXTENSION_CONFLICT_PROMPT:-true}" == "false" ]]; then
        if [[ "$action" == "prompt" || "$action" == "prompt-per-file" ]]; then
            print_debug "EXTENSION_CONFLICT_PROMPT=false, changing action from '${action}' to 'skip' for ${dir_path}"
            action="skip"
        fi
    fi

    print_info "Resolving directory conflict: ${dir_path} (action: ${action})"

    case "$action" in
        backup)
            if [[ -d "$full_path" ]]; then
                local timestamp
                timestamp=$(date +%Y%m%d_%H%M%S)
                local backup_path="${full_path}${backup_suffix}.${timestamp}"
                print_info "Backing up ${dir_path} to ${backup_path}"
                cp -r "$full_path" "$backup_path"
            fi
            ;;

        backup-and-replace)
            if [[ -d "$full_path" ]]; then
                local timestamp
                timestamp=$(date +%Y%m%d_%H%M%S)
                local backup_path="${full_path}${backup_suffix}.${timestamp}"
                print_info "Backup and replace: ${dir_path}"
                mv "$full_path" "$backup_path"
                # Extension initialization will create fresh directory
            fi
            ;;

        merge)
            if [[ "$backup_enabled" == "true" ]]; then
                local timestamp
                timestamp=$(date +%Y%m%d_%H%M%S)
                local backup_path="${full_path}${backup_suffix}.${timestamp}"
                print_info "Backing up before merge: ${dir_path}"
                cp -r "$full_path" "$backup_path"
            fi
            print_info "Merging into ${dir_path}"
            # Extension writes files, existing files preserved unless overwritten
            ;;

        prompt-per-file)
            prompt_directory_conflict "$ext" "$dir_path" "$rule"
            ;;

        skip)
            print_info "Skipping ${dir_path} (already exists)"
            ;;

        *)
            print_warning "Unknown action '${action}' for ${dir_path}, proceeding with merge"
            ;;
    esac

    return 0
}

# Prompt user for file-level decision
# Usage: prompt_user_file_action <extension_name> <file_path> <original_file> <new_file>
prompt_user_file_action() {
    local ext="$1"
    local file_path="$2"
    local original_file="$3"
    local new_file="$4"

    # Check if prompting is disabled
    if [[ "${EXTENSION_CONFLICT_PROMPT:-true}" == "false" ]]; then
        print_info "EXTENSION_CONFLICT_PROMPT=false, skipping ${file_path} (keeping original)"
        mv "$original_file" "$file_path"
        return 0
    fi

    print_warning "File conflict detected: ${file_path}"
    print_info "Extension ${ext} wants to modify this file."
    echo ""

    PS3="Choose action: "
    select action in "Overwrite" "Append" "Prepend" "Skip" "Backup then Overwrite"; do
        case "$action" in
            "Overwrite")
                cp "$new_file" "$file_path"
                print_success "Overwritten ${file_path}"
                break;;
            "Append")
                cat "$new_file" >> "$file_path"
                print_success "Appended to ${file_path}"
                break;;
            "Prepend")
                local temp
                temp=$(mktemp)
                cat "$new_file" "$original_file" > "$temp"
                mv "$temp" "$file_path"
                print_success "Prepended to ${file_path}"
                break;;
            "Skip")
                mv "$original_file" "$file_path"
                print_info "Skipped ${file_path}"
                break;;
            "Backup then Overwrite")
                local timestamp
                timestamp=$(date +%Y%m%d_%H%M%S)
                mv "$file_path" "${file_path}.backup.${timestamp}"
                cp "$new_file" "$file_path"
                print_success "Backed up and overwritten ${file_path}"
                break;;
        esac
    done
}

# Prompt for directory merge (prompts per-file)
# Usage: prompt_directory_conflict <extension_name> <dir_path> <rule_yaml>
prompt_directory_conflict() {
    local ext="$1"
    local dir_path="$2"
    local rule="$3"

    # Check if prompting is disabled
    if [[ "${EXTENSION_CONFLICT_PROMPT:-true}" == "false" ]]; then
        print_info "EXTENSION_CONFLICT_PROMPT=false, skipping ${dir_path} (keeping existing)"
        return 1  # Signal to skip extension
    fi

    print_warning "Directory conflict detected: ${dir_path}"
    print_info "Extension ${ext} wants to write to this directory."
    echo ""

    PS3="Choose action: "
    select action in "Merge (prompt per file)" "Backup and Replace" "Skip"; do
        case "$action" in
            "Merge (prompt per file)")
                print_info "Will prompt for each file conflict during initialization"
                # Set flag for extension to prompt on each file write
                export PROMPT_ON_FILE_CONFLICT=true
                break;;
            "Backup and Replace")
                local timestamp
                timestamp=$(date +%Y%m%d_%H%M%S)
                local full_path="${PWD}/${dir_path}"
                mv "$full_path" "${full_path}.backup.${timestamp}"
                print_success "Backed up to ${dir_path}.backup.${timestamp}"
                break;;
            "Skip")
                print_info "Skipping ${dir_path} - extension will not initialize"
                return 1;;  # Signal to skip extension
        esac
    done

    return 0
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
