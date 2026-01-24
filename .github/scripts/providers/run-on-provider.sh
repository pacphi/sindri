#!/usr/bin/env bash
# Unified remote command execution for all providers
# Usage: run-on-provider.sh <provider> <target-id> <command>
# Abstracts provider-specific remote execution (docker exec, flyctl ssh, devpod ssh)

set -euo pipefail

PROVIDER="${1:?Provider required}"
TARGET_ID="${2:?Target ID required (container name, app name, or workspace ID)}"
COMMAND="${3:?Command required}"

case "$PROVIDER" in
    docker)
        # Use login shell (-l) to ensure mise shims and PATH are properly loaded
        # Run as developer user (where extensions are installed)
        # Pass GITHUB_TOKEN if available for mise API rate limits
        ENV_ARGS=""
        if [[ -n "${GITHUB_TOKEN:-}" ]]; then
            ENV_ARGS="-e GITHUB_TOKEN=$GITHUB_TOKEN"
        fi
        docker exec $ENV_ARGS --user developer "$TARGET_ID" bash -l -c "$COMMAND"
        ;;
    fly)
        flyctl ssh console -a "$TARGET_ID" --command "$COMMAND"
        ;;
    devpod-*|kubernetes|ssh)
        devpod ssh "$TARGET_ID" --command "$COMMAND"
        ;;
    *)
        echo "::error::Unknown provider: $PROVIDER"
        exit 1
        ;;
esac
