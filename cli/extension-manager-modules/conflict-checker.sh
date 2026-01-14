#!/usr/bin/env bash

# conflict-checker.sh - Extension conflict detection
#
# Prevents installation of mutually exclusive extensions
# Extensions can declare conflicts in registry.yaml

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine lib directory
if [[ -d "/docker/lib" ]]; then
    DOCKER_LIB="/docker/lib"
else
    DOCKER_LIB="$(cd "${SCRIPT_DIR}/../../docker/lib" && pwd)"
fi

source "${DOCKER_LIB}/common.sh"

REGISTRY_FILE="${DOCKER_LIB}/registry.yaml"

###############################################################################
# Conflict Detection Functions
###############################################################################

# Check if extension has conflicts defined in registry
# Usage: get_conflicts <extension_name>
# Returns: Space-separated list of conflicting extension names
get_conflicts() {
    local ext_name="$1"

    if [[ ! -f "$REGISTRY_FILE" ]]; then
        return 0
    fi

    # Query registry for conflicts
    local conflicts
    conflicts=$(yq eval ".extensions[] | select(.name == \"${ext_name}\") | .conflicts[]" \
        "$REGISTRY_FILE" 2>/dev/null || echo "")

    echo "$conflicts"
}

# Check for conflicting extensions before installation
# Usage: check_conflicts <extension_name>
# Returns: 0 if no conflicts, 1 if conflict detected
check_conflicts() {
    local ext_name="$1"

    # Get conflicts list from registry
    local conflicts
    conflicts=$(get_conflicts "$ext_name")

    if [[ -z "$conflicts" ]]; then
        # No conflicts defined
        return 0
    fi

    # Check if any conflicting extension is installed
    local has_conflict=false
    while IFS= read -r conflict_ext; do
        if [[ -z "$conflict_ext" ]]; then
            continue
        fi

        if is_extension_installed "$conflict_ext"; then
            print_error "Cannot install ${ext_name}: conflicts with installed extension ${conflict_ext}"
            print_info "To proceed, first remove the conflicting extension:"
            print_info "  extension-manager remove ${conflict_ext}"
            has_conflict=true
        fi
    done <<< "$conflicts"

    if [[ "$has_conflict" == "true" ]]; then
        return 1
    fi

    return 0
}

# List all conflicts for an extension
# Usage: list_conflicts <extension_name>
list_conflicts() {
    local ext_name="$1"

    local conflicts
    conflicts=$(get_conflicts "$ext_name")

    if [[ -z "$conflicts" ]]; then
        echo "No conflicts defined for ${ext_name}"
    else
        echo "Extensions that conflict with ${ext_name}:"
        while IFS= read -r conflict_ext; do
            if [[ -n "$conflict_ext" ]]; then
                # Check if installed
                if is_extension_installed "$conflict_ext"; then
                    echo "  ✗ ${conflict_ext} (INSTALLED - cannot install ${ext_name})"
                else
                    echo "  ○ ${conflict_ext} (not installed)"
                fi
            fi
        done <<< "$conflicts"
    fi
}

###############################################################################
# Main Entry Point (for testing)
###############################################################################

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "Conflict Checker - Test Mode"
    echo "============================"
    echo ""

    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 <command> [args]"
        echo ""
        echo "Commands:"
        echo "  check <extension>     Check for conflicts before installation"
        echo "  list <extension>      List conflicts for extension"
        echo "  get <extension>       Get conflict list (space-separated)"
        echo ""
        exit 0
    fi

    case "$1" in
        check)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 check <extension>"
                exit 1
            fi
            if check_conflicts "$2"; then
                echo "✓ No conflicts detected for $2"
            else
                exit 1
            fi
            ;;
        list)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 list <extension>"
                exit 1
            fi
            list_conflicts "$2"
            ;;
        get)
            if [[ $# -lt 2 ]]; then
                echo "Usage: $0 get <extension>"
                exit 1
            fi
            get_conflicts "$2"
            ;;
        *)
            echo "Unknown command: $1"
            exit 1
            ;;
    esac
fi
