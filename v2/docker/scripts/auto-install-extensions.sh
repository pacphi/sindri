#!/bin/bash
# ==============================================================================
# Auto-Install Extensions
# ==============================================================================
# Automatically installs extensions based on environment configuration during
# container bootstrap. Called by entrypoint.sh on first boot.
#
# Environment Variables:
#   SKIP_AUTO_INSTALL     - Set to "true" to disable auto-install (default: false)
#   INSTALL_PROFILE       - Profile name (e.g., "fullstack", "minimal")
#   CUSTOM_EXTENSIONS     - Comma-separated list of extensions
#   ADDITIONAL_EXTENSIONS - Extensions to add on top of profile (future)
#
# Creates: $WORKSPACE/.system/bootstrap.yaml (prevents re-run on restart)
#
# Use Cases:
#   - CI testing: Set SKIP_AUTO_INSTALL=true to test manual extension installation
#   - Manual control: Users who prefer to install extensions themselves
#   - Debugging: Skip auto-install to isolate extension issues
# ==============================================================================

set -e

# Source common functions if not already sourced
if ! type print_status &>/dev/null; then
    if [[ -f "/docker/lib/common.sh" ]]; then
        source /docker/lib/common.sh
    else
        # Fallback print functions
        print_status() { echo "[INFO] $*"; }
        print_success() { echo "[OK] $*"; }
        print_warning() { echo "[WARN] $*"; }
        print_error() { echo "[ERROR] $*"; }
    fi
fi

