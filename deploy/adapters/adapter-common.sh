#!/bin/bash
# Common functions for adapter scripts
# Source this file in provider-specific adapters.
#
# Shared functionality:
#   - Script initialization
#   - Command-line argument parsing
#   - Configuration file validation
#   - Base sindri.yaml parsing (NAME, PROFILE, resources, GPU)
#   - Auto-install conversion logic
#   - Command dispatch

# ============================================================================
# Initialization
# ============================================================================

# Initialize common adapter variables
# Call this first in adapter scripts, passing BASH_SOURCE[0] from the caller
# Usage: adapter_init "${BASH_SOURCE[0]}"
adapter_init() {
    local caller_script="$1"
    ADAPTER_SCRIPT_DIR="$(cd "$(dirname "$caller_script")" && pwd)"
    # shellcheck disable=SC2034  # Used by sourcing scripts
    BASE_DIR="$(cd "$ADAPTER_SCRIPT_DIR/../.." && pwd)"

    # Common defaults (used by sourcing scripts)
    # shellcheck disable=SC2034
    COMMAND=""
    SINDRI_YAML=""
    # shellcheck disable=SC2034
    CONFIG_ONLY=false
    # shellcheck disable=SC2034
    OUTPUT_DIR="."
    # shellcheck disable=SC2034
    OUTPUT_VARS=false
    CI_MODE=false
    # shellcheck disable=SC2034
    FORCE=false
}

# ============================================================================
# Argument Parsing
# ============================================================================

# Show help from script header comments
# Usage: adapter_show_help "$0" [num_lines]
# num_lines defaults to 30 (head 30, tail 28)
adapter_show_help() {
    local script="$1"
    local head_lines="${2:-30}"
    local tail_lines=$((head_lines - 2))
    head -"$head_lines" "$script" | tail -"$tail_lines"
    exit 0
}

# Handle unknown option error
# Usage: adapter_unknown_option "$1"
adapter_unknown_option() {
    echo "Unknown option: $1" >&2
    exit 1
}

