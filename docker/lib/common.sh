#!/bin/bash
# common.sh - Shared utilities for Sindri

# Prevent multiple sourcing
if [[ "${COMMON_SH_LOADED:-}" == "true" ]]; then
    return 0
fi
COMMON_SH_LOADED="true"

# Colors for output
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export CYAN='\033[0;36m'
export NC='\033[0m'

# Directory paths (support both container and local execution)
if [[ -z "${DOCKER_LIB:-}" ]]; then
    if [[ -d "/docker/lib" ]]; then
        export DOCKER_LIB="/docker/lib"
    else
        # Assume we're being sourced from within /docker/lib
        DOCKER_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        export DOCKER_LIB
    fi
fi
export EXTENSIONS_DIR="$DOCKER_LIB/extensions"
export SCHEMAS_DIR="$DOCKER_LIB/schemas"
export TEMPLATES_DIR="$DOCKER_LIB/templates"

# Workspace paths (derived from HOME, volume mount is at $HOME)
# New architecture: $HOME = /alt/home/developer (volume), $WORKSPACE = $HOME/workspace
if [[ -z "${WORKSPACE:-}" ]]; then
    if [[ -n "${HOME:-}" ]] && [[ -d "${HOME}/workspace" ]]; then
        export WORKSPACE="${HOME}/workspace"
    elif [[ -d "/alt/home/developer/workspace" ]]; then
        export WORKSPACE="/alt/home/developer/workspace"
    elif [[ -d "/workspace" ]]; then
        # Backward compatibility for legacy deployments
        export WORKSPACE="/workspace"
    else
        # For local testing, use a workspace directory in home
        export WORKSPACE="${HOME:-/alt/home/developer}/workspace"
        mkdir -p "$WORKSPACE" 2>/dev/null || true
    fi
fi
export WORKSPACE_PROJECTS="${WORKSPACE_PROJECTS:-$WORKSPACE/projects}"
export WORKSPACE_CONFIG="$WORKSPACE/config"
export WORKSPACE_SCRIPTS="$WORKSPACE/scripts"
export WORKSPACE_BIN="$WORKSPACE/bin"
export WORKSPACE_SYSTEM="$WORKSPACE/.system"
export WORKSPACE_MANIFEST="$WORKSPACE_SYSTEM/manifest"
export WORKSPACE_LOGS="$WORKSPACE_SYSTEM/logs"

# Print functions
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

print_debug() {
    if [[ "${DEBUG:-}" == "true" ]]; then
        echo -e "${CYAN}[DEBUG]${NC} $1"
    fi
}

