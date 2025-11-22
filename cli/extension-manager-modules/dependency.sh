#!/bin/bash
# dependency.sh - Dependency resolution (declarative)

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Detect environment and source common functions
if [[ -f "/docker/lib/common.sh" ]]; then
    source /docker/lib/common.sh
else
    source "${MODULE_DIR}/../../docker/lib/common.sh"
fi

# Get dependencies from registry
get_dependencies() {
    local ext_name="$1"
    local registry_file="${DOCKER_LIB}/registry.yaml"

    # First check extension YAML
    local ext_yaml="$EXTENSIONS_DIR/$ext_name/extension.yaml"
    if [[ -f "$ext_yaml" ]]; then
        local deps
        deps=$(load_yaml "$ext_yaml" '.metadata.dependencies[]?' 2>/dev/null || true)
        if [[ -n "$deps" ]]; then
            echo "$deps"
            return 0
        fi
    fi

    # Fallback to registry
    if [[ -f "$registry_file" ]]; then
        load_yaml "$registry_file" ".extensions.\"$ext_name\".dependencies[]?" 2>/dev/null || true
    fi
}

# Topological sort of extensions based on dependencies
resolve_dependencies() {
    local extensions=("$@")
    local resolved=()
    local seen=()

    visit() {
        local ext="$1"

        # Check for cycles
        for s in "${seen[@]}"; do
            if [[ "$s" == "$ext" ]]; then
                print_error "Circular dependency detected: $ext"
                return 1
            fi
        done

        # Already resolved
        for r in "${resolved[@]}"; do
            if [[ "$r" == "$ext" ]]; then
                return 0
            fi
        done

        seen+=("$ext")

        # Visit dependencies first
        local deps
        deps=$(get_dependencies "$ext")
        for dep in $deps; do
            visit "$dep" || return 1
        done

        # Remove from seen (backtrack)
        local new_seen=()
        for s in "${seen[@]}"; do
            [[ "$s" != "$ext" ]] && new_seen+=("$s")
        done
        seen=("${new_seen[@]}")

        resolved+=("$ext")
    }

    # Visit all extensions
    for ext in "${extensions[@]}"; do
        visit "$ext" || return 1
    done

    # Return resolved order
    echo "${resolved[@]}"
}

# Check if all dependencies are installed
check_dependencies() {
    local ext_name="$1"
    local deps
    deps=$(get_dependencies "$ext_name")

    for dep in $deps; do
        if ! is_extension_installed "$dep"; then
            print_error "Missing dependency: $dep (required by $ext_name)"
            return 1
        fi
    done

    return 0
}

# Check if extension is installed
is_extension_installed() {
    local ext_name="$1"
    local marker_file="${WORKSPACE_SYSTEM:-/workspace/.system}/installed/$ext_name.installed"

    [[ -f "$marker_file" ]]
}

# Mark extension as installed
mark_installed() {
    local ext_name="$1"
    local installed_dir="${WORKSPACE_SYSTEM:-/workspace/.system}/installed"
    ensure_directory "$installed_dir"
    date -u +"%Y-%m-%dT%H:%M:%SZ" > "$installed_dir/$ext_name.installed"
}

# Mark extension as uninstalled
mark_uninstalled() {
    local ext_name="$1"
    rm -f "${WORKSPACE_SYSTEM:-/workspace/.system}/installed/$ext_name.installed"
}

# Export functions
export -f get_dependencies resolve_dependencies check_dependencies
export -f is_extension_installed mark_installed mark_uninstalled