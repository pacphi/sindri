#!/bin/bash
# DevPod setup helper for GitHub Actions
# Generates devcontainer.json using adapter or creates minimal fallback config
#
# Usage:
#   devpod-setup.sh --workspace-name NAME [--sindri-config PATH] [--provider TYPE] [--output-dir DIR]
#
# When --sindri-config is provided, uses the devpod-adapter.sh to generate comprehensive config.
# Otherwise, creates a minimal devcontainer.json suitable for CI testing.

set -euo pipefail

# Source common setup functions
# shellcheck source=common-setup.sh
source "$(dirname "${BASH_SOURCE[0]}")/common-setup.sh"

# Initialize common variables
init_common_vars "${BASH_SOURCE[0]}"

# Provider-specific defaults
WORKSPACE_NAME=""
PROVIDER_TYPE="docker"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --workspace-name)
            WORKSPACE_NAME="$2"
            shift 2
            ;;
        --sindri-config)
            SINDRI_CONFIG="$2"
            shift 2
            ;;
        --provider)
            PROVIDER_TYPE="$2"
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
validate_required "workspace-name" "$WORKSPACE_NAME"

# Ensure output directory exists
ensure_output_dir

# Check if devcontainer.json already exists
check_existing_config "$OUTPUT_DIR/.devcontainer/devcontainer.json"

# If sindri-config provided and exists, use the adapter
if has_sindri_config; then
    run_adapter_with_config "devpod-adapter.sh" deploy \
        --config-only \
        --output-dir "$OUTPUT_DIR" \
        --workspace-name "$WORKSPACE_NAME"
    exit 0
fi

# Fallback: Use the devpod-adapter with a minimal inline config
TEMP_CONFIG=$(create_temp_config)

write_minimal_config "$TEMP_CONFIG" << YAML
version: "1.0"
name: ${WORKSPACE_NAME}
deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 10GB
extensions:
  profile: minimal
providers:
  devpod:
    type: ${PROVIDER_TYPE}
YAML

run_adapter_with_fallback "devpod-adapter.sh" "$TEMP_CONFIG" deploy \
    --config-only \
    --output-dir "$OUTPUT_DIR" \
    --workspace-name "$WORKSPACE_NAME"

cleanup_temp_config "$TEMP_CONFIG"

print_generated_success "devcontainer.json"
