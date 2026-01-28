# Sindri V3 Changelog

All notable changes to Sindri V3 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [3.1.0] - 2026-01-28

### :boom: Breaking Changes

**Two-Dockerfile Architecture**

Sindri v3 now uses separate Dockerfiles for production and development modes:

- **Production** (`Dockerfile`): Pre-built binary, runtime extension installation, ~800MB
- **Development** (`Dockerfile.dev`): Source build, bundled extensions, ~1.2GB

**Environment Variable Changes**

Replaced dual environment variables with single unified variable:

- ❌ Removed: `SINDRI_BUILD_FROM_SOURCE` (boolean flag)
- ❌ Removed: `SINDRI_EXTENSIONS_SOURCE` (path string)
- ✅ Added: `SINDRI_EXT_HOME` (unified path variable)

**Path Resolution**

- Extension paths now use `${HOME}` variable expansion (not hardcoded `~` or `/home/developer`)
- Respects `ALT_HOME=/alt/home/developer` volume mount in containers
- Fallback resolution uses `dirs::home_dir()` and `$HOME` env var

### :sparkles: Features

- **Dockerfile Separation**: Production builds (2-5 min, ~800MB) vs Development builds (~8 min, ~1.2GB)
- **Simplified Configuration**: Single `SINDRI_EXT_HOME` variable replaces dual-variable system
- **Faster Production Builds**: Removed extension bundling reduces build time by 40-60%
- **Smaller Production Images**: Removed bundled extensions reduces image size by 33%
- **Volume Mount Support**: Proper `ALT_HOME` volume mount support via `${HOME}` expansion

### :bug: Bug Fixes

- **Path Resolution**: Fixed hardcoded paths that didn't respect `ALT_HOME` volume mount
- **Extension Loading**: Simplified extension loading priority eliminates confusion
- **Registry Loading**: Registry now correctly derives path from `SINDRI_EXT_HOME`

### :rocket: Improvements

- **Code Reduction**: 40-47% code reduction in extension loading logic across 5 Rust files
- **Build Performance**: Production builds 60% faster than previous unified Dockerfile
- **Image Size**: Production images 33% smaller (800MB vs 1.2GB)
- **Clarity**: Dockerfile choice (`Dockerfile` vs `Dockerfile.dev`) makes build mode immediately obvious

### :books: Documentation

