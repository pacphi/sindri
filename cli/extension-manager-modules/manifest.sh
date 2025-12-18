#!/bin/bash
# manifest.sh - Manifest management (declarative)

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Detect environment and source common functions
if [[ -f "/docker/lib/common.sh" ]]; then
    source /docker/lib/common.sh
else
    source "${MODULE_DIR}/../../docker/lib/common.sh"
fi

# WORKSPACE_MANIFEST is set by common.sh (sourced above)
MANIFEST_FILE="${WORKSPACE_MANIFEST}/active-extensions.yaml"

#######################################
# Validate extension name (H-3 security fix)
# Prevents YAML injection by enforcing safe naming pattern
# Arguments:
#   $1 - Extension name to validate
# Returns:
#   0 if valid, 1 if invalid
#######################################
validate_extension_name() {
    local ext_name="$1"

    # Extension names must be lowercase alphanumeric with hyphens only
    if [[ ! "$ext_name" =~ ^[a-z0-9-]+$ ]]; then
        print_error "Invalid extension name: $ext_name"
        print_error "Extension names must contain only lowercase letters, numbers, and hyphens"
        return 1
    fi

    # Prevent names that are just hyphens
    if [[ "$ext_name" =~ ^-+$ ]]; then
        print_error "Invalid extension name: $ext_name"
        print_error "Extension name cannot be only hyphens"
        return 1
    fi

    return 0
}

# Read manifest
read_manifest() {
    if [[ ! -f "$MANIFEST_FILE" ]]; then
        print_warning "No manifest found, initializing..."
        initialize_manifest
    fi

    load_yaml "$MANIFEST_FILE"
}

# Initialize manifest with base extensions only
initialize_manifest() {
    ensure_directory "$(dirname "$MANIFEST_FILE")"

    # Load base extensions from registry
    local registry_file="${DOCKER_LIB}/registry.yaml"

    cat > "$MANIFEST_FILE" << EOF
version: "1.0"
generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

extensions: []
EOF

    # Add protected base extensions from registry
    if [[ -f "$registry_file" ]]; then
        local base_exts
        base_exts=$(load_yaml "$registry_file" '.extensions | to_entries[] | select(.value.category == "base" and .value.protected == true) | .key')

        for ext in $base_exts; do
            add_to_manifest "$ext" "base" true
        done
    fi

    print_success "Initialized manifest at $MANIFEST_FILE"
}

# Get active extensions
get_active_extensions() {
    load_yaml "$MANIFEST_FILE" '.extensions[] | select(.active == true) | .name' 2>/dev/null || true
}

# Add extension to manifest
add_to_manifest() {
    local ext_name="$1"
    local category="${2:-undefined}"
    local protected="${3:-false}"

    # H-3 Security Fix: Validate extension name to prevent YAML injection
    if ! validate_extension_name "$ext_name"; then
        return 1
    fi

    if ! command_exists yq; then
        print_error "yq is required for manifest management"
        return 1
    fi

    # Ensure manifest file exists before modifying
    if [[ ! -f "$MANIFEST_FILE" ]]; then
        initialize_manifest
    fi

    # Check if already in manifest using environment variable (safe from injection)
    local exists
    exists=$(EXT_NAME="$ext_name" yq eval '.extensions[] | select(.name == env(EXT_NAME))' "$MANIFEST_FILE" 2>/dev/null || echo "")

    if [[ -n "$exists" ]]; then
        # Update to active using environment variable
        EXT_NAME="$ext_name" yq eval -i '(.extensions[] | select(.name == env(EXT_NAME))).active = true' "$MANIFEST_FILE"
    else
        # Add new entry using environment variables for all dynamic values
        EXT_NAME="$ext_name" CATEGORY="$category" PROTECTED="$protected" \
            yq eval -i '.extensions += [{"name": env(EXT_NAME), "active": true, "category": env(CATEGORY), "protected": env(PROTECTED)}]' "$MANIFEST_FILE"
    fi

    [[ "${VERBOSE:-false}" == "true" ]] && print_success "Added $ext_name to manifest"
    return 0
}

# Remove extension from manifest
remove_from_manifest() {
    local ext_name="$1"

    # H-3 Security Fix: Validate extension name to prevent YAML injection
    if ! validate_extension_name "$ext_name"; then
        return 1
    fi

    if ! command_exists yq; then
        print_error "yq is required for manifest management"
        return 1
    fi

    # Check if protected using environment variable (safe from injection)
    local is_protected
    is_protected=$(EXT_NAME="$ext_name" yq eval '.extensions[] | select(.name == env(EXT_NAME)) | .protected // false' "$MANIFEST_FILE" 2>/dev/null || echo "false")

    if [[ "$is_protected" == "true" ]]; then
        print_error "Cannot remove protected extension: $ext_name"
        return 1
    fi

    # Set to inactive instead of removing using environment variable
    EXT_NAME="$ext_name" yq eval -i '(.extensions[] | select(.name == env(EXT_NAME))).active = false' "$MANIFEST_FILE"

    [[ "${VERBOSE:-false}" == "true" ]] && print_success "Deactivated $ext_name in manifest"
    return 0
}

# List extensions from manifest
list_manifest_extensions() {
    local format="${1:-short}"

    if [[ "$format" == "detailed" ]]; then
        load_yaml "$MANIFEST_FILE" '.extensions[] | "\(.name) (\(.category)) - " + (if .active then "active" else "inactive" end) + (if .protected then " [protected]" else "" end)'
    else
        load_yaml "$MANIFEST_FILE" '.extensions[] | select(.active == true) | .name'
    fi
}

# Export functions
export -f read_manifest initialize_manifest get_active_extensions
export -f add_to_manifest remove_from_manifest list_manifest_extensions