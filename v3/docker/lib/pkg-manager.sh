#!/usr/bin/env bash
# ==============================================================================
# pkg-manager.sh — Multi-Distro Package Manager Abstraction
# ==============================================================================
# Provides distro-agnostic functions for package installation, detection,
# and system management across Ubuntu, Fedora, and openSUSE.
#
# Supported distros:
#   - ubuntu  (apt-get)
#   - fedora  (dnf)
#   - opensuse (zypper)
#
# Usage:
#   source /docker/lib/pkg-manager.sh
#   detect_distro   # → "ubuntu" | "fedora" | "opensuse"
#   detect_arch     # → "amd64" | "arm64"
#   pkg_update
#   pkg_install curl wget git
#   pkg_clean
#
# Environment:
#   SINDRI_DISTRO — Override auto-detection (for testing or build args)
# ==============================================================================

set -euo pipefail

# ==============================================================================
# Distro & Architecture Detection
# ==============================================================================

# Detect the running Linux distribution.
# Returns: "ubuntu", "fedora", or "opensuse"
# Exits 1 if the distro is unrecognized.
detect_distro() {
    # Allow override via environment variable (used during Docker builds)
    if [[ -n "${SINDRI_DISTRO:-}" ]]; then
        echo "${SINDRI_DISTRO}"
        return 0
    fi

    if [[ ! -f /etc/os-release ]]; then
        echo "ERROR: /etc/os-release not found — cannot detect distro" >&2
        return 1
    fi

    # shellcheck disable=SC1091
    source /etc/os-release

    case "${ID:-}" in
        ubuntu)
            echo "ubuntu"
            ;;
        fedora)
            echo "fedora"
            ;;
        opensuse-leap|opensuse-tumbleweed|opensuse)
            echo "opensuse"
            ;;
        *)
            echo "ERROR: Unsupported distro: ${ID:-unknown} (PRETTY_NAME=${PRETTY_NAME:-?})" >&2
            return 1
            ;;
    esac
}

# Detect the system architecture in Docker/OCI nomenclature.
# Returns: "amd64" or "arm64"
detect_arch() {
    local machine
    machine="$(uname -m)"

    case "${machine}" in
        x86_64)
            echo "amd64"
            ;;
        aarch64|arm64)
            echo "arm64"
            ;;
        *)
            echo "ERROR: Unsupported architecture: ${machine}" >&2
            return 1
            ;;
    esac
}

# ==============================================================================
# Package Manager Operations
# ==============================================================================

# Update the package index.
pkg_update() {
    local distro
    distro="$(detect_distro)"

    case "${distro}" in
        ubuntu)
            apt-get update
            ;;
        fedora)
            dnf makecache --refresh
            ;;
        opensuse)
            zypper --non-interactive refresh
            ;;
    esac
}

# Install one or more packages.
# Usage: pkg_install curl wget git
pkg_install() {
    local distro
    distro="$(detect_distro)"

    case "${distro}" in
        ubuntu)
            DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "$@"
            ;;
        fedora)
            dnf install -y --setopt=install_weak_deps=False "$@"
            ;;
        opensuse)
            zypper --non-interactive install --no-recommends "$@"
            ;;
    esac
}

