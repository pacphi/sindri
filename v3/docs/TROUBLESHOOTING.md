# Sindri V3 Troubleshooting Guide

> **Version:** 3.0.0
> **Last Updated:** January 2026

This guide helps you diagnose and resolve common issues with Sindri V3.

## Table of Contents

- [Overview](#overview)
- [Using sindri doctor](#using-sindri-doctor)
- [Common Issues](#common-issues)
  - [Installation Problems](#installation-problems)
  - [Configuration Issues](#configuration-issues)
  - [Extension Problems](#extension-problems)
  - [Provider-Specific Issues](#provider-specific-issues)
  - [Permission Problems](#permission-problems)
- [Debugging](#debugging)
- [Getting Help](#getting-help)

---

## Overview

When troubleshooting Sindri V3, follow this general approach:

1. **Run diagnostics** - Use `sindri doctor` to identify missing tools or configuration problems
2. **Check configuration** - Validate your `sindri.yaml` with `sindri config validate`
3. **Enable verbose output** - Add `-v`, `-vv`, or `-vvv` flags for detailed logging
4. **Review exit codes** - Check the command exit code for error categorization
5. **Consult logs** - Review provider-specific logs for deployment issues

### Exit Codes Reference

| Code | Description          |
| ---- | -------------------- |
| 0    | Success              |
| 1    | General error        |
| 2    | Configuration error  |
| 3    | Provider error       |
| 4    | Network error        |
| 5    | Authentication error |

---

## Using sindri doctor

The `sindri doctor` command is your primary diagnostic tool. It performs comprehensive system checks and provides actionable remediation steps.

For complete documentation, see [DOCTOR.md](./DOCTOR.md).

### Quick Diagnostics

```bash
# Check all required tools
sindri doctor

# Check for a specific provider
sindri doctor --provider docker
sindri doctor --provider fly
sindri doctor --provider k8s

# Check with authentication status
sindri doctor --check-auth

# Verbose output with timing
sindri doctor --all --verbose-output

# JSON output for scripting
sindri doctor --ci --format json
```

### Auto-Fix Mode

The doctor can automatically install missing tools:

```bash
# Preview what would be installed
sindri doctor --fix --dry-run

# Install missing tools interactively
sindri doctor --fix

# Non-interactive installation
sindri doctor --fix --yes
```

### CI/CD Integration

```bash
# Exit with non-zero code if required tools are missing
sindri doctor --ci --provider docker
```

Exit codes for CI mode:

| Code | Meaning                                                      |
| ---- | ------------------------------------------------------------ |
| 0    | All required tools available (optional tools may be missing) |
| 1    | Missing required tools                                       |
| 2    | Tools present but version too old                            |
| 3    | Tools present but not authenticated (when auth is required)  |

---

## Common Issues

### Installation Problems

#### CLI Binary Not Found

**Symptom:**

```text
sindri: command not found
```

**Solution:**

Ensure the binary is in your PATH:

```bash
# Check installation location
which sindri

# Add to PATH if needed (add to ~/.bashrc or ~/.zshrc)
export PATH="$PATH:/usr/local/bin"

# Verify
sindri --version
```

#### Permission Denied on Binary

**Symptom:**

```text
bash: /usr/local/bin/sindri: Permission denied
```

**Solution:**

```bash
# Make executable
chmod +x /usr/local/bin/sindri

# Or reinstall with correct permissions
sudo install -m 755 sindri /usr/local/bin/
```

#### macOS Security Block

**Symptom:**

```text
"sindri" cannot be opened because it is from an unidentified developer.
```

**Solution:**

```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /usr/local/bin/sindri

# Or allow in System Preferences > Security & Privacy
```

---

### Configuration Issues

#### Invalid sindri.yaml

**Symptom:**

```text
Error: Schema validation failed
```

**Diagnosis:**

```bash
# Validate configuration
sindri config validate

# Show resolved configuration
sindri config show

# Check YAML syntax
sindri config validate --file ./sindri.yaml
```

**Common causes:**

1. **Invalid version format:**

   ```yaml
   # Wrong
   version: 3.0

   # Correct
   version: "3.0"
   ```

2. **Invalid name format:**

   ```yaml
   # Wrong - uppercase, underscore
   name: My_Project

   # Correct - lowercase, hyphens only
   name: my-project
   ```

3. **Both profile and active specified:**

   ```yaml
   # Wrong - mutually exclusive
   extensions:
     profile: minimal
     active:
       - nodejs

   # Correct - use one or the other
   extensions:
     profile: minimal
   ```

4. **Unknown provider:**

   ```yaml
   # Valid providers: docker, docker-compose, fly, devpod, e2b, kubernetes
   deployment:
     provider: docker
   ```

#### Configuration Not Found

**Symptom:**

```text
Error: Configuration file not found
```

**Solution:**

Sindri looks for `sindri.yaml` in these locations (in order):

1. `./sindri.yaml` (current directory)
2. `~/.config/sindri/sindri.yaml`
3. `/etc/sindri/sindri.yaml`

Create a configuration or specify the path:

```bash
# Initialize new configuration
sindri config init

# Or specify path
sindri deploy --config /path/to/sindri.yaml
```

#### Image Configuration Errors

**Symptom:**

```text
Error: Failed to resolve image version
```

**Diagnosis:**

```bash
# List available images
sindri image list

# Check version compatibility
sindri image versions --cli-version 3.0.0
```

**Common fixes:**

```yaml
# Use explicit version
image_config:
  registry: ghcr.io/pacphi/sindri
  version: "^3.0.0"
  # Or pin to specific version
  tag_override: v3.0.0
  resolution_strategy: explicit
```

---

### Extension Problems

#### Extension Install Fails

**Symptom:**

```text
Error: Extension 'nodejs' installation failed
```

**Diagnosis:**

```bash
# Validate extension definition
sindri extension validate nodejs

# Check extension info
sindri extension info nodejs

# List installed extensions
sindri extension list --installed
```

**Common causes:**

1. **Missing dependencies:**

   ```bash
   # Check dependencies
   sindri extension info nodejs --json | jq '.dependencies'

   # Install dependencies first
   sindri extension install mise
   ```

2. **Network issues:**

   ```bash
   # Test connectivity
   curl -I https://registry.npmjs.org

   # Check proxy settings
   echo $HTTP_PROXY
   echo $HTTPS_PROXY
   ```

3. **Insufficient disk space:**

   ```bash
   df -h
   ```

#### Extension Validation Fails

**Symptom:**

```text
Error: Extension 'my-extension' validation failed
```

**Solution:**

```bash
# Validate with verbose output
sindri extension validate my-extension -v

# Check extension file syntax
sindri extension validate --file ./extension.yaml
```

#### Extension Not Found After Install

**Symptom:**

Tools installed by extension not available.

**Solution:**

```bash
# Restart shell or source profile
source ~/.bashrc
# or
source ~/.zshrc

# Check mise shims
mise list
mise doctor
```

---

### Provider-Specific Issues

#### Docker Provider

**Docker not running:**

```text
Error: Docker is not running. Please start Docker and try again.
```

**Solution:**

```bash
# Check Docker status
docker info

# Start Docker (macOS)
open /Applications/Docker.app

# Start Docker (Linux)
sudo systemctl start docker
```

**Permission denied:**

```text
Error: permission denied while trying to connect to the Docker daemon socket
```

**Solution:**

```bash
# Add user to docker group (Linux)
sudo usermod -aG docker $USER

# Apply group change
newgrp docker

# Verify
docker ps
```

**Container build fails:**

```bash
# Check Docker disk space
docker system df

# Clean up unused resources
docker system prune -a

# Rebuild without cache
docker build --no-cache .
```

#### Fly.io Provider

**Not authenticated:**

```text
Error: Not authenticated with Fly.io
```

**Solution:**

```bash
# Check authentication
flyctl auth whoami

# Login
flyctl auth login
```

**App name taken:**

```text
Error: App name 'my-app' is already taken
```

**Solution:**

Update the name in `sindri.yaml`:

```yaml
name: my-unique-app-name
```

**Region not available:**

```bash
# List available regions
flyctl platform regions

# Update sindri.yaml
providers:
  fly:
    region: iad  # Use available region
```

**SSH connection refused:**

```text
Error: ssh: connect to host my-app.fly.dev port 10022: Connection refused
```

**Diagnosis:**

```bash
# Check machine status
flyctl status -a my-app

# Check if suspended
flyctl machine list -a my-app

# Start machine if needed
flyctl machine start <machine-id> -a my-app
```

#### Kubernetes Provider

**kubectl not configured:**

```text
Error: kubectl is not configured
```

**Solution:**

```bash
# Check kubectl configuration
kubectl config current-context

# Set context
kubectl config use-context my-cluster

# Test connectivity
kubectl cluster-info
```

**Namespace not found:**

```text
Error: namespace 'sindri' not found
```

**Solution:**

```bash
# Create namespace
kubectl create namespace sindri

# Or update sindri.yaml
providers:
  kubernetes:
    namespace: default
```

**Image pull fails:**

```bash
# Check image pull secrets
kubectl get secrets

# Create registry secret if needed
kubectl create secret docker-registry ghcr-secret \
  --docker-server=ghcr.io \
  --docker-username=$GITHUB_USER \
  --docker-password=$GITHUB_TOKEN
```

#### DevPod Provider

**Provider not installed:**

```text
Error: DevPod provider 'aws' is not installed
```

**Solution:**

```bash
# List available providers
devpod provider list

# Install provider
devpod provider add aws
```

**Build repository required:**

```text
Error: buildRepository is required for cloud deployments
```

**Solution:**

Update `sindri.yaml`:

```yaml
providers:
  devpod:
    type: aws
    buildRepository: ghcr.io/myorg/sindri
```

#### E2B Provider

**Not authenticated:**

```text
Error: E2B authentication required
```

**Solution:**

```bash
# Check authentication
e2b auth status

# Login
e2b auth login
```

**GPU not supported:**

```text
Error: GPU configuration not supported by E2B provider
```

E2B does not support GPU. Remove GPU configuration:

```yaml
deployment:
  resources:
    memory: 4GB
    cpus: 2
    # Remove gpu section for E2B
```

---

### Permission Problems

#### Volume Permission Denied

**Symptom:**

```text
Error: Permission denied: /home/developer/workspace
```

**Solution (Docker):**

```bash
# Fix ownership (run inside container as root)
docker exec -u root <container> chown -R developer:developer /home/developer/workspace

# Or recreate volume
docker volume rm sindri-workspace
sindri deploy
```

**Solution (Kubernetes):**

Check security context in deployment:

```yaml
securityContext:
  runAsUser: 1000
  runAsGroup: 1000
  fsGroup: 1000
```

#### SSH Key Permission Denied

**Symptom:**

```text
Error: Permission denied (publickey)
```

**Solution:**

```bash
# Check SSH key exists
ls -la ~/.ssh/id_*.pub

# Generate if needed
ssh-keygen -t ed25519 -C "your@email.com"

# Add to ssh-agent
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519

# For Fly.io
flyctl ssh issue --agent -a my-app
```

#### Sudo/Apt Installation Fails in Container

**Symptom:**

```text
sudo: effective uid is not 0, is /usr/bin/sudo on a file system with the 'nosuid' option set or an NFS file system without root privileges?
```

or:

```text
sudo: PERM_SUDOERS: setresuid(-1, 1, -1): Operation not permitted
```

**Cause:**

The container is running with `no-new-privileges` security flag, which blocks sudo. This happens in **socket** mode (when sharing the host Docker daemon).

**Solution:**

Check the DinD mode in your deployment:

```bash
# Check current DinD mode
grep SINDRI_DIND_MODE docker-compose.yml
```

| Mode         | sudo Works? | Description                         |
| ------------ | ----------- | ----------------------------------- |
| `none`       | YES         | Default development mode            |
| `sysbox`     | YES         | User namespace isolation            |
| `privileged` | YES         | Legacy DinD                         |
| `socket`     | NO          | Production security (shared daemon) |

**Options if sudo is blocked:**

1. **Use a different DinD mode** (if you need sudo):

   ```yaml
   # sindri.yaml
   providers:
     docker:
       dind:
         mode: none # or sysbox
   ```

2. **Use sudo-free installation methods** (recommended for production):
   - Extensions like `cloud-tools` use pip and tarball extraction
   - See [ADR-041](architecture/adr/041-security-hardened-extension-installation.md) for patterns

3. **Pre-install at build time**:
   - Add apt packages to your Dockerfile
   - Use `extensions.profile` in sindri.yaml for build-time installation

**Security Note:**

Socket mode applies `no-new-privileges` intentionally - when sharing the host Docker daemon, preventing privilege escalation is important. For development, use `none` or `sysbox` mode to allow sudo.

---

## Debugging

### Enable Verbose Output

Use verbosity flags for detailed output:

```bash
# Minimal verbosity
sindri deploy -v

# Medium verbosity
sindri deploy -vv

# Maximum verbosity (debug level)
sindri deploy -vvv
```

### Environment Variables

Set logging level via environment:

```bash
# Set log level
export SINDRI_LOG_LEVEL=debug

# Trace level for maximum detail
export SINDRI_LOG_LEVEL=trace

sindri deploy
```

Available log levels: `trace`, `debug`, `info`, `warn`, `error`

### Dry Run Mode

Preview changes without executing:

```bash
# Deploy dry run
sindri deploy --dry-run

# Backup dry run
sindri backup --dry-run

# Restore dry run
sindri restore backup.tar.gz --dry-run
```

### Collect Diagnostic Information

When reporting issues, collect this information:

```bash
# System information
sindri version --json
uname -a
docker version 2>/dev/null || echo "Docker not installed"
kubectl version --client 2>/dev/null || echo "kubectl not installed"

# Doctor output
sindri doctor --all --format json

# Configuration (sanitize secrets!)
sindri config show

# Extension status
sindri extension status --json
```

### Provider-Specific Logs

**Docker:**

```bash
docker logs <container-name>
docker logs -f <container-name>  # Follow logs
```

**Fly.io:**

```bash
flyctl logs -a my-app
flyctl logs -a my-app --region sjc
```

**Kubernetes:**

```bash
kubectl logs deployment/my-app
kubectl logs -f deployment/my-app
kubectl describe pod <pod-name>
```

---

## Getting Help

### Check Documentation

- [CLI Reference](./CLI.md) - Complete command documentation
- [Configuration Reference](./CONFIGURATION.md) - sindri.yaml schema
- [Doctor Guide](./DOCTOR.md) - Diagnostic command details
- [Getting Started](./GETTING_STARTED.md) - Installation and first steps
- [Image Management](./IMAGE_MANAGEMENT.md) - Container image security
- [Secrets Management](./SECRETS_MANAGEMENT.md) - Secrets configuration

### Report Issues

When reporting issues on GitHub, include:

1. **Sindri version:** `sindri version --json`
2. **Error message:** Full error output
3. **Configuration:** `sindri.yaml` (remove secrets!)
4. **Provider:** Which provider you are using
5. **Doctor output:** `sindri doctor --all --format json`
6. **Steps to reproduce:** Minimal steps to trigger the issue

**GitHub Issues:** https://github.com/pacphi/sindri/issues

### Community Resources

- **GitHub Discussions:** https://github.com/pacphi/sindri/discussions
- **FAQ:** https://sindri-faq.fly.dev

### Search Existing Issues

Before creating a new issue, search for existing solutions:

```bash
# Search GitHub issues
gh issue list --repo pacphi/sindri --search "your error message"
```

---

## Related Documentation

- [CLI Reference](./CLI.md)
- [Configuration Reference](./CONFIGURATION.md)
- [Doctor Guide](./DOCTOR.md)
- [Getting Started](./GETTING_STARTED.md)
- [Image Management](./IMAGE_MANAGEMENT.md)
- [Secrets Management](./SECRETS_MANAGEMENT.md)
