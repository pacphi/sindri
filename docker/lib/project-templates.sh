#!/bin/bash
#
# project-templates.sh - Template loading, validation, and processing
#
# This library provides functions for loading project templates from YAML,
# validating them against JSON Schema, and processing template variables.
# Uses yq for reliable YAML parsing.
#

# Note: set -euo pipefail is set by the calling script

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[[ "${COMMON_SH_LOADED:-}" != "true" ]] && source "${SCRIPT_DIR}/common.sh"

TEMPLATES_CONFIG="${TEMPLATES_CONFIG:-${SCRIPT_DIR}/project-templates.yaml}"
TEMPLATES_SCHEMA="${TEMPLATES_SCHEMA:-${SCRIPT_DIR}/schemas/project-templates.schema.json}"

_check_yq() {
    if ! command_exists yq; then
        print_error "yq is required for template processing"
        print_error "Install: brew install yq (macOS) or sudo apt install yq (Linux)"
        return 1
    fi
    return 0
}

get_template_types() {
    _check_yq || return 1

    if [[ ! -f "$TEMPLATES_CONFIG" ]]; then
        print_error "Templates config not found: $TEMPLATES_CONFIG"
        return 1
    fi

    yq eval '.templates | keys | .[]' "$TEMPLATES_CONFIG"
}

get_template_description() {
    local template_type="$1"
    _check_yq || return 1

    yq eval ".templates.${template_type}.description // \"\"" "$TEMPLATES_CONFIG"
}

load_project_template() {
    local template_type="$1"
    _check_yq || return 1

    if [[ ! -f "$TEMPLATES_CONFIG" ]]; then
        print_error "Templates config not found: $TEMPLATES_CONFIG"
        return 1
    fi

    local template
    template=$(yq eval ".templates.${template_type} // null" -o=json "$TEMPLATES_CONFIG")

    if [[ "$template" == "null" ]]; then
        print_error "Template not found: $template_type"
        return 1
    fi

    echo "$template"
}

validate_template_schema() {
    if [[ ! -f "$TEMPLATES_SCHEMA" ]]; then
        print_debug "Schema file not found: $TEMPLATES_SCHEMA (skipping validation)"
        return 0
    fi

    if ! command_exists ajv; then
        print_debug "ajv-cli not available (skipping schema validation)"
        return 0
    fi

    print_status "Validating template schema..."

    if ajv validate -s "$TEMPLATES_SCHEMA" -d "$TEMPLATES_CONFIG" 2>/dev/null; then
        print_success "Template schema validation passed"
        return 0
    else
        print_error "Template schema validation failed"
        return 1
    fi
}

resolve_template_variables() {
    local content="$1"
    local variables_json="$2"

    local result="$content"

    while IFS= read -r key; do
        local value
        value=$(echo "$variables_json" | yq eval ".${key}" -)
        result=$(echo "$result" | sed "s|{${key}}|${value}|g")
    done < <(echo "$variables_json" | yq eval 'keys | .[]' -)

    echo "$result"
}

execute_template_setup() {
    local template_json="$1"
    local variables_json="$2"

    print_status "Executing setup commands..."

    local commands
    commands=$(echo "$template_json" | yq eval '.setup_commands // [] | .[]' -)

    if [[ -z "$commands" ]]; then
        print_debug "No setup commands to execute"
        return 0
    fi

    while IFS= read -r cmd; do
        if [[ -n "$cmd" ]]; then
            cmd=$(resolve_template_variables "$cmd" "$variables_json")

            print_debug "Running: $cmd"
            if eval "$cmd" 2>/dev/null; then
                print_debug "Command succeeded: $cmd"
            else
                print_warning "Command failed: $cmd"
            fi
        fi
    done <<< "$commands"

    return 0
}

create_template_files() {
    local template_json="$1"
    local variables_json="$2"

    print_status "Creating template files..."

    local files_json
    files_json=$(echo "$template_json" | yq eval '.files // {}' -o=json -)

    if [[ "$files_json" == "{}" ]]; then
        print_debug "No template files to create"
        return 0
    fi

    while IFS= read -r filepath; do
        if [[ -z "$filepath" ]]; then
            continue
        fi

        filepath=$(resolve_template_variables "$filepath" "$variables_json")

        local content
        # Need to extract the file content from the original template
        content=$(echo "$files_json" | jq -r --arg fp "$filepath" '.[$fp] // ""')

        content=$(resolve_template_variables "$content" "$variables_json")

        mkdir -p "$(dirname "$filepath")"

        echo "$content" > "$filepath"
        print_debug "Created file: $filepath"
    done < <(echo "$files_json" | jq -r 'keys[]')

    print_success "Template files created"
    return 0
}

