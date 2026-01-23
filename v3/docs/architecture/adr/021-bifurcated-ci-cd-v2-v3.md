# ADR 021: Bifurcated CI/CD Pipeline for v2 and v3

**Status**: Accepted
**Date**: 2026-01-22
**Deciders**: Core Team
**Related**: Repository reorganization into v2/ and v3/ structure

## Context

Sindri is undergoing a major architectural evolution with the introduction of v3, a complete Rust-based rewrite of the CLI. The repository has been reorganized into distinct `v2/` and `v3/` directories to support parallel development and eventual migration:

- **v2**: Bash/shell-based CLI (version 2.x.x)
  - Docker-based deployment
  - Extension system in `v2/docker/lib/extensions/`
  - Includes VisionFlow-specific extensions (vf-\* prefixed)
  - Mature, production-ready codebase

- **v3**: Rust-based CLI (version 3.x.x)
  - Cargo workspace architecture with multiple crates
  - Standalone extension system in `v3/extensions/`
  - Extensions copied from v2 (excluding vf-\* prefixed ones)
  - Clean break from v2 extension management
  - In active development

The current GitHub Actions workflows in `.github/workflows/` were designed for a single-version repository and do not distinguish between v2 and v3 changes. This creates several problems:

1. **Unnecessary CI Runs**: v2 changes trigger v3 CI (and vice versa)
2. **Mixed Validation**: Rust checks run on v2 PRs, Docker builds run on v3 PRs
3. **Release Confusion**: Single release workflow cannot handle both Docker images (v2) and Rust binaries (v3)
4. **Developer Experience**: Unclear which CI failures are relevant to their changes
5. **Resource Waste**: Running full CI for both versions on every change
6. **Extension Conflicts**: v2 and v3 have separate extension registries but shared validation

The goal is to create **clean separation** of CI/CD pipelines while maintaining shared validation for common concerns (markdown, root-level docs).

## Decision

### 1. Bifurcate CI Workflows

**Replace** `.github/workflows/ci.yml` with two separate workflows:

#### `ci-v2.yml` - v2 Bash/Docker CI

**Path Triggers**:

```yaml
on:
  push:
    branches: [main, develop]
    paths:
      - "v2/**"
      - ".github/workflows/ci-v2.yml"
      - ".github/actions/v2/**"
      - ".github/actions/shared/**"
      - "package.json"
  pull_request:
    branches: [main, develop]
    paths:
      - "v2/**"
      - ".github/workflows/ci-v2.yml"
      - ".github/actions/v2/**"
```

**Jobs**:

- **Shellcheck**: Validate v2 shell scripts (`v2/**/*.sh`)
- **Markdownlint**: Validate v2 documentation (`v2/**/*.md`)
- **Validate YAML**: Validate v2 extensions, registry, schemas
- **Build Docker Image**: Build from `v2/Dockerfile`
- **Test Extensions**: Test extensions in `v2/docker/lib/extensions/`
- **Test Providers**: Test Docker, Fly.io, DevPod deployments
- **CI Status**: Unified status check

#### `ci-v3.yml` - v3 Rust CI

**Path Triggers**:

```yaml
on:
  push:
    branches: [main, develop]
    paths:
      - "v3/**"
      - ".github/workflows/ci-v3.yml"
      - ".github/actions/v3/**"
      - ".github/actions/shared/**"
  pull_request:
    branches: [main, develop]
    paths:
      - "v3/**"
      - ".github/workflows/ci-v3.yml"
      - ".github/actions/v3/**"
```

**Jobs**:

- **Rust Format**: `cargo fmt --check`
- **Rust Clippy**: `cargo clippy --workspace -- -D warnings`
- **Rust Test**: `cargo test --workspace`
- **Rust Build**: `cargo build --release`
- **Validate YAML**: Validate v3 extensions, registry, schemas
- **Security Audit**: `cargo audit`
- **Test Extensions**: Test extensions in `v3/extensions/`
- **CI Status**: Unified status check

### 2. Separate Release Workflows

#### `release-v2.yml` - v2 Docker Releases

**Trigger**: Git tags matching `v2.*.*` (e.g., `v2.3.0`, `v2.3.1-beta.1`)

**Process**:

1. Validate tag format (`v2.x.y`)
2. Generate changelog from `v2/` commits
3. Build Docker image from `v2/Dockerfile`
4. Push to GHCR with tags:
   - `ghcr.io/pacphi/sindri:v2`
   - `ghcr.io/pacphi/sindri:v2.3`
   - `ghcr.io/pacphi/sindri:v2.3.0`
   - `ghcr.io/pacphi/sindri:latest` (until v3 stable)
