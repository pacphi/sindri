# ADR 041: Security-Hardened Extension Installation

**Status**: Accepted
**Date**: 2026-01-29
**Deciders**: Core Team
**Related**: [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md), [ADR-040: Two-Dockerfile Architecture](040-two-dockerfile-architecture.md)

## Context

The cloud-tools extension failed to install in containers with security hardening enabled, specifically when running with the `no-new-privileges` flag and `/tmp` mounted with `noexec`. These security measures prevent:

1. **Privilege Escalation**: `no-new-privileges` blocks sudo and setuid binaries
2. **Execution from Temporary Directories**: `noexec` on `/tmp` prevents script execution
3. **System-Wide Installation**: Package managers require root access

Original cloud-tools installation methods:

- **Azure CLI**: `curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash`
- **Google Cloud SDK**: `sudo apt-get install google-cloud-cli`
- **IBM Cloud CLI**: `curl -fsSL https://clis.cloud.ibm.com/install/linux | sh` (requires sudo)

These methods violate the security model enforced by container security policies and fail with exit code 126 ("Permission denied" errors).

Security hardening context:

- **C-5 Security Compliance**: Already documented for AWS CLI user-local installation
- **Container Security Best Practices**: Minimize attack surface by blocking privilege escalation
- **Production Deployments**: Kubernetes security policies often require `no-new-privileges`
- **Build Environments**: CI/CD systems increasingly enforce security hardening

The bash implementation (V2) used sudo-based installation, which worked in permissive environments but failed in hardened containers. V3 architecture requires compatibility with security-first deployment models.

## Decision

### Sudo-Free User-Local Installation for All Cloud CLIs

Rewrite cloud-tools installer to use sudo-free, user-local installation methods exclusively:

**Azure CLI**: Python pip installation (requires Python 3.10+)

```bash
python3 -m pip install --user azure-cli
# Installs to: ~/.local/bin/az
```

**Google Cloud SDK**: Tarball extraction

```bash
curl -fsSL "https://dl.google.com/dl/cloudsdk/channels/rapid/downloads/google-cloud-cli-linux-${ARCH}.tar.gz" | tar -xzf - -C "$HOME"
"$HOME/google-cloud-sdk/install.sh" --quiet --usage-reporting=false --path-update=false --command-completion=false
# Installs to: ~/google-cloud-sdk/bin/gcloud
```

**IBM Cloud CLI**: Tarball extraction with version detection

```bash
IBM_VERSION=$(get_github_release_version "IBM-Cloud/ibm-cloud-cli-release" false)
curl -fsSL "https://download.clis.cloud.ibm.com/ibm-cloud-cli/${IBM_VERSION}/binaries/IBM_Cloud_CLI_${IBM_VERSION}_linux_${IBM_ARCH}.tgz" -o "/tmp/ibmcloud.tgz"
tar -xzf /tmp/ibmcloud.tgz -C "$HOME/.local/ibmcloud" --strip-components=1
ln -sf "$HOME/.local/ibmcloud/ibmcloud" "$HOME/.local/bin/ibmcloud"
# Installs to: ~/.local/bin/ibmcloud
```

**Existing User-Local Tools** (no changes needed):

- AWS CLI: `~/.local/bin/aws`
- Fly.io CLI: `~/.fly/bin/flyctl`
- Oracle CLI: `~/bin/oci`
- Alibaba CLI: `~/.local/bin/aliyun`
- DigitalOcean CLI: `~/.local/bin/doctl`

### Python Extension Dependency

Azure CLI requires Python 3.10+ for pip installation. Add python as extension dependency:

```yaml
metadata:
  name: cloud-tools
  version: 2.0.0
  dependencies:
    - python # Required for Azure CLI (pip install azure-cli)
```

Sindri's dependency resolution ensures python is installed before cloud-tools.

### Python Version Detection

Installer checks Python availability and version before attempting Azure CLI installation:

