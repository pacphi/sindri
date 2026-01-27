# ADR 034: Image Handling Consistency Framework

**Status**: Accepted
**Date**: 2026-01-27
**Updated**: 2026-01-27
**Deciders**: Core Team
**Related**: [ADR-002: Provider Abstraction Layer](002-provider-abstraction-layer.md), [ADR-005: Provider-Specific Implementations](005-provider-specific-implementations.md)

## Context

The Sindri v3 container-based providers (Docker, Fly, DevPod, E2B, Kubernetes) have inconsistent image handling behavior that creates confusion and limits flexibility:

### Current Inconsistencies

1. **Image Config Unused**: Despite `image_config` being defined in the schema with semantic versioning, signature verification, and provenance attestation support, NONE of the providers use it
2. **Build Support Discrepancy**:
   - Docker: Has unused `build_image()` method (dead code)
   - Fly: Always builds from Dockerfile, ignores `image` field entirely
   - DevPod: Smart conditional builds (cloud vs local)
   - E2B: Always builds from Dockerfile, transforms to template
   - Kubernetes: Image pull only (correct per K8s best practices)
3. **Dockerfile Path Inconsistency**:
   - Fly: Hardcoded `v3/Dockerfile`
   - DevPod: `./Dockerfile` at base_dir (project root parent)
   - E2B: `./Dockerfile` at current working directory
4. **Image Field Relevance Varies**:
   - Docker/Kubernetes: Critical (specifies which image to use)
   - DevPod: Conditional (only used if not building)
   - Fly: Ignored (always builds from Dockerfile)
   - E2B: Not used at all

### Industry Best Practices Research

