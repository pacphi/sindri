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

# Print a stage step with consistent formatting
# Usage: print_step <stage> <provider_prefix> <step_num> <description>
# Example: print_step 2 D 1 "Pre-cleanup" → "→ Step 2.D1: Pre-cleanup"
print_step() {
    local stage="$1"
    local provider="$2"
    local step="$3"
    local description="$4"
    echo "→ Step ${stage}.${provider}${step}: ${description}"
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
        # Security (M-4): Capture detailed error for logging, show generic message to user
        # Reference: https://cheatsheetseries.owasp.org/cheatsheets/Error_Handling_Cheat_Sheet.html
        local validation_output validation_error
        validation_output=$(python3 -c "
import json, sys
try:
    import jsonschema
    schema = json.load(open('$schema_file'))
    data = json.loads('''$json_data''')
    jsonschema.validate(data, schema)
    print('✓ Valid')
except jsonschema.ValidationError as e:
    # Print detailed error for logging (captured in bash)
    print(f'VALIDATION_ERROR: {e.message} at path: {list(e.path)}', file=sys.stderr)
    sys.exit(1)
except ImportError:
    print('⚠ jsonschema not installed, skipping validation', file=sys.stderr)
    sys.exit(0)
except Exception as e:
    print(f'VALIDATION_ERROR: Unexpected error: {str(e)}', file=sys.stderr)
    sys.exit(1)
" 2>&1)
        local exit_code=$?

        if [[ $exit_code -eq 0 ]]; then
            echo "$validation_output"
            return 0
        elif echo "$validation_output" | grep -q "VALIDATION_ERROR:"; then
            # Extract and log detailed error message
            validation_error=$(echo "$validation_output" | grep "VALIDATION_ERROR:" | sed 's/VALIDATION_ERROR: //')

            # Log detailed error to security log for diagnostics
            if command -v security_log_validation >/dev/null 2>&1; then
                security_log_validation "schema_validation" "failure" "$(basename "$yaml_file")" "$validation_error"
            fi

            # Show generic error message to user (OWASP best practice)
            print_error "✗ Configuration validation failed"
            print_error "   File: $(basename "$yaml_file")"
            print_error "   Check logs for details: \${WORKSPACE_LOGS:-/var/log}/sindri-security.log"

            return 1
        else
            # Handle other cases (e.g., jsonschema not installed)
            echo "$validation_output"
            return 0
        fi
    else
        print_warning "Python3 not available, skipping schema validation"
    fi

    return 0
}

# Check DNS resolution with configurable timeout
# Usage: check_dns <domain> [timeout_seconds]
# Default timeout: 3 seconds (down from 5+ sec default for nslookup/dig)
check_dns() {
    local domain="$1"
    local timeout_sec="${2:-${SINDRI_DNS_TIMEOUT:-3}}"  # Use env var or default to 3s

    if command_exists nslookup; then
        timeout "$timeout_sec" nslookup "$domain" >/dev/null 2>&1
    elif command_exists dig; then
        timeout "$timeout_sec" dig +short "$domain" >/dev/null 2>&1
    else
        # Fallback to ping with timeout
        ping -c 1 -W "$timeout_sec" "$domain" >/dev/null 2>&1
    fi
}

# =============================================================================
# DNS Caching Functions
# =============================================================================
# DNS cache using simple variables (compatible with bash 3.2+)
# Format: DNS_CACHE_<sanitized_domain>=<result>
# Prevents redundant DNS lookups across multiple extensions

# Sanitize domain name for use as variable name
# Replaces dots and hyphens with underscores
_dns_cache_key() {
    echo "$1" | tr '.-' '__'
}

# Check DNS with caching
# Usage: check_dns_cached <domain> [timeout_seconds]
# Returns cached result if available, otherwise performs check and caches
check_dns_cached() {
    local domain="$1"
    local timeout_sec="${2:-${SINDRI_DNS_TIMEOUT:-3}}"

    # Skip caching if explicitly disabled
    if [[ "${SINDRI_ENABLE_DNS_CACHE:-true}" != "true" ]]; then
        check_dns "$domain" "$timeout_sec"
        return $?
    fi

    # Get cache key (bash 3.2 compatible)
    local cache_key
    cache_key="DNS_CACHE_$(_dns_cache_key "$domain")"

    # Check cache first using indirect variable expansion
    local cached_value
    cached_value="${!cache_key:-}"

    if [[ -n "$cached_value" ]]; then
        [[ "${VERBOSE:-false}" == "true" ]] && echo "  (DNS cache hit: $domain)"
        return "$cached_value"
    fi

    # Perform DNS check and cache result
    if check_dns "$domain" "$timeout_sec"; then
        eval "${cache_key}=0"
        [[ "${VERBOSE:-false}" == "true" ]] && echo "  (DNS resolved: $domain)"
        return 0
    else
        eval "${cache_key}=1"
        [[ "${VERBOSE:-false}" == "true" ]] && echo "  (DNS failed: $domain)"
        return 1
    fi
}

# Pre-flight DNS check for all domains required by an extension
# Usage: preflight_dns_check <extension_yaml_path>
# Checks all domains listed in requirements.domains[] and caches results
preflight_dns_check() {
    local ext_yaml="$1"
    local ext_name
    ext_name=$(basename "$(dirname "$ext_yaml")")

    # Get domains from extension YAML
    local domains
    domains=$(load_yaml "$ext_yaml" '.requirements.domains[]' 2>/dev/null || echo "")

    if [[ -z "$domains" ]]; then
        [[ "${VERBOSE:-false}" == "true" ]] && echo "No DNS requirements for $ext_name"
        return 0
    fi

    print_status "Pre-flight DNS checks for $ext_name..."
    local all_resolved=true

    for domain in $domains; do
        if ! check_dns_cached "$domain" 3; then
            print_warning "Cannot resolve domain: $domain (required by $ext_name)"
            all_resolved=false
        fi
    done

    if [[ "$all_resolved" == "true" ]]; then
        [[ "${VERBOSE:-false}" == "true" ]] && print_success "All DNS requirements resolved for $ext_name"
        return 0
    else
        print_warning "Some DNS requirements not met for $ext_name (installation may fail or be slow)"
        return 0  # Don't fail, just warn
    fi
}

# Clear DNS cache (useful for testing or when network changes)
# Usage: clear_dns_cache
clear_dns_cache() {
    # Unset all DNS_CACHE_* variables
    for var in $(compgen -v | grep '^DNS_CACHE_'); do
        unset "$var"
    done
    [[ "${VERBOSE:-false}" == "true" ]] && echo "DNS cache cleared"
}

# =============================================================================
# End DNS Caching Functions
# =============================================================================

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

# Retry command with exponential backoff and jitter
# Usage: retry_command <max_attempts> <initial_delay> <command...>
# Jitter prevents thundering herd problem when multiple processes retry simultaneously
retry_command() {
    local max_attempts="${1:-3}"
    local delay="${2:-${SINDRI_RETRY_DELAY:-2}}"  # Use env var or default
    shift 2
    local cmd=("$@")

    local attempt=1
    local jitter=0
    local sleep_time=0

    while [[ $attempt -le $max_attempts ]]; do
        if "${cmd[@]}"; then
            return 0
        fi

        if [[ $attempt -lt $max_attempts ]]; then
            # Add jitter (0-3 seconds) to prevent synchronized retries
            # Security (M-5): Use /dev/urandom for cryptographically secure randomness
            # Reference: https://www.2uo.de/myths-about-urandom/
            jitter=$(($(od -An -N2 -i /dev/urandom 2>/dev/null || echo "$((RANDOM))") % 3))
            sleep_time=$((delay + jitter))
            print_warning "Command failed (attempt $attempt/$max_attempts), retrying in ${sleep_time}s (delay: ${delay}s + jitter: ${jitter}s)..."
            sleep "$sleep_time"
            # Exponential backoff: double the delay for next attempt
            delay=$((delay * 2))
        fi

        attempt=$((attempt + 1))
    done

    print_error "Command failed after $max_attempts attempts"
    return 1
}

# =============================================================================
# GitHub Release Version Detection Functions
# =============================================================================
# Standardized pattern for fetching latest GitHub release versions
# Uses gh CLI as primary method with curl fallback for reliability

# Get latest GitHub release version
# Usage: get_github_release_version <owner/repo> [include_v_prefix] [include_prereleases]
# Example: get_github_release_version "digitalocean/doctl" false false
# Example: get_github_release_version "pacphi/claude-code-agent-manager" true true
# Returns: version string (e.g., "1.2.3" or "v1.2.3" if include_v_prefix=true)
get_github_release_version() {
    local repo="$1"
    local include_v="${2:-false}"
    local include_prereleases="${3:-false}"
    local version=""

    # Method 1: gh CLI (handles auth automatically, avoids rate limits)
    if command_exists gh; then
        print_debug "Fetching version for $repo via gh CLI (prereleases=$include_prereleases)..."
        if [[ "$include_prereleases" == "true" ]]; then
            # Use gh release list to include prereleases, take the first (most recent)
            version=$(gh release list --repo "$repo" --limit 1 --json tagName --jq '.[0].tagName' 2>/dev/null || echo "")
        else
            # Use gh release view for latest stable release only
            version=$(gh release view --repo "$repo" --json tagName --jq '.tagName' 2>/dev/null || echo "")
        fi
        if [[ -n "$version" ]]; then
            print_debug "Got version $version via gh CLI"
        fi
    fi

    # Method 2: curl with GitHub API (fallback)
    if [[ -z "$version" ]]; then
        print_debug "Fetching version for $repo via GitHub API (prereleases=$include_prereleases)..."
        local curl_args=(-fsSL)
        if [[ -n "${GITHUB_TOKEN:-}" ]]; then
            curl_args+=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
        fi
        if [[ "$include_prereleases" == "true" ]]; then
            # Fetch all releases and take the first one (includes prereleases)
            version=$(curl "${curl_args[@]}" "https://api.github.com/repos/${repo}/releases" 2>/dev/null | \
                grep -o '"tag_name":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        else
            # Fetch only the latest stable release
            version=$(curl "${curl_args[@]}" "https://api.github.com/repos/${repo}/releases/latest" 2>/dev/null | \
                grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4 || echo "")
        fi
        if [[ -n "$version" ]]; then
            print_debug "Got version $version via GitHub API"
        fi
    fi

    # Strip 'v' prefix if not wanted
    if [[ "$include_v" != "true" ]] && [[ "$version" == v* ]]; then
        version="${version#v}"
    fi

    echo "$version"
}

# =============================================================================
# End GitHub Release Version Detection Functions
# =============================================================================

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

# =============================================================================
# Rate Limiting Functions (Security fix H-11)
# =============================================================================
# File-based rate limiting using flock for atomic operations
# Prevents DoS attacks via rapid extension install/uninstall

# Check rate limit for an operation
# Usage: check_rate_limit <operation> <max_attempts> <time_window_seconds>
# Returns: 0 if allowed, 1 if rate limited
check_rate_limit() {
    local operation="$1"
    local max_attempts="${2:-5}"        # Default: 5 attempts
    local time_window="${3:-300}"       # Default: 5 minutes (300 seconds)
    local rate_limit_dir="${WORKSPACE_SYSTEM:-/tmp}/.rate-limits"

    # Create rate limit directory if it doesn't exist
    mkdir -p "$rate_limit_dir" 2>/dev/null || true

    local rate_file="$rate_limit_dir/${operation}.lock"
    local count_file="$rate_limit_dir/${operation}.count"

    # Use flock for atomic file locking (prevents race conditions)
    (
        # Acquire exclusive lock with 200 file descriptor
        flock -x 200 || {
            print_warning "Could not acquire rate limit lock (skipping rate check)"
            return 0
        }

        local current_time
        current_time=$(date +%s)

        # Read existing count and timestamp (if file exists)
        local attempt_count=0
        local first_attempt_time=$current_time

        if [[ -f "$count_file" ]]; then
            local stored_data
            stored_data=$(cat "$count_file" 2>/dev/null || echo "0:$current_time")
            attempt_count=$(echo "$stored_data" | cut -d: -f1)
            first_attempt_time=$(echo "$stored_data" | cut -d: -f2)
        fi

        # Calculate time elapsed since first attempt in window
        local elapsed=$((current_time - first_attempt_time))

        # If time window has passed, reset counter
        if [[ $elapsed -gt $time_window ]]; then
            attempt_count=0
            first_attempt_time=$current_time
        fi

        # Increment attempt count
        attempt_count=$((attempt_count + 1))

        # Check if rate limit exceeded
        if [[ $attempt_count -gt $max_attempts ]]; then
            local remaining=$((time_window - elapsed))
            print_error "Rate limit exceeded for operation: $operation"
            print_status "Maximum $max_attempts attempts per $time_window seconds"
            print_status "Please wait $remaining seconds before trying again"
            return 1
        fi

        # Update count file
        echo "${attempt_count}:${first_attempt_time}" > "$count_file"

        [[ "${VERBOSE:-false}" == "true" ]] && \
            print_debug "Rate limit check: $operation ($attempt_count/$max_attempts)"

        return 0

    ) 200>"$rate_file"
}

# Clear rate limit for an operation (for testing or manual reset)
# Usage: clear_rate_limit <operation>
clear_rate_limit() {
    local operation="$1"
    local rate_limit_dir="${WORKSPACE_SYSTEM:-/tmp}/.rate-limits"

    rm -f "$rate_limit_dir/${operation}.lock" \
          "$rate_limit_dir/${operation}.count" 2>/dev/null || true

    [[ "${VERBOSE:-false}" == "true" ]] && \
        print_status "Rate limit cleared for: $operation"
}

# =============================================================================
# End Rate Limiting Functions
# =============================================================================

# =============================================================================
# Security Logging Functions (Security fix H-12)
# =============================================================================
# Comprehensive security event logging with syslog and local file support
# Following NIST SP 800-92 and OWASP logging guidelines

# Log security events with structured format
# Usage: security_log <event_type> <action> <result> [resource] [details]
# Example: security_log "auth" "ssh_key_setup" "success" "developer" "ED25519 key configured"
security_log() {
    local event_type="$1"      # auth, config, install, access, error
    local action="$2"           # What action was performed
    local result="$3"           # success, failure, denied
    local resource="${4:-}"     # What resource was affected (optional)
    local details="${5:-}"      # Additional details (optional)

    # ISO 8601 timestamp in UTC
    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null) || timestamp=$(date -Iseconds)

    # Get actor (current user)
    local actor="${USER:-unknown}"

    # Structured log format (key-value pairs for easy parsing)
    local log_entry="timestamp=$timestamp event_type=$event_type actor=$actor action=$action result=$result"

    if [[ -n "$resource" ]]; then
        log_entry="$log_entry resource=$resource"
    fi

    if [[ -n "$details" ]]; then
        # Escape quotes in details for safe logging
        local safe_details="${details//\"/\\\"}"
        log_entry="$log_entry details=\"$safe_details\""
    fi

    # Log to local security log file
    local security_log_file="${WORKSPACE_LOGS:-/var/log}/sindri-security.log"
    if [[ -w "$(dirname "$security_log_file")" ]] || [[ -w "$security_log_file" ]]; then
        echo "$log_entry" >> "$security_log_file" 2>/dev/null || true
    fi

    # Log to syslog if available (standard for SIEM integration)
    if command_exists logger; then
        # Use auth facility for security events, notice priority
        logger -t "sindri-security" -p auth.notice "$log_entry" 2>/dev/null || true
    fi

    # Also log to stderr for immediate visibility if verbose mode
    if [[ "${VERBOSE:-false}" == "true" ]] || [[ "$result" == "failure" ]] || [[ "$result" == "denied" ]]; then
        echo "[SECURITY] $log_entry" >&2
    fi
}

# Log authentication events
# Usage: security_log_auth <action> <result> [details]
security_log_auth() {
    security_log "auth" "$1" "$2" "${USER:-unknown}" "$3"
}

# Log configuration changes
# Usage: security_log_config <action> <result> <resource> [details]
security_log_config() {
    security_log "config" "$1" "$2" "$3" "$4"
}

# Log installation/extension events
# Usage: security_log_install <action> <result> <extension> [details]
security_log_install() {
    security_log "install" "$1" "$2" "$3" "$4"
}

# Log access control events
# Usage: security_log_access <action> <result> <resource> [details]
security_log_access() {
    security_log "access" "$1" "$2" "$3" "$4"
}

# Log validation events
# Usage: security_log_validation <action> <result> <resource> [details]
security_log_validation() {
    security_log "validation" "$1" "$2" "$3" "$4"
}

# =============================================================================
# End Security Logging Functions
# =============================================================================

# =============================================================================
# Disk Space Management
# =============================================================================

# Clean up APT caches and temporary files to free disk space
# This is critical on disk-constrained environments like Fly.io (8GB root FS)
# Usage: cleanup_apt_cache
cleanup_apt_cache() {
    print_status "Cleaning up APT caches to free disk space..."

    # Clean apt cache (safe to run without sudo if no packages to clean)
    if command_exists apt-get; then
        sudo apt-get clean 2>/dev/null || true
        sudo rm -rf /var/lib/apt/lists/* 2>/dev/null || true
        sudo rm -rf /var/cache/apt/archives/* 2>/dev/null || true
    fi

    # Clean up temporary files
    sudo rm -rf /tmp/* 2>/dev/null || true
    sudo rm -rf /var/tmp/* 2>/dev/null || true

    print_success "APT cache cleanup complete"
}

# =============================================================================
# End Disk Space Management
# =============================================================================

# Export functions for use in subshells
export -f print_status print_success print_warning print_error print_header print_step print_debug
export -f command_exists is_user ensure_directory
export -f load_yaml validate_yaml_schema check_dns check_disk_space retry_command cleanup_apt_cache
export -f get_github_release_version
export -f check_gpu_available get_gpu_count get_gpu_memory validate_gpu_config check_extension_gpu_requirements
export -f check_rate_limit clear_rate_limit
export -f security_log security_log_auth security_log_config security_log_install security_log_access
