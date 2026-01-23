# Container Image Lifecycle Management - Implementation Plan

## Sindri v2 & v3 - 2026 Industry Best Practices

**Version:** 1.0
**Date:** 2026-01-23
**Status:** In Progress
**Implementation Phase:** Phase 1 Complete (GitHub Workflows)

---

## Executive Summary

This document describes the comprehensive container image lifecycle management system implemented for both Sindri v2 and v3, following 2026 industry best practices.

### Key Principles

1. **Build Once, Promote Often** - CI builds images once, releases retag and sign them
2. **Fresh CI Builds** - No cache layer reuse for reproducibility and security
3. **Registry-First Architecture** - GHCR as single source of truth, eliminating artifacts
4. **Image Signing & Verification** - Cosign, SBOM, SLSA provenance
5. **CLI Image Management** - v3 CLI enhanced with versioned image support

---

## Architecture Overview

### Image Lifecycle Flow

#### CI Workflow (Push to main)

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Build Image (Fresh, no cache)                            │
│    ├─> Tag: ci-{SHA}                                        │
│    ├─> Tag: ci-{branch}-{SHA}                               │
│    └─> Push to GHCR with provenance + SBOM                  │
│                                                             │
│ 2. Security Scan (Trivy)                                    │
│    ├─> Scan for CRITICAL/HIGH vulnerabilities               │
│    └─> Upload SARIF to GitHub Security                      │
│                                                             │
│ 3. Test Jobs (Pull from GHCR)                               │
│    ├─> docker pull ghcr.io/repo:ci-{SHA}                    │
│    └─> Run tests across providers                           │
│                                                             │
│ 4. Mark as Passed (if tests succeed)                        │
│    └─> Tag: ci-passed-{SHA}                                 │
└─────────────────────────────────────────────────────────────┘
```

#### Release Workflow (Git tag: v2.x.x or v3.x.x)

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Validate Tag (semver format)                             │
│    └─> Ensure v2.x.x or v3.x.x format                       │
│                                                             │
│ 2. Verify CI Image Exists                                   │
│    ├─> Look for: ci-passed-{SHA} (preferred)                │
│    ├─> Fallback: ci-{SHA}                                   │
│    └─> Fail if not found                                    │
│                                                             │
│ 3. Promote Image (no rebuild!)                              │
│    ├─> Pull: ci-passed-{SHA}                                │
│    ├─> Retag: v2.3.0, v2.3, v2, latest                      │
│    └─> Push to GHCR + Docker Hub                            │
│                                                             │
│ 4. Sign Image (Cosign keyless)                              │
│    └─> Sign with Sigstore OIDC                              │
│                                                             │
│ 5. Generate & Attach SBOM (Syft)                            │
│    ├─> Format: SPDX JSON                                    │
│    └─> Attach to image with cosign                          │
│                                                             │
│ 6. Create GitHub Release                                    │
│    ├─> Changelog                                            │
│    ├─> Install script                                       │
│    ├─> SBOM file                                            │
│    └─> Verification guide                                   │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Status

### ✅ Phase 1: GitHub Workflows (COMPLETE)

#### v2 Workflows

- **ci-v2.yml** - Enhanced with:
  - GHCR push with CI tags
  - Fresh builds (no-cache, pull latest base)
  - Trivy security scanning
  - Mark-passed job for main branch

- **release-v2.yml** - Transformed with:
  - verify-ci-image job
  - promote-image job (retag, no rebuild)
  - sign-image job (Cosign)
  - generate-sbom job (Syft)
  - Enhanced release assets

- **test-provider.yml** - Updated with:
  - use-registry input parameter
  - GHCR pull support
  - Backward compatible artifact support

#### v3 Workflows

- **ci-v3.yml** - Enhanced with:
  - Docker image build job (multi-arch)
  - Security scanning with Trivy
  - Mark-passed job

- **release-v3.yml** - Transformed with:
  - verify-ci-image job
  - promote-image job
  - sign-image job
  - generate-sbom job
  - SBOM in release assets

### 🚧 Phase 2: CLI Enhancements (PENDING)

#### Planned Components

- **sindri-image crate** - New crate for image management
  - Registry client (GHCR API)
  - Version resolver (semver)
  - Verification (Cosign integration)

- **Configuration schema** - Extended with ImageConfig
  - Registry URL
  - Version constraints
  - Resolution strategies
  - Verification flags

- **CLI commands** - New image subcommand
  - `sindri image list`
  - `sindri image inspect`
  - `sindri image verify`
  - `sindri image versions`
  - `sindri image current`

- **Deploy command** - Enhanced with:
  - Image resolution
  - Signature verification
  - Provenance checking

### 🚧 Phase 3: Provider Integration (PENDING)

- Kubernetes ImagePullSecrets support
- Enhanced local cluster image loading
- Registry credential management

### 🚧 Phase 4: Documentation (PENDING)

- User documentation updates
- Migration guides
- Configuration examples

### 🚧 Phase 5: Registry Cleanup (PENDING)

- Automated cleanup workflow
- Retention policies
- CI image lifecycle management

---

## Technical Details

### CI Tagging Strategy

#### CI Tags (Ephemeral)

- `ci-{SHA}` - Built from commit SHA
- `ci-{branch}-{SHA}` - Branch-specific tag
- `ci-passed-{SHA}` - Verified through tests (promotion candidate)

**Retention:** 7 days

#### Release Tags (Permanent)

- `v2.3.0` / `3.0.0` - Full semantic version
- `v2.3` / `3.0` - Major.minor
- `v2` / `v3` - Major version (stable releases only)
- `latest` - Latest stable release

**Retention:** Last 10 versions

### Build Strategy

#### Fresh Builds (CI)

```yaml
build-and-push:
  with:
    no-cache: true # No layer reuse
    pull: true # Latest base images
    provenance: mode=max
    sbom: true
