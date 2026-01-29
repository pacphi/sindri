#!/bin/bash
# V3 Extension Testing Script
# Local testing for extension lifecycle validation
#
# Usage:
#   ./scripts/v3-extension-test.sh --scheme serial --extensions "python,nodejs"
#   ./scripts/v3-extension-test.sh --scheme parallel --max-parallel 4
#   ./scripts/v3-extension-test.sh --profile minimal

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Defaults
SCHEME="serial"
EXTENSIONS=""
PROFILE=""
MAX_PARALLEL=2
VERBOSE=false
DRY_RUN=false

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
V3_DIR="$PROJECT_ROOT/v3"
SINDRI_BIN="$V3_DIR/target/release/sindri"

# Heavy extensions to exclude from parallel testing
HEAVY_EXTENSIONS="cuda ollama android-sdk pytorch tensorflow"

usage() {
    cat << EOF
V3 Extension Testing Script

Usage: $0 [OPTIONS]

Options:
    --scheme <serial|parallel>   Execution scheme (default: serial)
    --extensions <list>          Comma-separated list of extensions
    --profile <name>             Profile name: minimal, ai-dev, full
    --max-parallel <n>           Max parallel tests (default: 2)
    --verbose                    Enable verbose output
    --dry-run                    Show what would be tested without running
    -h, --help                   Show this help message

Profiles:
    minimal     Basic extensions: python, nodejs, golang
    ai-dev      AI development: python, nodejs, claude-code, cursor
    full        All non-heavy extensions

Examples:
    $0 --scheme serial --extensions "python,nodejs"
    $0 --scheme parallel --profile minimal --max-parallel 4
    $0 --profile ai-dev --verbose
EOF
    exit 0
}

log() {
    echo -e "${BLUE}[$(date +%H:%M:%S)]${NC} $*"
}

log_success() {
    echo -e "${GREEN}✓${NC} $*"
}

log_error() {
    echo -e "${RED}✗${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $*"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --scheme)
            SCHEME="$2"
            shift 2
            ;;
        --extensions)
            EXTENSIONS="$2"
            shift 2
            ;;
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        --max-parallel)
            MAX_PARALLEL="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Expand profile to extensions
expand_profile() {
    local profile="$1"
    case "$profile" in
        minimal)
            echo "python,nodejs,golang"
            ;;
        ai-dev)
            echo "python,nodejs,claude-code,cursor"
            ;;
        full)
            # Find all extensions, exclude heavy ones
            local all_ext=""
            if [[ -d "$V3_DIR/extensions" ]]; then
                all_ext=$(find "$V3_DIR/extensions" -name "extension.yaml" -exec dirname {} \; | xargs -n1 basename | tr '\n' ',')
                all_ext="${all_ext%,}"
            fi
            # Filter heavy extensions
            for heavy in $HEAVY_EXTENSIONS; do
                all_ext=$(echo "$all_ext" | sed "s/,$heavy,/,/g" | sed "s/^$heavy,//" | sed "s/,$heavy$//")
            done
            echo "$all_ext"
            ;;
        *)
            echo "$profile"
            ;;
    esac
}

# Resolve extensions from profile or list
if [[ -n "$PROFILE" ]]; then
    EXTENSIONS=$(expand_profile "$PROFILE")
fi

if [[ -z "$EXTENSIONS" ]]; then
    log_error "No extensions specified. Use --extensions or --profile"
    exit 1
fi

# Convert to array
IFS=',' read -ra EXT_ARRAY <<< "$EXTENSIONS"
EXT_COUNT=${#EXT_ARRAY[@]}

log "V3 Extension Testing"
log "===================="
log "Scheme: $SCHEME"
log "Extensions: ${EXT_ARRAY[*]}"
log "Count: $EXT_COUNT"
if [[ "$SCHEME" == "parallel" ]]; then
    log "Max Parallel: $MAX_PARALLEL"
fi

# Check for sindri binary
if [[ ! -x "$SINDRI_BIN" ]]; then
    log_warning "Sindri binary not found, building..."
    if [[ "$DRY_RUN" == "false" ]]; then
        (cd "$V3_DIR" && cargo build --release)
    fi
fi

if [[ "$DRY_RUN" == "true" ]]; then
    log "Dry run mode - would test the following extensions:"
    for ext in "${EXT_ARRAY[@]}"; do
        echo "  - $ext"
    done
    exit 0
fi

# Results tracking
PASSED=0
FAILED=0
RESULTS=()

test_extension() {
    local ext="$1"
    local start_time=$(date +%s)
    local result="success"

    log "Testing extension: $ext"

    # Install
    if [[ "$VERBOSE" == "true" ]]; then
        "$SINDRI_BIN" extension install "$ext" --yes 2>&1 || result="install_failed"
    else
        "$SINDRI_BIN" extension install "$ext" --yes >/dev/null 2>&1 || result="install_failed"
    fi

    # Validate (only if install succeeded)
    if [[ "$result" == "success" ]]; then
        if [[ "$VERBOSE" == "true" ]]; then
            "$SINDRI_BIN" extension validate "$ext" 2>&1 || result="validate_failed"
        else
            "$SINDRI_BIN" extension validate "$ext" >/dev/null 2>&1 || result="validate_failed"
        fi
    fi

    # Remove (cleanup)
    if [[ "$VERBOSE" == "true" ]]; then
        "$SINDRI_BIN" extension remove "$ext" --yes 2>&1 || true
    else
        "$SINDRI_BIN" extension remove "$ext" --yes >/dev/null 2>&1 || true
    fi

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    if [[ "$result" == "success" ]]; then
        log_success "$ext completed in ${duration}s"
        PASSED=$((PASSED + 1))
    else
        log_error "$ext failed: $result"
        FAILED=$((FAILED + 1))
    fi

    RESULTS+=("$ext:$result:${duration}s")
}

# Serial testing
run_serial() {
    log "Running serial tests..."
    for ext in "${EXT_ARRAY[@]}"; do
        test_extension "$ext"
    done
}

# Parallel testing (using background jobs)
run_parallel() {
    log "Running parallel tests (max $MAX_PARALLEL concurrent)..."

    local running=0
    local pids=()

    for ext in "${EXT_ARRAY[@]}"; do
        # Wait if we've hit max parallel
        while [[ $running -ge $MAX_PARALLEL ]]; do
            for i in "${!pids[@]}"; do
                if ! kill -0 "${pids[$i]}" 2>/dev/null; then
                    wait "${pids[$i]}" || true
                    unset 'pids[$i]'
                    running=$((running - 1))
                fi
            done
            sleep 1
        done

        # Start background test
        (test_extension "$ext") &
        pids+=($!)
        running=$((running + 1))
    done

    # Wait for all remaining jobs
    for pid in "${pids[@]}"; do
        wait "$pid" || true
    done
}

# Main execution
echo ""
if [[ "$SCHEME" == "parallel" ]]; then
    run_parallel
else
    run_serial
fi

# Summary
echo ""
log "===================="
log "Test Summary"
log "===================="
log "Passed: $PASSED"
log "Failed: $FAILED"
log "Total: $EXT_COUNT"

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi

exit 0