- **New Guide**: [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) - Comprehensive Dockerfile selection guide
- **New ADR**: [ADR-040: Two-Dockerfile Architecture](docs/architecture/adr/040-two-dockerfile-architecture.md)
- **Updated**: [README.md](README.md#building-docker-images) - Build instructions for both Dockerfiles
- **Updated**: [docs/EXTENSIONS.md](docs/EXTENSIONS.md#extension-loading-mechanisms) - New extension loading priority

### :wrench: Migration Guide

#### For Default Users (No Action Required)

If using default `sindri.yaml` without custom build scripts, migration is automatic.

#### For Users with Custom Build Scripts

**Update Docker Build Commands**:

```bash
# Before (v3.0.x)
docker build -f v3/Dockerfile --build-arg BUILD_FROM_SOURCE=true -t sindri:dev .

# After (v3.1.0)
docker build -f v3/Dockerfile.dev -t sindri:dev .  # Development mode
docker build -f v3/Dockerfile -t sindri:prod .     # Production mode
```

**Update Environment Variable References**:

```bash
# Before (v3.0.x)
if [ "$SINDRI_BUILD_FROM_SOURCE" = "true" ]; then
    EXT_PATH=$SINDRI_EXTENSIONS_SOURCE
else
    EXT_PATH=~/.sindri/extensions
fi

# After (v3.1.0)
EXT_PATH=${SINDRI_EXT_HOME:-${HOME}/.sindri/extensions}
```

**Update Path References**:

```bash
# Before (v3.0.x) - Hardcoded paths
EXTENSIONS_DIR=~/.sindri/extensions
EXTENSIONS_DIR=/home/developer/.sindri/extensions

# After (v3.1.0) - Use ${HOME} variable
EXTENSIONS_DIR=${HOME}/.sindri/extensions
EXTENSIONS_DIR=${SINDRI_EXT_HOME}
```

#### For CI/CD Pipelines

**GitHub Actions**:

```yaml
# No changes needed if using v3/Dockerfile
# CI automatically uses production Dockerfile with pre-built binary

# Before (v3.0.x)
build-args: |
  BUILD_FROM_SOURCE=false

# After (v3.1.0) - Remove build arg
build-args: |
  SINDRI_VERSION=${{ env.VERSION }}
```

**Makefile**:

```makefile
# Before (v3.0.x)
docker build --build-arg BUILD_FROM_SOURCE=true -f v3/Dockerfile .

# After (v3.1.0)
docker build -f v3/Dockerfile.dev .  # Development
# OR
docker build -f v3/Dockerfile .       # Production
```

### :construction: Affected Files

**Core Changes** (7 files):

- `v3/Dockerfile` - Rewritten for production (pre-built binary, no bundled extensions)
- `v3/Dockerfile.dev` - New development Dockerfile (source build, bundled extensions)
- `v3/crates/sindri-extensions/src/profile.rs` - Simplified extension loading (43 → 25 lines)
- `v3/crates/sindri-extensions/src/registry.rs` - Unified registry loading
- `v3/crates/sindri/src/commands/extension.rs` - Simplified get_extensions_dir() (17 → 9 lines)
- `v3/crates/sindri/src/commands/profile.rs` - Unified path resolution
- `v3/crates/sindri-providers/src/templates/context.rs` - SINDRI_EXT_HOME configuration

**Provider Integration** (4 files):

- `v3/crates/sindri-providers/src/docker.rs` - Dockerfile selection logic
- `v3/crates/sindri-providers/src/fly.rs` - Dockerfile path selection
- `v3/crates/sindri-providers/src/devpod.rs` - Build mode handling
- `v3/crates/sindri-providers/src/e2b.rs` - Template generation

**Build System** (2 files):

- `Makefile` - Updated Docker build targets
- `.github/workflows/ci-v3.yml` - Removed BUILD_FROM_SOURCE arg

**Documentation** (5 files):

- `v3/README.md` - Build instructions
- `v3/docs/EXTENSIONS.md` - Extension loading mechanisms
- `v3/docs/DEPLOYMENT.md` - Dockerfile selection guide
- `v3/docs/architecture/adr/040-two-dockerfile-architecture.md` - ADR document
- `v3/CHANGELOG.md` - This file

### Installation

**Using Sindri CLI** (Recommended):

```bash
# Deploy with production image (default)
sindri deploy

# Deploy with development image (build from source)
sindri config init --profile fullstack
# Edit sindri.yaml:
#   deployment:
#     buildFromSource:
#       enabled: true
sindri deploy
```

**Manual Docker Build**:

```bash
# Production image (pre-built binary, no extensions)
docker build -f v3/Dockerfile -t sindri:3.1.0 .

# Development image (source build, bundled extensions)
docker build -f v3/Dockerfile.dev -t sindri:3.1.0-dev .
```

**Using Makefile**:

```bash
# Production
make v3-docker-build-from-binary

# Development
make v3-docker-build-from-source
```

### :link: References

- **Full Changelog**: https://github.com/pacphi/sindri/compare/v3.0.0...v3.1.0
- **ADR-040**: [Two-Dockerfile Architecture](docs/architecture/adr/040-two-dockerfile-architecture.md)
- **Migration Guide**: [docs/DEPLOYMENT.md#migration-from-v2](docs/DEPLOYMENT.md#migration-from-v2)

---

## [3.0.0] - 2026-01-XX

### :sparkles: Initial Release

Complete rewrite of Sindri in Rust with improved performance, enhanced security, and native container image management.

**Key Features**:

- Multi-provider support (Docker, Fly.io, DevPod, Kubernetes, E2B)
- 40+ modular extensions
- Multi-backend secrets management
- Full workspace backup with encryption
- Project scaffolding templates
- Image security with signature verification
- Local Kubernetes cluster management
- Automatic CLI updates
- Schema validation
- System diagnostics with auto-fix

See [v3/README.md](README.md) for complete feature list and getting started guide.

---

[Unreleased]: https://github.com/pacphi/sindri/compare/v3.1.0...HEAD
[3.1.0]: https://github.com/pacphi/sindri/compare/v3.0.0...v3.1.0
[3.0.0]: https://github.com/pacphi/sindri/releases/tag/v3.0.0
