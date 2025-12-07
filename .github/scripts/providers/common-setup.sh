#!/bin/bash
# Common setup functions for GitHub Actions provider scripts
# Source this file in provider-specific setup scripts.
#
# Shared functionality:
#   - Script initialization (set paths, default variables)
#   - Common argument parsing (--sindri-config, --output-dir, --help)
#   - Required argument validation
#   - Output directory management
#   - Existing config detection
#   - Adapter invocation patterns
#   - Temporary config lifecycle

# Strict mode - provider scripts should also set this
set -euo pipefail

# ============================================================================
# Initialization
# ============================================================================

# Initialize common variables
# Call this first in provider scripts, passing BASH_SOURCE[0] from the caller
# Usage: init_common_vars "${BASH_SOURCE[0]}"
init_common_vars() {
    local caller_script="$1"
    SCRIPT_DIR="$(cd "$(dirname "$caller_script")" && pwd)"
    REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

    # Common defaults
    SINDRI_CONFIG=""
    OUTPUT_DIR="."
}

# ============================================================================
# Argument Parsing
# ============================================================================

# Show help from script header comments (lines 2-10 of the script)
# Usage: show_help "$0"
show_help() {
    local script="$1"
    # Extract comment block after shebang, strip # prefix
    tail -n +2 "$script" | while IFS= read -r line; do
        if [[ "$line" =~ ^# ]]; then
            # Strip "# " or just "#" from the start
            line="${line#\#}"
            line="${line# }"
            echo "$line"
        else
            break
        fi
    done
    exit 0
}

# Handle unknown option error
# Usage: unknown_option "$1"
unknown_option() {
    echo "Unknown option: $1" >&2
    exit 1
}

# ============================================================================
# Validation
# ============================================================================

# Validate that a required argument is provided
# Usage: validate_required "container-name" "$CONTAINER_NAME"
validate_required() {
    local arg_name="$1"
    local arg_value="$2"
    if [[ -z "$arg_value" ]]; then
        echo "Error: --${arg_name} is required" >&2
        exit 1
    fi
}

# ============================================================================
# Output Directory Management
# ============================================================================

# Ensure output directory exists
# Usage: ensure_output_dir
ensure_output_dir() {
    mkdir -p "$OUTPUT_DIR"
}

# ============================================================================
# Existing Config Detection
# ============================================================================

# Check if config file already exists and exit early if so
# Usage: check_existing_config "$OUTPUT_DIR/docker-compose.yml"
check_existing_config() {
    local config_file="$1"
    if [[ -f "$config_file" ]]; then
        echo "source=existing"
        echo "Using existing $(basename "$config_file")"
        exit 0
    fi
}

# ============================================================================
# Adapter Invocation
# ============================================================================

# Check if sindri-config is provided and valid
# Usage: if has_sindri_config; then ... fi
has_sindri_config() {
    [[ -n "$SINDRI_CONFIG" ]] && [[ -f "$SINDRI_CONFIG" ]]
}

# Run adapter with sindri-config
# Usage: run_adapter_with_config "fly-adapter.sh" deploy --config-only --output-dir "$OUTPUT_DIR" --app-name "$APP_NAME"
# Note: SINDRI_CONFIG is automatically appended as the last argument
run_adapter_with_config() {
    local adapter="$1"
    shift

    echo "source=adapter"
    echo "Generating config using adapter with config: $SINDRI_CONFIG"

    "$REPO_ROOT/deploy/adapters/$adapter" "$@" "$SINDRI_CONFIG"
}

# Run adapter with temporary fallback config
# Usage: run_adapter_with_fallback "docker-adapter.sh" "$TEMP_CONFIG" --config-only --output-dir "$OUTPUT_DIR"
# Note: The temp config path should be provided, it's appended as last argument
run_adapter_with_fallback() {
    local adapter="$1"
    local temp_config="$2"
    shift 2

    echo "source=fallback"
    echo "Generating minimal config (no sindri-config provided)"

    "$REPO_ROOT/deploy/adapters/$adapter" "$@" "$temp_config"
}

# ============================================================================
# Temporary Config Lifecycle
# ============================================================================

# Create a temporary config file
# Usage: TEMP_CONFIG=$(create_temp_config)
create_temp_config() {
    mktemp
}

# Write minimal sindri.yaml content to temp file
# Usage: write_minimal_config "$TEMP_CONFIG" <<'YAML'
# version: "1.0"
# ...
# YAML
write_minimal_config() {
    local temp_file="$1"
    cat > "$temp_file"
}

# Cleanup temporary config file
# Usage: cleanup_temp_config "$TEMP_CONFIG"
cleanup_temp_config() {
    local temp_file="$1"
    rm -f "$temp_file"
}

# ============================================================================
# Success Messages
# ============================================================================

# Print success message for config generation
# Usage: print_generated_success "docker-compose.yml"
print_generated_success() {
    local config_name="$1"
    echo "Generated $config_name successfully"
}
