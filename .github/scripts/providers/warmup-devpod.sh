#!/usr/bin/env bash
# DevPod connection warmup and Kubernetes pod readiness checks
# Usage: warmup-devpod.sh <provider> <workspace-id> [namespace]
# Ensures DevPod connection is established and pod is ready before running tests

set -euo pipefail

PROVIDER="${1:?Provider required}"
WORKSPACE_ID="${2:?Workspace ID required}"
NAMESPACE="${3:-default}"

echo "Warming up DevPod connection..."

# =============================================================================
# Connection Warmup (all DevPod providers)
# =============================================================================
# Pre-authenticate and cache credentials to avoid first-call delays
set +e
if ! devpod ssh "$WORKSPACE_ID" --command "echo 'connection established'"; then
    echo "::warning::DevPod connection warmup failed, retrying..."
    sleep 2
    if ! devpod ssh "$WORKSPACE_ID" --command "echo 'connection established'"; then
        echo "::error::DevPod connection warmup failed after retry"
        exit 1
    fi
fi
set -e

echo "✅ DevPod connection warmed"

# =============================================================================
# Kubernetes Pod Readiness Check
# =============================================================================
if [[ "$PROVIDER" == "devpod-k8s" ]] || [[ "$PROVIDER" == "kubernetes" ]]; then
    echo ""
    echo "Verifying pod readiness for Kubernetes backend..."

    # Wait for pod to be ready (120s timeout - pods can take time to start)
    echo "Waiting for pod in namespace '$NAMESPACE' (timeout: 120s)..."
    if ! kubectl wait --for=condition=Ready \
        pod -l devpod.sh/workspace="$WORKSPACE_ID" \
        -n "$NAMESPACE" \
        --timeout=120s 2>&1; then

        echo "::warning::Pod not ready after 120s, checking pod status..."
        echo ""
        echo "Pod status:"
        kubectl get pods -l devpod.sh/workspace="$WORKSPACE_ID" -n "$NAMESPACE" || true
        echo ""
        echo "Pod details:"
        kubectl describe pods -l devpod.sh/workspace="$WORKSPACE_ID" -n "$NAMESPACE" || true

        # Retry once after showing diagnostics
        echo ""
        echo "Retrying pod wait (additional 60s)..."
        if ! kubectl wait --for=condition=Ready \
            pod -l devpod.sh/workspace="$WORKSPACE_ID" \
            -n "$NAMESPACE" \
            --timeout=60s 2>&1; then

            echo "::error::Pod still not ready after 180s total"
            exit 1
        fi
    fi

    echo "✅ Kubernetes pod is ready"
fi

echo ""
echo "✅ DevPod warmup complete - ready for testing"
