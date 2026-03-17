#!/usr/bin/env bash
# ==============================================================================
# pkg-manager.sh Integration Tests
# ==============================================================================
# Docker-based tests that validate pkg-manager.sh against real distro containers.
# Each test spins up a container for the target distro, mounts the library,
# and exercises the functions in a realistic environment.
#
# Prerequisites:
#   - Docker daemon running
#   - Internet access (to pull base images on first run)
#
# Usage:
#   ./v3/tests/pkg-manager-test.sh              # Run all tests
#   ./v3/tests/pkg-manager-test.sh ubuntu       # Test one distro
#   ./v3/tests/pkg-manager-test.sh --quick      # Skip slow install tests
#
# The script exits 0 on success, 1 on any failure.
# ==============================================================================

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
LIB_PATH="${PROJECT_ROOT}/v3/docker/lib/pkg-manager.sh"

# Distro → Docker base image mapping
declare -A DISTRO_IMAGES=(
    [ubuntu]="ubuntu:24.04"
    [fedora]="fedora:41"
    [opensuse]="opensuse/leap:15.6"
)

# Colors
BOLD=$(tput bold 2>/dev/null || true)
RED=$(tput setaf 1 2>/dev/null || true)
GREEN=$(tput setaf 2 2>/dev/null || true)
YELLOW=$(tput setaf 3 2>/dev/null || true)
BLUE=$(tput setaf 4 2>/dev/null || true)
RESET=$(tput sgr0 2>/dev/null || true)

# Counters
PASSED=0
FAILED=0
SKIPPED=0

# ── Helpers ───────────────────────────────────────────────────────────────────

log_pass() { echo "  ${GREEN}PASS${RESET} $1"; ((PASSED++)); }
log_fail() { echo "  ${RED}FAIL${RESET} $1"; ((FAILED++)); }
log_skip() { echo "  ${YELLOW}SKIP${RESET} $1"; ((SKIPPED++)); }
log_section() { echo ""; echo "${BOLD}${BLUE}── $1 ──${RESET}"; }

# Run a command inside a distro container with pkg-manager.sh mounted.
# Usage: run_in_container <distro> <bash_commands>
run_in_container() {
    local distro="$1"
    shift
    local image="${DISTRO_IMAGES[$distro]}"

    docker run --rm \
        -v "${LIB_PATH}:/docker/lib/pkg-manager.sh:ro" \
        "${image}" \
        /bin/bash -c "source /docker/lib/pkg-manager.sh && $*" 2>&1
}

# Assert that a command's output equals expected.
# Usage: assert_eq <test_name> <actual> <expected>
assert_eq() {
    local name="$1" actual="$2" expected="$3"
    if [[ "${actual}" == "${expected}" ]]; then
        log_pass "${name}: got '${actual}'"
    else
        log_fail "${name}: expected '${expected}', got '${actual}'"
    fi
}

# Assert that a command exits successfully.
# Usage: assert_ok <test_name> <distro> <commands>
assert_ok() {
    local name="$1" distro="$2"
    shift 2
    if run_in_container "${distro}" "$@" >/dev/null 2>&1; then
        log_pass "${name}"
    else
        log_fail "${name}"
    fi
}

# Assert that a command exits with failure.
# Usage: assert_fail <test_name> <distro> <commands>
assert_fail() {
    local name="$1" distro="$2"
    shift 2
    if run_in_container "${distro}" "$@" >/dev/null 2>&1; then
        log_fail "${name} (expected failure, got success)"
    else
        log_pass "${name}"
    fi
}

# ── Test Suites ───────────────────────────────────────────────────────────────

test_detect_distro() {
    local distro="$1"
    log_section "detect_distro [${distro}]"

    # Basic detection
    local result
    result="$(run_in_container "${distro}" "detect_distro")"
    assert_eq "detects ${distro}" "${result}" "${distro}"
}

test_detect_distro_override() {
    local distro="$1"
    log_section "detect_distro override [${distro}]"

    # SINDRI_DISTRO override
    local result
    result="$(docker run --rm \
        -e SINDRI_DISTRO=fedora \
        -v "${LIB_PATH}:/docker/lib/pkg-manager.sh:ro" \
        "${DISTRO_IMAGES[$distro]}" \
        /bin/bash -c "source /docker/lib/pkg-manager.sh && detect_distro" 2>&1)"
    assert_eq "SINDRI_DISTRO override in ${distro} container" "${result}" "fedora"
}

test_detect_arch() {
    local distro="$1"
    log_section "detect_arch [${distro}]"

    local result
    result="$(run_in_container "${distro}" "detect_arch")"
    # Should be amd64 or arm64 depending on host
    local expected_arch
    case "$(uname -m)" in
        x86_64)   expected_arch="amd64" ;;
        aarch64|arm64) expected_arch="arm64" ;;
        *) expected_arch="unknown" ;;
    esac
    assert_eq "detects architecture" "${result}" "${expected_arch}"
}

