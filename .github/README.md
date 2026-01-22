# GitHub Actions CI/CD Architecture

This directory contains the bifurcated CI/CD pipeline for Sindri v2 (Bash/Docker) and v3 (Rust).

## Overview

Sindri maintains two parallel versions:

- **v2**: Bash/Docker-based CLI (stable, production-ready)
- **v3**: Rust-based CLI (in active development)

To support independent development and releases, the CI/CD pipeline is split based on path triggers.

## Directory Structure

```
.github/
├── workflows/
│   ├── ci-v2.yml              # v2 CI pipeline (Docker builds, shell tests)
│   ├── ci-v3.yml              # v3 CI pipeline (Rust builds, cargo tests)
│   ├── release-v2.yml         # v2 releases (Docker images)
│   ├── release-v3.yml         # v3 releases (Rust binaries)
│   ├── validate-yaml.yml      # YAML validation for both versions
│   ├── check-links.yml        # Link checking for documentation
│   ├── test-extensions-v2.yml # v2 extension testing
│   └── ...
├── actions/
│   ├── v2/                    # v2-specific reusable actions
│   ├── v3/                    # v3-specific reusable actions
│   │   ├── setup-rust/        # Rust toolchain setup with caching
│   │   └── build-rust/        # Rust workspace build
│   └── shared/                # Shared actions (formerly core/)
└── dependabot.yml             # Dependency updates for npm, cargo, docker, actions
```

## Path-Based Triggers

| Changed Path                  | Triggers         | Example                                 |
| ----------------------------- | ---------------- | --------------------------------------- |
| `v2/**`                       | `ci-v2.yml`      | Changes to v2 code, scripts, extensions |
| `v3/**`                       | `ci-v3.yml`      | Changes to v3 Rust code, extensions     |
| `.github/workflows/ci-v2.yml` | `ci-v2.yml`      | Self-trigger for workflow changes       |
| `.github/workflows/ci-v3.yml` | `ci-v3.yml`      | Self-trigger for workflow changes       |
| `.github/actions/v2/**`       | `ci-v2.yml`      | v2 action changes                       |
| `.github/actions/v3/**`       | `ci-v3.yml`      | v3 action changes                       |
| `.github/actions/shared/**`   | Both             | Shared action changes affect both       |
| `package.json`                | `ci-v2.yml`      | Root tooling affects v2 validation      |
| Tags `v2.*.*`                 | `release-v2.yml` | v2 release trigger                      |
| Tags `v3.*.*`                 | `release-v3.yml` | v3 release trigger                      |

## CI Workflows

### ci-v2.yml - v2 Bash/Docker CI

**Triggers**: Changes to `v2/` directory

**Jobs**:

1. **Validation**
   - Shellcheck (v2 shell scripts)
   - Markdownlint (v2 documentation)
   - YAML validation (v2 extensions, registry)

2. **Build**
   - Build Docker image from `v2/Dockerfile`
   - Save as artifact for testing

3. **Testing**
   - Test on multiple providers (docker, fly, devpod-k8s)
   - Extension tests
   - Profile tests

4. **Status**
   - Required checks gate
   - Overall CI status

**Manual Triggers**:

```bash
# Workflow dispatch allows customization
- Select providers to test
- Choose test level (quick, extension, profile, all)
- Select extension profile
- Skip cleanup for debugging
```

### ci-v3.yml - v3 Rust CI

**Triggers**: Changes to `v3/` directory

**Jobs**:

1. **Rust Validation**
   - `cargo fmt --check` (formatting)
   - `cargo clippy` (linting)
   - `cargo test` (unit tests)
   - `cargo build --release` (release build)

2. **YAML Validation**
   - Validate v3 extensions
   - Validate v3 registry
   - Validate v3 schemas

3. **Security & Documentation**
   - `cargo audit` (security vulnerabilities)
   - `cargo doc` (documentation build)

4. **Extension Tests**
   - Validate v3 extensions (when available)

5. **Status**
   - Required checks gate
   - Overall CI status

**Caching**:

- Cargo dependencies cached with `actions/cache@v5`
- Cache key includes `Cargo.lock` hash
- Restore keys for fallback

## Release Workflows

### release-v2.yml - v2 Docker Releases

**Trigger**: Git tags matching `v2.*.*` (e.g., `v2.3.0`, `v2.3.1-beta.1`)

**Process**:

