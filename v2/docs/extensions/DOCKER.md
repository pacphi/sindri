# Docker

Docker Engine and Compose with Docker-in-Docker (DinD) support.

## Overview

| Property         | Value          |
| ---------------- | -------------- |
| **Category**     | infrastructure |
| **Version**      | 1.1.0          |
| **Installation** | apt + script   |
| **Disk Space**   | 1000 MB        |
| **Dependencies** | None           |

## Description

Docker Engine and Compose - provides containerization capabilities with Docker CE, Docker CLI, containerd, Docker Compose plugin, and fuse-overlayfs for Docker-in-Docker scenarios.

## Installed Tools

| Tool             | Type     | Description                            |
| ---------------- | -------- | -------------------------------------- |
| `docker`         | server   | Docker daemon and CLI                  |
| `docker-compose` | cli-tool | Multi-container orchestration          |
| `containerd`     | server   | Container runtime                      |
| `fuse-overlayfs` | utility  | FUSE-based overlay filesystem for DinD |

## Configuration

### Environment Variables

| Variable          | Value | Scope  |
| ----------------- | ----- | ------ |
| `DOCKER_BUILDKIT` | `1`   | bashrc |

### APT Repository

```yaml
repositories:
  - gpgKey: https://download.docker.com/linux/ubuntu/gpg
    sources: deb [arch=amd64] https://download.docker.com/linux/ubuntu jammy stable
```

## Network Requirements

- `download.docker.com` - Docker packages
- `hub.docker.com` - Docker Hub

## Installation

```bash
extension-manager install docker
```

## Validation

```bash
docker --version    # Expected: Docker version X.X.X
docker compose version
```

## Removal

### Requires confirmation

```bash
extension-manager remove docker
```

Removes Docker CE, CLI, containerd, compose plugin, and fuse-overlayfs.

---

## Docker-in-Docker (DinD) Support

Sindri v1.1.0+ provides comprehensive Docker-in-Docker support for running containerized workloads that need to build or run their own containers.

### Quick Start: Which Mode Should I Use?

```text
┌───────────────────────────────────────────────────────────────────────┐
│                    DinD MODE DECISION TREE                            │
├───────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  Can you install software on the HOST machine?                        │
│       │                                                               │
│       ├── YES ──► Install Sysbox for best security & performance      │
│       │           Run: ./scripts/setup-sysbox-host.sh                 │
│       │           Then use: mode: sysbox (or auto)                    │
│       │                                                               │
│       └── NO ───► Are you okay with privileged containers?            │
│                       │                                               │
│                       ├── YES ──► Use: mode: privileged               │
│                       │           (Works everywhere, less secure)     │
│                       │                                               │
│                       └── NO ───► Use: mode: socket                   │
│                                   (Shares host Docker, less isolated) │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

### Important: Host Setup for Sysbox Mode

**Sysbox is a container runtime that must be installed on the HOST machine**, not inside the Sindri container. This is a one-time setup per host.

**When to run the setup script:**

- Local development machine (macOS is NOT supported - use privileged mode)
- Cloud VMs (EC2, Azure VM, GCP Compute Engine)
- CI runners you control
- Kubernetes nodes

**When you CANNOT use Sysbox:**

- Fly.io (use privileged mode instead)
- Managed Kubernetes (EKS, AKS, GKE) without node access
- CI services you don't control (GitHub-hosted runners)
- macOS or Windows hosts

**Setup command:**

```bash
# Run on the HOST machine (not inside Sindri)
./scripts/setup-sysbox-host.sh

# Verify installation
docker info | grep sysbox-runc
```

### DinD Mode Comparison

| Mode           | Security       | Performance     | Host Setup | Use Case                           |
| -------------- | -------------- | --------------- | ---------- | ---------------------------------- |
| **sysbox**     | Excellent      | Native overlay2 | Required   | Production CI/CD, multi-tenant     |
| **privileged** | Poor           | Slower (vfs)    | None       | Quick testing, single-tenant       |
| **socket**     | Shared         | Native          | None       | Simple builds, shared environments |
| **auto**       | Best available | Varies          | Optional   | Recommended default                |

### Sysbox Mode (Recommended)

Sysbox provides secure DinD without privileged containers by using Linux user namespaces for isolation.

**Prerequisites:**

- Sysbox installed on host machine
- Ubuntu/Debian with kernel 5.12+ (5.19+ optimal)

**Host Setup:**

```bash
./scripts/setup-sysbox-host.sh
```

**Configuration:**

```yaml
# sindri.yaml
providers:
  docker:
    dind:
      enabled: true
      mode: sysbox
