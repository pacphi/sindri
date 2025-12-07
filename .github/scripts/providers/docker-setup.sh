#!/bin/bash
# Docker setup helper for GitHub Actions
# Generates docker-compose.yml using adapter or creates minimal fallback config
#
# Usage:
#   docker-setup.sh --container-name NAME [--sindri-config PATH] [--output-dir DIR]
#
# When --sindri-config is provided, uses the docker-adapter.sh to generate comprehensive config.
# Otherwise, creates a minimal docker-compose.yml suitable for CI testing.

set -euo pipefail

# Source common setup functions
# shellcheck source=common-setup.sh
source "$(dirname "${BASH_SOURCE[0]}")/common-setup.sh"

# Initialize common variables
init_common_vars "${BASH_SOURCE[0]}"

# Provider-specific defaults
CONTAINER_NAME=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --container-name)
            CONTAINER_NAME="$2"
            shift 2
            ;;
        --sindri-config)
            SINDRI_CONFIG="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help)
            show_help "$0"
            ;;
        *)
            unknown_option "$1"
            ;;
    esac
done

# Validate required inputs
validate_required "container-name" "$CONTAINER_NAME"

# Ensure output directory exists
ensure_output_dir

# Check if docker-compose.yml already exists
check_existing_config "$OUTPUT_DIR/docker-compose.yml"

# If sindri-config provided and exists, use the adapter
if has_sindri_config; then
    run_adapter_with_config "docker-adapter.sh" deploy \
        --config-only \
        --output-dir "$OUTPUT_DIR" \
        --container-name "$CONTAINER_NAME"
    exit 0
fi

# Fallback: Use the docker-adapter with a minimal inline config
TEMP_CONFIG=$(create_temp_config)

write_minimal_config "$TEMP_CONFIG" << YAML
version: "1.0"
name: ${CONTAINER_NAME}
deployment:
  provider: docker
  resources:
    memory: 4GB
    cpus: 2
extensions:
  profile: minimal
YAML

run_adapter_with_fallback "docker-adapter.sh" "$TEMP_CONFIG" deploy \
    --config-only \
    --output-dir "$OUTPUT_DIR" \
    --container-name "$CONTAINER_NAME"

cleanup_temp_config "$TEMP_CONFIG"

print_generated_success "docker-compose.yml"
