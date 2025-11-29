#!/bin/bash
# executor.sh - YAML execution engine (declarative)

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Detect environment and source common functions
if [[ -f "/docker/lib/common.sh" ]]; then
    source /docker/lib/common.sh
else
    source "${MODULE_DIR}/../../docker/lib/common.sh"
fi

source "${MODULE_DIR}/dependency.sh"
source "${MODULE_DIR}/bom.sh"

# Execute extension action
execute_extension() {
    local ext_name="$1"
    local action="${2:-install}"

    local ext_dir="$EXTENSIONS_DIR/$ext_name"
    local ext_yaml="$ext_dir/extension.yaml"

    if [[ ! -d "$ext_dir" ]]; then
        print_error "Extension not found: $ext_name"
        return 1
    fi

    if [[ ! -f "$ext_yaml" ]]; then
        print_error "No extension.yaml found for: $ext_name"
        return 1
    fi

    # Validate YAML against schema
    local schema="$SCHEMAS_DIR/extension.schema.json"
    if [[ -f "$schema" ]] && [[ "${VERBOSE:-false}" == "true" ]]; then
        print_status "Validating $ext_name against schema..."
        validate_yaml_schema "$ext_yaml" "$schema" || return 1
    fi

    # Execute based on action
    case "$action" in
        install)     install_extension "$ext_name" "$ext_yaml" ;;
        configure)   configure_extension "$ext_name" "$ext_yaml" ;;
        validate)    validate_extension "$ext_name" "$ext_yaml" ;;
        remove)      remove_extension "$ext_name" "$ext_yaml" ;;
        status)      status_extension "$ext_name" "$ext_yaml" ;;
        *)
            print_error "Unknown action: $action"
            return 1
            ;;
    esac
}

# Install extension
install_extension() {
    local ext_name="$1"
    local ext_yaml="$2"

    print_header "Installing extension: $ext_name"

    # Check if already installed
    if is_extension_installed "$ext_name" && [[ "${DRY_RUN:-false}" != "true" ]]; then
        print_warning "$ext_name is already installed"
        return 0
    fi

    # Check dependencies
    if [[ "${DRY_RUN:-false}" != "true" ]]; then
        check_dependencies "$ext_name" || return 1
    fi

    # Check requirements
    check_requirements "$ext_name" "$ext_yaml" || return 1

    # Get install method
    local install_method
    install_method=$(load_yaml "$ext_yaml" '.install.method')

    if [[ "${DRY_RUN:-false}" == "true" ]]; then
        print_status "Would install $ext_name using method: $install_method"
        return 0
    fi

    # Execute installation based on method
    case "$install_method" in
        mise)    install_via_mise "$ext_name" "$ext_yaml" ;;
        apt)     install_via_apt "$ext_name" "$ext_yaml" ;;
        binary)  install_via_binary "$ext_name" "$ext_yaml" ;;
        npm)     install_via_npm "$ext_name" "$ext_yaml" ;;
        script)  install_via_script "$ext_name" "$ext_yaml" ;;
        hybrid)  install_hybrid "$ext_name" "$ext_yaml" ;;
        *)
            print_error "Unknown install method: $install_method"
            return 1
            ;;
    esac

    local install_status=$?

    if [[ $install_status -eq 0 ]]; then
        # Configure extension
        configure_extension "$ext_name" "$ext_yaml"

        # Mark as installed
        mark_installed "$ext_name"

        # Generate BOM
        generate_extension_bom "$ext_name"

        print_success "Installed $ext_name"
    else
        print_error "Failed to install $ext_name"
        return 1
    fi

    return 0
}

# Check requirements
check_requirements() {
    local ext_name="$1"
    local ext_yaml="$2"

    # Check disk space
    local required_space
    required_space=$(load_yaml "$ext_yaml" '.requirements.diskSpace' 2>/dev/null || echo "0")

    if [[ -n "$required_space" ]] && [[ "$required_space" != "null" ]] && [[ "$required_space" -gt 0 ]]; then
        [[ "${VERBOSE:-false}" == "true" ]] && print_status "Checking disk space: ${required_space}MB required"
        check_disk_space "$required_space" || return 1
    fi

    # Check DNS for required domains
    local domains
    domains=$(load_yaml "$ext_yaml" '.requirements.domains[]?' 2>/dev/null || true)

    for domain in $domains; do
        [[ "${VERBOSE:-false}" == "true" ]] && print_status "Checking DNS: $domain"
        if ! check_dns "$domain"; then
            print_warning "Cannot resolve domain: $domain"
        fi
    done

    return 0
}