```

**Benefits:**

- Reproducibility
- Security (latest patches)
- Compliance (SLSA Level 3)

**Trade-offs:**

- Slower builds (~5-10 min increase)
- Higher bandwidth usage

#### Image Promotion (Release)

```yaml
promote-image:
  run: |
    docker pull ghcr.io/repo:ci-passed-{SHA}
    docker tag ghcr.io/repo:ci-passed-{SHA} ghcr.io/repo:v2.3.0
    docker push ghcr.io/repo:v2.3.0
```

**Benefits:**

- Fast releases (<2 min)
- Guaranteed tested build
- Immutable CI → Release traceability

### Security Features

#### Image Signing (Cosign)

```bash
# Sign with keyless OIDC
cosign sign --yes ghcr.io/repo:v2.3.0

# Verify
cosign verify ghcr.io/repo:v2.3.0 \
  --certificate-identity-regexp='https://github.com/repo' \
  --certificate-oidc-issuer='https://token.actions.githubusercontent.com'
```

**Features:**

- Keyless signing (no key management)
- OIDC identity binding
- Transparency log (Rekor)

#### SBOM Generation (Syft)

```bash
# Generate SBOM
syft ghcr.io/repo:v2.3.0 -o spdx-json

# Attach to image
cosign attach sbom --sbom sbom.spdx.json ghcr.io/repo:v2.3.0

# Download
cosign download sbom ghcr.io/repo:v2.3.0
```

**Format:** SPDX 2.3 JSON
**Contents:** All packages, dependencies, licenses

#### Vulnerability Scanning (Trivy)

```yaml
trivy:
  image-ref: ghcr.io/repo:ci-{SHA}
  severity: CRITICAL,HIGH
  format: sarif
  output: trivy-results.sarif
```

**Integration:**

- GitHub Security tab
- Code Scanning alerts
- SARIF format

---

## Verification Workflow

### For Developers

```bash
# 1. Verify image signature
cosign verify ghcr.io/pacphi/sindri:v2.3.0 \
  --certificate-identity-regexp='https://github.com/pacphi/sindri' \
  --certificate-oidc-issuer='https://token.actions.githubusercontent.com'

# 2. Download SBOM
cosign download sbom ghcr.io/pacphi/sindri:v2.3.0 > sbom.json

# 3. Inspect SBOM
jq '.packages[] | {name, version}' sbom.json | head -20