```

**Benefits:**

- No privileged mode required
- Native overlay2 storage driver (full performance)
- Full systemd support inside containers
- User-namespace isolation (root in container = unprivileged on host)

### Privileged Mode (Legacy Fallback)

For hosts without Sysbox, privileged mode provides DinD capability at the cost of security.

**Configuration:**

```yaml
# sindri.yaml
providers:
  docker:
    privileged: true
    dind:
      enabled: true
      mode: privileged
      storageDriver: vfs # or auto, fuse-overlayfs
      storageSize: 30GB
```

**Limitations:**

- Container has nearly full host access (security risk)
- Uses vfs storage driver (no copy-on-write, slower)
- Not suitable for multi-tenant environments

### Socket Binding Mode

Share the host's Docker daemon instead of running a nested daemon.

**Configuration:**

```yaml
# sindri.yaml
providers:
  docker:
    dind:
      enabled: true
      mode: socket
```

**How it works:**

- Mounts `/var/run/docker.sock` from host
- Docker commands use host's daemon
- No nested daemon, no storage driver issues

**Limitations:**

- Containers visible on host (less isolation)
- Cannot run different Docker versions
- Actions affect host Docker environment

### Auto Mode (Recommended)

Automatically detects and uses the best available DinD mode.

**Configuration:**

```yaml
# sindri.yaml
providers:
  docker:
    dind:
      enabled: true
      mode: auto # Default
```

**Detection Order:**

1. Sysbox (if `sysbox-runc` available)
2. Privileged (if `privileged: true` set)
3. Warning (DinD may not work)

### Storage Driver Configuration

When using privileged mode, you can configure the inner Docker's storage driver:

| Driver           | Performance | Compatibility | When to Use                      |
| ---------------- | ----------- | ------------- | -------------------------------- |
| `auto`           | Varies      | Best          | Let Sindri choose (default)      |
| `overlay2`       | Best        | Limited       | Only if /var/lib/docker on ext4  |
| `fuse-overlayfs` | Good        | Good          | Nested without privileged volume |
| `vfs`            | Slowest     | Universal     | Always works, fallback option    |

**Configuration:**

```yaml
providers:
  docker:
    dind:
      enabled: true
      mode: privileged
      storageDriver: auto # or overlay2, fuse-overlayfs, vfs
      storageSize: 20GB # Storage limit for vfs driver