# Install via mise
install_via_mise() {
    local ext_name="$1"
    local ext_yaml="$2"
    local ext_dir
    ext_dir=$(dirname "$ext_yaml")
    local workspace="${WORKSPACE:-/workspace}"

    print_status "Installing $ext_name via mise..."

    # Check if mise is available
    if ! command_exists mise; then
        print_error "mise is not available"
        return 1
    fi

    # Get config file
    local config_file
    config_file=$(load_yaml "$ext_yaml" '.install.mise.configFile')

    if [[ -z "$config_file" ]] || [[ "$config_file" == "null" ]]; then
        print_error "No mise config file specified"
        return 1
    fi

    local config_path="$ext_dir/$config_file"

    if [[ ! -f "$config_path" ]]; then
        print_error "Mise config not found: $config_path"
        return 1
    fi

    # Copy mise config to user's config directory
    local mise_conf_dir="$workspace/.config/mise/conf.d"
    ensure_directory "$mise_conf_dir"
    cp "$config_path" "$mise_conf_dir/${ext_name}.toml" || {
        print_error "Failed to copy mise config to $mise_conf_dir"
        return 1
    }

    # Install tools
    cd "$workspace" || return 1

    # Run mise install (returns 0 even when "all tools are installed")
    if ! mise install 2>&1; then
        print_error "mise install failed"
        return 1
    fi

    # Reshim if needed - handle errors gracefully
    local reshim
    reshim=$(load_yaml "$ext_yaml" '.install.mise.reshimAfterInstall' 2>/dev/null || echo "true")

    if [[ "$reshim" == "true" ]]; then
        # mise reshim can fail if there's nothing to reshim - that's OK
        mise reshim 2>/dev/null || true
    fi

    return 0
}

# Install via apt
install_via_apt() {
    local ext_name="$1"
    local ext_yaml="$2"

    print_status "Installing $ext_name via apt..."

    # This requires root, check first
    if [[ "$USER" != "root" ]]; then
        print_error "apt installation requires root privileges"
        return 1
    fi

    # Add repositories
    local repos_count
    repos_count=$(load_yaml "$ext_yaml" '.install.apt.repositories | length' 2>/dev/null || echo "0")

    if [[ "$repos_count" != "null" ]] && [[ "$repos_count" -gt 0 ]]; then
        for i in $(seq 0 $((repos_count - 1))); do
            local gpg_key sources
            gpg_key=$(load_yaml "$ext_yaml" ".install.apt.repositories[$i].gpgKey")
            sources=$(load_yaml "$ext_yaml" ".install.apt.repositories[$i].sources")

            if [[ -n "$gpg_key" ]] && [[ "$gpg_key" != "null" ]]; then
                curl -fsSL "$gpg_key" | apt-key add -
            fi

            if [[ -n "$sources" ]] && [[ "$sources" != "null" ]]; then
                echo "$sources" >> /etc/apt/sources.list.d/${ext_name}.list
            fi
        done
    fi

    # Install packages
    local packages
    packages=$(load_yaml "$ext_yaml" '.install.apt.packages[]' 2>/dev/null | tr '\n' ' ')

    if [[ -n "$packages" ]] && [[ "$packages" != "null" ]]; then
        print_status "Installing packages: $packages"
        apt-get update -qq
        # shellcheck disable=SC2086
        apt-get install -y $packages
    fi

    return 0
}

# Install via binary download
install_via_binary() {
    local ext_name="$1"
    local ext_yaml="$2"

    print_status "Installing $ext_name via binary download..."

    # Parse downloads
    local downloads_count
    downloads_count=$(load_yaml "$ext_yaml" '.install.binary.downloads | length' 2>/dev/null || echo "0")

    if [[ "$downloads_count" == "null" ]] || [[ "$downloads_count" -eq 0 ]]; then
        print_error "No binary downloads specified"
        return 1
    fi

    ensure_directory "${WORKSPACE:-/workspace}/bin"

    # Download each binary
    for i in $(seq 0 $((downloads_count - 1))); do
        local name url destination extract
        name=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].name")
        url=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].source.url")
        destination=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].destination" 2>/dev/null || echo "${WORKSPACE:-/workspace}/bin")
        extract=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].extract" 2>/dev/null || echo "false")

        print_status "Downloading $name..."

        ensure_directory "$destination"

        local temp_file="/tmp/${name}.download"
        curl -fsSL -o "$temp_file" "$url" || return 1

        if [[ "$extract" == "true" ]]; then
            tar -xzf "$temp_file" -C "$destination"
        else
            mv "$temp_file" "$destination/$name"
            chmod +x "$destination/$name"
        fi
    done

    return 0
}

