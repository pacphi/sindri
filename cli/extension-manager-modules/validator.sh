#!/bin/bash
# validator.sh - Validation logic (declarative)

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${MODULE_DIR}/../../docker/lib/common.sh"
source "${MODULE_DIR}/executor.sh"

# Validate all installed extensions
validate_all_extensions() {
    local extensions
    extensions=$(find "${WORKSPACE_SYSTEM:-/workspace/.system}/installed" -name "*.installed" -exec basename {} .installed \; 2>/dev/null)

    if [[ -z "$extensions" ]]; then
        print_warning "No extensions installed"
        return 0
    fi

    local all_valid=true

    for ext in $extensions; do
        if ! execute_extension "$ext" "validate"; then
            all_valid=false
        fi
    done

    if [[ "$all_valid" == "true" ]]; then
        print_success "All extensions validated successfully"
        return 0
    else
        print_error "Some extensions failed validation"
        return 1
    fi
}

# Validate extension against schema
validate_extension_schema() {
    local ext_name="$1"
    local ext_yaml="$EXTENSIONS_DIR/$ext_name/extension.yaml"
    local schema="$SCHEMAS_DIR/extension.schema.json"

    if [[ ! -f "$ext_yaml" ]]; then
        print_error "Extension YAML not found: $ext_yaml"
        return 1
    fi

    if [[ ! -f "$schema" ]]; then
        print_warning "Schema not found: $schema"
        return 0
    fi

    validate_yaml_schema "$ext_yaml" "$schema"
}

# Export functions
export -f validate_all_extensions validate_extension_schema