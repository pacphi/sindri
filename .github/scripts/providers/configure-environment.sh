#!/usr/bin/env bash
# Configure provider-specific environment and feature flags
# Usage: configure-environment.sh <provider>
# Sets environment variables for timeout configuration, feature flags, and provider tuning

set -euo pipefail

PROVIDER="${1:?Provider required (docker, fly, devpod-*, kubernetes, ssh)}"

# =============================================================================
# Core Timeout Configuration
# =============================================================================
export SINDRI_MISE_TIMEOUT="${SINDRI_MISE_TIMEOUT:-300}"              # 5 minutes for mise install
export SINDRI_DNS_TIMEOUT="${SINDRI_DNS_TIMEOUT:-3}"                  # 3 seconds for DNS checks
export SINDRI_VALIDATION_TIMEOUT="${SINDRI_VALIDATION_TIMEOUT:-10}"   # 10 seconds for validation commands
export SINDRI_PROFILE_INSTALL_TIMEOUT="${SINDRI_PROFILE_INSTALL_TIMEOUT:-600}"  # 10 minutes for full profile

# =============================================================================
# Feature Flags (Day 2 optimizations)
# =============================================================================
export SINDRI_ENABLE_PARALLEL_VALIDATION="${SINDRI_ENABLE_PARALLEL_VALIDATION:-true}"
export SINDRI_ENABLE_BATCHED_REMOTE_CALLS="${SINDRI_ENABLE_BATCHED_REMOTE_CALLS:-true}"
export SINDRI_ENABLE_DNS_CACHE="${SINDRI_ENABLE_DNS_CACHE:-true}"
export SINDRI_ENABLE_RETRY_LOGIC="${SINDRI_ENABLE_RETRY_LOGIC:-true}"
export SINDRI_ENABLE_PROGRESS_INDICATORS="${SINDRI_ENABLE_PROGRESS_INDICATORS:-true}"

# =============================================================================
# Monitoring & Debugging
# =============================================================================
export SINDRI_ENABLE_TIMING_METRICS="${SINDRI_ENABLE_TIMING_METRICS:-true}"
export SINDRI_ENABLE_DEBUG_OUTPUT="${SINDRI_ENABLE_DEBUG_OUTPUT:-false}"
export SINDRI_FAIL_FAST="${SINDRI_FAIL_FAST:-true}"

# =============================================================================
# Emergency Kill Switch
# =============================================================================
if [[ "${SINDRI_EMERGENCY_REVERT:-false}" == "true" ]]; then
    echo "⚠️  EMERGENCY REVERT MODE ACTIVE"
    echo "Disabling all Day 2 optimizations (batching, parallelism)"
    export SINDRI_ENABLE_PARALLEL_VALIDATION=false
    export SINDRI_ENABLE_BATCHED_REMOTE_CALLS=false
    export SINDRI_PARALLEL_JOBS=1
fi

# =============================================================================
# Provider-Specific Tuning
# =============================================================================
case "$PROVIDER" in
    docker)
        # Docker: Fast local exec, minimal overhead
        export SINDRI_PARALLEL_JOBS=5          # More parallelism
        export SINDRI_CONNECTION_OVERHEAD=0    # Negligible
        export SINDRI_RETRY_DELAY=1            # Fast retries
        ;;
    fly)
        # Fly.io: SSH overhead, balanced parallelism
        export SINDRI_PARALLEL_JOBS=3
        export SINDRI_CONNECTION_OVERHEAD=2    # ~2s per SSH call
        export SINDRI_RETRY_DELAY=3
        ;;
    devpod-k8s|kubernetes)
        # Kubernetes: kubectl exec can be slow
        export SINDRI_PARALLEL_JOBS=2          # Lower parallelism
        export SINDRI_CONNECTION_OVERHEAD=3    # kubectl overhead
        export SINDRI_RETRY_DELAY=5            # Pod recovery time
        ;;
    devpod-*|ssh)
        # DevPod cloud/SSH: Similar to Fly.io
        export SINDRI_PARALLEL_JOBS=3
        export SINDRI_CONNECTION_OVERHEAD=2    # SSH connection time
        export SINDRI_RETRY_DELAY=3
        ;;
    *)
        # Unknown provider: conservative defaults
        export SINDRI_PARALLEL_JOBS=2
        export SINDRI_CONNECTION_OVERHEAD=2
        export SINDRI_RETRY_DELAY=3
        ;;
esac

# =============================================================================
# Output Configuration Summary
# =============================================================================
echo "Provider: $PROVIDER"
echo "  Parallel jobs: $SINDRI_PARALLEL_JOBS"
echo "  Connection overhead: ${SINDRI_CONNECTION_OVERHEAD}s"
echo "  Retry delay: ${SINDRI_RETRY_DELAY}s"
echo "  Batched calls: $SINDRI_ENABLE_BATCHED_REMOTE_CALLS"
echo "  Parallel validation: $SINDRI_ENABLE_PARALLEL_VALIDATION"