# Install via npm
install_via_npm() {
    local ext_name="$1"
    local ext_yaml="$2"

    print_status "Installing $ext_name via npm..."

    # Check npm is available
    if ! command_exists npm; then
        print_error "npm is not available (install nodejs extension first)"
        return 1
    fi

    # Get packages
    local packages
    packages=$(load_yaml "$ext_yaml" '.install.npm.packages[]' 2>/dev/null | tr '\n' ' ')

    if [[ -n "$packages" ]] && [[ "$packages" != "null" ]]; then
        print_status "Installing npm packages: $packages"
        # shellcheck disable=SC2086
        npm install -g $packages
    fi

    return 0
}

# Install via script
install_via_script() {
    local ext_name="$1"
    local ext_yaml="$2"
    local ext_dir
    ext_dir=$(dirname "$ext_yaml")

    print_status "Installing $ext_name via script..."

    local script_path
    script_path=$(load_yaml "$ext_yaml" '.install.script.path')

    if [[ -z "$script_path" ]] || [[ "$script_path" == "null" ]]; then
        print_error "No script path specified"
        return 1
    fi

    local full_script_path="$ext_dir/$script_path"

    if [[ ! -f "$full_script_path" ]]; then
        print_error "Script not found: $full_script_path"
        return 1
    fi

    # Execute script
    bash "$full_script_path"
}

# Install hybrid
install_hybrid() {
    local ext_name="$1"
    local ext_yaml="$2"

    print_status "Installing $ext_name via hybrid method..."

    # Get steps count
    local steps_count
    steps_count=$(load_yaml "$ext_yaml" '.install.steps | length' 2>/dev/null || echo "0")

    if [[ "$steps_count" == "null" ]] || [[ "$steps_count" -eq 0 ]]; then
        print_error "No installation steps specified"
        return 1
    fi

    # Execute each step
    for i in $(seq 0 $((steps_count - 1))); do
        local method
        method=$(load_yaml "$ext_yaml" ".install.steps[$i].method")

        case "$method" in
            mise)    install_via_mise "$ext_name" "$ext_yaml" ;;
            apt)     install_via_apt "$ext_name" "$ext_yaml" ;;
            binary)  install_via_binary "$ext_name" "$ext_yaml" ;;
            npm)     install_via_npm "$ext_name" "$ext_yaml" ;;
            script)  install_via_script "$ext_name" "$ext_yaml" ;;
            *)
                print_error "Unknown method in hybrid: $method"
                return 1
                ;;
        esac
    done

    return 0
}

# Configure extension
configure_extension() {
    local ext_name="$1"
    local ext_yaml="$2"
    local ext_dir
    ext_dir=$(dirname "$ext_yaml")
    local workspace="${WORKSPACE:-/workspace}"

    [[ "${VERBOSE:-false}" == "true" ]] && print_status "Configuring $ext_name..."

    # Process templates
    local templates_count
    templates_count=$(load_yaml "$ext_yaml" '.configure.templates | length' 2>/dev/null || echo "0")

    if [[ "$templates_count" != "null" ]] && [[ "$templates_count" -gt 0 ]]; then
        for i in $(seq 0 $((templates_count - 1))); do
            local source dest mode
            source=$(load_yaml "$ext_yaml" ".configure.templates[$i].source")
            dest=$(load_yaml "$ext_yaml" ".configure.templates[$i].destination")
            mode=$(load_yaml "$ext_yaml" ".configure.templates[$i].mode" 2>/dev/null || echo "overwrite")

            local source_path="$ext_dir/$source"

            # Expand home directory
            dest="${dest/#\~/$workspace}"

            # Ensure destination directory exists
            ensure_directory "$(dirname "$dest")"

            case "$mode" in
                overwrite)
                    cp "$source_path" "$dest"
                    ;;
                append)
                    cat "$source_path" >> "$dest"
                    ;;
                *)
                    print_warning "Unknown template mode: $mode, using overwrite"
                    cp "$source_path" "$dest"
                    ;;
            esac
        done
    fi

    # Set environment variables
    local env_count
    env_count=$(load_yaml "$ext_yaml" '.configure.environment | length' 2>/dev/null || echo "0")

    if [[ "$env_count" != "null" ]] && [[ "$env_count" -gt 0 ]]; then
        local bashrc_file="$workspace/.bashrc"

        # Ensure .bashrc exists
        if [[ ! -f "$bashrc_file" ]]; then
            touch "$bashrc_file" 2>/dev/null || {
                print_warning "Cannot create $bashrc_file - skipping environment configuration"
                return 0
            }
        fi

        for i in $(seq 0 $((env_count - 1))); do
            local key value scope
            key=$(load_yaml "$ext_yaml" ".configure.environment[$i].key")
            value=$(load_yaml "$ext_yaml" ".configure.environment[$i].value")
            scope=$(load_yaml "$ext_yaml" ".configure.environment[$i].scope" 2>/dev/null || echo "bashrc")

            if [[ "$scope" == "bashrc" ]]; then
                # Only add if not already present
                if ! grep -q "^export ${key}=" "$bashrc_file" 2>/dev/null; then
                    echo "export ${key}=\"${value}\"" >> "$bashrc_file"
                fi
            fi
        done
    fi

    return 0
}

