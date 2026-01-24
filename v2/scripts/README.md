# Sindri Scripts

This directory contains utility scripts for Sindri administration.

## setup-sysbox-host.sh

**Purpose:** Install Sysbox container runtime on the HOST machine for secure Docker-in-Docker support.

### When to Run This Script

Run this script **on the host machine** (not inside Sindri) when you want to enable secure Docker-in-Docker without privileged mode.

| Environment              | Run This Script? | Notes                          |
| ------------------------ | ---------------- | ------------------------------ |
| Local Linux machine      | ✅ Yes           | One-time setup                 |
| AWS EC2 / Azure VM / GCP | ✅ Yes           | Run via SSH                    |
| Self-hosted CI runners   | ✅ Yes           | Add to runner setup            |
| Fly.io                   | ❌ No            | Use `mode: privileged` instead |
| GitHub-hosted runners    | ❌ No            | Use `mode: privileged` instead |
| macOS / Windows          | ❌ No            | Sysbox is Linux-only           |

### Usage

```bash
# Install latest Sysbox version
./scripts/setup-sysbox-host.sh

# Install specific version
./scripts/setup-sysbox-host.sh --version v0.6.7

# Show help
./scripts/setup-sysbox-host.sh --help
```

### Requirements

- Ubuntu 18.04-24.04 or Debian 10-11
- Docker installed (not via snap)
- Linux kernel 5.12+ (5.19+ recommended)
- Root/sudo access
- gh CLI (optional, but recommended for avoiding rate limits)

### What It Does

1. Checks prerequisites (OS, kernel, Docker)
2. Downloads Sysbox package from GitHub releases
3. Stops running containers (required by installer)
4. Installs Sysbox package
5. Verifies installation
6. Registers `sysbox-runc` runtime with Docker

### After Installation

Configure Sindri to use Sysbox:

```yaml
# sindri.yaml
providers:
  docker:
    dind:
      enabled: true
      mode: sysbox # or "auto" to auto-detect
```

### Verification

```bash
# Check if Sysbox is registered
docker info | grep sysbox-runc

# Test Sysbox container
docker run --rm --runtime=sysbox-runc alpine echo "Sysbox works!"
```

### More Information

- [Docker Extension Documentation](../../docs/extensions/DOCKER.md)
- [Sysbox GitHub](https://github.com/nestybox/sysbox)
- [Sysbox DinD Guide](https://github.com/nestybox/sysbox/blob/master/docs/user-guide/dind.md)