```bash
PYTHON_AVAILABLE=false
if command_exists python3; then
  PYTHON_VERSION=$(python3 --version 2>&1 | grep -oP '(?<=Python )\d+\.\d+' || echo "0.0")
  PYTHON_MAJOR=$(echo "$PYTHON_VERSION" | cut -d. -f1)
  PYTHON_MINOR=$(echo "$PYTHON_VERSION" | cut -d. -f2)
  if [[ "$PYTHON_MAJOR" -ge 3 ]] && [[ "$PYTHON_MINOR" -ge 10 ]]; then
    PYTHON_AVAILABLE=true
  fi
fi
```

If Python is unavailable or <3.10, Azure CLI installation is skipped with a warning.

### PATH Configuration

Bashrc template updated to include all user-local bin directories:

```bash
export PATH="$HOME/.local/bin:$HOME/google-cloud-sdk/bin:$HOME/.fly/bin:$HOME/bin:$PATH"
```

### Enhanced Validation

Now that all tools install without sudo, validation expanded to include Azure CLI and Google Cloud SDK:

```yaml
validate:
  commands:
    - name: aws
      expectedPattern: "aws-cli"
    - name: az
    - name: gcloud
    - name: flyctl
      expectedPattern: "flyctl"
```

### Domain Requirements

Additional domains added for pip and tarball downloads:

```yaml
requirements:
  domains:
    - pypi.org
    - files.pythonhosted.org
    - dl.google.com
    - download.clis.cloud.ibm.com
```

## Consequences

### Positive

1. **Security Hardening Compatibility**: Works in containers with `no-new-privileges` flag
2. **No Root Required**: All 8 cloud CLIs install to user directories without sudo
3. **Consistent Installation Pattern**: User-local installation across all tools
4. **C-5 Compliance Alignment**: Extends existing AWS CLI security compliance pattern
5. **Production Ready**: Compatible with Kubernetes security policies
6. **Dependency Management**: Python dependency ensures Azure CLI prerequisites
7. **Architecture Detection**: Proper x86_64/aarch64 binary selection for all tools
8. **Graceful Degradation**: Azure CLI skipped if Python unavailable, other 7 CLIs still install
9. **Clean PATH Management**: All tools accessible via standardized PATH configuration
10. **Validation Coverage**: All major CLIs (AWS, Azure, GCP, Fly.io) validated on installation

### Negative

1. **Python Dependency**: Azure CLI requires Python 3.10+ (adds ~50MB to image)
2. **Azure CLI Limitations**: pip installation may lack some features of apt-get version
3. **Version Lag**: pip azure-cli may lag behind official apt repository
4. **Disk Space**: User-local installations (~1.5GB for all tools combined)
5. **GitHub API Dependency**: IBM Cloud CLI version detection requires GitHub API access
6. **Tarball Size**: Google Cloud SDK tarball is ~100MB download

### Neutral

1. **Installation Time**: Similar total time (no sudo overhead, but larger downloads)
2. **Python Source**: Works with both mise-installed Python and system Python
3. **CI Mode**: Optional CLIs (Oracle, Alibaba, DigitalOcean, IBM) still skipped in CI
4. **Cleanup**: APT cache cleanup still runs (no-op if APT not used)

## Alternatives Considered

### 1. Keep Sudo-Based Installation with Fallback

**Description**: Try sudo-based installation first, fall back to user-local if sudo unavailable.

**Pros**:

- Maintains compatibility with existing installations
- Uses official installation methods when available
- No new dependencies

**Cons**:

- Complexity: Two installation paths for each tool
- Inconsistent: Different tools installed in different locations
- Security: Still vulnerable in permissive environments
- Maintenance: Must maintain both installation methods

**Rejected**: Security-first approach with single consistent installation method is clearer.

### 2. Docker Image with Pre-Installed CLIs

**Description**: Pre-install all cloud CLIs in base Docker image instead of extension-based installation.

**Pros**:

- No runtime installation needed
- Faster environment startup
- Guaranteed availability

**Cons**:

- Larger base image (~2GB additional size)
- Forces all users to install all CLIs
- Violates extension modularity principle
- Harder to update individual tools
- No user choice in cloud provider selection

**Rejected**: Conflicts with Sindri's extension-based architecture and user choice philosophy.

### 3. Binary-Only Installation (No Azure CLI)

**Description**: Skip Azure CLI entirely, only install CLIs available as standalone binaries.

**Pros**:

- No Python dependency
- Simpler installation
- Smaller disk footprint

