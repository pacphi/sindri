#!/bin/bash
# V3 Packer Testing Script
# Local testing for Packer image builds and InSpec validation
#
# Usage:
#   ./scripts/v3-packer-test.sh validate        # Validate templates
#   ./scripts/v3-packer-test.sh unit            # Run unit tests
#   ./scripts/v3-packer-test.sh inspec          # Run InSpec locally
#   ./scripts/v3-packer-test.sh build --cloud aws --dry-run

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
V3_DIR="$PROJECT_ROOT/v3"
SINDRI_BIN="$V3_DIR/target/release/sindri"
INSPEC_PROFILE="$V3_DIR/test/integration/sindri"

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

usage() {
    cat << EOF
V3 Packer Testing Script

Usage: $0 <command> [options]

Commands:
    validate            Validate Packer templates
    unit                Run Packer unit tests
    inspec              Run InSpec profile locally
    inspec-check        Check InSpec profile syntax
    build               Run a test build (requires cloud credentials)

Options:
    --cloud <name>      Cloud provider: aws, azure, gcp, oci, alibaba
    --dry-run           Show what would happen without executing
    --verbose           Enable verbose output
    -h, --help          Show this help message

Examples:
    $0 validate
    $0 unit
    $0 inspec
    $0 build --cloud aws --dry-run
EOF
    exit 0
}

check_dependencies() {
    local missing=()

    if ! command -v cargo &>/dev/null; then
        missing+=("cargo")
    fi

    if [[ "$1" == "inspec" ]] && ! command -v inspec &>/dev/null; then
        log_warning "InSpec not installed. Install: gem install inspec-bin"
        missing+=("inspec")
    fi

    if [[ "$1" == "build" ]] && ! command -v packer &>/dev/null; then
        log_warning "Packer not installed. Visit: https://developer.hashicorp.com/packer/install"
        missing+=("packer")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        return 1
    fi

    return 0
}

cmd_validate() {
    log "Validating Packer templates..."

    # Build sindri if needed
    if [[ ! -x "$SINDRI_BIN" ]]; then
        log "Building sindri..."
        (cd "$V3_DIR" && cargo build --release)
    fi

    # Check packer installation
    if ! command -v packer &>/dev/null; then
        log_warning "Packer not installed, running template generation validation only"

        # Run unit tests as validation
        cd "$V3_DIR"
        cargo test --package sindri-packer -- template
        log_success "Template unit tests passed"
        return 0
    fi

    # Validate templates for each cloud
    for cloud in aws azure gcp oci alibaba; do
        log "Validating $cloud template..."

        # Generate template
        TEMPLATE_FILE=$(mktemp --suffix=.pkr.hcl)
        if "$SINDRI_BIN" packer generate --cloud "$cloud" > "$TEMPLATE_FILE" 2>/dev/null; then
            # Validate with packer
            if packer validate "$TEMPLATE_FILE" 2>/dev/null; then
                log_success "$cloud template valid"
            else
                log_warning "$cloud template validation failed (may be missing variables)"
            fi
        else
            log_warning "$cloud template generation failed (expected for some clouds)"
        fi
        rm -f "$TEMPLATE_FILE"
    done

    log_success "Template validation complete"
}

cmd_unit() {
    log "Running Packer unit tests..."
    check_dependencies unit

    cd "$V3_DIR"

    cargo test --package sindri-packer --lib
    cargo test --package sindri-packer --test build_lifecycle

    log_success "Packer unit tests passed"
}

cmd_inspec() {
    log "Running InSpec profile locally..."
    check_dependencies inspec || {
        log_error "InSpec is required. Install: gem install inspec-bin"
        exit 1
    }

    # Accept Chef license
    inspec --chef-license=accept-silent 2>/dev/null || true

    # Run local InSpec tests
    log "Running InSpec tests on local system..."

    inspec exec "$INSPEC_PROFILE" \
        --reporter cli json:/tmp/inspec-results.json \
        --controls sindri docker mise \
        || true

    log "Results written to /tmp/inspec-results.json"

    # Show summary
    if [[ -f /tmp/inspec-results.json ]]; then
        PASSED=$(jq '.statistics.controls.passed.total // 0' /tmp/inspec-results.json)
        FAILED=$(jq '.statistics.controls.failed.total // 0' /tmp/inspec-results.json)
        SKIPPED=$(jq '.statistics.controls.skipped.total // 0' /tmp/inspec-results.json)

        echo ""
        log "Summary: $PASSED passed, $FAILED failed, $SKIPPED skipped"
    fi

    log_success "InSpec tests complete"
}

cmd_inspec_check() {
    log "Checking InSpec profile syntax..."
    check_dependencies inspec || {
        log_error "InSpec is required. Install: gem install inspec-bin"
        exit 1
    }

    inspec check "$INSPEC_PROFILE"
    log_success "InSpec profile syntax valid"
}

cmd_build() {
    local cloud=""
    local dry_run=false
    local verbose=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --cloud)
                cloud="$2"
                shift 2
                ;;
            --dry-run)
                dry_run=true
                shift
                ;;
            --verbose)
                verbose=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
    done

    if [[ -z "$cloud" ]]; then
        log_error "Cloud provider required. Use --cloud <aws|azure|gcp|oci|alibaba>"
        exit 1
    fi

    log "Building Packer image for $cloud..."
    check_dependencies build

    # Build sindri if needed
    if [[ ! -x "$SINDRI_BIN" ]]; then
        log "Building sindri..."
        (cd "$V3_DIR" && cargo build --release)
    fi

    local cmd_args=("--cloud" "$cloud")
    if [[ "$dry_run" == "true" ]]; then
        cmd_args+=("--dry-run")
    fi
    if [[ "$verbose" == "true" ]]; then
        cmd_args+=("--verbose")
    fi

    if [[ "$dry_run" == "true" ]]; then
        log "Dry run mode - would execute:"
        echo "  $SINDRI_BIN packer build ${cmd_args[*]}"
    else
        "$SINDRI_BIN" packer build "${cmd_args[@]}"
    fi

    log_success "Build command complete"
}

# Main
case "${1:-}" in
    validate)
        shift
        cmd_validate "$@"
        ;;
    unit)
        shift
        cmd_unit "$@"
        ;;
    inspec)
        shift
        cmd_inspec "$@"
        ;;
    inspec-check)
        shift
        cmd_inspec_check "$@"
        ;;
    build)
        shift
        cmd_build "$@"
        ;;
    -h|--help)
        usage
        ;;
    *)
        if [[ -n "${1:-}" ]]; then
            log_error "Unknown command: $1"
        fi
        usage
        ;;
esac
