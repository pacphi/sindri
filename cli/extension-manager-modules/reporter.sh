#!/bin/bash
# reporter.sh - Status reporting (declarative)

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${MODULE_DIR}/../../docker/lib/common.sh"
source "${MODULE_DIR}/dependency.sh"

# List all available extensions from registry
list_extensions() {
    local registry_file="${DOCKER_LIB}/registry.yaml"
    local categories_file="${DOCKER_LIB}/categories.yaml"

    if [[ ! -f "$registry_file" ]]; then
        print_error "Registry not found"
        return 1
    fi

    print_header "Available Extensions"

    # Load categories from declarative file
    local categories
    if [[ ! -f "$categories_file" ]]; then
        print_error "Categories file not found: $categories_file"
        return 1
    fi
    categories=$(load_yaml "$categories_file" '.categories | keys[]' 2>/dev/null || echo "")

    for category in $categories; do
        local found=false

        # Get category info
        local cat_name cat_icon
        if [[ -f "$categories_file" ]]; then
            cat_name=$(load_yaml "$categories_file" ".categories.\"$category\".name" 2>/dev/null || echo "$category")
            cat_icon=$(load_yaml "$categories_file" ".categories.\"$category\".icon" 2>/dev/null || echo "")
        else
            cat_name="$category"
            cat_icon=""
        fi

        # List extensions in this category
        local ext_names
        ext_names=$(load_yaml "$registry_file" ".extensions | to_entries[] | select(.value.category == \"$category\") | .key" 2>/dev/null || echo "")

        if [[ -n "$ext_names" ]]; then
            if [[ "$found" == "false" ]]; then
                echo ""
                echo "$cat_icon $cat_name:"
                found=true
            fi

            for ext_name in $ext_names; do
                local description protected
                description=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".description" 2>/dev/null || echo "")
                protected=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".protected" 2>/dev/null || echo "false")

                local status_mark=""
                if is_extension_installed "$ext_name"; then
                    status_mark="${GREEN}✓${NC}"
                else
                    status_mark=" "
                fi

                local protected_mark=""
                [[ "$protected" == "true" ]] && protected_mark=" [protected]"

                echo -e "  $status_mark $ext_name - $description$protected_mark"
            done
        fi
    done

    echo ""
}

# List available profiles
list_profiles() {
    local profiles_file="${DOCKER_LIB}/profiles.yaml"

    if [[ ! -f "$profiles_file" ]]; then
        print_error "Profiles not found"
        return 1
    fi

    print_header "Available Profiles"

    local profile_names
    profile_names=$(load_yaml "$profiles_file" '.profiles | keys[]' 2>/dev/null || echo "")

    for profile in $profile_names; do
        local description extensions_count
        description=$(load_yaml "$profiles_file" ".profiles.\"$profile\".description" 2>/dev/null || echo "")
        extensions_count=$(load_yaml "$profiles_file" ".profiles.\"$profile\".extensions | length" 2>/dev/null || echo "0")

        echo "  $profile - $description ($extensions_count extensions)"
    done

    echo ""
}

# List categories
list_categories() {
    local categories_file="${DOCKER_LIB}/categories.yaml"

    if [[ ! -f "$categories_file" ]]; then
        print_error "Categories not found"
        return 1
    fi

    print_header "Extension Categories"

    local categories
    categories=$(load_yaml "$categories_file" '.categories | keys[]' 2>/dev/null || echo "")

    for category in $categories; do
        local name description icon priority
        name=$(load_yaml "$categories_file" ".categories.\"$category\".name" 2>/dev/null || echo "$category")
        description=$(load_yaml "$categories_file" ".categories.\"$category\".description" 2>/dev/null || echo "")
        icon=$(load_yaml "$categories_file" ".categories.\"$category\".icon" 2>/dev/null || echo "")
        priority=$(load_yaml "$categories_file" ".categories.\"$category\".priority" 2>/dev/null || echo "")

        echo "  $icon $name ($category) - $description [priority: $priority]"
    done

    echo ""
}