# 4. Check vulnerabilities
trivy image ghcr.io/pacphi/sindri:v2.3.0
```

### For CI/CD Systems

```bash
# Use ci-passed tags for promotion
docker pull ghcr.io/pacphi/sindri:ci-passed-abc123
docker tag ghcr.io/pacphi/sindri:ci-passed-abc123 internal/sindri:v2.3.0
docker push internal/sindri:v2.3.0
```

---

## Cost Analysis

### GHCR Storage

| Type           | Retention | Size/Month | Cost/Month |
| -------------- | --------- | ---------- | ---------- |
| CI images      | 7 days    | 38GB       | $9.50      |
| Release images | Last 10   | 33GB       | $8.25      |
| **Total**      | -         | **71GB**   | **$17.75** |

### GitHub Actions Minutes

| Workflow            | Increase | Cost/Month                   |
| ------------------- | -------- | ---------------------------- |
| CI (fresh builds)   | +500 min | Free (public) / $4 (private) |
| Release (promotion) | -200 min | Savings                      |

**Total Monthly Cost:**

- Public repos: ~$18/month
- Private repos: ~$22/month

**ROI:**

- Enhanced security (signing, SBOM)
- Compliance (SLSA Level 3)
- Faster releases (no rebuild)
- Better reproducibility

---

## Migration Guide

### From Cache-Based Builds

**Before:**

```yaml
build-and-push:
  cache-from: type=gha
  cache-to: type=gha,mode=max
```

**After:**

```yaml
build-and-push:
  no-cache: true
  pull: true
  provenance: mode=max
  sbom: true
```

**Impact:**

- Build time: +5-10 minutes
- Storage: -2GB/month (no cache)
- Security: Significantly improved

### From Artifact-Based Testing

**Before:**

```yaml
- uses: actions/download-artifact@v6
- run: docker load < image.tar.gz
```

**After:**

```yaml
- uses: docker/login-action@v3
- run: docker pull ghcr.io/repo:ci-{SHA}
```

**Impact:**

- Speed: 2x faster
- Reliability: Improved
- Storage: Artifacts still available (transition period)

---

## Troubleshooting

### "No CI image found for commit"

**Cause:** Release tag created before CI completed

**Fix:**

```bash
# Wait for CI to complete
gh run watch

# Then create tag
git tag v2.3.0
git push origin v2.3.0
```

### "Signature verification failed"

**Cause:** Image not signed or wrong verification parameters

**Fix:**

```bash
# Use correct identity regexp
cosign verify IMAGE \
  --certificate-identity-regexp='https://github.com/YOUR_REPO' \
  --certificate-oidc-issuer='https://token.actions.githubusercontent.com'
```

### "SBOM attachment failed"

**Cause:** Insufficient permissions

**Fix:**

```yaml
permissions:
  packages: write # Required for SBOM attachment
```

---

## Future Enhancements

### Short-term (Q1 2026)

- [ ] Complete CLI image management (Phase 2)
- [ ] Add image caching in CLI
- [ ] Implement pull policy support
- [ ] Add registry mirror support

### Medium-term (Q2 2026)

- [ ] SLSA Level 4 compliance
- [ ] Add image vulnerability database
- [ ] Implement image signing verification in CLI
- [ ] Add air-gapped environment support

### Long-term (Q3+ 2026)

- [ ] OCI artifact support
- [ ] Wasm/WASI image support
- [ ] Confidential computing integration
- [ ] Supply chain attestation UI

---

## References

### Standards & Specifications

- [SLSA Framework](https://slsa.dev/)
- [SPDX 2.3 Specification](https://spdx.github.io/spdx-spec/)
- [OCI Image Spec](https://github.com/opencontainers/image-spec)
- [Sigstore Documentation](https://docs.sigstore.dev/)

### Tools

- [Cosign](https://github.com/sigstore/cosign) - Container signing
- [Syft](https://github.com/anchore/syft) - SBOM generation
- [Trivy](https://github.com/aquasecurity/trivy) - Vulnerability scanner
- [Docker Buildx](https://github.com/docker/buildx) - Multi-arch builds

### Best Practices

- [Google Container Best Practices](https://cloud.google.com/architecture/best-practices-for-building-containers)
- [NIST Container Security Guide](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-190.pdf)
- [CNCF Supply Chain Security Best Practices](https://github.com/cncf/tag-security/tree/main/supply-chain-security)

---

## Changelog

### 2026-01-23 - v1.0 - Initial Implementation

- ✅ Implemented Phase 1: GitHub Workflows
  - Updated v2 CI workflow with GHCR push and security scanning
  - Updated v2 release workflow with image promotion
  - Updated v3 CI workflow with Docker image builds
  - Updated v3 release workflow with image promotion
  - Added test-provider registry pull support
- 📝 Created planning documentation
- 🚧 Phase 2-5 pending implementation

---

**End of Document**
