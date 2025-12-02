#!/usr/bin/env bash
# Calculate resource requirements for an extension profile
# Usage: ./calculate-profile-resources.sh <profile-name> [provider]
# Output: GitHub Actions output format (key=value)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PROFILE="${1:-minimal}"
PROVIDER="${2:-}"  # Optional: fly, docker, aws, gcp, azure, digitalocean, kubernetes

PROFILES_FILE="$REPO_ROOT/docker/lib/profiles.yaml"
EXTENSIONS_DIR="$REPO_ROOT/docker/lib/extensions"
VM_SIZES_FILE="$REPO_ROOT/docker/lib/vm-sizes.yaml"

# Check if profile exists
if ! yq -e ".profiles.${PROFILE}" "$PROFILES_FILE" &>/dev/null; then
    echo "Error: Profile '$PROFILE' not found in $PROFILES_FILE" >&2
    exit 1
fi

# Get extensions in profile
EXTENSIONS=$(yq ".profiles.${PROFILE}.extensions[]" "$PROFILES_FILE" 2>/dev/null)

if [[ -z "$EXTENSIONS" ]]; then
    echo "Error: No extensions found in profile '$PROFILE'" >&2
    exit 1
fi

TOTAL_DISK=0
TOTAL_MEMORY=0
TOTAL_INSTALL_TIME=0
EXT_COUNT=0

for ext in $EXTENSIONS; do
    EXT_FILE="$EXTENSIONS_DIR/$ext/extension.yaml"

    if [[ ! -f "$EXT_FILE" ]]; then
        echo "Warning: Extension '$ext' not found at $EXT_FILE" >&2
        continue
    fi

    DISK=$(yq '.requirements.diskSpace // 0' "$EXT_FILE" 2>/dev/null)
    MEM=$(yq '.requirements.memory // 0' "$EXT_FILE" 2>/dev/null)
    TIME=$(yq '.requirements.installTime // 60' "$EXT_FILE" 2>/dev/null)

    TOTAL_DISK=$((TOTAL_DISK + DISK))
    TOTAL_MEMORY=$((TOTAL_MEMORY + MEM))
    TOTAL_INSTALL_TIME=$((TOTAL_INSTALL_TIME + TIME))
    EXT_COUNT=$((EXT_COUNT + 1))
done

# Calculate recommended timeout with buffer (base 5 min + install time + 20% overhead)
BASE_TIMEOUT=300  # 5 minutes base
OVERHEAD_PERCENT=20
TIMEOUT_SECONDS=$((BASE_TIMEOUT + TOTAL_INSTALL_TIME + (TOTAL_INSTALL_TIME * OVERHEAD_PERCENT / 100)))
TIMEOUT_MINUTES=$(( (TIMEOUT_SECONDS + 59) / 60 ))  # Round up to nearest minute

# Determine VM size tier based on memory requirements
if [[ $TOTAL_MEMORY -lt 2048 ]]; then
    VM_SIZE_TIER="small"
elif [[ $TOTAL_MEMORY -lt 4096 ]]; then
    VM_SIZE_TIER="medium"
elif [[ $TOTAL_MEMORY -lt 8192 ]]; then
    VM_SIZE_TIER="large"
else
    VM_SIZE_TIER="xlarge"
fi

# Determine disk tier
if [[ $TOTAL_DISK -lt 2000 ]]; then
    DISK_TIER="small"
elif [[ $TOTAL_DISK -lt 5000 ]]; then
    DISK_TIER="medium"
elif [[ $TOTAL_DISK -lt 10000 ]]; then
    DISK_TIER="large"
else
    DISK_TIER="xlarge"
fi

# Use the higher tier between memory and disk requirements
tier_to_num() {
    case "$1" in
        small) echo 1 ;;
        medium) echo 2 ;;
        large) echo 3 ;;
        xlarge) echo 4 ;;
        *) echo 1 ;;
    esac
}

DISK_TIER_NUM=$(tier_to_num "$DISK_TIER")
VM_TIER_NUM=$(tier_to_num "$VM_SIZE_TIER")