# Clean the package manager cache to reduce image size.
pkg_clean() {
    local distro
    distro="$(detect_distro)"

    case "${distro}" in
        ubuntu)
            apt-get clean
            rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*
            ;;
        fedora)
            dnf clean all
            rm -rf /var/cache/dnf /tmp/* /var/tmp/*
            ;;
        opensuse)
            zypper clean --all
            rm -rf /var/cache/zypp /tmp/* /var/tmp/*
            ;;
    esac
}

# ==============================================================================
# Distro-Specific Package Name Mapping
# ==============================================================================
# Some packages have different names across distros. This function maps
# a canonical package name to the distro-specific equivalent.

# Map a canonical package name to the distro-specific name.
# Usage: pkg_name build-essential  →  "build-essential" | "@development-tools" | "patterns-devel-base"
pkg_name() {
    local canonical="$1"
    local distro
    distro="$(detect_distro)"

    case "${canonical}" in
        build-essential)
            case "${distro}" in
                ubuntu)  echo "build-essential" ;;
                fedora)  echo "@development-tools" ;;
                opensuse) echo "-t pattern devel_basis" ;;
            esac
            ;;
        libssl-dev)
            case "${distro}" in
                ubuntu)  echo "libssl-dev" ;;
                fedora)  echo "openssl-devel" ;;
                opensuse) echo "libopenssl-devel" ;;
            esac
            ;;
        pkg-config)
            case "${distro}" in
                ubuntu)  echo "pkg-config" ;;
                fedora)  echo "pkgconf-pkg-config" ;;
                opensuse) echo "pkg-config" ;;
            esac
            ;;
        *)
            # Most packages share the same name across distros
            echo "${canonical}"
            ;;
    esac
}

# Install the common Sindri system packages (distro-aware).
# This replaces the inline apt-get install blocks in Dockerfiles.
pkg_install_sindri_base() {
    local distro
    distro="$(detect_distro)"

    pkg_update

    case "${distro}" in
        ubuntu)
            DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
                ca-certificates curl wget git openssh-server sudo \
                locales tzdata fontconfig unzip zip \
                build-essential pkg-config libssl-dev \
                gnupg \
                iproute2 \
                nano vim
            # Upgrade libgnutls (Ubuntu-specific security fix)
            apt-get install -y --only-upgrade libgnutls30t64 2>/dev/null || true
            # Generate locale
            locale-gen en_US.UTF-8
            update-locale LANG=en_US.UTF-8
            ;;
        fedora)
            dnf install -y --setopt=install_weak_deps=False \
                ca-certificates curl wget git openssh-server sudo \
                glibc-langpack-en tzdata fontconfig unzip zip \
                gcc gcc-c++ make pkgconf-pkg-config openssl-devel \
                gnupg2 \
                iproute \
                nano vim-enhanced \
                which findutils procps-ng passwd
            ;;
        opensuse)
            zypper --non-interactive install --no-recommends \
                ca-certificates curl wget git openssh sudo \
                glibc-locale timezone fontconfig unzip zip \
                tar gzip xz bzip2 \
                which file findutils \
                gcc gcc-c++ make pkg-config libopenssl-devel \
                gpg2 \
                iproute2 \
                nano vim \
                shadow gawk procps
            ;;
    esac

    # Clean SSH host keys (regenerated at runtime)
    rm -f /etc/ssh/ssh_host_*

    pkg_clean
}

# ==============================================================================
# User Management (distro-aware)
# ==============================================================================

# Create the developer user with sudo access.
# Usage: create_developer_user <username> <uid> <gid> <home_dir>
create_developer_user() {
    local username="$1"
    local uid="$2"
    local gid="$3"
    local home_dir="$4"
    local distro
    distro="$(detect_distro)"

    groupadd -g "${gid}" "${username}" 2>/dev/null || true

    case "${distro}" in
        ubuntu|opensuse)
            useradd -m -d "${home_dir}" -u "${uid}" -g "${gid}" -s /bin/bash "${username}" 2>/dev/null || true
            ;;
        fedora)
            useradd -m -d "${home_dir}" -u "${uid}" -g "${gid}" -s /bin/bash "${username}" 2>/dev/null || true
            ;;
    esac

    echo "${username} ALL=(ALL) NOPASSWD:ALL" > "/etc/sudoers.d/${username}"
    chmod 0440 "/etc/sudoers.d/${username}"
}

# ==============================================================================
# GitHub CLI Download (architecture-aware)
# ==============================================================================

# Download and install the GitHub CLI.
# Usage: install_gh_cli <version>
install_gh_cli() {
    local version="$1"
    local arch
    arch="$(detect_arch)"

    wget -qO /tmp/gh.tar.gz \
        "https://github.com/cli/cli/releases/download/v${version}/gh_${version}_linux_${arch}.tar.gz"
    tar -xzf /tmp/gh.tar.gz -C /tmp
    mv "/tmp/gh_${version}_linux_${arch}/bin/gh" /usr/local/bin/gh
    chmod +x /usr/local/bin/gh
    rm -rf /tmp/gh.tar.gz "/tmp/gh_${version}_linux_${arch}"
}

# ==============================================================================
# Cosign Download (architecture-aware)
# ==============================================================================

# Download and install Cosign.
# Usage: install_cosign <version>
install_cosign() {
    local version="$1"
    local arch
    arch="$(detect_arch)"

    wget -qO /usr/local/bin/cosign \
        "https://github.com/sigstore/cosign/releases/download/v${version}/cosign-linux-${arch}"
    chmod +x /usr/local/bin/cosign
}