get_template_dependencies() {
    local template_type="$1"
    _check_yq || return 1

    local deps
    deps=$(yq eval ".templates.${template_type}.dependencies // null" -o=json "$TEMPLATES_CONFIG")

    if [[ "$deps" == "null" ]]; then
        return 1
    fi

    echo "$deps"
}

get_all_dependencies_configs() {
    _check_yq || return 1

    # Returns JSON array of all dependency configs with their template names
    yq eval '
        [.templates | to_entries[] |
         select(.value.dependencies != null) |
         {"template": .key} + .value.dependencies]
    ' -o=json "$TEMPLATES_CONFIG"
}

# Resolve an alias to its canonical template name
# Returns the canonical name, or the input if no alias match
resolve_template_alias() {
    local input="$1"
    _check_yq || return 1

    local input_lower
    input_lower=$(echo "$input" | tr '[:upper:]' '[:lower:]')

    # First check if it's already a valid template name
    if yq eval ".templates.${input_lower} // null" "$TEMPLATES_CONFIG" | grep -qv '^null$'; then
        echo "$input_lower"
        return 0
    fi

    # Search through aliases
    local result
    result=$(yq eval "
        .templates | to_entries[] |
        select(.value.aliases != null) |
        select(.value.aliases[] | . == \"${input_lower}\") |
        .key
    " "$TEMPLATES_CONFIG" | head -1)

    if [[ -n "$result" ]]; then
        echo "$result"
    else
        echo "$input_lower"
    fi
}

# Detect project type from name using detection_rules.name_patterns from YAML
# Returns: single type if unambiguous, "ambiguous:<type1>,<type2>,..." if multiple, empty if no match
detect_type_from_name() {
    local project_name="$1"
    _check_yq || return 1

    local name_lower
    name_lower=$(echo "$project_name" | tr '[:upper:]' '[:lower:]')

    # Iterate through name_patterns in order
    local pattern type types
    while IFS= read -r rule_json; do
        [[ -z "$rule_json" ]] && continue

        pattern=$(echo "$rule_json" | yq eval '.pattern' -)
        type=$(echo "$rule_json" | yq eval '.type // ""' -)
        types=$(echo "$rule_json" | yq eval '.types // [] | join(",")' -)

        # Check if pattern matches (case-insensitive regex)
        if echo "$name_lower" | grep -qiE "$pattern"; then
            if [[ -n "$type" ]]; then
                # Single type match
                echo "$type"
                return 0
            elif [[ -n "$types" ]]; then
                # Multiple suggestions - ambiguous
                echo "ambiguous:$types"
                return 0
            fi
        fi
    done < <(yq eval '.detection_rules.name_patterns[] | @json' "$TEMPLATES_CONFIG")

    # No match found
    echo ""
}

# Get suggestions for a specific ambiguous category (api, web, service, etc.)
# Input: comma-separated list of types
# Output: formatted suggestions with descriptions
get_type_suggestions() {
    local types_csv="$1"
    _check_yq || return 1

    local i=1
    local IFS=','
    for type in $types_csv; do
        local desc
        desc=$(get_template_description "$type")
        printf "  %d) %-10s - %s\n" "$i" "$type" "$desc"
        ((i++))
    done
}

# Convert numbered choice to type from a CSV list
# Usage: resolve_type_choice "1" "node,go,python"
resolve_type_choice() {
    local choice="$1"
    local types_csv="$2"

    # If choice is a number, extract from list
    if [[ "$choice" =~ ^[0-9]+$ ]]; then
        local i=1
        local IFS=','
        for type in $types_csv; do
            if [[ "$i" -eq "$choice" ]]; then
                echo "$type"
                return 0
            fi
            ((i++))
        done
    fi

    # Otherwise return as-is (user typed a name)
    echo "$choice"
}

export -f get_template_types
export -f get_template_description
export -f load_project_template
export -f validate_template_schema
export -f resolve_template_variables
export -f execute_template_setup
export -f create_template_files
export -f get_template_dependencies
export -f get_all_dependencies_configs
export -f resolve_template_alias
export -f detect_type_from_name
export -f get_type_suggestions
export -f resolve_type_choice
