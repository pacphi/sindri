# ADR 037: Image Naming and Tagging Strategy

**Status**: Accepted
**Date**: 2026-01-27
**Deciders**: Core Team
**Related**: [ADR-034: Image Handling Consistency Framework](034-image-handling-consistency-framework.md)

## Context

During implementation of ADR-034, we discovered inconsistencies in how Docker images are named and tagged:

### Current Issues

1. **On-demand builds used project name**: When building from cloned Sindri repository, images were tagged as `{project-name}:latest` instead of `sindri:on-demand`
2. **"latest" tag ambiguity**: The `latest` tag was being used for on-demand builds, conflicting with official release tagging
3. **Confusion about official tagging**: Need to clearly document the release tagging strategy for v2 and v3

### Requirements

- On-demand builds should clearly indicate they're locally built, not official releases
- Image naming should be consistent (always "sindri", never project name)
- Tagging should distinguish between stable releases, prereleases, and on-demand builds
- Users should easily identify official vs locally-built images

## Decision

### On-Demand Build Naming (Source Builds)

When users deploy without specifying an image, Sindri clones its own repository and builds locally. These builds MUST be tagged following semantic versioning pre-release convention:

```
sindri:{cli_version}-{gitsha}
```

**Example**:

```
sindri:3.0.0-a1b2c3d
```

Where:

- `3.0.0` = CLI version (from `CARGO_PKG_VERSION`)
- `a1b2c3d` = Short Git SHA (7 characters) of the cloned commit

**Rationale**:

- **Semantic Versioning**: Follows semver pre-release format (`{version}-{prerelease}`)
- **Traceability**: Git SHA uniquely identifies the exact source code used
- **Version Alignment**: Shows which CLI version it's associated with
- **Distinguishes from Releases**: Pre-release format clearly indicates not an official release
- **Consistent Naming**: Always "sindri", never project-specific
- **No Ambiguity**: Can't be confused with `latest` or official release tags

### Official Release Tagging

#### v2 Release Tags

For stable release `v2.3.0`:

```
ghcr.io/pacphi/sindri:v2.3.0    # Full version (with v prefix)
ghcr.io/pacphi/sindri:v2.3      # Major.minor
ghcr.io/pacphi/sindri:v2        # Version alias (stable only)
ghcr.io/pacphi/sindri:latest    # Latest stable release
```

For prerelease `v2.3.0-beta.1`:

```
ghcr.io/pacphi/sindri:v2.3.0-beta.1  # Full version
ghcr.io/pacphi/sindri:v2.3           # Major.minor (allows prereleases)
# No v2 or latest tags for prereleases
```

#### v3 Release Tags

For stable release `v3.0.0`:

```
ghcr.io/pacphi/sindri:3.0.0     # Full version (NO v prefix)
ghcr.io/pacphi/sindri:3.0       # Major.minor
ghcr.io/pacphi/sindri:3         # Major version (stable only)
ghcr.io/pacphi/sindri:v3        # Version alias (stable only)
ghcr.io/pacphi/sindri:latest    # Latest stable release across all versions
```

For prerelease `v3.0.0-alpha.1`:

```
ghcr.io/pacphi/sindri:3.0.0-alpha.1  # Full version
ghcr.io/pacphi/sindri:3.0            # Major.minor (allows prereleases)
# No 3, v3, or latest tags for prereleases
```

### Key Differences Between v2 and v3

| Aspect             | v2                | v3                  |
| ------------------ | ----------------- | ------------------- |
| Version tag prefix | `v2.3.0` (with v) | `3.0.0` (without v) |
| Version alias      | `v2`              | `v3`                |
| Major version tag  | No                | `3` (stable only)   |
| Latest tag         | Yes (stable only) | Yes (stable only)   |

### User-Facing Image References

**Recommended deployment configurations**:

```yaml
# Pin to specific stable version (recommended for production)
deployment:
  image: ghcr.io/pacphi/sindri:3.0.0

# Or use image_config for version resolution
deployment:
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"

# Track latest v3 stable (convenient for development)
deployment:
  image: ghcr.io/pacphi/sindri:v3

# On-demand build from source (no image specified)
deployment:
  provider: docker
  # No image field = builds from GitHub repo as sindri:{cli_version}-{gitsha}
  # Example: sindri:3.0.0-a1b2c3d
```

## Consequences

### Positive

1. **Clear Distinction**: Users can easily tell official releases from source builds
2. **Full Traceability**: Git SHA uniquely identifies exact source code used
3. **Consistent Naming**: All images named "sindri", never project-specific
4. **Semver Compliance**: Source builds follow semantic versioning pre-release conventions
5. **Safe Defaults**: `latest` always points to most recent stable release
6. **Version Flexibility**: Users can pin to exact version, major.minor, or version alias
7. **Prerelease Safety**: Prereleases don't pollute stable tags
8. **Reproducible Builds**: Can rebuild exact same image using the git SHA
9. **Documentation**: Clear tagging strategy documented

### Negative

1. **Breaking Change**: Projects using old `{name}:latest` builds will need to update
2. **Two Schemas**: v2 and v3 have different tagging patterns
3. **Migration**: Existing deployments may reference old tags

### Risks & Mitigation

**Risk**: Users confused about which tag to use
**Mitigation**: Document recommended patterns in CONFIGURATION.md

**Risk**: On-demand builds conflict with pulled images
**Mitigation**: Use distinct `on-demand` tag instead of `latest`

**Risk**: Prereleases accidentally used in production
**Mitigation**: Prerelease tags never get `latest` or version alias

## Implementation

### Changes Made (2026-01-27)

- `sindri-providers/src/utils.rs:193-211`: Added `get_git_sha()` function to extract git SHA from cloned repository
- `sindri-providers/src/docker.rs:463-497`: Changed source build tag from `{name}:latest` to `sindri:{cli_version}-{gitsha}`
- `sindri-providers/src/docker.rs:741-753`: Updated plan() to use `sindri:{cli_version}-SOURCE` placeholder
- `sindri-providers/src/templates/context.rs:164-168`: Changed template default to `sindri:{cli_version}-SOURCE`

### Future Work

- Update CONFIGURATION.md with tagging strategy examples
- Update IMAGE_MANAGEMENT.md with official vs on-demand guidance
- Consider adding `sindri image list` command to show available tags

## References

**Workflows**:

- `.github/workflows/release-v2.yml:349-354` - v2 tagging strategy
- `.github/workflows/release-v3.yml:385-390` - v3 tagging strategy

**Related ADRs**:

- [ADR-034: Image Handling Consistency Framework](034-image-handling-consistency-framework.md)
- [ADR-036: Build-Time Image Metadata Caching](036-build-time-image-metadata-caching.md)
