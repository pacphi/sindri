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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Default values
WORKSPACE_NAME=""
SINDRI_CONFIG=""
PROVIDER_TYPE="docker"
OUTPUT_DIR="."

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
            head -12 "$0" | tail -10
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Validate required inputs
if [[ -z "$WORKSPACE_NAME" ]]; then
    echo "Error: --workspace-name is required" >&2
    exit 1
fi

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Check if devcontainer.json already exists
if [[ -f "$OUTPUT_DIR/.devcontainer/devcontainer.json" ]]; then
    echo "source=existing"
    echo "Using existing devcontainer.json"
    exit 0
fi

# If sindri-config provided and exists, use the adapter
if [[ -n "$SINDRI_CONFIG" ]] && [[ -f "$SINDRI_CONFIG" ]]; then
    echo "source=adapter"
    echo "Generating devcontainer.json using adapter with config: $SINDRI_CONFIG"

    "$REPO_ROOT/deploy/adapters/devpod-adapter.sh" \
        --config-only \
        --output-dir "$OUTPUT_DIR" \
        --workspace-name "$WORKSPACE_NAME" \
        "$SINDRI_CONFIG"

    exit 0
fi

# Fallback: Use the devpod-adapter with a minimal inline config
echo "source=fallback"
echo "Generating minimal devcontainer.json (no sindri-config provided)"

TEMP_CONFIG=$(mktemp)
cat > "$TEMP_CONFIG" << YAML
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

# Use the adapter with the temporary config
"$REPO_ROOT/deploy/adapters/devpod-adapter.sh" \
    --config-only \
    --output-dir "$OUTPUT_DIR" \
    --workspace-name "$WORKSPACE_NAME" \
    "$TEMP_CONFIG"

# Cleanup
rm -f "$TEMP_CONFIG"

echo "Generated devcontainer.json successfully"