# Parse command from arguments (first positional)
# Usage: adapter_parse_command "$@"; set -- "${REMAINING_ARGS[@]}"
# Sets: COMMAND, REMAINING_ARGS array
adapter_parse_command() {
    if [[ $# -eq 0 ]]; then
        return 1  # Signal that help should be shown
    fi
    # shellcheck disable=SC2034  # Used by sourcing scripts
    COMMAND="$1"
    shift
    # shellcheck disable=SC2034  # Used by sourcing scripts
    REMAINING_ARGS=("$@")
}

# ============================================================================
# Configuration Validation
# ============================================================================

# Set default SINDRI_YAML and validate it exists
# Usage: adapter_validate_config
adapter_validate_config() {
    SINDRI_YAML="${SINDRI_YAML:-sindri.yaml}"

    if [[ ! -f "$SINDRI_YAML" ]]; then
        echo "Error: $SINDRI_YAML not found" >&2
        exit 1
    fi
}

# ============================================================================
# Base Configuration Parsing
# ============================================================================

# Parse common sindri.yaml configuration values
# Sets: NAME, PROFILE, AUTO_INSTALL, CUSTOM_EXTENSIONS, MEMORY, CPUS, VOLUME_SIZE
# Sets: GPU_ENABLED, GPU_TYPE, GPU_TIER, GPU_COUNT
# Usage: adapter_parse_base_config [name_override_var]
# name_override_var: Optional variable name containing override (e.g., "CONTAINER_NAME_OVERRIDE")
adapter_parse_base_config() {
    local name_override_var="${1:-}"

    # Parse name with optional override
    # shellcheck disable=SC2034  # Used by sourcing scripts
    NAME=$(yq '.name' "$SINDRI_YAML")
    if [[ -n "$name_override_var" ]] && [[ -n "${!name_override_var:-}" ]]; then
        # shellcheck disable=SC2034
        NAME="${!name_override_var}"
    fi

    # Extension configuration
    # shellcheck disable=SC2034  # Used by sourcing scripts
    PROFILE=$(yq '.extensions.profile // "minimal"' "$SINDRI_YAML")

    # Auto-install: default true for end users, false for CI testing
    # Read from config without default (returns 'null' if not set)
    AUTO_INSTALL=$(yq '.extensions.autoInstall' "$SINDRI_YAML")
    if [[ "$AUTO_INSTALL" == "null" ]]; then
        AUTO_INSTALL="true"  # Default: auto-install enabled
    fi

    # CI mode override: Force autoInstall=false for clean testing
    if [[ "$CI_MODE" == "true" ]]; then
        AUTO_INSTALL="false"
    fi

    # shellcheck disable=SC2034  # Used by sourcing scripts
    CUSTOM_EXTENSIONS=$(yq '.extensions.active[]? // ""' "$SINDRI_YAML" | tr '\n' ',' | sed 's/,$//')

    # Resource configuration (adapters may override/transform these)
    # shellcheck disable=SC2034  # Used by sourcing scripts
    MEMORY=$(yq '.deployment.resources.memory // "4GB"' "$SINDRI_YAML")
    # shellcheck disable=SC2034  # Used by sourcing scripts
    CPUS=$(yq '.deployment.resources.cpus // 2' "$SINDRI_YAML")
    # shellcheck disable=SC2034  # Used by sourcing scripts
    VOLUME_SIZE=$(yq '.deployment.volumes.workspace.size // "10GB"' "$SINDRI_YAML" | sed 's/GB//')

    # GPU configuration
    GPU_ENABLED=$(yq '.deployment.resources.gpu.enabled // false' "$SINDRI_YAML")
    GPU_TYPE=$(yq '.deployment.resources.gpu.type // "nvidia"' "$SINDRI_YAML")
    # shellcheck disable=SC2034  # Used by sourcing scripts
    GPU_TIER=$(yq '.deployment.resources.gpu.tier // "gpu-small"' "$SINDRI_YAML")
    # shellcheck disable=SC2034  # Used by sourcing scripts
    GPU_COUNT=$(yq '.deployment.resources.gpu.count // 1' "$SINDRI_YAML")
}

# ============================================================================
# Auto-Install Conversion
# ============================================================================

# Convert AUTO_INSTALL (true/false) to SKIP_AUTO_INSTALL (inverted)
# Returns: "true" if auto-install should be skipped, "false" otherwise
# Usage: local skip=$(adapter_get_skip_auto_install)
adapter_get_skip_auto_install() {
    local auto_install_normalized
    auto_install_normalized=$(echo "$AUTO_INSTALL" | tr '[:upper:]' '[:lower:]' | xargs)
    if [[ "$auto_install_normalized" == "false" ]]; then
        echo "true"
    else
        echo "false"
    fi
}

# ============================================================================
# Command Dispatch
# ============================================================================

# Dispatch command to handler functions
# Usage: adapter_dispatch "$COMMAND" cmd_deploy cmd_connect cmd_destroy cmd_plan cmd_status [show_help_func]
# Handler functions should be defined before calling this
adapter_dispatch() {
    local command="$1"
    local deploy_func="$2"
    local connect_func="$3"
    local destroy_func="$4"
    local plan_func="$5"
    local status_func="$6"
    local help_func="${7:-adapter_show_help}"

    case "$command" in
        deploy)  "$deploy_func" ;;
        connect) "$connect_func" ;;
        destroy) "$destroy_func" ;;
        plan)    "$plan_func" ;;
        status)  "$status_func" ;;
        help|--help|-h) "$help_func" ;;
        *)
            echo "Unknown command: $command" >&2
            echo "Commands: deploy, connect, destroy, plan, status"
            exit 1
            ;;
    esac
}

# ============================================================================
# GPU Validation Helpers
# ============================================================================

# Validate GPU configuration for Docker (nvidia only)
# Usage: adapter_validate_docker_gpu
adapter_validate_docker_gpu() {
    if [[ "$GPU_ENABLED" != "true" ]]; then
        return 0
    fi

    if [[ "$GPU_TYPE" != "nvidia" ]]; then
        print_error "Docker adapter only supports nvidia GPUs (requested: $GPU_TYPE)"
        exit 1
    fi

    # Check for NVIDIA runtime availability
    if ! docker info 2>/dev/null | grep -q "nvidia"; then
        print_warning "NVIDIA Docker runtime not detected"
        echo "Install nvidia-container-toolkit for GPU support." >&2
    fi
}
