#!/usr/bin/env bash
# test/unit/yaml/test-domain-requirements.sh
# Validates extension domain requirements
#
# Validates:
# 1. Domain format is valid (hostname syntax)
# 2. No duplicate domain entries
# 3. Domains are reachable via DNS (optional, controlled by VALIDATE_DNS)
# 4. Heuristic detection of undeclared domains in scripts
#
# Environment variables:
#   VALIDATE_DNS=true|false  - Enable DNS resolution checks (default: false in CI)
#   DNS_TIMEOUT=N            - DNS lookup timeout in seconds (default: 3)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
EXTENSIONS_DIR="$PROJECT_ROOT/docker/lib/extensions"

# Configuration
VALIDATE_DNS="${VALIDATE_DNS:-false}"
DNS_TIMEOUT="${DNS_TIMEOUT:-3}"

# Counters
ERRORS=0
WARNINGS=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

log_error() {
    echo -e "${RED}ERROR:${NC} $1"
    ((ERRORS++)) || true
}

log_warning() {
    echo -e "${YELLOW}WARN:${NC} $1"
    ((WARNINGS++)) || true
}

log_success() {
    echo -e "${GREEN}OK:${NC} $1"
}

log_info() {
    echo "  $1"
}

# Check if required tools are available
if ! command -v yq &> /dev/null; then
    echo -e "${RED}ERROR: yq is required but not installed${NC}"
    exit 1
fi

cd "$PROJECT_ROOT"

echo "=== Extension Domain Requirements Validation ==="
echo ""
echo "Configuration:"
echo "  VALIDATE_DNS: $VALIDATE_DNS"
echo "  DNS_TIMEOUT: ${DNS_TIMEOUT}s"
echo ""

# Validate domain format (RFC 1123 hostname)
# Returns 0 if valid, 1 if invalid
validate_domain_format() {
    local domain="$1"

    # Must not be empty
    [[ -z "$domain" ]] && return 1

    # Must not start or end with hyphen or dot
    [[ "$domain" =~ ^[-.]|[-.]$ ]] && return 1

    # Must contain at least one dot (TLD required)
    [[ ! "$domain" =~ \. ]] && return 1

    # Basic hostname validation: alphanumeric, hyphens, and dots
    # Each label: 1-63 chars, starts/ends with alphanumeric
    if [[ ! "$domain" =~ ^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$ ]]; then
        return 1
    fi

    return 0
}

# Check DNS resolution for a domain
check_dns_resolution() {
    local domain="$1"
    local timeout="${2:-$DNS_TIMEOUT}"

    # Try getent first (most common)
    if timeout "$timeout" getent hosts "$domain" >/dev/null 2>&1; then
        return 0
    fi

    # Fallback to host command
    if command -v host &>/dev/null; then
        if timeout "$timeout" host -W "$timeout" "$domain" >/dev/null 2>&1; then
            return 0
        fi
    fi

    # Fallback to nslookup
    if command -v nslookup &>/dev/null; then
        if timeout "$timeout" nslookup -timeout="$timeout" "$domain" >/dev/null 2>&1; then
            return 0
        fi
    fi

    return 1
}

