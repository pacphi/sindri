#!/bin/bash
# validator.sh - Validation logic (declarative)

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Detect environment and source common functions
if [[ -f "/docker/lib/common.sh" ]]; then
    source /docker/lib/common.sh
else
    source "${MODULE_DIR}/../../docker/lib/common.sh"
fi

source "${MODULE_DIR}/executor.sh"

# Validate all installed extensions
validate_all_extensions() {
    local extensions
    extensions=$(find "${WORKSPACE_SYSTEM}/installed" -name "*.installed" -exec basename {} .installed \; 2>/dev/null)

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

# Validate domain format (RFC 1123 hostname)
validate_domain_format() {
    local domain="$1"

    # Must not be empty
    [[ -z "$domain" ]] && return 1

    # Must not start or end with hyphen or dot
    [[ "$domain" =~ ^[-.]|[-.]$ ]] && return 1

    # Must contain at least one dot (TLD required)
    [[ ! "$domain" =~ \. ]] && return 1

    # Basic hostname validation
    if [[ ! "$domain" =~ ^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$ ]]; then
        return 1
    fi

    return 0
}

# Validate domains for a single extension
validate_extension_domains() {
    local ext_name="$1"
    local ext_yaml="$EXTENSIONS_DIR/$ext_name/extension.yaml"
    local errors=0
    local warnings=0

    if [[ ! -f "$ext_yaml" ]]; then
        print_error "Extension not found: $ext_name"
        return 1
    fi

    print_status "Validating domains for: $ext_name"

    # Get domains from extension
    local domains
    domains=$(load_yaml "$ext_yaml" '.requirements.domains[]?' 2>/dev/null || true)

    if [[ -z "$domains" ]]; then
        print_status "  No domains defined"
        return 0
    fi

    # Check format
    for domain in $domains; do
        if ! validate_domain_format "$domain"; then
            print_error "  Invalid domain format: $domain"
            ((errors++)) || true
        else
            [[ "${VERBOSE:-false}" == "true" ]] && print_status "  Format OK: $domain"
        fi
    done

    # Check duplicates
    local duplicates
    duplicates=$(echo "$domains" | sort | uniq -d)
    if [[ -n "$duplicates" ]]; then
        for dup in $duplicates; do
            print_error "  Duplicate domain: $dup"
            ((errors++)) || true
        done
    fi

    # DNS check (optional)
    if [[ "${CHECK_DNS:-false}" == "true" ]]; then
        for domain in $domains; do
            if check_dns "$domain" 3; then
                print_status "  DNS OK: $domain"
            else
                print_warning "  DNS failed: $domain"
                ((warnings++)) || true
            fi
        done
    fi

    if [[ $errors -gt 0 ]]; then
        print_error "$ext_name: $errors domain error(s)"
        return 1
    fi

    if [[ $warnings -gt 0 ]]; then
        print_warning "$ext_name: $warnings domain warning(s)"
    else
        print_success "$ext_name: domains valid"
    fi

    return 0
}

# Validate domains for all extensions
validate_all_domains() {
    local errors=0
    local checked=0

    print_header "Validating extension domain requirements"

    for ext_dir in "$EXTENSIONS_DIR"/*/; do
        [[ -f "$ext_dir/extension.yaml" ]] || continue
        local ext_name
        ext_name=$(basename "$ext_dir")

        if ! validate_extension_domains "$ext_name"; then
            ((errors++)) || true
        fi
        ((checked++)) || true
    done

    echo ""
    print_status "Checked: $checked extensions"

    if [[ $errors -gt 0 ]]; then
        print_error "Domain validation failed: $errors error(s)"
        return 1
    fi

    print_success "All domain requirements valid"
    return 0
}

# Export functions
export -f validate_all_extensions validate_extension_schema validate_domain_format validate_extension_domains validate_all_domains