**Fly.io** ([docs](https://fly.io/docs/flyctl/deploy/)):

- **Supports both** pre-built images AND Dockerfile builds
- Priority: `--image` flag > `[build]` section > Dockerfile
- Best practice: "Build once, deploy many times"
- Performance: Skip 2-5 minute builds when using pre-built images

**DevPod** ([docs](https://devpod.sh/docs/developing-in-workspaces/prebuild-a-workspace)):

- Smart prebuild system with hash-based caching
- Performance: Pre-built images reduce startup from 4-5 min to <10 sec
- Auto-caches builds locally with Docker provider

**Kubernetes** ([Google Cloud](https://cloud.google.com/blog/products/containers-kubernetes/kubernetes-best-practices-how-and-why-to-build-small-container-images)):

- **DO NOT rebuild images** during deployment
- Build once in CI/CD, promote using immutable digests
- Avoid `latest` tag in production

### User Impact

**Current Experience**:

```yaml
# User expects this to work for Docker provider:
deployment:
  provider: docker
  image_config:
    registry: ghcr.io/myorg/app
    version: "^1.0.0"  # ❌ IGNORED - not implemented

# User expects this to work for Fly provider:
deployment:
  provider: fly
  image: ghcr.io/myorg/app:v1.0.0  # ❌ IGNORED - always builds
```

**Desired Experience**:

- Consistent behavior across providers
- Pre-built images work everywhere (except E2B if unsupported)
- Dockerfile builds work for local dev (Docker, DevPod, E2B)
- CI/CD workflows supported (build in CI, deploy via Sindri)

## Decision

### Standardized Image Resolution Priority

All providers follow this **5-level priority chain** (no default fallback):

```
1. image_config.digest         → Immutable (production-safe)
2. image_config.tag_override   → Explicit tag
3. image_config.version        → Semantic version constraint
4. image                       → Legacy full reference
5. Local Dockerfile            → Build on-demand (provider-dependent)
```

**No Default Fallback**: When no image is configured (neither `image` nor `image_config`), the system returns an error from `resolve_image()` rather than defaulting to `ghcr.io/pacphi/sindri:latest`. This supports:

- **Build-on-demand providers** (Docker, Fly, DevPod, E2B) that can build from local Dockerfile
- **Clear status reporting**: `sindri status` displays `Image: none` instead of showing a misleading default
- **Explicit configuration**: Users must consciously choose an image or rely on provider-specific build logic

**Implementation**: The `config.resolve_image().await?` method in `sindri-core/src/config/loader.rs:185-297` implements this priority chain and returns `Error::invalid_config("No image configured")` when none is specified.

### Provider-Specific Build Support

| Provider       | Build from Dockerfile? | When?                                     | Override with Image? |
| -------------- | ---------------------- | ----------------------------------------- | -------------------- |
| **Docker**     | ✅ **YES (activate)**  | No image specified OR force               | ✅ Yes               |
| **Fly**        | ✅ YES (keep)          | No image specified OR force               | ✅ **YES (new)**     |
| **DevPod**     | ✅ YES (keep)          | Smart: cloud=build+push, local=dockerfile | ✅ Yes (works)       |
| **E2B**        | ✅ YES (keep)          | Always (template system)                  | ⚠️ TBD (research)    |
| **Kubernetes** | ❌ NO (keep)           | Never - CI/CD only                        | ✅ Yes (works)       |

### Dockerfile Path Standardization

**Search order** for all providers:

```
1. ./Dockerfile            # Project root (default)
2. ./v3/Dockerfile         # Sindri v3 specific (fallback)
3. ./deploy/Dockerfile     # Deploy-specific (fallback)
```

**Shared helper** in `sindri-providers/src/utils.rs`:

```rust
pub fn find_dockerfile() -> Option<PathBuf> {
    let candidates = vec!["./Dockerfile", "./v3/Dockerfile", "./deploy/Dockerfile"];
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}
```

### Key Changes by Provider

**Docker** (`docker.rs`):

- Remove `#[allow(dead_code)]` from `build_image()` method
- Add build logic when no image specified
- Use `find_dockerfile()` helper

**Fly** (`fly.rs`):

- Add `flyctl_deploy_image()` method for pre-built images
- Check for `image`/`image_config` before building
- Route to image deploy OR Dockerfile deploy

**DevPod** (`devpod.rs`):

- ✅ No changes (already optimal)

**E2B** (`e2b.rs`):

- Use `find_dockerfile()` helper
- Research E2B CLI support for image-based templates

**Kubernetes** (`kubernetes.rs`):

- Use `resolve_image()` for `image_config` support
- ✅ No build changes (correctly enforces pre-built images)

## Consequences

### Positive

1. **Consistency**: All providers follow same resolution priority
2. **Flexibility**: Users choose pre-built images OR Dockerfile builds
3. **Performance**: Pre-built images skip build time (2-5 minutes)
4. **CI/CD Ready**: Build in CI, deploy via Sindri
5. **Local Dev**: Docker/DevPod/E2B build locally for fast iteration
6. **Production Safe**: Kubernetes enforces pre-built images from registries
7. **Explicit Configuration**: No hidden defaults - users see `Image: none` in status when unconfigured
8. **Build-on-Demand Support**: Providers can implement their own fallback to Dockerfile builds

### Negative

1. **Code Changes**: ~450 lines across 14 files
2. **Testing Burden**: New unit + integration tests required
3. **Documentation**: All provider docs need updates
4. **E2B Research**: May not support image-based templates
5. **Breaking Change**: Configs without image specification that relied on default fallback will show `Image: none` in status (not an error, just more explicit)

### Risks & Mitigation

**Risk**: Breaking existing Fly workflows that depend on always-build behavior
**Mitigation**: No image specified = build from Dockerfile (backward compatible)

**Risk**: E2B may not support image-based templates
**Mitigation**: Keep Dockerfile-only if unsupported, document limitation

**Risk**: Users confused about when builds happen
**Mitigation**: Comprehensive documentation with decision tree

## Implementation

### Phase 0: GitHub Repository Cloning for Build Context ✅ **COMPLETE**

- Priority: CRITICAL
- Effort: Medium
- **Status**: Implemented on 2026-01-27
- **Changes**:
  - `sindri-core/src/config/loader.rs:288-296`: Removed default fallback from `resolve_image()`, now returns `Error::invalid_config("No image configured")`
  - `sindri/src/commands/status.rs:43-49`: Always display `Image:` field, show "none" when unconfigured
  - `sindri-providers/src/utils.rs:111-211`: **REPLACED** `find_dockerfile()` with `fetch_sindri_build_context()` that shallow clones the Sindri repository from GitHub, added `get_git_sha()` to extract commit SHA
  - `sindri-providers/src/docker.rs:463-497`: Updated `deploy()` to clone Sindri repo, use v3 directory as build context, and tag as `sindri:{cli_version}-{gitsha}`
  - `sindri-providers/src/docker.rs:741-753`: Updated `plan()` to expect GitHub-cloned build context and use `sindri:{cli_version}-SOURCE` placeholder tag
  - `sindri-providers/src/templates/context.rs:164-168`: Changed template context to use `sindri:{cli_version}-SOURCE` when no image specified
- **Rationale**:
  - **CRITICAL**: Sindri v3 provides the containerized development environment - users should NOT provide their own Dockerfiles
  - Shallow clones the entire Sindri repository to get v3 directory with all dependencies (Dockerfile, scripts, build context)
  - Version-matched to CLI version (tries `v{version}` tag, falls back to `main` branch)
  - Cached in `~/.cache/sindri/repos/sindri-{version}/v3/` for reuse
  - **Source build tagging**: Uses semver pre-release format `sindri:{version}-{gitsha}` for full traceability
  - Ensures consistency across all deployments
  - Eliminates user confusion about Dockerfile ownership
  - Maintains compatibility with pre-built image workflow

### Phase 1: image_config Support (All Providers)

- Priority: HIGH
- Effort: Medium
- Update all providers to use `config.resolve_image().await?`

### Phase 2: Dockerfile Path Standardization

- Priority: MEDIUM
- Effort: Low
- Create `find_dockerfile()` helper, update all providers

### Phase 3: Activate Docker Build Support

- Priority: HIGH
- Effort: Low
- Remove dead code annotation, add build logic

### Phase 4: Add Fly Image Override

- Priority: HIGH
- Effort: Medium
- Add `flyctl_deploy_image()`, route based on config

### Phase 5: Documentation Updates

- Priority: HIGH
- Effort: Medium
- Update CONFIGURATION.md, IMAGE_MANAGEMENT.md, provider docs

## Alternatives Considered

### Alternative 1: Keep Current Behavior

**Rejected**: Leaves inconsistencies and limits CI/CD workflows

### Alternative 2: Force All Providers to Build

**Rejected**: Anti-pattern for Kubernetes, slower for production deploys

### Alternative 3: Force All Providers to Pre-built Only

**Rejected**: Breaks local dev workflow, requires pushing images

### Alternative 4: Configurable Build/Image Mode

**Rejected**: Too complex, adds another config field

## References

**Industry Best Practices**:

- [Deploy with a Dockerfile · Fly Docs](https://fly.io/docs/languages-and-frameworks/dockerfile/)
- [Prebuild a Workspace | DevPod docs](https://devpod.sh/docs/developing-in-workspaces/prebuild-a-workspace)
- [Kubernetes best practices: Small Container Images | Google Cloud](https://cloud.google.com/blog/products/containers-kubernetes/kubernetes-best-practices-how-and-why-to-build-small-container-images)
- [Building Docker images in Kubernetes | Snyk](https://snyk.io/blog/building-docker-images-kubernetes/)

**Code References**:

- `v3/crates/sindri-core/src/config/loader.rs:185-297` - Image resolution
- `v3/crates/sindri-providers/src/docker.rs:291-323` - Unused build method
- `v3/crates/sindri-providers/src/fly.rs:338-361` - Dockerfile deploy
- `v3/crates/sindri-providers/src/devpod.rs:312-371` - Smart builds

**Planning Document**:

- `v3/docs/planning/complete/image-handling-consistency.md` - Full implementation plan (✅ COMPLETE)