print_header() {
    echo -e "${CYAN}==>${NC} ${1}"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if running as specific user
is_user() {
    [[ "${USER:-}" == "$1" ]] || [[ "${USER:-}" == "root" ]]
}

# Ensure directory exists with proper ownership
ensure_directory() {
    local dir="$1"
    local owner="${2:-developer:developer}"

    if [[ ! -d "$dir" ]]; then
        mkdir -p "$dir"
        if [[ "${USER:-}" == "root" ]]; then
            chown -R "$owner" "$dir"
        fi
    fi
}

# Load YAML file (requires yq)
load_yaml() {
    local yaml_file="$1"
    local query="${2:-.}"

    if ! command_exists yq; then
        print_error "yq is required for YAML parsing"
        return 1
    fi

    yq eval "$query" "$yaml_file"
}

# Validate YAML against JSON schema
validate_yaml_schema() {
    local yaml_file="$1"
    local schema_file="$2"

    if ! command_exists yq; then
        print_error "yq is required for validation"
        return 1
    fi

    # Convert YAML to JSON for validation
    local json_data
    json_data=$(yq eval -o=json "$yaml_file")

    # Use Python's jsonschema if available
    if command_exists python3; then
        python3 -c "
import json, sys
try:
    import jsonschema
    schema = json.load(open('$schema_file'))
    data = json.loads('''$json_data''')
    jsonschema.validate(data, schema)
    print('✓ Valid')
except jsonschema.ValidationError as e:
    print(f'✗ Validation error: {e.message}', file=sys.stderr)
    sys.exit(1)
except ImportError:
    print('⚠ jsonschema not installed, skipping validation', file=sys.stderr)
    sys.exit(0)
" || return 1
    else
        print_warning "Python3 not available, skipping schema validation"
    fi

    return 0
}

# Check DNS resolution
check_dns() {
    local domain="$1"

    if command_exists nslookup; then
        nslookup "$domain" >/dev/null 2>&1
    elif command_exists dig; then
        dig +short "$domain" >/dev/null 2>&1
    else
        # Fallback to ping
        ping -c 1 -W 2 "$domain" >/dev/null 2>&1
    fi
}

# Check disk space (in MB)
check_disk_space() {
    local required_mb="$1"
    local mount_point="${2:-$WORKSPACE}"

    local available_mb
    available_mb=$(df -BM "$mount_point" | awk 'NR==2 {print $4}' | sed 's/M//')

    if [[ $available_mb -lt $required_mb ]]; then
        print_error "Insufficient disk space: ${available_mb}MB available, ${required_mb}MB required"
        return 1
    fi

    return 0
}

# Retry command with exponential backoff
retry_command() {
    local max_attempts="${1:-3}"
    local delay="${2:-2}"
    shift 2
    local cmd=("$@")

    local attempt=1
    while [[ $attempt -le $max_attempts ]]; do
        if "${cmd[@]}"; then
            return 0
        fi

        if [[ $attempt -lt $max_attempts ]]; then
            print_warning "Command failed (attempt $attempt/$max_attempts), retrying in ${delay}s..."
            sleep "$delay"
            delay=$((delay * 2))
        fi

        attempt=$((attempt + 1))
    done

    print_error "Command failed after $max_attempts attempts"
    return 1
}

# =============================================================================
# GPU Detection and Validation Functions
# =============================================================================

# Check if GPU is available on host
# Usage: check_gpu_available [gpu_type]
# gpu_type: nvidia (default), amd
# shellcheck disable=SC2120  # Function accepts optional argument with default
check_gpu_available() {
    local gpu_type="${1:-nvidia}"

    if [[ "$gpu_type" == "nvidia" ]]; then
        if command_exists nvidia-smi; then
            if nvidia-smi &>/dev/null; then
                return 0
            fi
        fi
    elif [[ "$gpu_type" == "amd" ]]; then
        if command_exists rocm-smi; then
            if rocm-smi &>/dev/null; then
                return 0
            fi
        fi
    fi

    return 1
}

# Get GPU count on host
# Usage: get_gpu_count [gpu_type]
get_gpu_count() {
    local gpu_type="${1:-nvidia}"

    if [[ "$gpu_type" == "nvidia" ]]; then
        if command_exists nvidia-smi; then
            nvidia-smi --list-gpus 2>/dev/null | wc -l
        else
            echo "0"
        fi
    elif [[ "$gpu_type" == "amd" ]]; then
        if command_exists rocm-smi; then
            rocm-smi --showproductname 2>/dev/null | grep -c "GPU" || echo "0"
        else
            echo "0"
        fi
    else
        echo "0"
    fi
}

# Get GPU memory in MB
# Usage: get_gpu_memory [gpu_type]
get_gpu_memory() {
    local gpu_type="${1:-nvidia}"

    if [[ "$gpu_type" == "nvidia" ]]; then
        if command_exists nvidia-smi; then
            nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits 2>/dev/null | head -1
        else
            echo "0"
        fi
    else
        echo "0"
    fi
}

# Validate GPU configuration for provider
# Usage: validate_gpu_config provider gpu_enabled gpu_tier [region]
validate_gpu_config() {
    local provider="$1"
    local gpu_enabled="$2"
    # shellcheck disable=SC2034  # gpu_tier reserved for future tier-specific validation
    local gpu_tier="${3:-gpu-small}"
    local region="${4:-}"

    if [[ "$gpu_enabled" != "true" ]]; then
        return 0
    fi

    case "$provider" in
        docker|docker-compose)
            # shellcheck disable=SC2119  # check_gpu_available uses default argument
            if ! check_gpu_available; then
                print_error "GPU requested but no NVIDIA GPU detected on host"
                print_status "Install nvidia-container-toolkit for GPU support"
                return 1
            fi
            ;;
        fly)
            local gpu_regions=("ord" "sjc")
            if [[ -n "$region" ]]; then
                local valid=false
                for r in "${gpu_regions[@]}"; do
                    if [[ "$region" == "$r" ]]; then
                        valid=true
                        break
                    fi
                done
                if [[ "$valid" != "true" ]]; then
                    print_warning "GPU may not be available in Fly.io region: $region"
                    print_status "GPU-enabled regions: ${gpu_regions[*]}"
                fi
            fi
            ;;
        devpod|aws|gcp|azure|kubernetes)
            # DevPod GPU validation depends on provider type
            print_status "GPU support validated at deployment time by cloud provider"
            ;;
        *)
            print_warning "Unknown provider: $provider - GPU support not validated"
            ;;
    esac

    return 0
}

# Check extension GPU requirements against deployment
# Usage: check_extension_gpu_requirements extension_dir gpu_enabled gpu_count
check_extension_gpu_requirements() {
    local extension_dir="$1"
    local gpu_enabled="$2"
    local gpu_count="${3:-0}"

    local ext_yaml="$extension_dir/extension.yaml"
    if [[ ! -f "$ext_yaml" ]]; then
        return 0
    fi

    local gpu_required
    local gpu_min_count
    gpu_required=$(yq '.requirements.gpu.required // false' "$ext_yaml" 2>/dev/null)
    gpu_min_count=$(yq '.requirements.gpu.minCount // 1' "$ext_yaml" 2>/dev/null)

    if [[ "$gpu_required" == "true" ]] && [[ "$gpu_enabled" != "true" ]]; then
        local ext_name
        ext_name=$(yq '.metadata.name' "$ext_yaml")
        print_error "Extension '$ext_name' requires GPU but GPU is not enabled"
        return 1
    fi

    if [[ "$gpu_required" == "true" ]] && [[ "$gpu_count" -lt "$gpu_min_count" ]]; then
        local ext_name
        ext_name=$(yq '.metadata.name' "$ext_yaml")
        print_error "Extension '$ext_name' requires $gpu_min_count GPUs but only $gpu_count configured"
        return 1
    fi

    return 0
}

# Export functions for use in subshells
export -f print_status print_success print_warning print_error print_header
export -f command_exists is_user ensure_directory
export -f load_yaml validate_yaml_schema check_dns check_disk_space retry_command
export -f check_gpu_available get_gpu_count get_gpu_memory validate_gpu_config check_extension_gpu_requirements
