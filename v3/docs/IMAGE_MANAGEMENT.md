# Container Image Management

Sindri v3 includes comprehensive container image management with version resolution, signature verification, and SBOM support.

## Quick Start

### Using Version Constraints

```yaml
# sindri.yaml
deployment:
  provider: kubernetes
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0" # Semantic version constraint
    verify_signature: true
    verify_provenance: true
```

```bash
# Deploy with automatic version resolution
sindri deploy
```

### List Available Versions

```bash
# List all available images
sindri image list --registry ghcr.io --repository pacphi/sindri

# Filter by pattern
sindri image list --filter "^v3\."

# Include prereleases
sindri image list --include-prerelease
```

### Inspect Image Details

```bash
# Inspect a specific image
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0

# Show digest
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --digest

# Download and show SBOM
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --sbom
```

### Verify Image Security

```bash
# Verify signature and provenance
sindri image verify ghcr.io/pacphi/sindri:v3.0.0

# Skip signature verification
sindri image verify ghcr.io/pacphi/sindri:v3.0.0 --no-signature

# Skip provenance verification
sindri image verify ghcr.io/pacphi/sindri:v3.0.0 --no-provenance
```

## Configuration Reference

### ImageConfig Structure

```yaml
deployment:
  provider: kubernetes
  image_config:
    # Registry URL (required)
    registry: ghcr.io/pacphi/sindri

    # Semantic version constraint (optional)
    version: "^3.0.0"

    # Explicit tag override (overrides version)
    tag_override: v3.1.0-beta.1

    # Pin to specific digest (immutable, overrides version and tag)
    digest: sha256:abc123...

    # Resolution strategy (default: semver)
    resolution_strategy: semver # Options: semver, latest-stable, pin-to-cli, explicit

    # Allow prerelease versions (default: false)
    allow_prerelease: false

    # Verify image signature (default: true)
    verify_signature: true

    # Verify SLSA provenance (default: true)
    verify_provenance: true

    # Pull policy (default: IfNotPresent)
    pull_policy: IfNotPresent # Options: Always, IfNotPresent, Never

    # Certificate identity for verification (optional)
    certificate_identity: https://github.com/pacphi/sindri

    # OIDC issuer for verification (optional)
    certificate_oidc_issuer: https://token.actions.githubusercontent.com
```

### Resolution Strategies

#### Semver (Default)

Uses semantic versioning constraints to find the highest matching version.

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  version: "^3.0.0" # Matches 3.x.x
  resolution_strategy: semver
```

**Constraint Syntax:**

- `^3.0.0` - Compatible versions (3.0.0 ≤ version < 4.0.0)
- `~3.1.0` - Approximate versions (3.1.0 ≤ version < 3.2.0)
- `>=3.0.0` - Greater than or equal
- `3.0.0` - Exact version

#### LatestStable

Always uses the latest stable (non-prerelease) version.

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  resolution_strategy: latest-stable
```

#### PinToCli

Uses the same version as the CLI.

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  resolution_strategy: pin-to-cli
```

Useful for ensuring CLI and container versions match.

#### Explicit

Uses an explicitly specified tag or digest.

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  tag_override: v3.0.0
  resolution_strategy: explicit
```

### Pull Policies

#### Always

Always pull the image, even if it exists locally.

```yaml
image_config:
  pull_policy: Always
```

Use when: You always want the latest version

#### IfNotPresent (Default)

Only pull if the image doesn't exist locally.

```yaml
image_config:
  pull_policy: IfNotPresent
```

Use when: You want to save bandwidth

#### Never

Never pull, only use local images.

```yaml
image_config:
  pull_policy: Never
```

Use when: Working in air-gapped environments

## Security

### Image Signing

