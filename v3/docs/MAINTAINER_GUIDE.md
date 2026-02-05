# Sindri v3 Maintainer Guide

## Table of Contents

- [Two Development Paths](#two-development-paths)
- [Quick Reference](#quick-reference)
- [Development Workflow](#development-workflow)
- [Base Image Management](#base-image-management)
- [Cache Management](#cache-management)
- [Build Optimization](#build-optimization)
- [Troubleshooting](#troubleshooting)
- [Release Process](#release-process)

## Two Development Paths

Sindri v3 supports two distinct development workflows. Understanding when to use each is essential for efficient development.

### Path A: Makefile (Local Development) - Recommended

**Use this for daily development. No git push required.**

```bash
make v3-cycle-fast CONFIG=sindri.yaml
```

**How it works:**

```
Your local working directory
         ↓
┌────────────────────────────────────────────────────┐
│ 1. docker build -f Dockerfile.dev .                │
│    • COPY v3/crates/ (your local files)            │
│    • COPY v3/extensions/ (your local files)        │
│    • Compiles from YOUR working directory          │
│    → Creates sindri:latest                         │
│                                                    │
│ 2. cargo install (installs CLI locally)            │
│                                                    │
│ 3. sindri deploy --config sindri.yaml              │
│    • Uses the already-built sindri:latest          │
│    • NO additional build step                      │
└────────────────────────────────────────────────────┘
         ↓
Container running with YOUR LOCAL changes
✅ No push to GitHub required
✅ Test uncommitted changes
✅ Fastest iteration (1-2 min incremental)
```

**When to use:**

- Daily development and testing
- Rapid iteration on CLI code
- Testing extension changes
- Debugging locally before committing

**Your sindri.yaml for Path A:**

```yaml
# sindri.yaml for local development (Path A)
workspace:
  name: sindri-dev02

deployment:
  provider: docker
  image: sindri:latest # ← Key: reference the locally-built image

extensions:
  - cloud-tools
  - github
  # ... your extensions
```

> **Important:** For Path A, use `image: sindri:latest` - nothing else!
>
> | Config Option                       | Use for Path A? | Why                              |
> | ----------------------------------- | --------------- | -------------------------------- |
> | `image: sindri:latest`              | ✅ **Yes**      | Uses the image built by Makefile |
> | `buildFromSource.enabled: true`     | ❌ No           | Would clone from GitHub          |
> | `imageConfig.registry: ghcr.io/...` | ❌ No           | Would pull from registry         |
>
> The Makefile builds `sindri:latest` from your local files. Your config just tells
> `sindri deploy` to use that already-built image.

---

### Path B: CLI --from-source (Remote Testing) - Requires Push

**Use this to verify pushed code works correctly.**

```bash
# First, push your changes
git push origin feature/my-branch

# Then deploy from that branch
sindri deploy --from-source --config sindri.yaml
```

Or via config:

```yaml
# sindri.yaml
deployment:
  buildFromSource:
    enabled: true
    gitRef: "feature/my-branch" # branch, tag, or commit SHA
```

**How it works:**

```
GitHub repository (pushed code)
         ↓
┌────────────────────────────────────────────────────┐
│ 1. sindri deploy --from-source                     │
│    • Clones github.com/pacphi/sindri               │
│    • Uses gitRef from config (or CLI version)      │
│    • Caches clone in ~/.cache/sindri/repos/        │
│                                                    │
│ 2. docker build -f Dockerfile.dev <cloned-repo>    │
│    • Builds from the CLONED repository             │
│    • NOT your local working directory              │
│    → Creates sindri:<version>-<sha>                │
└────────────────────────────────────────────────────┘
         ↓
Container running with PUSHED changes
⚠️  Requires git push first
⚠️  Tests the remote state, not local
```

**When to use:**

- Verifying pushed code before PR merge
- CI/CD pipelines
- Testing specific branches, tags, or commits
- Reproducing issues from a known git state

---

### Quick Decision Guide

| Scenario                              | Use    | Command                         |
| ------------------------------------- | ------ | ------------------------------- |
| "I changed some code, let me test it" | Path A | `make v3-cycle-fast CONFIG=...` |
| "Does my pushed PR work?"             | Path B | `sindri deploy --from-source`   |
| "Test this specific commit"           | Path B | Set `gitRef: "abc1234"`         |
| "Rapid iteration (many changes)"      | Path A | `make v3-cycle-fast CONFIG=...` |
| "CI/CD build"                         | Path B | `--from-source` with gitRef     |

---

### Key Differences Summary

| Aspect                    | Path A (Makefile)       | Path B (--from-source)   |
| ------------------------- | ----------------------- | ------------------------ |
| Source                    | Local working directory | Cloned from GitHub       |
| Requires push             | ❌ No                   | ✅ Yes                   |
| Tests uncommitted changes | ✅ Yes                  | ❌ No                    |
| Build location            | Your machine            | Your machine             |
| Typical use               | Daily dev               | Pre-merge validation     |
| Config needed             | Just `CONFIG=` path     | `gitRef` in yaml or flag |

## Quick Reference

### Daily Development Commands

```bash
# Fast rebuild and deploy (3-5 min)
make v3-cycle-fast CONFIG=sindri.yaml

# Check cache status
make v3-cache-status

# Clear soft cache (incremental only)
make v3-cache-clear-soft
```

### Build Commands

```bash
# Build base image (15-20 min, rare)
make v3-docker-build-base

# Build using base (3-5 min, frequent)
make v3-docker-build-fast

# Build without cache (5-8 min)
make v3-docker-build-fast-nocache
```

### Development Cycle Modes

```bash
# Mode 1: Fast cycle (recommended for daily dev)
make v3-cycle-fast CONFIG=sindri.yaml
# Time: 3-5 min (incremental: 1-2 min)
# Use when: Normal code/extension changes

# Mode 2: Clean cycle (when things break)
make v3-cycle-clean CONFIG=sindri.yaml
# Time: 10-15 min
# Use when: Weird build errors, dependency conflicts

# Mode 3: Nuclear cycle (rarely needed)
make v3-cycle-nuclear CONFIG=sindri.yaml
# Time: 40-50 min
# Use when: Major system changes, complete reset
```

## Development Workflow

### Initial Setup (One-Time)

```bash
# 1. Build base image locally
make v3-docker-build-base
# Time: 15-20 minutes
# This creates: sindri:base-3.0.0, sindri:base-latest

# 2. Verify base image
docker images sindri:base-*

# 3. Build development image
make v3-docker-build-fast
# Time: 3-5 minutes
```

### Daily Development Cycle

#### Typical Workflow

```bash
# 1. Make changes to CLI or extensions
vim v3/crates/sindri/src/commands/deploy.rs
vim v3/extensions/cloud-tools/extension.yaml

# 2. Fast rebuild and deploy
make v3-cycle-fast CONFIG=sindri.yaml

# 3. Test changes
sindri connect --config sindri.yaml

# 4. Iterate quickly (1-2 min rebuilds)
# Make more changes...
make v3-cycle-fast CONFIG=sindri.yaml
```

#### What Triggers Different Build Times

**1-2 minute builds (incremental):**

- Code changes in `v3/crates/**`
- Extension file changes (`v3/extensions/**`)
- Configuration file updates (`registry.yaml`, `profiles.yaml`)

**3-5 minute builds (clean cargo):**

- `Cargo.toml` dependency changes
- After running `make v3-cache-clear-medium`

**10-15 minute builds:**

- After running `make v3-cycle-clean`
- When cargo cache is corrupted

**40-50 minute builds:**

- After running `make v3-cycle-nuclear`
- When base image needs rebuilding from scratch

### When to Use Each Cycle Mode

#### Use `v3-cycle-fast` (87-90% faster)

**When:**

- Normal CLI code changes
- Extension development
- Configuration updates
- Daily development

**What it does:**

- Destroys existing deployment
- Clears incremental compilation cache only
- Rebuilds using cargo dependency cache
- Deploys new container

**What it keeps:**

- ✅ Base image
- ✅ Cargo dependency cache
- ✅ BuildKit cache
- ✅ Docker system cache

#### Use `v3-cycle-clean` (70-80% faster)

**When:**

- Build errors that don't make sense
- Cargo cache seems corrupted
- After major dependency updates
- Weekly cleanup

**What it does:**

- Destroys deployment
- Runs `cargo clean`
- Clears recent BuildKit cache
- Removes all sindri images except base
- Rebuilds from base

**What it keeps:**

- ✅ Base image

#### Use `v3-cycle-nuclear` (nuclear option)

**When:**

- Base image needs rebuilding
- Major Rust version upgrade
- Complete system reset needed
- Rarely (once per quarter)

**What it does:**

- Destroys everything
- Removes ALL images including base
- Clears ALL BuildKit cache
- Runs `cargo clean`
- Rebuilds everything from scratch

**What it keeps:**

- ❌ Nothing (nuclear!)

## Base Image Management

### Understanding the Base Image

The base image (`sindri:base-X.Y.Z`) contains:

- Rust 1.93 toolchain (246MB)
- cargo-chef (for dependency caching)
- System packages (Ubuntu 24.04)
- GitHub CLI v2.86.0
- Developer user setup

**Size:** ~1.2GB

**Build time:** 15-20 minutes

**Rebuild frequency:** Quarterly or when Rust version changes

### Building the Base Image

#### Local Build

```bash
# Build with version tag
make v3-docker-build-base

# This creates two tags:
# - sindri:base-3.0.0 (version-specific)
# - sindri:base-latest (convenience tag)

# Verify
docker images sindri:base-*
```

#### Manual Build

```bash
cd v3
docker build -f Dockerfile.base -t sindri:base-3.0.0 .
docker tag sindri:base-3.0.0 sindri:base-latest
```

### Publishing to GHCR

#### Via GitHub Actions (Recommended)

```bash
# 1. Go to GitHub Actions
# 2. Select "Build Base Image" workflow
# 3. Click "Run workflow"
# 4. Wait ~20 minutes for build
# 5. Verify: docker pull ghcr.io/pacphi/sindri:base-latest
```

#### Manual Push (For Maintainers)

```bash
# 1. Build locally
make v3-docker-build-base

# 2. Login to GHCR
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# 3. Tag for GHCR
docker tag sindri:base-3.0.0 ghcr.io/pacphi/sindri:base-3.0.0
docker tag sindri:base-3.0.0 ghcr.io/pacphi/sindri:base-latest

# 4. Push
docker push ghcr.io/pacphi/sindri:base-3.0.0
docker push ghcr.io/pacphi/sindri:base-latest
```

### When to Rebuild Base Image

Rebuild the base image when:

- **Rust version changes** (e.g., 1.93 → 1.93)

  ```bash
  # Edit v3/Dockerfile.base
  # Change: ARG RUST_VERSION=1.93
  # To:     ARG RUST_VERSION=1.93
  make v3-docker-build-base
  ```

- **Ubuntu version changes** (e.g., 24.04 → 24.10)

  ```bash
  # Edit v3/Dockerfile.base
  # Change: ARG UBUNTU_VERSION=24.04
  make v3-docker-build-base
  ```

- **System package requirements change**

  ```bash
  # After adding new apt packages to Dockerfile.base
  make v3-docker-build-base
  ```

- **GitHub CLI version changes** (optional)
  ```bash
  # Edit v3/Dockerfile.base
  # Change: ARG GH_VERSION=2.86.0
  make v3-docker-build-base
  ```

**Don't rebuild for:**

- CLI code changes (use fast builds)
- Extension changes (use fast builds)
- Configuration updates (use fast builds)
- Cargo.toml changes (cargo-chef handles this)

## Cache Management

### Understanding Caches

Sindri uses multiple cache layers:

1. **Base image** (~1.2GB)
   - Location: Docker images
   - Contains: Rust, system packages, tools
   - Persistence: Permanent until deleted

2. **BuildKit cache** (1-5GB)
   - Location: Docker BuildKit cache
   - Contains: cargo registry, git repos
   - Persistence: Until pruned

3. **Cargo target directory** (2-5GB)
   - Location: `v3/target/`
   - Contains: Compiled dependencies, incremental artifacts
   - Persistence: Until `cargo clean`

4. **Sindri repo cache** (<100MB)
   - Location: `~/.cache/sindri/repos` or `~/Library/Caches/sindri/repos`
   - Contains: Cloned Sindri repository for building
   - Persistence: Until manually deleted

### Cache Commands

#### Check Cache Status

```bash
make v3-cache-status
```

Output shows:

- Docker images (base vs development)
- BuildKit cache size
- Cargo target directory size
- Sindri repo cache

#### Clear Caches Granularly

```bash
# Level 1: Soft clear (incremental only)
make v3-cache-clear-soft
# Clears: Incremental compilation cache, repo cache
# Keeps: Base image, cargo dependencies, BuildKit cache
# Time saved on next build: ~30 seconds

# Level 2: Medium clear (cargo + recent cache)
make v3-cache-clear-medium
# Clears: All cargo artifacts, BuildKit cache <1 hour old
# Keeps: Base image, older BuildKit cache
# Next build time: ~5-8 minutes

# Level 3: Hard clear (everything except base)
make v3-cache-clear-hard
# Clears: Everything except base image
# Keeps: Only base image
# Next build time: ~3-5 minutes (builds from base)

# Level 4: Nuclear (everything including base)
make v3-cache-nuke
# Clears: EVERYTHING
# Keeps: Nothing
# Next build time: 40-50 minutes (full rebuild)
```

### Weekly Maintenance

Run weekly to prevent cache bloat:

```bash
# Option 1: Use medium clear
make v3-cache-clear-medium

# Option 2: Prune old BuildKit cache
docker buildx prune --filter "until=168h" --force

# Option 3: Check and decide
make v3-cache-status
# If BuildKit cache > 10GB, run:
docker buildx prune --filter "until=72h" --force
```

### Monthly Maintenance

Run monthly for deep cleanup:

```bash
# 1. Check what's using space
make v3-cache-status

# 2. Hard clear (keeps base)
make v3-cache-clear-hard

# 3. Remove old base versions
docker images sindri:base-* --format "{{.ID}}\t{{.Tag}}\t{{.CreatedAt}}" | \
  sort -k3 | head -n -2 | awk '{print $1}' | xargs docker rmi -f

# 4. Prune unused images
docker image prune -af --filter "until=720h"  # 30 days
```

## Build Optimization

### Optimizing Incremental Builds

#### Tips for Faster Rebuilds

1. **Touch only necessary files:**

   ```bash
   # Good: Specific file change
   vim v3/crates/sindri/src/commands/deploy.rs
   make v3-cycle-fast  # 1-2 min

   # Bad: Unnecessary changes
   touch v3/Cargo.toml  # Forces dependency rebuild
   make v3-cycle-fast  # 3-5 min
   ```

2. **Use soft cache clears:**

   ```bash
   # Instead of:
   make v3-cycle-clean  # 10-15 min

   # Try:
   make v3-cache-clear-soft
   make v3-cycle-fast   # 3-5 min
   ```

3. **Batch changes:**
   ```bash
   # Instead of: build after each change (5x 1-2 min = 5-10 min)
   # Do: make all changes, then build once (1x 2-3 min)
   ```

### Parallel Development

If working on multiple features:

```bash
# Terminal 1: Feature A
git checkout feature/auth
make v3-cycle-fast CONFIG=sindri-auth.yaml

# Terminal 2: Feature B (separate config)
git checkout feature/deploy
make v3-cycle-fast CONFIG=sindri-deploy.yaml
```

**Note:** Each uses the same base image, different containers.

### CI/CD Optimization

For GitHub Actions:

```yaml
# Cache base image between runs
- name: Cache base image
  uses: actions/cache@v4
  with:
    path: /tmp/base-image.tar
    key: base-image-${{ hashFiles('v3/Dockerfile.base') }}

- name: Load or pull base
  run: |
    if [ -f /tmp/base-image.tar ]; then
      docker load -i /tmp/base-image.tar
    else
      docker pull ghcr.io/pacphi/sindri:base-latest
      docker save ghcr.io/pacphi/sindri:base-latest -o /tmp/base-image.tar
    fi
```

## Troubleshooting

### Common Issues

#### Issue: "sindri:base-latest not found"

**Cause:** Base image hasn't been built yet.

**Solution:**

```bash
make v3-docker-build-base
```

Or pull from GHCR:

```bash
docker pull ghcr.io/pacphi/sindri:base-latest
docker tag ghcr.io/pacphi/sindri:base-latest sindri:base-latest
```

#### Issue: Builds still taking 40+ minutes

**Diagnosis:**

```bash
# Check if using base image
docker history sindri:latest | grep base-latest

# If not found, you're not using the base
```

**Solution:**

```bash
# Ensure Dockerfile.dev starts with:
FROM sindri:base-latest

# Rebuild:
make v3-docker-build-fast
```

#### Issue: Cargo errors about corrupted cache

**Symptoms:**

- `error: could not compile...`
- `error: failed to read metadata`
- Weird compilation errors

**Solution:**

```bash
# Clear cargo cache and rebuild
make v3-cache-clear-medium
make v3-cycle-fast CONFIG=sindri.yaml
```

#### Issue: Docker build fails with "COPY failed"

**Cause:** Building from wrong directory.

**Solution:**

```bash
# Docker context must be repo root:
cd /path/to/sindri
docker build -f v3/Dockerfile.dev -t sindri:latest .
#                                                 ^ Note: . (repo root)
```

#### Issue: apt-get still taking 28 minutes

**Cause:** Not using base image (building from scratch).

**Diagnosis:**

```bash
# Check Dockerfile.dev first line
head -n 50 v3/Dockerfile.dev | grep FROM

# Should see: FROM sindri:base-latest
# If you see: FROM ubuntu:24.04 or FROM rust:1.93
# Then you're not using base!
```

**Solution:**

```bash
# Update Dockerfile.dev to use base
vim v3/Dockerfile.dev
# Change first FROM to: FROM sindri:base-latest
```

### Debug Commands

```bash
# Check what's in an image
docker history sindri:latest --no-trunc

# Check build args
docker inspect sindri:latest | jq '.[0].Config.Env'

# Check base image metadata
docker inspect sindri:base-latest | jq '.[0].Config.Labels'

# List all sindri images
docker images sindri* --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"

# Check BuildKit cache usage
docker buildx du --verbose
```

## Release Process

### Pre-Release Checklist

- [ ] Base image for current Rust version exists on GHCR
- [ ] CI builds passing with base image
- [ ] Documentation up to date
- [ ] CHANGELOG.md updated
- [ ] Version bumped in `Cargo.toml`

### Publishing a Release

```bash
# 1. Ensure base exists
docker pull ghcr.io/pacphi/sindri:base-latest

# 2. Build release binary locally
cd v3
cargo build --release

# 3. Test release binary
./target/release/sindri --version

# 4. Build production Docker image
docker build -f v3/Dockerfile -t sindri:3.1.0 .

# 5. Tag and push
docker tag sindri:3.1.0 ghcr.io/pacphi/sindri:3.1.0
docker tag sindri:3.1.0 ghcr.io/pacphi/sindri:latest
docker push ghcr.io/pacphi/sindri:3.1.0
docker push ghcr.io/pacphi/sindri:latest

# 6. Create GitHub release
gh release create v3.1.0 \
  --title "v3.1.0 - Fast Development Builds" \
  --notes "See CHANGELOG.md"
```

### Post-Release

- [ ] Verify image works: `docker run ghcr.io/pacphi/sindri:3.1.0 --version`
- [ ] Update documentation to reference new version
- [ ] Announce in team channels
- [ ] Monitor for issues

## Performance Benchmarks

### Expected Build Times (ARM64/OrbStack)

| Scenario              | Time      | Notes                     |
| --------------------- | --------- | ------------------------- |
| **Base image build**  | 15-20 min | Once per Rust version     |
| **First fast build**  | 3-5 min   | With clean cargo cache    |
| **Incremental build** | 1-2 min   | Only code changed         |
| **Clean build**       | 3-5 min   | From base, no cargo cache |
| **Full rebuild**      | 40-50 min | Everything from scratch   |

### Expected Build Times (AMD64/Linux)

| Scenario              | Time      | Notes                     |
| --------------------- | --------- | ------------------------- |
| **Base image build**  | 8-12 min  | Faster apt-get on AMD64   |
| **First fast build**  | 2-3 min   | Faster cargo compilation  |
| **Incremental build** | 30-60s    | Only code changed         |
| **Clean build**       | 2-3 min   | From base, no cargo cache |
| **Full rebuild**      | 15-20 min | Everything from scratch   |

### Tracking Build Times

```bash
# Time a build
time make v3-docker-build-fast

# Log build times
echo "$(date): $(time make v3-docker-build-fast 2>&1)" >> build-times.log

# Compare before/after
# Before: mean 45 min
# After: mean 2.5 min
# Improvement: 94.4%
```

## Additional Resources

- [Implementation Plan](planning/complete/fast-dev-builds-base-image.md)
- [Contributing Guide](../../docs/CONTRIBUTING.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)
<!-- - [Architecture: Docker Builds](architecture/docker-build-architecture.md) (TODO: Create this document) -->

## Getting Help

If you encounter issues:

1. **Check this guide** - Most common issues covered
2. **Check cache status** - `make v3-cache-status`
3. **Try clean build** - `make v3-cycle-clean CONFIG=sindri.yaml`
4. **Check logs** - Look for error messages in build output
5. **Ask the team** - Post in #sindri-dev channel

## Appendix: Makefile Target Reference

### Build Targets

| Target                         | Description               | Time      | Use When            |
| ------------------------------ | ------------------------- | --------- | ------------------- |
| `v3-docker-build-base`         | Build base image          | 15-20 min | Rust version change |
| `v3-docker-build-fast`         | Build using base          | 3-5 min   | Daily dev           |
| `v3-docker-build-fast-nocache` | Build without cargo cache | 5-8 min   | Cargo issues        |
| `v3-docker-build-from-source`  | Original slow build       | 40-50 min | Legacy reference    |

### Cache Targets

| Target                  | Description           | Keeps                | Clears              |
| ----------------------- | --------------------- | -------------------- | ------------------- |
| `v3-cache-status`       | Show cache usage      | N/A                  | None                |
| `v3-cache-clear-soft`   | Clear incremental     | Base, deps, BuildKit | Incremental         |
| `v3-cache-clear-medium` | Clear cargo + cache   | Base                 | Cargo, BuildKit <1h |
| `v3-cache-clear-hard`   | Clear all except base | Base                 | Everything else     |
| `v3-cache-nuke`         | Nuclear option        | Nothing              | Everything          |

### Cycle Targets

| Target             | Description              | Time      | Use When     |
| ------------------ | ------------------------ | --------- | ------------ |
| `v3-cycle-fast`    | Fast cycle (recommended) | 3-5 min   | Daily dev    |
| `v3-cycle-clean`   | Clean rebuild            | 10-15 min | Things break |
| `v3-cycle-nuclear` | Full rebuild             | 40-50 min | Rarely       |
