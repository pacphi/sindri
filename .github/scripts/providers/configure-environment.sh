#!/usr/bin/env bash
# Configure provider-specific environment and feature flags
# Usage: configure-environment.sh <provider>
# Sets environment variables for timeout configuration, feature flags, and provider tuning

set -euo pipefail

PROVIDER="${1:?Provider required (docker, fly, devpod-*, kubernetes, ssh)}"

# =============================================================================
# Core Timeout Configuration
# Write to GITHUB_ENV so variables persist across workflow steps
# =============================================================================
{
    echo "SINDRI_MISE_TIMEOUT=${SINDRI_MISE_TIMEOUT:-300}"
    echo "SINDRI_DNS_TIMEOUT=${SINDRI_DNS_TIMEOUT:-3}"
    echo "SINDRI_VALIDATION_TIMEOUT=${SINDRI_VALIDATION_TIMEOUT:-10}"
    echo "SINDRI_PROFILE_INSTALL_TIMEOUT=${SINDRI_PROFILE_INSTALL_TIMEOUT:-600}"

    # Feature Flags
    echo "SINDRI_ENABLE_PARALLEL_VALIDATION=${SINDRI_ENABLE_PARALLEL_VALIDATION:-true}"
    echo "SINDRI_ENABLE_BATCHED_REMOTE_CALLS=${SINDRI_ENABLE_BATCHED_REMOTE_CALLS:-true}"
    echo "SINDRI_ENABLE_DNS_CACHE=${SINDRI_ENABLE_DNS_CACHE:-true}"
    echo "SINDRI_ENABLE_RETRY_LOGIC=${SINDRI_ENABLE_RETRY_LOGIC:-true}"
    echo "SINDRI_ENABLE_PROGRESS_INDICATORS=${SINDRI_ENABLE_PROGRESS_INDICATORS:-true}"

    # Monitoring
    echo "SINDRI_ENABLE_TIMING_METRICS=${SINDRI_ENABLE_TIMING_METRICS:-true}"
    echo "SINDRI_ENABLE_DEBUG_OUTPUT=${SINDRI_ENABLE_DEBUG_OUTPUT:-false}"
    echo "SINDRI_FAIL_FAST=${SINDRI_FAIL_FAST:-true}"
} >> "${GITHUB_ENV:-/dev/null}"

# Emergency Kill Switch
if [[ "${SINDRI_EMERGENCY_REVERT:-false}" == "true" ]]; then
    echo "⚠️  EMERGENCY REVERT MODE ACTIVE"
    echo "Disabling all Day 2 optimizations (batching, parallelism)"
    {
        echo "SINDRI_ENABLE_PARALLEL_VALIDATION=false"
        echo "SINDRI_ENABLE_BATCHED_REMOTE_CALLS=false"
        echo "SINDRI_PARALLEL_JOBS=1"
    } >> "${GITHUB_ENV:-/dev/null}"
fi

# Provider-Specific Tuning
case "$PROVIDER" in
    docker)
        {
            echo "SINDRI_PARALLEL_JOBS=5"
            echo "SINDRI_CONNECTION_OVERHEAD=0"
            echo "SINDRI_RETRY_DELAY=1"
        } >> "${GITHUB_ENV:-/dev/null}"
        ;;
    fly)
        {
            echo "SINDRI_PARALLEL_JOBS=3"
            echo "SINDRI_CONNECTION_OVERHEAD=2"
            echo "SINDRI_RETRY_DELAY=3"
        } >> "${GITHUB_ENV:-/dev/null}"
        ;;
    devpod-k8s|kubernetes)
        {
            echo "SINDRI_PARALLEL_JOBS=2"
            echo "SINDRI_CONNECTION_OVERHEAD=3"
            echo "SINDRI_RETRY_DELAY=5"
        } >> "${GITHUB_ENV:-/dev/null}"
        ;;
    devpod-*|ssh)
        {
            echo "SINDRI_PARALLEL_JOBS=3"
            echo "SINDRI_CONNECTION_OVERHEAD=2"
            echo "SINDRI_RETRY_DELAY=3"
        } >> "${GITHUB_ENV:-/dev/null}"
        ;;
    *)
        {
            echo "SINDRI_PARALLEL_JOBS=2"
            echo "SINDRI_CONNECTION_OVERHEAD=2"
            echo "SINDRI_RETRY_DELAY=3"
        } >> "${GITHUB_ENV:-/dev/null}"
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