if [[ $DISK_TIER_NUM -gt $VM_TIER_NUM ]]; then
    VM_SIZE_TIER="$DISK_TIER"
fi

# Output base information
echo "profile=$PROFILE"
echo "extension_count=$EXT_COUNT"
echo "disk_mb=$TOTAL_DISK"
echo "memory_mb=$TOTAL_MEMORY"
echo "install_time_sec=$TOTAL_INSTALL_TIME"
echo "recommended_timeout=$TIMEOUT_MINUTES"
echo "vm_size_tier=$VM_SIZE_TIER"
echo "disk_tier=$DISK_TIER"

# Get provider-specific sizes from vm-sizes.yaml
if [[ -f "$VM_SIZES_FILE" ]]; then
    # Get recommended volume sizes
    WORKSPACE_VOLUME=$(yq ".volumes.workspace.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null || echo "10")
    HOME_VOLUME=$(yq ".volumes.home.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null || echo "5")
    RECOMMENDED_TIMEOUT=$(yq ".timeouts.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null || echo "$TIMEOUT_MINUTES")

    echo "workspace_volume_gb=$WORKSPACE_VOLUME"
    echo "home_volume_gb=$HOME_VOLUME"
    echo "timeout_from_tier=$RECOMMENDED_TIMEOUT"

    # If provider specified, output provider-specific configuration
    if [[ -n "$PROVIDER" ]]; then
        # Normalize provider name (devpod-aws -> aws, etc.)
        PROVIDER_KEY="$PROVIDER"
        case "$PROVIDER" in
            devpod-aws) PROVIDER_KEY="aws" ;;
            devpod-gcp) PROVIDER_KEY="gcp" ;;
            devpod-azure) PROVIDER_KEY="azure" ;;
            devpod-do) PROVIDER_KEY="digitalocean" ;;
            devpod-k8s) PROVIDER_KEY="kubernetes" ;;
        esac

        if yq -e ".providers.${PROVIDER_KEY}" "$VM_SIZES_FILE" &>/dev/null; then
            VM_SIZE=$(yq ".providers.${PROVIDER_KEY}.sizes.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null)
            VM_MEMORY=$(yq ".providers.${PROVIDER_KEY}.memory.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null)
            VM_DISK=$(yq ".providers.${PROVIDER_KEY}.disk.${VM_SIZE_TIER} // .providers.${PROVIDER_KEY}.storage.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null)
            VM_CPUS=$(yq ".providers.${PROVIDER_KEY}.vcpus.${VM_SIZE_TIER} // .providers.${PROVIDER_KEY}.cpus.${VM_SIZE_TIER} // .providers.${PROVIDER_KEY}.cpu.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null)
            VM_SWAP=$(yq ".providers.${PROVIDER_KEY}.swap.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null)

            echo "provider=$PROVIDER_KEY"
            echo "provider_vm_size=$VM_SIZE"
            if [[ -n "$VM_MEMORY" && "$VM_MEMORY" != "null" ]]; then echo "provider_memory=$VM_MEMORY"; fi
            if [[ -n "$VM_DISK" && "$VM_DISK" != "null" ]]; then echo "provider_disk=$VM_DISK"; fi
            if [[ -n "$VM_CPUS" && "$VM_CPUS" != "null" ]]; then echo "provider_cpus=$VM_CPUS"; fi
            if [[ -n "$VM_SWAP" && "$VM_SWAP" != "null" ]]; then echo "provider_swap=$VM_SWAP"; fi
        else
            echo "Warning: Provider '$PROVIDER_KEY' not found in $VM_SIZES_FILE" >&2
        fi
    else
        # Output all provider sizes for reference
        for provider in fly docker aws gcp azure digitalocean kubernetes; do
            SIZE=$(yq ".providers.${provider}.sizes.${VM_SIZE_TIER}" "$VM_SIZES_FILE" 2>/dev/null)
            if [[ "$SIZE" != "null" && -n "$SIZE" ]]; then
                echo "${provider}_vm_size=$SIZE"
            fi
        done
    fi
fi