test_pkg_name_mapping() {
    local distro="$1"
    log_section "pkg_name mapping [${distro}]"

    # build-essential
    local result
    result="$(run_in_container "${distro}" "pkg_name build-essential")"
    case "${distro}" in
        ubuntu)  assert_eq "build-essential → build-essential" "${result}" "build-essential" ;;
        fedora)  assert_eq "build-essential → @development-tools" "${result}" "@development-tools" ;;
        opensuse) assert_eq "build-essential → pattern devel_basis" "${result}" "-t pattern devel_basis" ;;
    esac

    # libssl-dev
    result="$(run_in_container "${distro}" "pkg_name libssl-dev")"
    case "${distro}" in
        ubuntu)  assert_eq "libssl-dev → libssl-dev" "${result}" "libssl-dev" ;;
        fedora)  assert_eq "libssl-dev → openssl-devel" "${result}" "openssl-devel" ;;
        opensuse) assert_eq "libssl-dev → libopenssl-devel" "${result}" "libopenssl-devel" ;;
    esac

    # Unknown package (passthrough)
    result="$(run_in_container "${distro}" "pkg_name curl")"
    assert_eq "curl → curl (passthrough)" "${result}" "curl"
}

test_pkg_update() {
    local distro="$1"
    log_section "pkg_update [${distro}]"

    assert_ok "package index update succeeds" "${distro}" "pkg_update"
}

test_pkg_install() {
    local distro="$1"
    log_section "pkg_install [${distro}]"

    # Install a small, fast package
    assert_ok "install curl" "${distro}" "pkg_update && pkg_install curl"
}

test_pkg_clean() {
    local distro="$1"
    log_section "pkg_clean [${distro}]"

    assert_ok "package cache cleanup succeeds" "${distro}" "pkg_update && pkg_clean"
}

test_pkg_install_sindri_base() {
    local distro="$1"
    log_section "pkg_install_sindri_base [${distro}] (SLOW ~1-2 min)"

    assert_ok "full base package install succeeds" "${distro}" "pkg_install_sindri_base"
}

test_unsupported_distro() {
    log_section "unsupported distro detection"

    # Use alpine which is not in the supported list
    local result
    if docker run --rm \
        -v "${LIB_PATH}:/docker/lib/pkg-manager.sh:ro" \
        alpine:latest \
        /bin/sh -c "apk add bash >/dev/null 2>&1 && /bin/bash -c 'source /docker/lib/pkg-manager.sh && detect_distro'" >/dev/null 2>&1; then
        log_fail "should reject unsupported distro (alpine)"
    else
        log_pass "rejects unsupported distro (alpine)"
    fi
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    local filter="${1:-all}"
    local quick=false

    if [[ "${filter}" == "--quick" ]]; then
        quick=true
        filter="all"
    fi

    echo "${BOLD}${BLUE}╔══════════════════════════════════════════════════════════════╗${RESET}"
    echo "${BOLD}${BLUE}║           pkg-manager.sh Integration Tests                   ║${RESET}"
    echo "${BOLD}${BLUE}╚══════════════════════════════════════════════════════════════╝${RESET}"
    echo ""

    # Verify prerequisites
    if ! command -v docker >/dev/null 2>&1; then
        echo "${RED}ERROR: Docker is required to run these tests${RESET}"
        exit 1
    fi

    if [[ ! -f "${LIB_PATH}" ]]; then
        echo "${RED}ERROR: pkg-manager.sh not found at ${LIB_PATH}${RESET}"
        exit 1
    fi

    # Determine which distros to test
    local distros=()
    if [[ "${filter}" == "all" ]]; then
        distros=(ubuntu fedora opensuse)
    else
        distros=("${filter}")
    fi

    # Pull base images in parallel (suppress output)
    echo "${BLUE}Pulling base images...${RESET}"
    for distro in "${distros[@]}"; do
        docker pull "${DISTRO_IMAGES[$distro]}" --quiet &
    done
    wait
    echo "${GREEN}Base images ready${RESET}"
    echo ""

    # Run tests for each distro
    for distro in "${distros[@]}"; do
        echo ""
        echo "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo "${BOLD}  Testing: ${distro} (${DISTRO_IMAGES[$distro]})${RESET}"
        echo "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

        test_detect_distro "${distro}"
        test_detect_distro_override "${distro}"
        test_detect_arch "${distro}"
        test_pkg_name_mapping "${distro}"

        if [[ "${quick}" == true ]]; then
            log_section "pkg_update [${distro}]"
            log_skip "skipped (--quick mode)"
            log_section "pkg_install [${distro}]"
            log_skip "skipped (--quick mode)"
            log_section "pkg_clean [${distro}]"
            log_skip "skipped (--quick mode)"
            log_section "pkg_install_sindri_base [${distro}]"
            log_skip "skipped (--quick mode)"
        else
            test_pkg_update "${distro}"
            test_pkg_install "${distro}"
            test_pkg_clean "${distro}"
            test_pkg_install_sindri_base "${distro}"
        fi
    done

    # Unsupported distro test
    test_unsupported_distro

    # Summary
    echo ""
    echo "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo "${BOLD}  Results${RESET}"
    echo "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
    echo "  ${GREEN}Passed:${RESET}  ${PASSED}"
    echo "  ${RED}Failed:${RESET}  ${FAILED}"
    echo "  ${YELLOW}Skipped:${RESET} ${SKIPPED}"
    echo ""

    if [[ ${FAILED} -gt 0 ]]; then
        echo "${RED}${BOLD}FAILED${RESET} — ${FAILED} test(s) failed"
        exit 1
    else
        echo "${GREEN}${BOLD}ALL TESTS PASSED${RESET}"
        exit 0
    fi
}

main "$@"
