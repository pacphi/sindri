# Sindri Doctor - System Diagnostics

The `sindri doctor` command provides comprehensive system diagnostics, checking for all external tools that Sindri depends on. It helps users identify missing or misconfigured tools before encountering runtime errors.

## Overview

Sindri V3 relies on various external tools depending on which provider or commands you intend to use:

| Use Case                | Required Tools                                                        |
| ----------------------- | --------------------------------------------------------------------- |
| **Docker provider**     | `docker`, Docker Compose v2                                           |
| **Fly.io provider**     | `flyctl` CLI (with authentication)                                    |
| **DevPod provider**     | `devpod`                                                              |
| **E2B provider**        | `e2b` CLI (with authentication)                                       |
| **Kubernetes provider** | `kubectl` (with cluster access)                                       |
| **Project commands**    | `git`, optionally `gh` for GitHub workflows                           |
| **Extension system**    | `mise`, `npm`, `apt-get`, or `curl` depending on installation methods |
| **Secret management**   | `vault` (optional)                                                    |

The doctor command performs:

- **Parallel tool checking** - All tools checked concurrently for fast diagnostics (~500ms)
- **Version validation** - Ensures tools meet minimum version requirements
- **Authentication checks** - Verifies login status for tools requiring auth
- **Platform detection** - Identifies OS, architecture, and available package managers
- **Platform-specific installation instructions** - Copy-paste commands for your exact environment

## CLI Reference

### Basic Usage

```bash
# Check all tools
sindri doctor

# Check tools for a specific provider
sindri doctor --provider docker
sindri doctor --provider fly
sindri doctor --provider devpod

# Check tools for a specific command
sindri doctor --command project
sindri doctor --command extension

# Check everything
sindri doctor --all
```

### Command Options

| Flag                 | Short | Description                                                              |
| -------------------- | ----- | ------------------------------------------------------------------------ |
| `--provider <NAME>`  | `-p`  | Check tools for a specific provider (docker, fly, devpod, e2b, k8s)      |
| `--command <NAME>`   |       | Check tools for a specific command (project, extension, secrets, deploy) |
| `--all`              | `-a`  | Check all tools regardless of current usage                              |
| `--ci`               |       | Exit with non-zero code if required tools are missing (for CI/CD)        |
| `--format <FORMAT>`  |       | Output format: `human` (default), `json`, `yaml`                         |
| `--verbose-output`   |       | Show detailed information including timing                               |
| `--check-auth`       |       | Check authentication status for tools that require it                    |
| `--fix`              |       | Attempt to install missing tools automatically                           |
| `--yes`              | `-y`  | Skip confirmation prompts when installing (use with `--fix`)             |
| `--dry-run`          |       | Show what would be installed without executing (use with `--fix`)        |
| `--check-extensions` |       | Check tools required by installed extensions                             |
| `--extension <NAME>` |       | Check a specific extension's tool requirements                           |

## Checks Performed

### System Requirements

The doctor command detects:

- **Operating System**: macOS, Linux (with distribution detection), Windows
- **CPU Architecture**: x86_64, aarch64, arm
- **Linux Distributions**: Debian/Ubuntu, Fedora/RHEL/CentOS, Arch, Alpine, NixOS
- **Available Package Managers**: Homebrew, APT, DNF, Yum, Pacman, APK, Nix, Winget, Chocolatey, Scoop

### Tool Categories

Tools are organized into categories:

| Category                | Tools                            | Description                         |
| ----------------------- | -------------------------------- | ----------------------------------- |
| **Core**                | `git`                            | Required for all operations         |
| **Docker Provider**     | `docker`, `docker-compose`       | Container runtime and orchestration |
| **Fly.io Provider**     | `flyctl`                         | Fly.io platform CLI                 |
| **DevPod Provider**     | `devpod`                         | Dev environments as code            |
| **E2B Provider**        | `e2b`                            | Cloud sandbox CLI                   |
| **Kubernetes Provider** | `kubectl`                        | Kubernetes cluster management       |
| **Extension Backends**  | `mise`, `npm`, `apt-get`, `curl` | Extension installation methods      |
| **Secret Management**   | `vault`                          | HashiCorp Vault (optional)          |
| **Optional**            | `gh`                             | GitHub CLI for enhanced workflows   |

