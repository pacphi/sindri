#!/bin/bash
# extensions-common.sh - Shared utilities for extension scripts
# This library provides common functions used across all extension .example files

# Prevent multiple sourcing
if [[ "${EXTENSIONS_COMMON_SH_LOADED:-}" == "true" ]]; then
    return 0
fi
EXTENSIONS_COMMON_SH_LOADED="true"

# ============================================================================
# CONSTANTS
# ============================================================================

# Protected extensions that cannot be removed or reordered
export PROTECTED_EXTENSIONS="workspace-structure mise-config ssh-environment"

# ============================================================================
# EXTENSION INITIALIZATION
# ============================================================================

# Initialize extension environment and load common utilities
# This function replaces the COMMON UTILITIES section in each extension
extension_init() {
    # Calculate script and library directories
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[1]}")" && pwd)"
    LIB_DIR="$(dirname "$SCRIPT_DIR")"

    # Try to source common.sh from known locations
    if [[ -f "$LIB_DIR/common.sh" ]]; then
        source "$LIB_DIR/common.sh"
    elif [[ -f "/workspace/scripts/lib/common.sh" ]]; then
        source "/workspace/scripts/lib/common.sh"
    else
        # Fallback: define minimal required functions (invoked indirectly by extension scripts)
        # shellcheck disable=SC2329
        print_status() { echo "[INFO] $1"; }
        # shellcheck disable=SC2329
        print_success() { echo "[SUCCESS] $1"; }
        # shellcheck disable=SC2329
        print_error() { echo "[ERROR] $1" >&2; }
        # shellcheck disable=SC2329
        print_warning() { echo "[WARNING] $1"; }
        # shellcheck disable=SC2329
        print_debug() { [[ "${DEBUG:-}" == "true" ]] && echo "[DEBUG] $1"; }
        # shellcheck disable=SC2329
        command_exists() { command -v "$1" >/dev/null 2>&1; }
    fi

    # Source registry retry helpers if available
    local registry_retry="$LIB_DIR/registry-retry.sh"
    if [[ -f "$registry_retry" ]]; then
        source "$registry_retry"
    fi

    # Export for use by extension functions
    export SCRIPT_DIR LIB_DIR
}

# ============================================================================
# DEPENDENCY CHECKING
# ============================================================================

