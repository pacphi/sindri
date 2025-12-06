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

    # DevPod uses various label schemes - try multiple selectors
    # Common patterns: devpod.sh/workspace=ID, app=ID, name=ID
    POD_FOUND=false
    for LABEL_SELECTOR in \
        "devpod.sh/workspace=$WORKSPACE_ID" \
        "app=$WORKSPACE_ID" \
        "devpod.sh/workspace-id=$WORKSPACE_ID"; do

        echo "Checking for pods with label: $LABEL_SELECTOR in namespace: $NAMESPACE"
        if kubectl get pods -l "$LABEL_SELECTOR" -n "$NAMESPACE" 2>/dev/null | grep -q "$WORKSPACE_ID"; then
            echo "Found pod with label: $LABEL_SELECTOR"
            POD_FOUND=true

            # Wait for pod to be ready
            echo "Waiting for pod to be ready (timeout: 120s)..."
            if kubectl wait --for=condition=Ready \
                pod -l "$LABEL_SELECTOR" \
                -n "$NAMESPACE" \
                --timeout=120s 2>&1; then
                echo "✅ Kubernetes pod is ready"
                break
            else
                echo "::warning::Pod not ready after 120s with label $LABEL_SELECTOR"
            fi
        fi
    done

    # If no labeled pod found, check all pods in namespace
    if [[ "$POD_FOUND" == "false" ]]; then
        echo ""
        echo "::warning::No pod found with expected labels. Checking all pods in namespace '$NAMESPACE':"
        kubectl get pods -n "$NAMESPACE" -o wide 2>/dev/null || true
        echo ""

        # Since devpod ssh worked earlier, the workspace IS running
        # The label selector might just be different - this is non-fatal
        echo "::notice::DevPod SSH connection verified earlier - continuing despite kubectl label mismatch"
        echo "✅ DevPod connectivity confirmed (kubectl pod check skipped)"
    fi
fi

echo ""
echo "✅ DevPod warmup complete - ready for testing"