**Cons**:

- Breaks Azure support
- Incomplete cloud provider coverage
- Poor user experience for Azure users

**Rejected**: Azure is too important to skip (one of the big three cloud providers).

### 4. Conditional Sudo with Permission Prompting

**Description**: Prompt user for permission to use sudo if available, fall back to user-local otherwise.

**Pros**:

- User control over installation method
- Can use official methods when sudo available

**Cons**:

- Breaks automation (requires interactive prompting)
- Inconsistent across environments
- Security risk (encourages sudo in production)
- Complex permission handling

**Rejected**: Automation-first design requires non-interactive installation.

### 5. Azure CLI from Tarball (Not pip)

**Description**: Use experimental Azure CLI standalone tarball instead of pip installation.

**Pros**:

- No Python dependency
- Matches Google Cloud SDK pattern

**Cons**:

- Unsupported by Microsoft (no official tarball releases)
- Requires building from source or unofficial mirrors
- Version lag and maintenance burden
- No official documentation

**Rejected**: pip installation is official Microsoft-supported method for user-local installation.

## Compliance

- ✅ No sudo required for any cloud CLI installation
- ✅ Works with `no-new-privileges` security flag
- ✅ Works with `/tmp` mounted `noexec`
- ✅ All tools install to user directories (`~/.local`, `~/google-cloud-sdk`, `~/.fly`, `~/bin`)
- ✅ Python dependency declared for Azure CLI
- ✅ Architecture detection for x86_64 and aarch64
- ✅ Graceful degradation if Python unavailable
- ✅ PATH configuration includes all tool directories
- ✅ Validation covers all major cloud CLIs
- ✅ Domain requirements include pip and tarball sources
- ✅ 100% sudo-free installation across all 8 cloud CLIs

## Notes

### Azure CLI Version Information

- Latest pip version: 2.82.0 (released Jan 13, 2026)
- Python requirement: >= 3.10.0
- Installation size: ~200MB
- Official source: https://pypi.org/project/azure-cli/

### Google Cloud SDK Version Information

- Tarball includes bundled Python (no system Python dependency)
- Installation size: ~500MB
- Interactive installer supports user directory selection
- Official source: https://cloud.google.com/sdk/docs/install-sdk

### IBM Cloud CLI Version Information

- Latest version: 2.40.0 (released Dec 10, 2025)
- GitHub releases: https://github.com/IBM-Cloud/ibm-cloud-cli-release/releases/
- Installation size: ~100MB
- Architecture support: amd64, arm64

### Security Hardening Test Cases

Test with `no-new-privileges` flag:

```yaml
docker-compose:
  security_opt:
    - no-new-privileges:true
```

Test with `/tmp` mounted `noexec`:

```yaml
docker-compose:
  tmpfs:
    - /tmp:noexec
```

Both scenarios now pass successfully.

### DinD Mode Interactions

The `no-new-privileges` flag is applied by `docker-compose.yml.tera` based on the DinD mode:

| Mode         | no-new-privileges | sudo Works | Impact on Extensions      |
| ------------ | ----------------- | ---------- | ------------------------- |
| `socket`     | YES               | NO         | Sudo-free extensions only |
| `sysbox`     | NO                | YES        | All extensions work       |
| `privileged` | NO                | YES        | All extensions work       |
| `none`       | NO                | YES        | All extensions work       |

**Recommendation**: Extensions requiring apt packages should use sudo-free installation methods (pip, tarball extraction, user-local binaries) to ensure compatibility with `socket` mode deployments. This ADR's cloud-tools patterns serve as a reference implementation.

### Future Enhancements

- **Offline Installation**: Support air-gapped environments with pre-downloaded tarballs
- **Version Pinning**: Allow users to specify specific CLI versions
- **Update Notifications**: Alert users when newer CLI versions are available
- **Health Checks**: Periodic validation that CLIs are still functional

## Related Decisions

- [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md) - Extension installation methods (mise, apt, binary, npm, script, hybrid)
- [ADR-040: Two-Dockerfile Architecture](040-two-dockerfile-architecture.md) - Security hardening context
- [ADR-027: Tool Dependency Management System](027-tool-dependency-management-system.md) - Python dependency resolution