```

### Example Configurations

See `examples/v2/docker/` for complete configurations:

- `dind-sysbox.sindri.yaml` - Secure DinD with Sysbox
- `dind-privileged.sindri.yaml` - Legacy privileged DinD
- `dind-socket.sindri.yaml` - Socket binding mode
- `devops.sindri.yaml` - Auto-detection (recommended)

### Troubleshooting DinD

**"failed to mount overlay filesystem"**

- Cause: overlay2 driver can't run nested on CoW filesystems
- Solution: Use Sysbox mode, or configure `storageDriver: vfs`

**Docker daemon not starting**

- Check: `sudo docker info` inside container
- Logs: `~/.local/state/dockerd.log`
- Verify DinD mode: `echo $SINDRI_DIND_MODE`

**Slow container builds**

- Cause: vfs storage driver (no copy-on-write)
- Solution: Install Sysbox for native overlay2 support

**Sysbox not detected**

- Verify on host: `docker info | grep sysbox-runc`
- Install: `./scripts/setup-sysbox-host.sh`

---

## Appendix A: DinD Research and Design Rationale

This appendix documents the research and citations that informed the Docker-in-Docker implementation in Sindri.

### Problem Statement

Running Docker inside a Sindri container presents challenges:

1. **Overlay Filesystem Failure**: `failed to mount overlay filesystem - invalid argument`
   - Root cause: overlay2 cannot run on top of another overlay2 filesystem
   - Impact: Complete failure of docker-compose and containerized services

2. **Storage Driver Conflicts**: Nested copy-on-write filesystems cause unpredictable behavior
   - AUFS on AUFS: Not supported
   - overlay2 on overlay2: Fails with "invalid argument"
   - Device Mapper: Complex configuration required

3. **Security Concerns**: Traditional DinD requires privileged mode
   - Full host access from container
   - Potential container escape vulnerabilities
   - Not suitable for multi-tenant environments

### Research Sources

#### Docker Official Documentation

- [OverlayFS Storage Driver](https://docs.docker.com/engine/storage/drivers/overlayfs-driver/)
  - overlay2 requires kernel 4.0+ and xfs/ext4 with d_type enabled
  - Recommended for most workloads

- [VFS Storage Driver](https://docs.docker.com/storage/storagedriver/vfs-driver/)
  - "Primarily intended for debugging purposes"
  - No copy-on-write (full layer copies)
  - Works on all filesystems

- [Select a Storage Driver](https://docs.docker.com/engine/storage/drivers/select-storage-driver/)
  - "Nested storage drivers can conflict; prefer overlay2 and test thoroughly"

- [Docker Seccomp Profiles](https://docs.docker.com/engine/security/seccomp/)
  - Default profile blocks mount syscalls
  - seccomp:unconfined required for DinD

#### Sysbox by Nestybox (Acquired by Docker 2022)

- [Sysbox GitHub Repository](https://github.com/nestybox/sysbox)
  - "Open-source container runtime that empowers rootless containers"
  - Kernel-level user namespace isolation
  - No VMs, pure Linux namespaces

- [Sysbox DinD Documentation](https://github.com/nestybox/sysbox/blob/master/docs/user-guide/dind.md)
  - "Docker-in-Docker without privileged mode or socket binding"
  - Inner Docker isolated from host
  - Standard overlay2 works natively

- [Sysbox Installation Guide](https://github.com/nestybox/sysbox/blob/master/docs/user-guide/install-package.md)
  - Supports Ubuntu 18.04-24.04, Debian 10-11
  - Kernel 5.12+ recommended (5.19+ optimal)
  - systemd required

- [Sysbox v0.6.7 Release](https://github.com/nestybox/sysbox/releases/tag/v0.6.7)
  - binfmt_misc namespacing (kernel 6.7+)
  - Fixed FUSE device unmounting
  - Kubernetes 1.32 support

#### Community Resources

- [Docker-in-Docker GitHub Issue #144](https://github.com/nestybox/sysbox/issues/144)
  - Docker Compose runtime configuration
  - `runtime: sysbox-runc` in compose files (v1.27+)

- [fuse-overlayfs](https://github.com/containers/fuse-overlayfs)
  - FUSE implementation for rootless containers
  - Fallback when kernel overlay not available

- [Coder Docker in Workspaces](https://coder.com/docs/admin/templates/extending-templates/docker-in-workspaces)
  - Four methods: Sysbox, Envbox, Podman, Privileged sidecar
  - Sysbox recommended for security

- [Baeldung: Why DinD Not Recommended](https://www.baeldung.com/ops/docker-in-docker)
  - Storage driver conflicts
  - Security implications
  - Socket binding as alternative

### Solution Selection Rationale

**Three-Tier Strategy:**

1. **Sysbox (Primary)**: Provides the best security and performance
   - User-namespace isolation without VMs
   - Native overlay2 support inside containers
   - Docker acquisition ensures long-term support

2. **Privileged + VFS (Fallback)**: Universal compatibility
   - Works without host modifications
   - vfs driver has no filesystem requirements
   - Acceptable for development/testing

3. **Socket Binding (Alternative)**: Simplest approach
   - No storage driver issues
   - Best performance (uses host daemon)
   - Trade-off: reduced isolation

**Storage Driver Selection:**

| Environment             | Recommended Driver | Rationale                          |
| ----------------------- | ------------------ | ---------------------------------- |
| Sysbox                  | overlay2           | Native support via user namespaces |
| Privileged + volume     | overlay2           | Volume provides real filesystem    |
| Privileged + overlay fs | fuse-overlayfs     | FUSE bypass for nested overlay     |
| Privileged (fallback)   | vfs                | Universal compatibility            |

### Version History

- **v1.0.0**: Initial Docker extension (basic DinD via privileged mode)
- **v1.1.0**: Comprehensive DinD support with Sysbox, auto-detection, fuse-overlayfs

### References

1. Docker Documentation. "OverlayFS storage driver." https://docs.docker.com/engine/storage/drivers/overlayfs-driver/
2. Docker Documentation. "VFS storage driver." https://docs.docker.com/storage/storagedriver/vfs-driver/
3. Docker Documentation. "Seccomp security profiles." https://docs.docker.com/engine/security/seccomp/
4. Nestybox. "Sysbox: An open-source container runtime." https://github.com/nestybox/sysbox
5. Nestybox. "Sysbox User Guide: Docker-in-Docker." https://github.com/nestybox/sysbox/blob/master/docs/user-guide/dind.md
6. Containers. "fuse-overlayfs." https://github.com/containers/fuse-overlayfs
7. Coder. "Docker in Workspaces." https://coder.com/docs/admin/templates/extending-templates/docker-in-workspaces
8. Baeldung. "Why Is Running Docker Inside Docker Not Recommended?" https://www.baeldung.com/ops/docker-in-docker
9. Nestybox Blog. "Comparison: Sysbox and Related Technologies." https://blog.nestybox.com/2020/10/06/related-tech-comparison.html
10. Docker Hub. "docker - Official Image (dind)." https://hub.docker.com/_/docker
