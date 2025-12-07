#!/bin/bash
# Fly.io setup helper for GitHub Actions
# Generates fly.toml using adapter or creates minimal fallback config
#
# Usage:
#   fly-setup.sh --app-name NAME [--sindri-config PATH] [--region REGION] [--output-dir DIR] [--ci-mode]
#
# When --sindri-config is provided, uses the fly-adapter.sh to generate comprehensive config.
# Otherwise, creates a minimal fly.toml suitable for CI testing.
# Use --ci-mode to generate CI-compatible config (empty services, no health checks).

set -euo pipefail

# Source common setup functions
# shellcheck source=common-setup.sh
source "$(dirname "${BASH_SOURCE[0]}")/common-setup.sh"

# Initialize common variables
init_common_vars "${BASH_SOURCE[0]}"

# Provider-specific defaults
APP_NAME=""
REGION="sjc"
CI_MODE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --app-name)
            APP_NAME="$2"
            shift 2
            ;;
        --sindri-config)
            SINDRI_CONFIG="$2"
            shift 2
            ;;
        --region)
            REGION="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --ci-mode)
            CI_MODE=true
            shift
            ;;
        --help)
            show_help "$0"
            ;;
        *)
            unknown_option "$1"
            ;;
    esac
done

# Build CI_MODE flag for adapter
CI_MODE_FLAG=""
[[ "$CI_MODE" == "true" ]] && CI_MODE_FLAG="--ci-mode"

# Validate required inputs
validate_required "app-name" "$APP_NAME"

# Ensure output directory exists
ensure_output_dir

# Check if fly.toml already exists
check_existing_config "$OUTPUT_DIR/fly.toml"

# If sindri-config provided and exists, use the adapter
if has_sindri_config; then
    [[ "$CI_MODE" == "true" ]] && echo "CI Mode: enabled"
    # shellcheck disable=SC2086
    run_adapter_with_config "fly-adapter.sh" deploy \
        --config-only \
        --output-dir "$OUTPUT_DIR" \
        --app-name "$APP_NAME" \
        $CI_MODE_FLAG
    exit 0
fi

# Fallback: Use the fly-adapter with a minimal inline config
[[ "$CI_MODE" == "true" ]] && echo "CI Mode: enabled"

TEMP_CONFIG=$(create_temp_config)

write_minimal_config "$TEMP_CONFIG" << YAML
version: "1.0"
name: ${APP_NAME}
deployment:
  provider: fly
  resources:
    memory: 1GB
    cpus: 1
  volumes:
    workspace:
      size: 10GB
extensions:
  profile: minimal
providers:
  fly:
    region: ${REGION}
    organization: personal
    autoStopMachines: true
    autoStartMachines: true
    cpuKind: shared
    sshPort: 10022
YAML

# shellcheck disable=SC2086
run_adapter_with_fallback "fly-adapter.sh" "$TEMP_CONFIG" deploy \
    --config-only \
    --output-dir "$OUTPUT_DIR" \
    --app-name "$APP_NAME" \
    $CI_MODE_FLAG

cleanup_temp_config "$TEMP_CONFIG"

print_generated_success "fly.toml"