### Version Validation

Each tool has minimum version requirements:

| Tool           | Minimum Version |
| -------------- | --------------- |
| Git            | 2.0.0           |
| Docker         | 20.10.0         |
| Docker Compose | 2.0.0           |
| flyctl         | 0.1.0           |
| DevPod         | 0.4.0           |
| kubectl        | 1.20.0          |
| mise           | 2024.0.0        |
| npm            | 8.0.0           |
| Vault          | 1.10.0          |
| GitHub CLI     | 2.0.0           |

### Authentication Status

For tools requiring authentication, the doctor checks login status:

| Tool       | Auth Check Command     | Auth Command        |
| ---------- | ---------------------- | ------------------- |
| Docker     | `docker info`          | (daemon status)     |
| flyctl     | `flyctl auth whoami`   | `flyctl auth login` |
| E2B        | `e2b auth status`      | `e2b auth login`    |
| kubectl    | `kubectl cluster-info` | (kubeconfig)        |
| Vault      | `vault status`         | `vault login`       |
| GitHub CLI | `gh auth status`       | `gh auth login`     |

## Auto-Fix Mode

The `--fix` flag enables automatic installation of missing tools using your system's package manager.

### How It Works

1. **Detect missing tools** from the diagnostic results
2. **Select best installation method** based on available package managers
3. **Display installation command** and confirm (unless `--yes`)
4. **Execute installation** using the appropriate package manager
5. **Verify installation** by checking if tool is now in PATH
6. **Re-run diagnostic** to confirm success

### Usage Examples

```bash
# Interactive installation (prompts for confirmation)
sindri doctor --fix

# Non-interactive installation (for scripts)
sindri doctor --fix --yes

# Preview what would be installed
sindri doctor --fix --dry-run
```

### What Can Be Fixed

The auto-fix feature can install:

- **Package manager tools** (brew, apt-get, dnf, pacman, winget, etc.)
- **CLI tools** with standard installation commands
- **Cross-platform tools** via curl/wget installers

**Limitations:**

- Cannot fix tools requiring manual download (Docker Desktop on Windows/macOS)
- Cannot fix authentication issues (requires user interaction)
- Cannot fix version issues (only installs missing tools)

### Platform-Specific Installation

The installer selects the best installation method for your platform:

**macOS:**

```bash
brew install git
brew install --cask docker
brew install flyctl
```

**Debian/Ubuntu:**

```bash
sudo apt-get install git
curl -fsSL https://get.docker.com | sh
curl -L https://fly.io/install.sh | sh
```

**Fedora:**

```bash
sudo dnf install git
sudo dnf install docker-ce docker-ce-cli containerd.io
```

**Windows:**

```bash
winget install Git.Git
winget install loft-sh.devpod
```

## CI Mode

For continuous integration pipelines, use the `--ci` flag for machine-friendly output and exit codes.

### Exit Codes

| Code | Meaning                                                      |
| ---- | ------------------------------------------------------------ |
| 0    | All required tools available (optional tools may be missing) |
| 1    | Missing required tools                                       |
| 2    | Tools present but version too old                            |
| 3    | Tools present but not authenticated (when auth is required)  |

### CI Examples

**GitHub Actions:**

```yaml
jobs:
  check-tools:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check Sindri dependencies
        run: sindri doctor --ci --provider docker
```

**GitLab CI:**

```yaml
check-tools:
  script:
    - sindri doctor --ci --all
  allow_failure: false
```

**Shell Script:**

```bash
#!/bin/bash
if ! sindri doctor --ci --provider fly; then
  echo "Missing required tools for Fly.io deployment"
  exit 1
fi
```

### JSON Output for CI

Use `--format json` for structured output:

```bash
sindri doctor --ci --format json | jq '.overall_status'
```

Example JSON output:

```json
{
  "platform": {
    "os": "Linux/Debian",
    "arch": "x86_64",
    "package_managers": ["Homebrew", "APT"]
  },
  "tools": [
    {
      "id": "docker",
      "name": "Docker",
      "state": "available",
      "version": "24.0.7",
      "required": true,
      "auth_status": "authenticated",
      "check_duration_ms": 45
    }
  ],
  "overall_status": "ready",
  "missing_required_count": 0,
  "missing_optional_count": 1,
  "auth_required_count": 0
}
```

## Output Formats

### Human-Readable (Default)

```
Sindri Doctor
Platform: macOS (aarch64)
Package managers: Homebrew

Core Tools
  ✓ Git 2.43.0 - Distributed version control system

Docker Provider
  ✓ Docker 24.0.7 - Container runtime (authenticated)
  ✓ Docker Compose 2.23.0 - Multi-container orchestration

Optional Tools
  ✗ GitHub CLI - GitHub's official command-line tool
      Install: brew install gh
      Note: Then run: gh auth login

Summary
  ✓ Ready to use Sindri (1 optional tool(s) missing)
```

### JSON Format

```bash
sindri doctor --format json
```

### YAML Format

```bash
sindri doctor --format yaml
```

## Extension Tool Checking

Extensions can define their own tool requirements in `extension.yaml`:

```yaml
# ~/.sindri/extensions/my-extension/extension.yaml
validate:
  commands:
    - name: python
      versionFlag: "--version"
    - name: pip
      versionFlag: "--version"
```

### Check All Extension Tools

```bash
sindri doctor --check-extensions
```

Output:

```
Extension Tool Checks
────────────────────────────────────
Checked 4 tool(s) from 2 extension(s)

  ✓ python [my-extension] (Python 3.11.0)
  ✓ pip [my-extension] (pip 23.3.1)
  ✓ node [web-toolkit] (v20.10.0)
  ✗ yarn [web-toolkit]

1 extension tool(s) missing.
```

### Check Specific Extension

```bash
sindri doctor --extension my-extension
```

## Common Usage Scenarios

### Initial Setup

Check everything before first use:

```bash
sindri doctor --all --check-auth
```

### Before Deployment

Verify provider tools are ready:

```bash
# For Docker deployments
sindri doctor --provider docker --check-auth

# For Fly.io deployments
sindri doctor --provider fly --check-auth

# For Kubernetes deployments
sindri doctor --provider k8s --check-auth
```

### Troubleshooting Failures

Get verbose output with timing:

```bash
sindri doctor --all --verbose-output
```

### Automated Environment Setup

Non-interactive installation of missing tools:

```bash
sindri doctor --all --fix --yes
```

### Extension Development

Check tools needed by your extension:

```bash
sindri doctor --extension my-new-extension
```

## Troubleshooting

### Tool Not Found After Installation

1. **Restart your shell** - PATH changes may require a new terminal session
2. **Check PATH** - Ensure the tool's installation directory is in your PATH
3. **Verify installation** - Run the tool's version command manually

### Version Too Old

The doctor command compares versions using semver. If you see "version too old":

```
⚠ Docker 19.03.0 - version too old (required: 20.10.0+)
```

Upgrade the tool using your package manager:

```bash
# macOS
brew upgrade docker

# Debian/Ubuntu
sudo apt-get update && sudo apt-get upgrade docker-ce
```

### Authentication Failed

For tools showing "not authenticated":

```
  ✓ Fly CLI 0.1.130 - (not authenticated)
      Authenticate: flyctl auth login
```

Run the suggested authentication command and try again.

### Check Timeout

Tool checks have a 5-second timeout. If a check times out:

- Verify the tool is working correctly
- Check for network issues (some auth checks require network)
- Try running the tool manually

### Extension Not Found

If `--extension <name>` fails:

```
Error: Extension 'my-extension' not found
```

Verify the extension is installed in `~/.sindri/extensions/`:

```bash
ls ~/.sindri/extensions/
```

## Related Documentation

- [ADR-027: Tool Dependency Management System](/v3/docs/architecture/adr/027-tool-dependency-management-system.md)
- [Extension Development Guide](/v3/docs/extensions/DEVELOPMENT.md)
- [Provider Configuration](/v3/docs/PROVIDERS.md)
