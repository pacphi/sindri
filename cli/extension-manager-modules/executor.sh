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
    local home_dir="${HOME:-/alt/home/developer}"
    local workspace="${WORKSPACE:-$home_dir/workspace}"

    print_status "Installing $ext_name via mise (this may take 1-5 minutes)..."

    # Progress indicator: Step 1 - Verify mise availability
    if [[ "${SINDRI_ENABLE_PROGRESS_INDICATORS:-true}" == "true" ]]; then
        echo "  [1/4] Verifying mise availability..."
    fi
    if ! command_exists mise; then
        print_error "mise is not available"
        return 1
    fi

    # Progress indicator: Step 2 - Load and verify configuration
    if [[ "${SINDRI_ENABLE_PROGRESS_INDICATORS:-true}" == "true" ]]; then
        echo "  [2/4] Loading mise configuration..."
    fi
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

    # Copy mise config to user's XDG config directory
    local mise_conf_dir="${MISE_CONFIG_DIR:-$home_dir/.config/mise}/conf.d"
    ensure_directory "$mise_conf_dir"
    cp "$config_path" "$mise_conf_dir/${ext_name}.toml" || {
        print_error "Failed to copy mise config to $mise_conf_dir"
        return 1
    }
    [[ "${SINDRI_ENABLE_PROGRESS_INDICATORS:-true}" == "true" ]] && echo "     Configuration copied to $mise_conf_dir"

    # Install tools
    cd "$workspace" || return 1

    # Progress indicator: Step 3 - Install tools
    if [[ "${SINDRI_ENABLE_PROGRESS_INDICATORS:-true}" == "true" ]]; then
        echo "  [3/4] Installing tools with mise (timeout: 5min, 3 retries if needed)..."
        echo "     This step may take several minutes for large tools like Node.js or Python"
    fi

    # Scope mise install to THIS extension's config only (not all conf.d files)
    # This prevents failures in unrelated extensions from breaking this install
    local scoped_config="$mise_conf_dir/${ext_name}.toml"

    # Ensure mise shims and installed tools are in PATH for npm backend
    # This fixes "node not found" errors when installing npm: packages
    # See: https://mise.jdx.dev/troubleshooting.html
    local mise_shims="${home_dir}/.local/share/mise/shims"
    if [[ -d "$mise_shims" ]] && [[ ":$PATH:" != *":$mise_shims:"* ]]; then
        export PATH="$mise_shims:$PATH"
    fi
    # Also add node install path directly as fallback for npm wrapper scripts
    local node_installs="${home_dir}/.local/share/mise/installs/node"
    if [[ -d "$node_installs" ]]; then
        local node_path
        node_path=$(find "$node_installs" -maxdepth 2 -name "bin" -type d 2>/dev/null | head -1 || true)
        if [[ -n "$node_path" ]] && [[ ":$PATH:" != *":$node_path:"* ]]; then
            export PATH="$node_path:$PATH"
        fi
    fi
    # Refresh command hash table
    hash -r 2>/dev/null || true

    # Run mise install with timeout and retry logic
    # Use MISE_CONFIG_FILE to scope to this extension only
    if ! MISE_CONFIG_FILE="$scoped_config" timeout 300 mise install 2>&1 | while IFS= read -r line; do
        # Indent mise output for better readability
        if [[ "${SINDRI_ENABLE_PROGRESS_INDICATORS:-true}" == "true" ]]; then
            echo "     $line"
        else
            echo "$line"
        fi
    done; then
        print_warning "mise install failed or timed out, retrying with exponential backoff..."
        # Use existing retry_command from common.sh (3 attempts, 2s initial delay)
        if ! retry_command 3 2 MISE_CONFIG_FILE="$scoped_config" timeout 300 mise install; then
            print_error "mise install failed after 3 attempts (total: 15min max)"
            return 1
        fi
    fi

    # Progress indicator: Step 4 - Reshim (always run to ensure shims exist)
    if [[ "${SINDRI_ENABLE_PROGRESS_INDICATORS:-true}" == "true" ]]; then
        echo "  [4/4] Running mise reshim to update shims..."
    fi
    # Always reshim to ensure installed tools have shims created
    # This fixes issues where tools install but shims don't exist
    mise reshim 2>/dev/null || true
    # Clear bash's command hash table so new commands are found
    hash -r 2>/dev/null || true

    print_success "$ext_name installation via mise completed successfully"

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
                echo "$sources" >> "/etc/apt/sources.list.d/${ext_name}.list"
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
    local home_dir="${HOME:-/alt/home/developer}"
    local workspace="${WORKSPACE:-$home_dir/workspace}"

    print_status "Installing $ext_name via binary download..."

    # Parse downloads
    local downloads_count
    downloads_count=$(load_yaml "$ext_yaml" '.install.binary.downloads | length' 2>/dev/null || echo "0")

    if [[ "$downloads_count" == "null" ]] || [[ "$downloads_count" -eq 0 ]]; then
        print_error "No binary downloads specified"
        return 1
    fi

    ensure_directory "$workspace/bin"

    # Download each binary
    for i in $(seq 0 $((downloads_count - 1))); do
        local name url destination extract
        name=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].name")
        url=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].source.url")
        destination=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].destination" 2>/dev/null || echo "$workspace/bin")
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
    local home_dir="${HOME:-/alt/home/developer}"
    local workspace="${WORKSPACE:-$home_dir/workspace}"

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

            # Expand home directory (~ means $HOME, not $WORKSPACE)
            dest="${dest/#\~/$home_dir}"

            # Ensure destination directory exists
            ensure_directory "$(dirname "$dest")"

            case "$mode" in
                overwrite)
                    cp "$source_path" "$dest"
                    ;;
                append)
                    cat "$source_path" >> "$dest"
                    ;;
                skip-if-exists)
                    if [[ ! -f "$dest" ]]; then
                        cp "$source_path" "$dest"
                    fi
                    ;;
                merge)
                    # Merge YAML/JSON files using yq if available, otherwise overwrite
                    if command_exists yq && [[ "$dest" =~ \.(yaml|yml|json)$ ]]; then
                        if [[ -f "$dest" ]]; then
                            yq eval-all 'select(fileIndex == 0) * select(fileIndex == 1)' "$dest" "$source_path" > "${dest}.tmp" && mv "${dest}.tmp" "$dest"
                        else
                            cp "$source_path" "$dest"
                        fi
                    else
                        cp "$source_path" "$dest"
                    fi
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
        # .bashrc lives in $HOME, not $WORKSPACE
        local bashrc_file="$home_dir/.bashrc"

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

    # Get validation timeout from extension or use default
    local validation_timeout
    validation_timeout=$(load_yaml "$ext_yaml" '.requirements.validationTimeout' 2>/dev/null || echo "null")
    # Handle null from yq (same issue as autoInstall bug)
    if [[ "$validation_timeout" == "null" ]]; then
        validation_timeout="${SINDRI_VALIDATION_TIMEOUT:-10}"
    fi

    # Get list of validation commands
    local num_commands
    num_commands=$(load_yaml "$ext_yaml" '.validate.commands | length' 2>/dev/null || echo "0")

    if [[ "$num_commands" == "0" ]]; then
        [[ "${VERBOSE:-false}" == "true" ]] && print_warning "No validation commands defined for $ext_name"
        return 0
    fi

    local all_valid=true
    local idx=0

    while [[ $idx -lt $num_commands ]]; do
        local cmd
        local expected_pattern
        cmd=$(load_yaml "$ext_yaml" ".validate.commands[$idx].name" 2>/dev/null || true)
        expected_pattern=$(load_yaml "$ext_yaml" ".validate.commands[$idx].expectedPattern" 2>/dev/null || true)

        if [[ -z "$cmd" ]]; then
            idx=$((idx + 1))
            continue
        fi

        # Check if command exists first
        if ! command_exists "$cmd"; then
            print_error "✗ $cmd not found"
            all_valid=false
            idx=$((idx + 1))
            continue
        fi

        # Execute command with timeout and validate output if pattern provided
        local output
        local exit_code
        if output=$(timeout "$validation_timeout" "$cmd" --version 2>&1); then
            exit_code=0
        else
            exit_code=$?
        fi

        # Check if timeout occurred (exit code 124)
        if [[ $exit_code -eq 124 ]]; then
            print_error "✗ $cmd validation timed out after ${validation_timeout}s"
            all_valid=false
        elif [[ $exit_code -ne 0 ]]; then
            print_error "✗ $cmd execution failed (exit code: $exit_code)"
            all_valid=false
        elif [[ -n "$expected_pattern" ]] && [[ "$expected_pattern" != "null" ]]; then
            # Validate output against expected pattern if provided
            # Use grep -P for Perl regex support (\d, \s, etc.)
            if echo "$output" | grep -qP "$expected_pattern"; then
                [[ "${VERBOSE:-false}" == "true" ]] && print_success "✓ $cmd validated (pattern matched)"
            else
                print_error "✗ $cmd output doesn't match expected pattern: $expected_pattern"
                [[ "${VERBOSE:-false}" == "true" ]] && echo "  Output: $output"
                all_valid=false
            fi
        else
            # No pattern check, just verify command runs
            [[ "${VERBOSE:-false}" == "true" ]] && print_success "✓ $cmd found and executable"
        fi

        idx=$((idx + 1))
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
    local home_dir="${HOME:-/alt/home/developer}"

    print_header "Removing extension: $ext_name"

    # Check if needs confirmation
    local needs_confirmation
    needs_confirmation=$(load_yaml "$ext_yaml" '.remove.confirmation' 2>/dev/null || echo "true")

    # Skip confirmation if --force is set or in dry-run mode
    if [[ "$needs_confirmation" == "true" ]] && [[ "${DRY_RUN:-false}" != "true" ]] && [[ "${FORCE_MODE:-false}" != "true" ]]; then
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

    # Remove mise config (from XDG config dir, not workspace)
    local has_mise_removal
    has_mise_removal=$(load_yaml "$ext_yaml" '.remove.mise' 2>/dev/null || echo "null")

    if [[ "$has_mise_removal" != "null" ]]; then
        rm -f "${MISE_CONFIG_DIR:-$home_dir/.config/mise}/conf.d/${ext_name}.toml"
    fi

    # Remove paths
    local paths
    paths=$(load_yaml "$ext_yaml" '.remove.paths[]' 2>/dev/null || true)

    for path in $paths; do
        # Expand home directory (~ means $HOME, not $WORKSPACE)
        path="${path/#\~/$home_dir}"
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