5. Update version files:
   - `v2/cli/VERSION`
   - `v2/CHANGELOG.md`
6. Create GitHub release with install script
7. Commit version updates to main branch

#### `release-v3.yml` - v3 Rust Binary Releases

**Trigger**: Git tags matching `v3.*.*` (e.g., `v3.0.0`, `v3.1.0-alpha.1`)

**Process**:

1. Validate tag format (`v3.x.y`)
2. Generate changelog from `v3/` commits
3. Build release binaries for multiple platforms:
   - Linux (x86_64, aarch64)
   - macOS (x86_64, aarch64/Apple Silicon)
   - Windows (x86_64)
4. Create release archives:
   - `sindri-v3.0.0-linux-x86_64.tar.gz`
   - `sindri-v3.0.0-macos-aarch64.tar.gz`
   - `sindri-v3.0.0-windows-x86_64.zip`
5. Update version files:
   - `v3/Cargo.toml` (workspace version)
   - `v3/CHANGELOG.md`
6. Create GitHub release with binary assets
7. (Optional) Publish to crates.io
8. Commit version updates to main branch

### 3. Reorganize GitHub Actions

**Directory Structure**:

```
.github/actions/
├── v2/                          # v2-specific actions
│   ├── build-image/
│   │   └── action.yml           # Build v2 Docker image
│   ├── test-extensions/
│   │   └── action.yml           # Test v2 extensions
│   └── deploy-provider/
│       └── action.yml           # Deploy v2 to provider
├── v3/                          # v3-specific actions
│   ├── setup-rust/
│   │   └── action.yml           # Setup Rust toolchain + caching
│   ├── build-rust/
│   │   └── action.yml           # Build v3 workspace
│   ├── test-extensions/
│   │   └── action.yml           # Test v3 extensions
│   └── release-binaries/
│       └── action.yml           # Cross-compile release binaries
└── shared/                      # Shared utilities (renamed from core/)
    ├── setup-common-tools/
    │   └── action.yml           # Install yq, jq, etc.
    └── checkout-with-lfs/
        └── action.yml
```

### 4. Update Validation Workflows

#### `validate-yaml.yml`

**Update to validate both v2 and v3 paths**:

```yaml
jobs:
  validate-v2-yaml:
    name: Validate v2 YAML
    steps:
      - yamllint v2/docker/lib/**/*.yaml
      - Validate v2 extensions against v2/docker/lib/schemas/
      - Validate v2/docker/lib/registry.yaml

  validate-v3-yaml:
    name: Validate v3 YAML
    steps:
      - yamllint v3/extensions/**/*.yaml
      - yamllint v3/schemas/**/*.yaml
      - Validate v3 extensions against v3/schemas/
      - Validate v3/registry.yaml

  validate-github-yaml:
    name: Validate GitHub Workflows
    steps:
      - yamllint .github/workflows/*.yml
```

#### `check-links.yml`

**Already scans all markdown**, ensure both `v2/docs/**` and `v3/docs/**` included.

#### Extension Testing

**Separate workflows**:

- `test-extensions-v2.yml`: Tests `v2/docker/lib/extensions/` (Docker-based)
- `test-extensions-v3.yml`: Tests `v3/extensions/` (Rust-based, when CLI ready)

### 5. Dependabot Configuration

**Update `.github/dependabot.yml`**:

```yaml
version: 2
updates:
  # Root package.json (tooling)
  - package-ecosystem: "npm"
    directory: "/"
    schedule:
      interval: "weekly"
    labels: ["dependencies", "tooling"]

  # v2 extension dependencies
  - package-ecosystem: "npm"
    directory: "/v2/docker/lib/extensions"
    schedule:
      interval: "weekly"
    labels: ["dependencies", "v2", "extensions"]

  # v3 Cargo workspace
  - package-ecosystem: "cargo"
    directory: "/v3"
    schedule:
      interval: "weekly"
    labels: ["dependencies", "v3"]
    groups:
      workspace-dependencies:
        patterns: ["*"]

  # GitHub Actions
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "monthly"
    labels: ["dependencies", "ci"]
```

### 6. Extension Architecture

**Clean Separation**:

- **v2**: Extensions in `v2/docker/lib/extensions/` (includes vf-\* VisionFlow extensions)
- **v3**: Extensions in `v3/extensions/` (copied from v2, excluding vf-\*)
- **v3 Registry**: Independent `v3/registry.yaml` (no shared state with v2)

**Migration Strategy**:

```bash
# Script to copy extensions from v2 to v3 (excluding vf-*)
for ext in v2/docker/lib/extensions/*/; do
  ext_name=$(basename "$ext")
  if [[ "$ext_name" != vf-* ]]; then
    cp -r "$ext" "v3/extensions/$ext_name"
  fi
done
```

## Consequences

### Positive

1. **Clean Separation**: v2 and v3 CI run independently based on changed paths
2. **Faster CI**: Only relevant checks run (no wasted resources)
3. **Clear Releases**: Separate tag namespaces (`v2.*.*` vs `v3.*.*`)
4. **Developer Clarity**: PRs show only relevant CI checks
5. **Independent Evolution**: v2 and v3 can evolve at different paces
6. **Extension Independence**: v3 extensions managed separately from v2
7. **Reduced Confusion**: Clear which version is being worked on
8. **Parallel Development**: Teams can work on v2 and v3 simultaneously
9. **Future-Proof**: Easy to deprecate v2 when v3 is stable
10. **Binary Distribution**: v3 releases produce platform-specific binaries (not Docker-only)

### Negative

1. **Workflow Duplication**: Some logic duplicated between ci-v2.yml and ci-v3.yml
2. **Action Duplication**: Similar actions in `v2/` and `v3/` directories
3. **Maintenance Burden**: Two sets of workflows to maintain
4. **Extension Migration**: Manual effort to copy extensions from v2 to v3
5. **Documentation Sprawl**: Need to document both v2 and v3 CI processes
6. **Testing Complexity**: Need to test both CI pipelines
7. **Release Coordination**: Potential confusion if both v2 and v3 release simultaneously
8. **Branch Protection**: Need to update required status checks for both versions

### Neutral

1. **Path-Based Triggers**: Requires developers to understand which paths trigger which CI
2. **Tag Naming**: Developers must use correct tag format (`v2.*.*` vs `v3.*.*`)
3. **Extension Divergence**: v2 and v3 extensions will diverge over time
4. **Shared Actions**: Some actions (like `setup-common-tools`) remain shared

## Alternatives Considered

### 1. Single Unified CI with Job Filtering

**Description**: Keep single `ci.yml` with conditional job execution based on changed paths.

**Pros**:

- Single workflow to maintain
- Less duplication
- Unified status checks

**Cons**:

- Complex conditional logic
- Hard to understand which jobs run when
- All jobs show in PR (even skipped ones)
- Harder to debug failures

**Rejected**: Complexity outweighs benefits. Separate workflows provide clearer separation.

### 2. Monorepo Tools (Turborepo, Nx)

**Description**: Use monorepo tools to manage v2 and v3 as separate projects.

**Pros**:

- Automatic change detection
- Smart caching
- Task orchestration

**Cons**:

- Heavy dependency (Node.js required)
- Overkill for two projects
- Learning curve
- Not designed for mixed Bash/Rust repos

**Rejected**: Too heavyweight for our needs. Path-based triggers are sufficient.

### 3. Separate Repositories

**Description**: Split v2 and v3 into completely separate repos.

**Pros**:

- Complete isolation
- No CI trigger conflicts
- Independent issue tracking
- Cleaner git history

**Cons**:

- Harder to share documentation
- Separate PRs for cross-version changes
- Duplicate tooling setup
- Lose shared history

**Rejected**: We want parallel development in same repo during transition period.

### 4. Feature Flags with Single CI

**Description**: Use environment variables to enable/disable v2 or v3 CI steps.

**Pros**:

- Single workflow
- Runtime control

**Cons**:

- Complex logic
- Easy to misconfigure
- Harder to test
- Not declarative

**Rejected**: Path-based triggers are more declarative and easier to understand.

### 5. Workflow Dispatch Only

**Description**: Remove automatic triggers, require manual workflow runs.

**Pros**:

- Complete control
- No accidental runs

**Cons**:

- Poor developer experience
- Easy to forget to run CI
- Breaks PR automation
- Requires constant vigilance

**Rejected**: Automatic CI is essential for code quality.

## Implementation Plan

### Phase 1: Workflow Creation (Day 1)

- [ ] Create `ci-v2.yml` from existing `ci.yml`
- [ ] Create `ci-v3.yml` with Rust-specific jobs
- [ ] Create `release-v2.yml`
- [ ] Create `release-v3.yml`
- [ ] Update `validate-yaml.yml` for v2/v3 paths
- [ ] Remove old `ci.yml` and `release.yml`

### Phase 2: Actions Reorganization (Day 1)

- [ ] Create `.github/actions/v2/` directory
- [ ] Create `.github/actions/v3/` directory
- [x] Rename `.github/actions/core/` to `.github/actions/shared/`
- [ ] Create `v3/setup-rust/action.yml`
- [ ] Create `v3/build-rust/action.yml`
- [ ] Create `v3/release-binaries/action.yml`