# Check which active extensions depend on commands provided by this extension
# Usage: check_dependent_extensions "command1" "command2" ...
# Returns: List of extension names that reference any of the provided commands
check_dependent_extensions() {
    local provided_commands=("$@")
    local dependent_extensions=()

    # Get manifest file location
    local manifest_file="$SCRIPT_DIR/active-extensions.conf"
    [[ ! -f "$manifest_file" ]] && manifest_file="/workspace/scripts/lib/extensions.d/active-extensions.conf"

    if [[ ! -f "$manifest_file" ]]; then
        return 0
    fi

    # Read active extensions from manifest
    while IFS= read -r line; do
        # Skip comments and empty lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue

        # Extract extension name
        local ext_name=$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
        [[ "$ext_name" == "${EXT_NAME:-}" ]] && continue

        # Find the extension file
        local ext_file="$SCRIPT_DIR/${ext_name}.sh"
        if [[ ! -f "$ext_file" ]]; then
            local -a matches=("$SCRIPT_DIR/"*"-${ext_name}.sh")
            [[ ${#matches[@]} -gt 0 ]] && ext_file="${matches[0]}"
        fi
        [[ ! -f "$ext_file" ]] && continue

        # Check if extension references any of the provided commands
        for cmd in "${provided_commands[@]}"; do
            if grep -q "$cmd" "$ext_file" 2>/dev/null; then
                dependent_extensions+=("$ext_name")
                break
            fi
        done
    done < "$manifest_file"

    printf '%s\n' "${dependent_extensions[@]}"
}

# ============================================================================
# ENVIRONMENT HELPERS
# ============================================================================

# Check if running in CI mode
# Usage: if is_ci_mode; then ... fi
# Returns: 0 if CI_MODE=true, 1 otherwise
is_ci_mode() {
    [[ "${CI_MODE:-false}" == "true" ]]
}

# Activate mise environment in current shell
# Usage: activate_mise_environment
# This function evaluates mise activation and adds shims to PATH
activate_mise_environment() {
    if command_exists mise; then
        # Activate mise with error capture
        local activation_output
        if activation_output=$(mise activate bash 2>&1); then
            eval "$activation_output"
            print_debug "mise activated successfully"
        else
            print_warning "mise activation returned non-zero: $activation_output"
            # Continue anyway as activation might still work
        fi

        # Ensure shims directory is in PATH
        if [[ -d "$HOME/.local/share/mise/shims" ]]; then
            # Only add if not already in PATH
            if [[ ":$PATH:" != *":$HOME/.local/share/mise/shims:"* ]]; then
                export PATH="$HOME/.local/share/mise/shims:$PATH"
                print_debug "Added mise shims to PATH"
            fi
        fi

        # Also add installs directories for tools that don't use shims
        if [[ -d "$HOME/.local/share/mise/installs" ]]; then
            for bin_dir in "$HOME/.local/share/mise/installs/"*/*/bin; do
                if [[ -d "$bin_dir" ]] && [[ ":$PATH:" != *":$bin_dir:"* ]]; then
                    export PATH="$bin_dir:$PATH"
                fi
            done
        fi
    else
        print_debug "mise not available - skipping activation"
    fi
}

# ============================================================================
# PREREQUISITE CHECKS
# ============================================================================

# Check if mise is available
# Usage: check_mise_prerequisite || return 1
# Returns: 0 if mise exists, 1 with error message otherwise
check_mise_prerequisite() {
    if command_exists mise; then
        return 0
    fi

    print_error "mise is required but not installed"
    print_status "Install mise-config extension first: extension-manager install mise-config"
    return 1
}

# Check available disk space
# Usage: check_disk_space [required_mb]
# Default: 600MB required
# Returns: 0 if sufficient space, 1 with warning otherwise
check_disk_space() {
    local required_mb="${1:-600}"
    local available_mb
    available_mb=$(df -BM / | awk 'NR==2 {print $4}' | sed 's/M//')

    if [[ $available_mb -lt $required_mb ]]; then
        print_warning "Low disk space: ${available_mb}MB available (${required_mb}MB recommended)"
        return 1
    fi
    return 0
}

# ============================================================================
# STATUS HELPERS
# ============================================================================

# Print standard extension header
# Usage: print_extension_header
# Requires: EXT_NAME, EXT_VERSION, EXT_DESCRIPTION, EXT_CATEGORY environment variables
print_extension_header() {
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Extension: ${EXT_NAME} v${EXT_VERSION}"
    echo "Description: ${EXT_DESCRIPTION}"
    echo "Category: ${EXT_CATEGORY}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
}

# ============================================================================
# VALIDATION HELPERS
# ============================================================================

# Validate multiple commands with version checks
# Usage:
#   declare -A checks=(
#       [command1]="--version"
#       [command2]="-v"
#   )
#   validate_commands checks
# Returns: 0 if all commands exist, 1 if any are missing
validate_commands() {
    local -n command_checks=$1
    local all_valid=true

    for cmd in "${!command_checks[@]}"; do
        local version_flag="${command_checks[$cmd]}"
        if ! command_exists "$cmd"; then
            print_error "$cmd not found"
            all_valid=false
        else
            local version
            version=$($cmd $version_flag 2>&1 | head -n1)
            print_success "$cmd: $version"
        fi
    done

    [[ "$all_valid" == "true" ]]
}

# ============================================================================
# MISE HELPERS
# ============================================================================

# Install mise configuration from TOML file
# Usage: install_mise_config "extension-name" ["script_dir"]
# Example: install_mise_config "nodejs"
# This function:
#   - Selects CI or dev TOML based on CI_MODE
#   - Copies TOML to mise config directory
#   - Runs mise install
# Returns: 0 on success, 1 on failure
install_mise_config() {
    local ext_name="$1"
    local script_dir="${2:-$SCRIPT_DIR}"

    # Determine which TOML to use (CI vs dev)
    local toml_suffix=""
    if is_ci_mode; then
        toml_suffix="-ci"
        print_status "CI mode detected - using minimal configuration"
    else
        print_status "Using full configuration"
    fi

    # Try to find TOML in multiple locations
    local source_toml=""
    local search_paths=(
        "${script_dir}/${ext_name}${toml_suffix}.toml"                    # Activated directory
        "/docker/lib/extensions.d/${ext_name}${toml_suffix}.toml"         # Source directory
    )

    for path in "${search_paths[@]}"; do
        if [[ -f "$path" ]]; then
            source_toml="$path"
            break
        fi
    done

    # Verify source file exists
    if [[ -z "$source_toml" ]] || [[ ! -f "$source_toml" ]]; then
        print_error "Configuration not found: ${ext_name}${toml_suffix}.toml"
        print_error "Searched in: ${search_paths[*]}"
        return 1
    fi

    print_debug "Using configuration: $source_toml"

    # Ensure mise config directory exists
    mkdir -p "$HOME/.config/mise/conf.d"

    # Copy TOML to mise config
    local dest_toml="$HOME/.config/mise/conf.d/${ext_name}.toml"
    if cp "$source_toml" "$dest_toml"; then
        print_success "Configuration copied to $dest_toml"
    else
        print_error "Failed to copy configuration"
        return 1
    fi

    # Install via mise with timeout and error capture
    print_status "Installing via mise..."
    local install_log="/tmp/mise-install-${ext_name}-$$.log"

    # Run mise install with output capture
    if timeout 600 mise install 2>&1 | tee "$install_log"; then
        local install_exit_code=${PIPESTATUS[0]}

        if [ $install_exit_code -eq 0 ]; then
            print_success "Installed successfully"

            # Verify installation by checking mise ls
            print_status "Verifying installation..."
            if mise ls 2>&1 | grep -q "$ext_name" || mise ls 2>&1 | head -1 | grep -q "node\|python\|go\|rust"; then
                print_success "Installation verified in mise"
            else
                print_warning "Installed but not showing in mise ls - may need environment refresh"
            fi

            rm -f "$install_log"
            return 0
        elif [ $install_exit_code -eq 124 ]; then
            print_error "mise install timed out after 600 seconds"
            echo "Last 30 lines of output:"
            tail -30 "$install_log"
            rm -f "$install_log"
            return 1
        else
            print_error "mise install failed with exit code $install_exit_code"
            echo "Last 30 lines of output:"
            tail -30 "$install_log"
            rm -f "$install_log"
            return 1
        fi
    else
        print_error "mise install command failed"
        rm -f "$install_log"
        return 1
    fi
}

# Remove mise configuration for an extension
# Usage: remove_mise_config "extension-name"
# Example: remove_mise_config "nodejs"
remove_mise_config() {
    local ext_name="$1"
    local config_file="$HOME/.config/mise/conf.d/${ext_name}.toml"

    if [[ -f "$config_file" ]]; then
        print_status "Removing mise configuration..."
        if rm -f "$config_file"; then
            print_success "Removed $config_file"
        fi
    fi

    print_status "Note: Tools are still installed by mise"
    print_status "Run 'mise prune' to remove unused tools"
}

# ============================================================================
# MAIN EXECUTION WRAPPER
# ============================================================================

# Standard main execution wrapper for extensions
# Usage: extension_main "$@"
# This function replaces the MAIN EXECUTION section in each extension
extension_main() {
    # Only execute if script is run directly (not sourced)
    if [[ "${BASH_SOURCE[1]}" == "${0}" ]]; then
        local command="${1:-status}"

        case "$command" in
            prerequisites|install|configure|validate|status|remove|upgrade)
                if "$command"; then
                    exit 0
                else
                    exit 1
                fi
                ;;
            *)
                echo "Usage: $0 {prerequisites|install|configure|validate|status|remove|upgrade}"
                exit 1
                ;;
        esac
    fi
}

# ============================================================================
# CLEANUP HELPERS
# ============================================================================

# Remove extension entries from .bashrc
# Usage: cleanup_bashrc "marker-text"
cleanup_bashrc() {
    local marker="$1"
    local bashrc="$HOME/.bashrc"

    if [[ ! -f "$bashrc" ]]; then
        return 0
    fi

    if grep -q "$marker" "$bashrc" 2>/dev/null; then
        # Calculate number of lines to remove (find the section and remove until next empty line or next comment)
        local start_line=$(grep -n "$marker" "$bashrc" | head -1 | cut -d: -f1)
        if [[ -n "$start_line" ]]; then
            # Simple approach: remove marker line and up to 5 following lines
            # This handles most cases (export statements, aliases, etc.)
            local line_count=1
            local current=$((start_line + 1))
            while [[ $line_count -lt 10 ]]; do
                local line_content=$(sed -n "${current}p" "$bashrc")
                # Stop if we hit an empty line or a new comment block
                if [[ -z "$line_content" ]] || [[ "$line_content" =~ ^[[:space:]]*$ ]]; then
                    break
                fi
                if [[ "$line_content" =~ ^[[:space:]]*#.*Extension ]]; then
                    break
                fi
                ((line_count++))
                ((current++))
            done

            # Use a more precise sed pattern based on the marker
            sed -i "/$marker/,+${line_count}d" "$bashrc" 2>/dev/null || true
            print_success "Removed entries from .bashrc"
        fi
    fi
}

# Setup git aliases for an extension
# Usage: setup_git_aliases "alias1:command1" "alias2:command2" ...
# Example: setup_git_aliases "py-test:!pytest" "py-lint:!ruff check"
setup_git_aliases() {
    local alias_defs=("$@")

    if ! command_exists git; then
        return 0
    fi

    for alias_def in "${alias_defs[@]}"; do
        local alias_name="${alias_def%%:*}"
        local alias_cmd="${alias_def#*:}"
        git config --global "alias.${alias_name}" "$alias_cmd" 2>/dev/null || true
        print_debug "Configured git alias: $alias_name"
    done
}

# Remove git aliases for an extension
# Usage: cleanup_git_aliases "alias1" "alias2" ...
cleanup_git_aliases() {
    local aliases=("$@")

    if ! command_exists git; then
        return 0
    fi

    for alias in "${aliases[@]}"; do
        if git config --global --get-all "alias.${alias}" >/dev/null 2>&1; then
            git config --global --unset-all "alias.${alias}" 2>/dev/null || true
            print_debug "Removed git alias: $alias"
        fi
    done
}

# Standardized confirmation prompt
# Usage: if prompt_confirmation "Question text?"; then ... fi
# Returns: 0 if user answered yes, 1 if no
prompt_confirmation() {
    local prompt="${1:-Continue?}"
    read -p "$prompt (y/N): " -r
    [[ $REPLY =~ ^[Yy]$ ]]
}

# Display list of dependent extensions
# Usage: show_dependent_extensions_warning "cmd1" "cmd2" ...
show_dependent_extensions_warning() {
    local dependent_exts=()
    mapfile -t dependent_exts < <(check_dependent_extensions "$@")

    if [[ ${#dependent_exts[@]} -gt 0 ]]; then
        print_warning "The following active extensions depend on this extension and may stop working:"
        for ext in "${dependent_exts[@]}"; do
            echo "  - $ext"
        done
        echo ""
        return 0
    fi
    return 1
}

# ============================================================================
# UPGRADE HELPERS - Extension API v2.0
# ============================================================================

# Check if extension supports upgrade (has upgrade() function)
# Usage: if supports_upgrade; then ... fi
# Returns: 0 if upgrade() function exists, 1 otherwise
supports_upgrade() {
    declare -f upgrade >/dev/null 2>&1
}

# Check if running in dry-run mode
# Usage: if is_dry_run; then ... fi
# Returns: 0 if DRY_RUN=true, 1 otherwise
is_dry_run() {
    [[ "${DRY_RUN:-false}" == "true" ]]
}

# Get dry-run prefix for logging
# Usage: echo "$(dry_run_prefix)Upgrading package..."
dry_run_prefix() {
    if is_dry_run; then
        echo "[DRY-RUN] "
    fi
}

# ============================================================================
# MISE UPGRADE HELPERS
# ============================================================================

# Upgrade mise-managed tools for an extension
# Usage: upgrade_mise_tools "extension-name"
# Returns: 0 on success, 1 on failure
upgrade_mise_tools() {
    local extension_name="$1"

    print_status "$(dry_run_prefix)Upgrading mise-managed tools for ${extension_name}..."

    if ! command_exists mise; then
        print_error "mise is not installed"
        return 1
    fi

    # Get tools managed by this extension's TOML
    local toml_path="$HOME/.config/mise/conf.d/${extension_name}.toml"
    if [[ ! -f "$toml_path" ]]; then
        print_warning "No mise configuration found for ${extension_name}"
        return 1
    fi

    # Dry-run mode
    if is_dry_run; then
        print_status "Would run: mise upgrade"
        local outdated
        if outdated=$(mise outdated 2>/dev/null); then
            echo "$outdated"
        fi
        return 0
    fi

    # Upgrade tools
    if mise upgrade; then
        print_success "Tools upgraded successfully"
        return 0
    else
        print_error "mise upgrade failed"
        return 1
    fi
}

# ============================================================================
# APT UPGRADE HELPERS
# ============================================================================

# Check which APT packages have updates available
# Usage: check_apt_updates "package1" "package2" ...
# Returns: 0 if updates available, 1 if all up-to-date
check_apt_updates() {
    local -a packages=("$@")

    if [[ ${#packages[@]} -eq 0 ]]; then
        print_error "No packages specified"
        return 1
    fi

    sudo apt-get update -qq >/dev/null 2>&1

    local has_updates=0
    for pkg in "${packages[@]}"; do
        if apt list --upgradable 2>/dev/null | grep -q "^${pkg}/"; then
            local current_ver available_ver
            current_ver=$(dpkg -l "$pkg" 2>/dev/null | awk '/^ii/ {print $3}')
            available_ver=$(apt-cache policy "$pkg" | awk '/Candidate:/ {print $2}')

            print_status "${pkg}: ${current_ver} → ${available_ver}"
            has_updates=1
        fi
    done

    return $has_updates
}

# Upgrade APT packages
# Usage: upgrade_apt_packages "package1" "package2" ...
# Returns: 0 on success, 1 on failure
upgrade_apt_packages() {
    local -a packages=("$@")

    if [[ ${#packages[@]} -eq 0 ]]; then
        print_error "No packages specified"
        return 1
    fi

    print_status "$(dry_run_prefix)Upgrading APT packages: ${packages[*]}"

    # Update package lists
    if ! sudo apt-get update -qq; then
        print_error "Failed to update package lists"
        return 1
    fi

    # Check which packages have updates available
    local has_updates=0
    local -a upgradeable=()

    for pkg in "${packages[@]}"; do
        if apt list --upgradable 2>/dev/null | grep -q "^${pkg}/"; then
            upgradeable+=("$pkg")
            has_updates=1
        fi
    done

    if [[ $has_updates -eq 0 ]]; then
        print_success "All packages are up to date"
        return 0
    fi

    print_status "Upgradeable packages: ${upgradeable[*]}"

    # Dry-run mode
    if is_dry_run; then
        print_status "Would upgrade: ${upgradeable[*]}"
        return 0
    fi

    # Upgrade packages
    if sudo apt-get install --only-upgrade -y "${upgradeable[@]}"; then
        print_success "Packages upgraded successfully"
        return 0
    else
        print_error "Package upgrade failed"
        return 1
    fi
}

# ============================================================================
# BINARY UPGRADE HELPERS
# ============================================================================

# Compare semantic versions
# Usage: version_gt "2.0.0" "1.9.0" && echo "2.0.0 is greater"
# Returns: 0 if first version > second, 1 otherwise
version_gt() {
    local ver1="$1"
    local ver2="$2"

    if [[ "$ver1" == "$ver2" ]]; then
        return 1
    fi

    printf '%s\n%s\n' "$ver1" "$ver2" | sort -V | head -n1 | grep -q "^${ver2}$"
}

# Upgrade GitHub binary release
# Usage: upgrade_github_binary "repo/name" "binary-name" "/path/to/install" ["--version-flag"]
# Example: upgrade_github_binary "docker/compose" "docker-compose" "/usr/local/bin/docker-compose" "version"
# Returns: 0 on success, 1 on failure
upgrade_github_binary() {
    local repo="$1"           # e.g., "docker/compose"
    local binary_name="$2"    # e.g., "docker-compose"
    local install_path="$3"   # e.g., "/usr/local/bin/docker-compose"
    local version_flag="${4:---version}"

    print_status "$(dry_run_prefix)Checking for updates to ${binary_name}..."

    # Get current version
    local current_version
    if [[ -f "$install_path" ]]; then
        current_version=$("$install_path" $version_flag 2>/dev/null | grep -oP '\d+\.\d+\.\d+' | head -1)
    else
        print_error "Binary not found: ${install_path}"
        return 1
    fi

    # Get latest release from GitHub API
    local latest_version
    latest_version=$(curl -s "https://api.github.com/repos/${repo}/releases/latest" | \
        grep -oP '"tag_name": "\K[^"]+' | sed 's/^v//')

    if [[ -z "$latest_version" ]]; then
        print_error "Failed to fetch latest version from GitHub"
        return 1
    fi

    # Compare versions
    if [[ "$current_version" == "$latest_version" ]]; then
        print_success "${binary_name} is up to date (${current_version})"
        return 0
    fi

    print_status "Update available: ${current_version} → ${latest_version}"

    # Dry-run mode
    if is_dry_run; then
        print_status "Would upgrade ${binary_name} to ${latest_version}"
        return 0
    fi

    # Download and install
    local download_url="https://github.com/${repo}/releases/download/v${latest_version}/${binary_name}-$(uname -s)-$(uname -m)"
    local temp_file="/tmp/${binary_name}-${latest_version}"

    if curl -L "$download_url" -o "$temp_file"; then
        sudo mv "$temp_file" "$install_path"
        sudo chmod +x "$install_path"
        print_success "${binary_name} upgraded to ${latest_version}"
        return 0
    else
        print_error "Failed to download ${binary_name}"
        return 1
    fi
}

# ============================================================================
# GIT UPGRADE HELPERS
# ============================================================================

# Upgrade git repository
# Usage: upgrade_git_repo "/path/to/repo" ["rebuild-command"]
# Returns: 0 on success, 1 on failure
upgrade_git_repo() {
    local repo_path="$1"
    local rebuild_cmd="${2:-}"

    if [[ ! -d "$repo_path" ]]; then
        print_error "Repository not found: ${repo_path}"
        return 1
    fi

    print_status "$(dry_run_prefix)Updating git repository: ${repo_path}"

    cd "$repo_path" || return 1

    # Get current commit
    local current_commit
    current_commit=$(git rev-parse HEAD)

    # Dry-run mode
    if is_dry_run; then
        local remote_commit
        git fetch --quiet
        # shellcheck disable=SC1083 # @{u} is valid git syntax for upstream branch
        remote_commit=$(git rev-parse @{u})
        if [[ "$current_commit" != "$remote_commit" ]]; then
            print_status "Would update: ${current_commit:0:8} → ${remote_commit:0:8}"
        else
            print_success "Repository is up to date"
        fi
        return 0
    fi

    # Pull latest changes
    if ! git pull --ff-only; then
        print_error "Failed to update repository"
        return 1
    fi

    local new_commit
    new_commit=$(git rev-parse HEAD)

    if [[ "$current_commit" == "$new_commit" ]]; then
        print_success "Repository is up to date"
        return 0
    fi

    print_status "Repository updated: ${current_commit:0:8} → ${new_commit:0:8}"

    # Rebuild if command provided
    if [[ -n "$rebuild_cmd" ]]; then
        print_status "Rebuilding..."
        if eval "$rebuild_cmd"; then
            print_success "Rebuild successful"
            return 0
        else
            print_error "Rebuild failed"
            return 1
        fi
    fi

    return 0
}

# ============================================================================
# NATIVE/MANUAL UPGRADE HELPERS
# ============================================================================

# Check native tool version (informational only)
# Usage: check_native_update "tool-name" ["--version"]
# Returns: 2 (special code indicating manual action required)
check_native_update() {
    local tool_name="$1"
    local version_cmd="${2:---version}"

    if ! command_exists "$tool_name"; then
        print_error "${tool_name} not found"
        return 1
    fi

    local current_version
    current_version=$($tool_name $version_cmd 2>/dev/null | head -1)

    print_status "${tool_name}: ${current_version}"
    print_warning "Native tools require Docker image rebuild to upgrade"
    print_status "Run: docker build --no-cache to rebuild image"

    return 2  # Special code: update requires manual action
}

# ============================================================================
# EXPORTS
# ============================================================================

# Export all functions for use by extensions
export -f extension_init
export -f extension_main

# Environment helpers
export -f is_ci_mode
export -f activate_mise_environment

# Prerequisite checks
export -f check_mise_prerequisite
export -f check_disk_space

# Status helpers
export -f print_extension_header

# Validation helpers
export -f validate_commands

# Mise helpers
export -f install_mise_config
export -f remove_mise_config

# Dependency checking
export -f check_dependent_extensions
export -f show_dependent_extensions_warning

# Cleanup helpers
export -f cleanup_bashrc
export -f setup_git_aliases
export -f cleanup_git_aliases
export -f prompt_confirmation

# Upgrade helpers (Extension API v2.0)
export -f supports_upgrade is_dry_run dry_run_prefix
export -f upgrade_mise_tools
export -f check_apt_updates upgrade_apt_packages
export -f version_gt upgrade_github_binary
export -f upgrade_git_repo
export -f check_native_update