# ------------------------------------------------------------------------------
# install_extensions - Auto-install extensions based on environment configuration
# ------------------------------------------------------------------------------
# Installs extensions on first boot based on INSTALL_PROFILE and CUSTOM_EXTENSIONS.
# Creates bootstrap.yaml marker to prevent re-installation on container restarts.
#
# Configuration Cases:
#   Case A: No profile/extensions       -> install 'minimal' profile as fallback
#   Case B: Profile only               -> extension-manager install-profile $PROFILE
#   Case C: Profile + additional       -> install profile, then install additional
#   Case D: Custom extensions only     -> install each extension from CUSTOM_EXTENSIONS
# ------------------------------------------------------------------------------
install_extensions() {
    local bootstrap_marker="${WORKSPACE}/.system/bootstrap.yaml"

    # Debug logging for CI troubleshooting
    print_status "Auto-install configuration:"
    print_status "  SKIP_AUTO_INSTALL='${SKIP_AUTO_INSTALL:-<not set>}'"
    print_status "  INSTALL_PROFILE='${INSTALL_PROFILE:-<not set>}'"
    print_status "  CUSTOM_EXTENSIONS='${CUSTOM_EXTENSIONS:-<not set>}'"

    # Skip if auto-install is disabled
    if [[ "${SKIP_AUTO_INSTALL:-false}" == "true" ]]; then
        print_status "Auto-install disabled (SKIP_AUTO_INSTALL=true)"
        print_status "Extensions can be installed manually with: extension-manager install-profile <profile>"
        return 0
    fi

    # Skip if already bootstrapped
    if [[ -f "$bootstrap_marker" ]]; then
        print_status "Extensions already installed (bootstrap marker exists)"
        return 0
    fi

    print_status "Auto-installing extensions..."

    # Normalize environment variables (yq returns "null" for missing keys)
    local profile="${INSTALL_PROFILE:-}"
    local custom_exts="${CUSTOM_EXTENSIONS:-}"
    local additional_exts="${ADDITIONAL_EXTENSIONS:-}"

    # Treat "null" or empty as unset
    [[ "$profile" == "null" || -z "$profile" ]] && profile=""
    [[ "$custom_exts" == "null" || -z "$custom_exts" ]] && custom_exts=""
    [[ "$additional_exts" == "null" || -z "$additional_exts" ]] && additional_exts=""

    # Determine installation strategy
    local install_mode=""
    if [[ -n "$custom_exts" && -z "$profile" ]]; then
        # Case D: Custom extensions only (no profile)
        install_mode="custom"
    elif [[ -n "$profile" && -n "$additional_exts" ]]; then
        # Case C: Profile + additional extensions
        install_mode="profile_plus_additional"
    elif [[ -n "$profile" ]]; then
        # Case B: Profile only
        install_mode="profile"
    else
        # Case A: No profile/extensions -> fallback to minimal
        install_mode="profile"
        profile="minimal"
        print_status "No profile or extensions specified, using 'minimal' fallback"
    fi

    # Export environment variables for sudo to preserve
    # sudo --preserve-env requires variables to be exported first
    export HOME="${ALT_HOME}"
    export PATH="${ALT_HOME}/.local/share/mise/shims:/docker/cli:${ALT_HOME}/workspace/bin:/usr/local/bin:${PATH}"
    export WORKSPACE="${WORKSPACE}"
    export ALT_HOME="${ALT_HOME}"
    export DOCKER_LIB="/docker/lib"
    export MISE_DATA_DIR="${ALT_HOME}/.local/share/mise"
    export MISE_CONFIG_DIR="${ALT_HOME}/.config/mise"
    export MISE_CACHE_DIR="${ALT_HOME}/.cache/mise"
    export MISE_STATE_DIR="${ALT_HOME}/.local/state/mise"

    # Build preserve list dynamically from environment (prevents staleness)
    # Auto-discovers all relevant variables instead of hardcoding
    local preserve_list="HOME,PATH,WORKSPACE,ALT_HOME,DOCKER_LIB"

    # Add all SINDRI_* and MISE_* variables
    local tool_vars=$(env | grep -E '^(SINDRI_|MISE_)' | cut -d= -f1 | tr '\n' ',' | sed 's/,$//')
    [[ -n "$tool_vars" ]] && preserve_list="${preserve_list},${tool_vars}"

    # Add all GIT_* variables
    local git_vars=$(env | grep -E '^GIT_' | cut -d= -f1 | tr '\n' ',' | sed 's/,$//')
    [[ -n "$git_vars" ]] && preserve_list="${preserve_list},${git_vars}"

    # Add all credential/secret variables (comprehensive pattern)
    # Matches: *_TOKEN, *_API_KEY, *_KEY, *_KEYS, *_PASSWORD, *_PASS, *_USERNAME, *_USER, *_URL, *_SECRET
    local credential_vars=$(env | grep -E '_(TOKEN|API_KEY|KEY|KEYS|PASSWORD|PASS|USERNAME|USER|URL|SECRET)$' | cut -d= -f1 | tr '\n' ',' | sed 's/,$//')
    [[ -n "$credential_vars" ]] && preserve_list="${preserve_list},${credential_vars}"

    local env_vars="$preserve_list"

    local install_success=true
    local installed_profile=""
    local installed_extensions=""

    # Execute installation based on mode
    case "$install_mode" in
        profile)
            print_status "Installing profile: $profile"
            if sudo -u "$DEVELOPER_USER" --preserve-env="${env_vars}" \
                /docker/cli/extension-manager install-profile "$profile"; then
                installed_profile="$profile"
                print_success "Profile '$profile' installed successfully"
            else
                print_error "Failed to install profile: $profile"
                install_success=false
            fi
            ;;

        profile_plus_additional)
            # Install profile first
            print_status "Installing profile: $profile"
            if sudo -u "$DEVELOPER_USER" --preserve-env="${env_vars}" \
                /docker/cli/extension-manager install-profile "$profile"; then
                installed_profile="$profile"
                print_success "Profile '$profile' installed successfully"
            else
                print_error "Failed to install profile: $profile"
                install_success=false
            fi

            # Install additional extensions if profile succeeded
            if [[ "$install_success" == "true" && -n "$additional_exts" ]]; then
                print_status "Installing additional extensions: $additional_exts"
                # Convert comma-separated to space-separated for iteration
                local ext_list="${additional_exts//,/ }"
                local added_exts=""
                for ext in $ext_list; do
                    ext=$(echo "$ext" | xargs)  # trim whitespace
                    [[ -z "$ext" ]] && continue
                    print_status "Installing extension: $ext"
                    if sudo -u "$DEVELOPER_USER" --preserve-env="${env_vars}" \
                        /docker/cli/extension-manager install "$ext"; then
                        added_exts="${added_exts:+$added_exts,}$ext"
                    else
                        print_warning "Failed to install additional extension: $ext"
                    fi
                done
                installed_extensions="$added_exts"
            fi
            ;;

        custom)
            print_status "Installing custom extensions: $custom_exts"
            # Convert comma-separated to space-separated for iteration
            local ext_list="${custom_exts//,/ }"
            local installed_list=""
            for ext in $ext_list; do
                ext=$(echo "$ext" | xargs)  # trim whitespace
                [[ -z "$ext" ]] && continue
                print_status "Installing extension: $ext"
                if sudo -u "$DEVELOPER_USER" --preserve-env="${env_vars}" \
                    /docker/cli/extension-manager install "$ext"; then
                    installed_list="${installed_list:+$installed_list,}$ext"
                else
                    print_warning "Failed to install extension: $ext"
                    # Continue with other extensions
                fi
            done
            installed_extensions="$installed_list"
            ;;

        *)
            print_error "Unknown install mode: $install_mode"
            install_success=false
            ;;
    esac

    # Create bootstrap marker with configuration snapshot
    print_status "Creating bootstrap marker..."

    local generated_time current_hostname
    generated_time=$(date -u +"%Y-%m-%dT%H:%M:%SZ") || true
    current_hostname=$(hostname) || true

    cat > "$bootstrap_marker" << EOF
# Sindri Bootstrap Configuration
# Auto-generated on first container boot - do not edit
# Delete this file to force re-installation on next restart

version: "1.0"
generated: ${generated_time}
hostname: ${current_hostname}

# Configuration at bootstrap time
config:
  install_mode: ${install_mode}
  profile: ${installed_profile:-null}
  custom_extensions: ${custom_exts:-null}
  additional_extensions: ${additional_exts:-null}

# Actual installation results
installed:
  profile: ${installed_profile:-null}
  extensions: ${installed_extensions:-null}
  success: ${install_success}

# Environment variables at runtime
environment:
  SKIP_AUTO_INSTALL: ${SKIP_AUTO_INSTALL:-false}
  INSTALL_PROFILE: ${INSTALL_PROFILE:-null}
  CUSTOM_EXTENSIONS: ${CUSTOM_EXTENSIONS:-null}
  ADDITIONAL_EXTENSIONS: ${ADDITIONAL_EXTENSIONS:-null}
EOF

    # Set ownership to developer user
    chown "${DEVELOPER_USER}:${DEVELOPER_USER}" "$bootstrap_marker"

    if [[ "$install_success" == "true" ]]; then
        print_success "Extension installation complete"
    else
        print_warning "Extension installation completed with errors"
    fi

    return 0
}

# Run if called directly (for testing)
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_extensions
fi