# Validate extension
validate_extension() {
    local ext_name="$1"
    local ext_yaml="$2"

    print_status "Validating $ext_name..."

    # Validate commands exist
    local commands
    commands=$(load_yaml "$ext_yaml" '.validate.commands[].name' 2>/dev/null || true)

    local all_valid=true
    for cmd in $commands; do
        if command_exists "$cmd"; then
            [[ "${VERBOSE:-false}" == "true" ]] && print_success "✓ $cmd found"
        else
            print_error "✗ $cmd not found"
            all_valid=false
        fi
    done

    if [[ "$all_valid" == "true" ]]; then
        print_success "$ext_name validation passed"
        return 0
    else
        print_error "$ext_name validation failed"
        return 1
    fi
}

# Remove extension
remove_extension() {
    local ext_name="$1"
    local ext_yaml="$2"

    print_header "Removing extension: $ext_name"

    # Check if needs confirmation
    local needs_confirmation
    needs_confirmation=$(load_yaml "$ext_yaml" '.remove.confirmation' 2>/dev/null || echo "true")

    if [[ "$needs_confirmation" == "true" ]] && [[ "${DRY_RUN:-false}" != "true" ]]; then
        read -p "Remove $ext_name? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_status "Cancelled"
            return 0
        fi
    fi

    if [[ "${DRY_RUN:-false}" == "true" ]]; then
        print_status "Would remove $ext_name"
        return 0
    fi

    # Remove mise config
    local has_mise_removal
    has_mise_removal=$(load_yaml "$ext_yaml" '.remove.mise' 2>/dev/null || echo "null")

    if [[ "$has_mise_removal" != "null" ]]; then
        rm -f "${WORKSPACE:-/workspace}/.config/mise/conf.d/${ext_name}.toml"
    fi

    # Remove paths
    local paths
    paths=$(load_yaml "$ext_yaml" '.remove.paths[]' 2>/dev/null || true)

    for path in $paths; do
        # Expand home directory
        path="${path/#\~/${WORKSPACE:-/workspace}}"
        if [[ -e "$path" ]]; then
            rm -rf "$path"
        fi
    done

    # Mark as uninstalled
    mark_uninstalled "$ext_name"

    print_success "Removed $ext_name"
    return 0
}

# Show extension status
status_extension() {
    local ext_name="$1"
    local ext_yaml="$2"

    echo "Extension: $ext_name"

    if is_extension_installed "$ext_name"; then
        echo "Status: Installed"
    else
        echo "Status: Not installed"
    fi

    # Show version if available
    local commands
    commands=$(load_yaml "$ext_yaml" '.validate.commands[].name' 2>/dev/null || true)

    for cmd in $commands; do
        if command_exists "$cmd"; then
            local version
            version=$($cmd --version 2>&1 | head -1 || echo "unknown")
            echo "  $cmd: $version"
        fi
    done
}

# Export functions
export -f execute_extension install_extension configure_extension
export -f validate_extension remove_extension status_extension
export -f check_requirements install_via_mise install_via_apt
export -f install_via_binary install_via_npm install_via_script
export -f install_hybrid