All release images are signed with [Cosign](https://docs.sigstore.dev/cosign/) using keyless signing (OIDC).

#### Verification

```bash
# Automatic verification during deploy
sindri deploy

# Manual verification
cosign verify ghcr.io/pacphi/sindri:v3.0.0 \
  --certificate-identity-regexp='https://github.com/pacphi/sindri' \
  --certificate-oidc-issuer='https://token.actions.githubusercontent.com'
```

#### Skip Verification

```bash
# Skip verification during deployment (not recommended)
sindri deploy --skip-image-verification
```

Or in configuration:

```yaml
image_config:
  verify_signature: false
  verify_provenance: false
```

### SBOM (Software Bill of Materials)

Every release includes an SBOM in SPDX format.

#### Download SBOM

```bash
# From cosign attestation
cosign download sbom ghcr.io/pacphi/sindri:v3.0.0 > sbom.spdx.json

# Or from GitHub Release assets
wget https://github.com/pacphi/sindri/releases/download/v3.0.0/sbom.spdx.json
```

#### Inspect SBOM

```bash
# View packages
jq '.packages[] | {name, version, license}' sbom.spdx.json | head -20

# Search for specific package
jq '.packages[] | select(.name | contains("openssl"))' sbom.spdx.json
```

### SLSA Provenance

Images include SLSA Level 3 provenance attestations.

```bash
# Verify provenance
cosign verify-attestation \
  --type slsaprovenance \
  --certificate-identity-regexp='https://github.com/pacphi/sindri' \
  --certificate-oidc-issuer='https://token.actions.githubusercontent.com' \
  ghcr.io/pacphi/sindri:v3.0.0
```

## Migration Guide

### From Legacy Image Field

**Before:**

```yaml
deployment:
  provider: kubernetes
  image: ghcr.io/pacphi/sindri:latest
```

**After:**

```yaml
deployment:
  provider: kubernetes
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
```

**Benefits:**

- Automatic version resolution
- Signature verification
- Provenance checking

### Backward Compatibility

The legacy `image` field is still supported:

```yaml
deployment:
  provider: kubernetes
  image: ghcr.io/pacphi/sindri:v3.0.0 # Still works!
```

No image verification is performed when using the legacy field.

## CLI Commands

### sindri image list

List available image versions from a registry.

```bash
# Basic usage
sindri image list

# Specify registry and repository
sindri image list --registry ghcr.io --repository pacphi/sindri

# Filter by pattern (regex)
sindri image list --filter "^v3\.[01]\."

# Include prereleases
sindri image list --include-prerelease

# JSON output
sindri image list --json
```

### sindri image inspect

Get detailed information about an image.

```bash
# Basic inspection
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0

# Show digest
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --digest

# Download and show SBOM
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --sbom

# JSON output
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --json
```

**Output:**

```
Image: ghcr.io/pacphi/sindri:v3.0.0
Digest: sha256:abc123...
Size: 1.2 GB
Created: 2026-01-23T10:30:00Z

Platforms:
  - linux/amd64
  - linux/arm64

Labels:
  org.opencontainers.image.version: 3.0.0
  sindri.version: v3
```

### sindri image verify

Verify image signatures and provenance.

```bash
# Verify both signature and provenance
sindri image verify ghcr.io/pacphi/sindri:v3.0.0

# Skip signature check
sindri image verify ghcr.io/pacphi/sindri:v3.0.0 --no-signature

# Skip provenance check
sindri image verify ghcr.io/pacphi/sindri:v3.0.0 --no-provenance
```

**Output:**

```
Verifying signature...
✅ Signature verified
   Issuer: https://token.actions.githubusercontent.com
   Subject: https://github.com/pacphi/sindri

Verifying provenance...
✅ Provenance verified (SLSA Level 3)
   Builder: GitHub Actions
   Source: https://github.com/pacphi/sindri
```

### sindri image versions

Show compatible image versions for the current CLI.

```bash
# Show versions compatible with current CLI
sindri image versions

# Check compatibility for specific CLI version
sindri image versions --cli-version 3.1.0

# JSON output
sindri image versions --format json
```

### sindri image current

Show the currently configured image.

```bash
# Show current image
sindri image current

# JSON output
sindri image current --json
```

## Environment Variables

### GITHUB_TOKEN

Required for accessing private GHCR repositories.

```bash
export GITHUB_TOKEN=ghp_your_token_here
sindri deploy
```

### Authentication

For private registries, authenticate with Docker:

```bash
# GitHub Container Registry
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# Docker Hub
docker login docker.io -u USERNAME

# Then deploy
sindri deploy
```

## Troubleshooting

### "Failed to resolve image version"

**Cause:** No matching versions found or registry unreachable

**Solutions:**

- Check internet connectivity
- Verify version constraint: `sindri image list --filter "^v3\."`
- Try with `--allow-prerelease` if using alpha/beta versions
- Check GITHUB_TOKEN for private registries

### "Signature verification failed"

**Cause:** Image not signed or using wrong verification parameters

**Solutions:**

- Verify image is signed: `cosign tree ghcr.io/pacphi/sindri:v3.0.0`
- Check certificate identity and OIDC issuer in config
- Skip verification: `sindri deploy --skip-image-verification` (not recommended)

### "cosign not found"

**Cause:** Cosign not installed

**Solution:**

```bash
# macOS
brew install cosign

# Linux
wget https://github.com/sigstore/cosign/releases/latest/download/cosign-linux-amd64
chmod +x cosign-linux-amd64
sudo mv cosign-linux-amd64 /usr/local/bin/cosign

# Or skip verification
sindri deploy --skip-image-verification
```

### "ImagePullSecret creation failed"

**Cause:** Docker credentials not available

**Solution:**

```bash
# Login to registry
docker login ghcr.io

# Or manually create secret
kubectl create secret docker-registry sindri-registry-creds \
  --docker-server=ghcr.io \
  --docker-username=USERNAME \
  --docker-password=TOKEN \
  -n default
```

## Best Practices

### Use Version Constraints

Instead of `latest`, use semantic versioning:

```yaml
# Good - automatic updates within major version
image_config:
  version: "^3.0.0"

# Bad - unpredictable behavior
image: ghcr.io/pacphi/sindri:latest
```

### Pin Production Deployments

For production, pin to specific digests:

```yaml
# Production
image_config:
  digest: sha256:abc123...
  verify_signature: true
```

### Enable Verification

Always verify images in production:

```yaml
image_config:
  verify_signature: true
  verify_provenance: true
```

### Use IfNotPresent Pull Policy

Save bandwidth in development:

```yaml
image_config:
  pull_policy: IfNotPresent
```

Use `Always` in production to ensure latest security patches.

## Advanced Usage

### Custom Registry

```yaml
image_config:
  registry: my-registry.example.com/team/sindri
  version: "^3.0.0"
  certificate_identity: https://github.com/my-org/sindri-fork
```

### Air-Gapped Environments

```yaml
image_config:
  registry: internal-registry.corp/sindri
  tag_override: v3.0.0
  pull_policy: Never
  verify_signature: false
  verify_provenance: false
```

### Development Builds

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  tag_override: ci-passed-abc123 # Use specific CI build
  verify_signature: false # CI builds aren't signed
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Deploy with specific version
  run: |
    cat > sindri.yaml << EOF
    deployment:
      provider: kubernetes
      image_config:
        registry: ghcr.io/pacphi/sindri
        version: "^3.0.0"
    EOF
    sindri deploy --wait
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Automated Version Updates

Use Dependabot or Renovate to update version constraints:

```yaml
# .github/renovate.json
{
  "extends": ["config:base"],
  "regexManagers":
    [
      {
        "fileMatch": ["sindri\\.yaml$"],
        "matchStrings": ['version: "(?<currentValue>[^"]+)"'],
        "datasourceTemplate": "github-tags",
        "depNameTemplate": "pacphi/sindri",
      },
    ],
}
```

## Related Documentation

- [Planning Documentation](planning/active/container-image-lifecycle-management.md) - Architecture and design