# Show extension info
show_extension_info() {
    local ext_name="$1"
    local registry_file="${DOCKER_LIB}/registry.yaml"

    if [[ ! -f "$registry_file" ]]; then
        print_error "Registry not found"
        return 1
    fi

    # Check if extension exists in registry
    local exists
    exists=$(load_yaml "$registry_file" ".extensions.\"$ext_name\"" 2>/dev/null || echo "null")

    if [[ "$exists" == "null" ]]; then
        print_error "Extension not found: $ext_name"
        return 1
    fi

    print_header "Extension: $ext_name"

    # Load extension info
    local category description protected dependencies
    category=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".category" 2>/dev/null || echo "unknown")
    description=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".description" 2>/dev/null || echo "")
    protected=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".protected" 2>/dev/null || echo "false")
    dependencies=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".dependencies[]?" 2>/dev/null || echo "none")

    echo "Category: $category"
    echo "Description: $description"
    [[ "$protected" == "true" ]] && echo "Protected: Yes (cannot be removed)"
    echo "Dependencies: ${dependencies:-none}"

    if is_extension_installed "$ext_name"; then
        echo "Status: Installed"
        local install_date
        install_date=$(cat "${WORKSPACE_SYSTEM:-/workspace/.system}/installed/$ext_name.installed" 2>/dev/null || echo "unknown")
        echo "Installed: $install_date"
    else
        echo "Status: Not installed"
    fi

    # Show extension YAML if exists
    local ext_yaml="$EXTENSIONS_DIR/$ext_name/extension.yaml"
    if [[ -f "$ext_yaml" ]]; then
        echo ""
        echo "Extension Definition: $ext_yaml"

        # Show requirements
        local disk_space domains
        disk_space=$(load_yaml "$ext_yaml" '.requirements.diskSpace' 2>/dev/null || echo "0")
        domains=$(load_yaml "$ext_yaml" '.requirements.domains[]?' 2>/dev/null | tr '\n' ', ' | sed 's/,$//')

        [[ "$disk_space" != "0" ]] && [[ "$disk_space" != "null" ]] && echo "Disk Space Required: ${disk_space}MB"
        [[ -n "$domains" ]] && echo "Required Domains: $domains"
    fi

    echo ""
}

# Search extensions
search_extensions() {
    local search_term="$1"
    local registry_file="${DOCKER_LIB}/registry.yaml"

    if [[ ! -f "$registry_file" ]]; then
        print_error "Registry not found"
        return 1
    fi

    print_header "Search Results for: $search_term"

    # Search in extension names and descriptions
    local matches
    matches=$(load_yaml "$registry_file" ".extensions | to_entries[] | select(.key | contains(\"$search_term\")) or select(.value.description | contains(\"$search_term\")) | .key" 2>/dev/null || echo "")

    if [[ -z "$matches" ]]; then
        print_status "No extensions found matching: $search_term"
        return 0
    fi

    for ext_name in $matches; do
        local description category
        description=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".description" 2>/dev/null || echo "")
        category=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".category" 2>/dev/null || echo "unknown")

        local status_mark=""
        if is_extension_installed "$ext_name"; then
            status_mark="${GREEN}✓${NC}"
        else
            status_mark=" "
        fi

        echo -e "  $status_mark $ext_name ($category) - $description"
    done

    echo ""
}

# Show status of all extensions
show_all_status() {
    print_header "Extension Status"

    local installed_count=0
    local total_count=0

    # Count from registry
    local registry_file="${DOCKER_LIB}/registry.yaml"
    if [[ -f "$registry_file" ]]; then
        total_count=$(load_yaml "$registry_file" '.extensions | length' 2>/dev/null || echo "0")
    fi

    # Count installed
    if [[ -d "${WORKSPACE_SYSTEM:-/workspace/.system}/installed" ]]; then
        installed_count=$(find "${WORKSPACE_SYSTEM:-/workspace/.system}/installed" -name "*.installed" | wc -l)
    fi

    echo "Installed: $installed_count / $total_count"
    echo ""

    # Show installed extensions
    if [[ $installed_count -gt 0 ]]; then
        echo "Installed extensions:"
        local installed_exts
        installed_exts=$(find "${WORKSPACE_SYSTEM:-/workspace/.system}/installed" -name "*.installed" -exec basename {} .installed \; 2>/dev/null)

        for ext_name in $installed_exts; do
            local category
            if [[ -f "$registry_file" ]]; then
                category=$(load_yaml "$registry_file" ".extensions.\"$ext_name\".category" 2>/dev/null || echo "unknown")
            else
                category="unknown"
            fi
            echo "  - $ext_name ($category)"
        done
    fi

    echo ""
}

# Export functions
export -f list_extensions list_profiles list_categories
export -f show_extension_info search_extensions show_all_status