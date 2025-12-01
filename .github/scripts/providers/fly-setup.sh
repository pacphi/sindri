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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Default values
APP_NAME=""
SINDRI_CONFIG=""
REGION="sjc"
OUTPUT_DIR="."
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
            head -12 "$0" | tail -10
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Build CI_MODE flag for adapter
CI_MODE_FLAG=""
[[ "$CI_MODE" == "true" ]] && CI_MODE_FLAG="--ci-mode"

# Validate required inputs
if [[ -z "$APP_NAME" ]]; then
    echo "Error: --app-name is required" >&2
    exit 1
fi

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Check if fly.toml already exists
if [[ -f "$OUTPUT_DIR/fly.toml" ]]; then
    echo "source=existing"
    echo "Using existing fly.toml"
    exit 0
fi

# If sindri-config provided and exists, use the adapter
if [[ -n "$SINDRI_CONFIG" ]] && [[ -f "$SINDRI_CONFIG" ]]; then
    echo "source=adapter"
    echo "Generating fly.toml using adapter with config: $SINDRI_CONFIG"
    [[ "$CI_MODE" == "true" ]] && echo "CI Mode: enabled"

    # shellcheck disable=SC2086
    "$REPO_ROOT/deploy/adapters/fly-adapter.sh" \
        --config-only \
        --output-dir "$OUTPUT_DIR" \
        --app-name "$APP_NAME" \
        $CI_MODE_FLAG \
        "$SINDRI_CONFIG"

    exit 0
fi

# Fallback: Use the fly-adapter with a minimal inline config
# Create a temporary minimal sindri.yaml
echo "source=fallback"
echo "Generating minimal fly.toml (no sindri-config provided)"
[[ "$CI_MODE" == "true" ]] && echo "CI Mode: enabled"

TEMP_CONFIG=$(mktemp)
cat > "$TEMP_CONFIG" << YAML
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

# Use the adapter with the temporary config
# shellcheck disable=SC2086
"$REPO_ROOT/deploy/adapters/fly-adapter.sh" \
    --config-only \
    --output-dir "$OUTPUT_DIR" \
    --app-name "$APP_NAME" \
    $CI_MODE_FLAG \
    "$TEMP_CONFIG"

# Cleanup
rm -f "$TEMP_CONFIG"

echo "Generated fly.toml successfully"
