#!/bin/bash
# common.sh - Shared utilities for Sindri v3 extensions
# Simplified version for v3 - focused on essential extension script functions

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

# Print functions for consistent output formatting
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

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
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

# Retry command with exponential backoff
# Usage: retry_command <max_attempts> <initial_delay> <command...>
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

# Get latest GitHub release version
# Usage: get_github_release_version <owner/repo> [include_v_prefix] [include_prereleases]
# Returns: version string (e.g., "1.2.3" or "v1.2.3" if include_v_prefix=true)
# When include_prereleases=true, returns the most recent release including prereleases
get_github_release_version() {
    local repo="$1"
    local include_v="${2:-false}"
    local include_prereleases="${3:-false}"
    local version=""

    # Method 1: gh CLI (preferred)
    if command_exists gh; then
        if [[ "$include_prereleases" == "true" ]]; then
            # List all releases (including prereleases) and take the first (most recent)
            version=$(gh release list --repo "$repo" --limit 1 --json tagName --jq '.[0].tagName' 2>/dev/null || echo "")
        else
            version=$(gh release view --repo "$repo" --json tagName --jq '.tagName' 2>/dev/null || echo "")
        fi
    fi

    # Method 2: curl with GitHub API (fallback)
    if [[ -z "$version" ]]; then
        local curl_args=(-fsSL)
        if [[ -n "${GITHUB_TOKEN:-}" ]]; then
            curl_args+=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
        fi
        if [[ "$include_prereleases" == "true" ]]; then
            # Use /releases endpoint (includes prereleases), take the first result
            version=$(curl "${curl_args[@]}" "https://api.github.com/repos/${repo}/releases?per_page=1" 2>/dev/null | \
                grep -o '"tag_name":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        else
            version=$(curl "${curl_args[@]}" "https://api.github.com/repos/${repo}/releases/latest" 2>/dev/null | \
                grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4 || echo "")
        fi
    fi

    # Strip 'v' prefix if not wanted
    if [[ "$include_v" != "true" ]] && [[ "$version" == v* ]]; then
        version="${version#v}"
    fi

    echo "$version"
}

# Clean up APT caches and temporary files
# Usage: cleanup_apt_cache
cleanup_apt_cache() {
    print_debug "Cleaning up APT caches to free disk space..."

    # Clean apt cache (handle sudo failure gracefully for no-new-privileges containers)
    if command_exists apt-get; then
        sudo apt-get clean 2>/dev/null || true
        sudo rm -rf /var/lib/apt/lists/* 2>/dev/null || true
        sudo rm -rf /var/cache/apt/archives/* 2>/dev/null || true
    fi

    # Clean up temporary files (safe without sudo)
    rm -rf /tmp/*.tar.gz /tmp/*.zip /tmp/*.deb 2>/dev/null || true
}

# Export functions for use in subshells
export -f print_status print_success print_warning print_error print_header print_debug print_info
export -f command_exists is_user ensure_directory retry_command get_github_release_version cleanup_apt_cache