1. Validate tag format (`v2.x.y`)
2. Generate changelog from `v2/` commits
3. Build Docker image from `v2/Dockerfile`
4. Push to GHCR with tags:
   - `ghcr.io/pacphi/sindri:v2.3.0`
   - `ghcr.io/pacphi/sindri:v2.3`
   - `ghcr.io/pacphi/sindri:v2`
   - `ghcr.io/pacphi/sindri:latest` (for stable releases)
5. Update `v2/cli/VERSION` and `v2/CHANGELOG.md`
6. Create GitHub release with install script
7. Commit version updates to main branch

**Creating a v2 Release**:

```bash
# Create and push tag
git tag v2.3.0
git push origin v2.3.0

# Or with message
git tag -a v2.3.0 -m "Release v2.3.0"
git push origin v2.3.0
```

### release-v3.yml - v3 Rust Binary Releases

**Trigger**: Git tags matching `v3.*.*` (e.g., `v3.0.0`, `v3.1.0-alpha.1`)

**Process**:

1. Validate tag format (`v3.x.y`)
2. Generate changelog from `v3/` commits
3. Build release binaries for multiple platforms:
   - Linux (x86_64, aarch64)
   - macOS (x86_64, aarch64/Apple Silicon)
   - Windows (x86_64)
4. Create release archives:
   - `.tar.gz` for Unix platforms
   - `.zip` for Windows
5. Update `v3/Cargo.toml` version and `v3/CHANGELOG.md`
6. Create GitHub release with binary assets
7. Include smart install script (auto-detects platform)
8. Commit version updates to main branch

**Creating a v3 Release**:

```bash
# Create and push tag
git tag v3.0.0
git push origin v3.0.0

# Or with message
git tag -a v3.0.0 -m "Release v3.0.0 - First Rust release"
git push origin v3.0.0
```

## Validation Workflows

### validate-yaml.yml - Unified YAML Validation

**Validates both v2 and v3**:

- **v2**: `v2/docker/lib/*.yaml`, `v2/docker/lib/extensions/*/extension.yaml`
- **v3**: `v3/extensions/*/extension.yaml`, `v3/registry.yaml`, `v3/schemas/*.yaml`
- **GitHub**: `.github/workflows/*.yml`

**Jobs**:

- YAML linting (yamllint)
- Schema validation (ajv against JSON schemas)
- Cross-reference checking (registry ↔ extensions)
- Extension consistency (naming, categories)

### check-links.yml - Documentation Link Checking

**Checks all markdown files** in both v2 and v3:

- Internal links (file:// scheme)
- External links (scheduled weekly, optional on PR)

## Extension Testing

### test-extensions-v2.yml

Tests v2 extensions in `v2/docker/lib/extensions/`:

- Docker-based testing
- Validates extension.yaml against schema
- Checks registry consistency
- Tests installation and functionality

### test-extensions-v3.yml (Future)

Will test v3 extensions in `v3/extensions/`:

- Rust-based testing using v3 CLI
- Schema validation
- Registry consistency
- Installation verification

## Dependabot Configuration

Automated dependency updates for all ecosystems:

```yaml
# Root npm (tooling)
- package-ecosystem: "npm"
  directory: "/"
  schedule: weekly
  labels: ["dependencies", "tooling"]

# v2 extensions npm
- package-ecosystem: "npm"
  directory: "/v2/docker/lib/extensions"
  schedule: weekly
  labels: ["dependencies", "v2", "extensions"]

# v3 Cargo workspace
- package-ecosystem: "cargo"
  directory: "/v3"
  schedule: weekly
  labels: ["dependencies", "v3"]
  groups: workspace-dependencies

# Docker (v2)
- package-ecosystem: "docker"
  directory: "/v2"
  schedule: weekly
  labels: ["dependencies", "v2"]

# GitHub Actions
- package-ecosystem: "github-actions"
  directory: "/"
  schedule: monthly
  labels: ["dependencies", "ci"]
```

## Package.json Scripts

All scripts are version-prefixed to avoid confusion:

**v2 Commands**:

```bash
pnpm v2:validate        # Validate v2 code
pnpm v2:lint            # Lint v2 code
pnpm v2:test            # Run v2 tests
pnpm v2:build           # Build v2 Docker image
pnpm v2:deploy          # Deploy v2
pnpm v2:ci              # Run v2 CI locally
```

**v3 Commands**:

```bash
pnpm v3:validate        # Validate v3 code (Rust + YAML)
pnpm v3:lint            # Lint v3 code
pnpm v3:test            # Run v3 tests (cargo test)
pnpm v3:build           # Build v3 binaries (cargo build --release)
pnpm v3:clippy          # Run clippy linter
pnpm v3:fmt             # Check Rust formatting
pnpm v3:audit           # Security audit
pnpm v3:ci              # Run v3 CI locally
```

**Shared Commands** (apply to both versions):

```bash
pnpm format             # Format all files (prettier)
pnpm links:check        # Check markdown links
pnpm deps:check         # Check for dependency updates
pnpm audit              # Security audit (npm)
```

## Branch Protection

Recommended branch protection rules for `main`:

**Status Checks Required**:

- `CI v2 Required Checks` (from ci-v2.yml)
- `CI v3 Required Checks` (from ci-v3.yml)

**Settings**:

- Require pull request reviews (1 approver)
- Require status checks to pass
- Require branches to be up to date
- Include administrators

## Common Tasks

### Running CI Locally

**v2**:

```bash
# Full v2 CI
pnpm v2:ci

# Individual steps
pnpm v2:validate
pnpm v2:lint
pnpm v2:test
pnpm v2:build
```

**v3**:

```bash
# Full v3 CI
pnpm v3:ci

# Individual steps
pnpm v3:validate
pnpm v3:lint
pnpm v3:test
pnpm v3:build
```

### Creating Releases

**v2 Release**:

```bash
# Update version in v2/cli/VERSION if needed
echo "2.3.0" > v2/cli/VERSION

# Commit changes
git add v2/
git commit -m "chore(v2): prepare for v2.3.0 release"

# Tag and push
git tag v2.3.0
git push origin main v2.3.0
```

**v3 Release**:

```bash
# Update version in v3/Cargo.toml (workspace.package.version)
sed -i 's/version = ".*"/version = "3.0.0"/' v3/Cargo.toml

# Commit changes
git add v3/
git commit -m "chore(v3): prepare for v3.0.0 release"

# Tag and push
git tag v3.0.0
git push origin main v3.0.0
```

### Debugging Failed Workflows

1. **Check the logs**: Click on the failed job in GitHub Actions
2. **Run locally**: Use `pnpm v2:ci` or `pnpm v3:ci`
3. **Manual trigger**: Use workflow_dispatch with custom options
4. **Skip cleanup**: Enable "skip-cleanup" option to inspect state

### Adding New Actions

**For v2**:

```bash
mkdir -p .github/actions/v2/my-action
# Create action.yml
# Reference in ci-v2.yml
```

**For v3**:

```bash
mkdir -p .github/actions/v3/my-action
# Create action.yml
# Reference in ci-v3.yml
```

**Shared**:

```bash
mkdir -p .github/actions/shared/my-action
# Create action.yml
# Reference in both ci-v2.yml and ci-v3.yml
```

## Extension Management

### v2 Extensions

**Location**: `v2/docker/lib/extensions/`
**Registry**: `v2/docker/lib/registry.yaml`
**Includes**: All extensions, including VisionFlow (vf-\* prefixed)

### v3 Extensions

**Location**: `v3/extensions/`
**Registry**: `v3/registry.yaml`
**Excludes**: VisionFlow extensions (clean break from v2)

**Migrated**: 44 extensions from v2 (excluding 33 vf-\* extensions)

## Troubleshooting

### CI Not Triggering

**Check path patterns**: Ensure changed files match path triggers in workflow `on.paths`

```yaml
# ci-v2.yml triggers on:
- v2/**
- .github/workflows/ci-v2.yml
- .github/actions/v2/**
```

### Both v2 and v3 CI Running

This is expected if you change files in both directories or shared actions.

### Release Tag Format Error

**Error**: "Invalid tag format"

**Solution**: Use correct format:

- v2 releases: `v2.x.y` (e.g., v2.3.0, v2.3.1-beta.1)
- v3 releases: `v3.x.y` (e.g., v3.0.0, v3.1.0-alpha.1)

### Cache Issues

**Clear cache**: Go to Actions → Caches → Delete cache

**Or**: Push with `[skip ci]` in commit message, then push again

## Future Enhancements

1. **v3 Extension Testing**: Once v3 CLI is functional, enable extension tests
2. **Cross-version Testing**: Test v2 → v3 migration scenarios
3. **Performance Benchmarks**: Compare v2 and v3 performance
4. **Automated Migration Tool**: Help users migrate from v2 to v3
5. **Feature Parity Dashboard**: Track v2 vs v3 capabilities

## Related Documentation

- [ADR-021: Bifurcated CI/CD v2 and v3](../v3/docs/architecture/adr/021-bifurcated-ci-cd-v2-v3.md)
- [v2 Documentation](../v2/docs/)
- [v3 Documentation](../v3/docs/)
- [Contributing Guide](../CONTRIBUTING.md)