### Phase 3: Extension Migration (Day 2)

- [ ] Create `v3/extensions/` directory
- [ ] Write migration script (copy from v2, exclude vf-\*)
- [ ] Run migration script
- [ ] Create `v3/registry.yaml`
- [ ] Create `test-extensions-v3.yml` workflow

### Phase 4: Dependabot & Package.json (Day 2)

- [ ] Update `.github/dependabot.yml` with v2/v3 configs
- [ ] Update `package.json` scripts for v2/v3 separation
- [ ] Test dependency updates

### Phase 5: Documentation (Day 2-3)

- [ ] Create `.github/README.md` explaining CI architecture
- [ ] Update root README.md with v2/v3 sections
- [ ] Create `docs/MIGRATION_V2_TO_V3.md`
- [ ] Update contributing guidelines

### Phase 6: Testing & Validation (Day 3)

- [ ] Test ci-v2.yml with v2-only PR
- [ ] Test ci-v3.yml with v3-only PR
- [ ] Test release-v2.yml with test tag (`v2.99.99`)
- [ ] Test release-v3.yml with test tag (`v3.99.99`)
- [ ] Verify path-based triggers work correctly

## Path-Based Trigger Matrix

| Changed Path                  | Triggers Workflow | Rationale                          |
| ----------------------------- | ----------------- | ---------------------------------- |
| `v2/**`                       | `ci-v2.yml`       | v2 code changes                    |
| `v3/**`                       | `ci-v3.yml`       | v3 code changes                    |
| `.github/workflows/ci-v2.yml` | `ci-v2.yml`       | Self-trigger for workflow changes  |
| `.github/workflows/ci-v3.yml` | `ci-v3.yml`       | Self-trigger for workflow changes  |
| `.github/actions/v2/**`       | `ci-v2.yml`       | v2 action changes                  |
| `.github/actions/v3/**`       | `ci-v3.yml`       | v3 action changes                  |
| `.github/actions/shared/**`   | Both              | Shared utility changes affect both |
| `package.json`                | `ci-v2.yml`       | Root tooling affects v2 validation |
| Root `docs/`, `README.md`     | Neither (manual)  | Documentation-only changes         |
| Tag `v2.*.*`                  | `release-v2.yml`  | v2 release                         |
| Tag `v3.*.*`                  | `release-v3.yml`  | v3 release                         |

## Success Criteria

1. ✅ v2 changes only trigger ci-v2.yml
2. ✅ v3 changes only trigger ci-v3.yml
3. ✅ Shared action changes trigger both workflows
4. ✅ v2 tags create Docker releases
5. ✅ v3 tags create binary releases
6. ✅ Extension tests run for appropriate version
7. ✅ Dependabot creates version-specific PRs
8. ✅ CI status checks are clear and relevant
9. ✅ Release process is documented and tested
10. ✅ Migration from v2 to v3 is clear for users

## Compliance

- ✅ Separate CI workflows for v2 and v3
- ✅ Path-based triggers prevent cross-version runs
- ✅ Independent release workflows with version-specific tags
- ✅ Reorganized actions into v2/, v3/, shared/
- ✅ Updated Dependabot for multi-ecosystem support
- ✅ Extension separation (v2 includes vf-_, v3 excludes vf-_)
- ✅ Documentation for both CI pipelines

## Notes

This bifurcation supports a **transition period** where both v2 and v3 are actively maintained. Once v3 reaches feature parity and stability:

1. Mark v2 as legacy/maintenance mode
2. Update `latest` Docker tag to point to v3 (when Dockerfile exists)
3. Consider archiving v2 CI (run only on explicit branches)
4. Eventually sunset v2 entirely

The clean separation makes it easy to gradually reduce v2 support without affecting v3 development.

## Related Decisions

- [ADR-001: Rust Migration Workspace Architecture](001-rust-migration-workspace-architecture.md) - Foundation for v3
- [ADR-008: Extension Type System](008-extension-type-system-yaml-deserialization.md) - v3 extension architecture
- [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md) - v3 installation methods

## Future Enhancements

1. **CI Performance Dashboard**: Track CI run times for v2 vs v3
2. **Feature Parity Tracking**: Automated report comparing v2 and v3 capabilities
3. **Automated Migration**: Tool to help users migrate from v2 to v3
4. **Benchmark Comparisons**: Performance comparison between v2 and v3
5. **Unified Documentation Site**: Single docs site with v2/v3 toggle