# Extract domain-like patterns from extension files (heuristic)
extract_referenced_domains() {
    local ext_dir="$1"
    # Pattern for real domains - require at least 2 chars before TLD and proper TLD
    # Exclude .sh to avoid matching script filenames
    local domain_pattern='[a-zA-Z0-9][-a-zA-Z0-9]+\.(com|org|io|dev|net|ai|cloud|app|run)'

    # Search install scripts and config files for domain patterns
    # Use nullglob behavior with explicit file existence check
    shopt -s nullglob
    local files=("$ext_dir"/*.sh "$ext_dir"/*.toml "$ext_dir"/*.yaml "$ext_dir"/*.json)
    shopt -u nullglob

    for file in "${files[@]}"; do
        [[ -f "$file" ]] || continue
        # Skip the extension.yaml itself for this check
        [[ "$(basename "$file")" == "extension.yaml" ]] && continue

        # Extract domain patterns
        grep -oE "$domain_pattern" "$file" 2>/dev/null || true
    done | sort -u
}

# ============================================================================
# Test 1: Domain Format Validation
# ============================================================================
echo "## Test 1: Domain Format Validation"
format_errors=0

for ext_dir in "$EXTENSIONS_DIR"/*/; do
    [[ -f "$ext_dir/extension.yaml" ]] || continue
    ext_name=$(basename "$ext_dir")

    # Get domains array
    domains=$(yq -r '.requirements.domains[]? // empty' "$ext_dir/extension.yaml" 2>/dev/null || true)

    for domain in $domains; do
        [[ -z "$domain" ]] && continue

        if ! validate_domain_format "$domain"; then
            log_error "$ext_name: Invalid domain format: '$domain'"
            ((format_errors++)) || true
        fi
    done
done

if [[ $format_errors -eq 0 ]]; then
    echo -e "${GREEN}  All domain formats valid${NC}"
fi
echo ""

# ============================================================================
# Test 2: Duplicate Domain Detection
# ============================================================================
echo "## Test 2: Duplicate Domain Detection"
dup_errors=0

for ext_dir in "$EXTENSIONS_DIR"/*/; do
    [[ -f "$ext_dir/extension.yaml" ]] || continue
    ext_name=$(basename "$ext_dir")

    # Get domains and check for duplicates
    domains=$(yq -r '.requirements.domains[]? // empty' "$ext_dir/extension.yaml" 2>/dev/null || true)

    if [[ -n "$domains" ]]; then
        duplicates=$(echo "$domains" | sort | uniq -d)
        if [[ -n "$duplicates" ]]; then
            for dup in $duplicates; do
                log_error "$ext_name: Duplicate domain entry: '$dup'"
                ((dup_errors++)) || true
            done
        fi
    fi
done

if [[ $dup_errors -eq 0 ]]; then
    echo -e "${GREEN}  No duplicate domains found${NC}"
fi
echo ""

# ============================================================================
# Test 3: DNS Resolution (optional)
# ============================================================================
if [[ "$VALIDATE_DNS" == "true" ]]; then
    echo "## Test 3: DNS Resolution Check"

    # Collect all unique domains first, then check each once
    all_domains=""
    for ext_dir in "$EXTENSIONS_DIR"/*/; do
        [[ -f "$ext_dir/extension.yaml" ]] || continue
        domains=$(yq -r '.requirements.domains[]? // empty' "$ext_dir/extension.yaml" 2>/dev/null || true)
        for domain in $domains; do
            [[ -n "$domain" ]] && all_domains="$all_domains $domain"
        done
    done

    # Get unique domains and check each
    dns_warnings=0
    unique_domains=$(echo "$all_domains" | tr ' ' '\n' | sort -u)

    for domain in $unique_domains; do
        [[ -z "$domain" ]] && continue

        if check_dns_resolution "$domain"; then
            log_info "DNS OK: $domain"
        else
            log_warning "DNS failed: $domain"
            ((dns_warnings++)) || true
        fi
    done

    if [[ $dns_warnings -eq 0 ]]; then
        echo -e "${GREEN}  All domains resolve${NC}"
    fi
    echo ""
else
    echo "## Test 3: DNS Resolution Check (SKIPPED)"
    echo "  Set VALIDATE_DNS=true to enable"
    echo ""
fi

# ============================================================================
# Test 4: Undeclared Domain Detection (heuristic)
# ============================================================================
echo "## Test 4: Undeclared Domain Detection (heuristic)"
undeclared_warnings=0

for ext_dir in "$EXTENSIONS_DIR"/*/; do
    [[ -f "$ext_dir/extension.yaml" ]] || continue
    ext_name=$(basename "$ext_dir")

    # Get declared domains
    declared=$(yq -r '.requirements.domains[]? // empty' "$ext_dir/extension.yaml" 2>/dev/null | sort -u || true)

    # Get referenced domains from scripts/configs
    referenced=$(extract_referenced_domains "$ext_dir")

    # Check each referenced domain
    for ref_domain in $referenced; do
        [[ -z "$ref_domain" ]] && continue

        # Skip common false positives and non-domains
        case "$ref_domain" in
            # Documentation/placeholder domains
            example.com|example.org|localhost) continue ;;

            # References in comments/docs, not actual domains
            Fly.io|fly.dev) continue ;;

            # Package manager false positives (version specifiers)
            *.co) continue ;;

            # Generic patterns that aren't real requirements
            *.local|*.test|*.invalid) continue ;;
        esac

        # Check if domain or parent domain is declared
        found=false
        for decl_domain in $declared; do
            # Exact match
            if [[ "$ref_domain" == "$decl_domain" ]]; then
                found=true
                break
            fi
            # Subdomain match (e.g., raw.githubusercontent.com matches github.com check)
            if [[ "$ref_domain" == *"$decl_domain" ]]; then
                found=true
                break
            fi
        done

        if [[ "$found" == "false" ]]; then
            log_warning "$ext_name: Domain '$ref_domain' found in scripts but not in requirements.domains"
            ((undeclared_warnings++)) || true
        fi
    done
done

if [[ $undeclared_warnings -eq 0 ]]; then
    echo -e "${GREEN}  No undeclared domains detected${NC}"
fi
echo ""

# ============================================================================
# Summary
# ============================================================================
echo "================================"
echo "Summary:"
echo "  Errors:   $ERRORS"
echo "  Warnings: $WARNINGS"
echo ""

if [[ $ERRORS -gt 0 ]]; then
    echo -e "${RED}Domain validation FAILED: $ERRORS error(s)${NC}"
    exit 1
fi

if [[ $WARNINGS -gt 0 ]]; then
    echo -e "${YELLOW}Domain validation PASSED with $WARNINGS warning(s)${NC}"
else
    echo -e "${GREEN}Domain validation PASSED${NC}"
fi

exit 0
