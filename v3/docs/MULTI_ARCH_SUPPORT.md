# Multi-Architecture Support

## Overview

Sindri v3 images are built for multiple architectures to ensure fast, native performance on any development machine:

- **linux/amd64** - x86_64 processors (Intel/AMD)
  - Intel Macs
  - Most Linux servers
  - GitHub Actions runners

- **linux/arm64** - ARM64/aarch64 processors
  - Apple Silicon Macs (M1/M2/M3)
  - ARM-based cloud instances
  - Raspberry Pi 4+ (64-bit)

## For Developers: Consuming Multi-Arch Images

### Automatic Architecture Detection

**Docker automatically pulls the correct architecture for your machine.** You don't need to do anything special:

```bash
# On Apple Silicon Mac (ARM64)
docker pull ghcr.io/pacphi/sindri:base-latest
# Pulls: linux/arm64 variant

# On Intel Mac or Linux x86_64 (AMD64)
docker pull ghcr.io/pacphi/sindri:base-latest
# Pulls: linux/amd64 variant
```

### Building from Source (Any Architecture)

#### Using Pre-Built Base (Fast - 3-5 min)

```bash
# Docker automatically uses the right base for your arch
docker build -f v3/Dockerfile.dev -t sindri:latest .
```

This works because:

1. `Dockerfile.dev` references `ghcr.io/pacphi/sindri:base-latest`
2. Docker pulls the base image for your architecture
3. Build completes in 3-5 minutes

#### Building Base Locally (Slow - 15-20 min)

If you need to build the base image yourself:

```bash
# Single-arch (your current machine)
docker build -f v3/Dockerfile.base -t sindri:base-latest v3

# Or use Makefile
make v3-docker-build-base
```

### Cross-Platform Development

#### Scenario: Building on Intel Mac for Apple Silicon Deployment

```bash
# Install QEMU emulators
docker run --rm --privileged multiarch/qemu-user-static --reset -p yes

# Build for ARM64
docker buildx build \
  --platform linux/arm64 \
  -f v3/Dockerfile.dev \
  -t sindri:arm64 \
  --load \
  .
```

#### Scenario: Building on Apple Silicon for Linux AMD64 Server

```bash
# Build for AMD64
docker buildx build \
  --platform linux/amd64 \
  -f v3/Dockerfile.dev \
  -t sindri:amd64 \
  --load \
  .
```

**Note:** Cross-arch builds are slower due to QEMU emulation. Expect:

- Native build: 3-5 minutes
- Emulated build: 10-20 minutes

### Verifying Image Architecture

```bash
# Check what architecture an image is
docker inspect sindri:latest | jq '.[0].Architecture'

# Expected output:
# "arm64" on Apple Silicon
# "amd64" on Intel/AMD

# Check manifest (shows all available architectures)
docker buildx imagetools inspect ghcr.io/pacphi/sindri:base-latest
```

## For Maintainers: Publishing Multi-Arch Images

### Quick Start

The easiest way is to use GitHub Actions:

```bash
# 1. Go to GitHub Actions
# 2. Select "Build Base Image" workflow
# 3. Click "Run workflow"
# 4. Wait ~30-40 minutes (parallel builds)
# 5. Both amd64 and arm64 images are published!
```

### Manual Multi-Arch Build

If you need to build and push manually:

```bash
# 1. Create buildx builder (one-time setup)
docker buildx create --name multiarch --driver docker-container --use

# 2. Build and push multi-arch base
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f v3/Dockerfile.base \
  -t ghcr.io/pacphi/sindri:base-3.0.0 \
  -t ghcr.io/pacphi/sindri:base-latest \
  --push \
  v3

# 3. Verify both architectures are available
docker buildx imagetools inspect ghcr.io/pacphi/sindri:base-latest
```

### Build Time Comparison

| Architecture                | Build Method   | Time      |
| --------------------------- | -------------- | --------- |
| **Single-arch (native)**    | Local build    | 15-20 min |
| **Multi-arch (parallel)**   | GitHub Actions | 30-40 min |
| **Multi-arch (sequential)** | Local buildx   | 45-60 min |

**Recommendation:** Use GitHub Actions for multi-arch builds. It runs AMD64 and ARM64 builds in parallel.

### Publishing Workflow Details

The GitHub Actions workflow (`.github/workflows/build-base-image.yml`):

1. **Sets up QEMU** - Enables cross-platform emulation
2. **Creates buildx builder** - Configured for multi-arch
3. **Builds in parallel** - AMD64 and ARM64 simultaneously
4. **Pushes to GHCR** - Creates multi-arch manifest
5. **Tests both images** - Verifies functionality

**Total time:** ~30-40 minutes (vs ~90 minutes if done sequentially)

## Architecture-Specific Considerations

### Performance Differences

#### ARM64 (Apple Silicon) Considerations

**Pros:**

- Native performance on M1/M2/M3 Macs
- Energy efficient
- Increasingly common in cloud (AWS Graviton, etc.)

**Cons:**

- Some apt packages slower to download (fewer mirrors)
- Older software may have ARM64 compatibility issues

**Build times on Apple Silicon:**

- Base image: 15-20 min
- Development image: 3-5 min (with base)

#### AMD64 (Intel/AMD) Considerations

**Pros:**

- Mature ecosystem
- More apt mirrors available
- Faster package downloads
- Universal compatibility

**Cons:**

- Slower on Apple Silicon (QEMU emulation)
- Larger energy consumption

**Build times on AMD64:**

- Base image: 8-12 min
- Development image: 2-3 min (with base)

### Rust Compilation

Rust cross-compilation is handled automatically:

```dockerfile
# In Dockerfile, TARGETARCH is set by buildx
ARG TARGETARCH

# Rust automatically detects target architecture
RUN cargo build --release
# Produces:
# - aarch64-unknown-linux-gnu on ARM64
# - x86_64-unknown-linux-gnu on AMD64
```

### System Packages

Some packages have different names or availability:

```dockerfile
# Works on both architectures
RUN apt-get install -y build-essential git curl

# Architecture-specific (if needed)
RUN if [ "$TARGETARCH" = "arm64" ]; then \
      # ARM64-specific packages
    else \
      # AMD64-specific packages
    fi
```

## Testing Multi-Arch Images

### Local Testing

```bash
# Test on your native architecture
docker run --rm ghcr.io/pacphi/sindri:base-latest gh --version

# Test cross-platform (using emulation)
docker run --rm --platform linux/amd64 ghcr.io/pacphi/sindri:base-latest gh --version
docker run --rm --platform linux/arm64 ghcr.io/pacphi/sindri:base-latest gh --version
```

### CI Testing

The workflow automatically tests both architectures:

```yaml
- name: Test base image
  run: |
    # Tests run against the multi-arch manifest
    # Docker uses the runner's native architecture
    docker run --rm $IMAGE gh --version
    docker run --rm $IMAGE rustc --version
```

## Troubleshooting

### Issue: "no matching manifest for linux/arm64"

**Cause:** Image wasn't built with ARM64 support.

**Solution:**

```bash
# Rebuild with multi-arch
docker buildx build --platform linux/amd64,linux/arm64 ...
```

### Issue: "exec format error"

**Cause:** Running wrong architecture binary.

**Solution:**

```bash
# Check what you're running
docker inspect IMAGE | jq '.[0].Architecture'

# Force correct platform
docker run --rm --platform linux/arm64 IMAGE
```

### Issue: Slow cross-platform builds

**Cause:** QEMU emulation is slow.

**Solution:**

1. Use GitHub Actions (runs native on both architectures)
2. Or build on native hardware:

   ```bash
   # On ARM64 machine
   docker build --platform linux/arm64 ...

   # On AMD64 machine
   docker build --platform linux/amd64 ...
   ```

### Issue: Buildx builder not found

**Cause:** No multi-arch builder configured.

**Solution:**

```bash
# Create builder
docker buildx create --name multiarch --use

# Verify
docker buildx ls
```

## Best Practices

### For Development

1. **Use native builds** - Fastest performance

   ```bash
   # Just works on any architecture
   docker build -f v3/Dockerfile.dev -t sindri:latest .
   ```

2. **Pull before build** - Ensures latest base

   ```bash
   docker pull ghcr.io/pacphi/sindri:base-latest
   docker build -f v3/Dockerfile.dev -t sindri:latest .
   ```

3. **Verify architecture** - After pulling/building
   ```bash
   docker inspect sindri:latest | jq '.[0].Architecture'
   ```

### For Publishing

1. **Use GitHub Actions** - Parallel multi-arch builds
2. **Tag with architecture** - For manual builds

   ```bash
   # Manual tagging (if needed)
   docker tag sindri:latest sindri:latest-arm64
   docker tag sindri:latest sindri:latest-amd64
   ```

3. **Test both architectures** - Before releasing
   ```bash
   docker run --platform linux/amd64 IMAGE command
   docker run --platform linux/arm64 IMAGE command
   ```

### For CI/CD

1. **Use native runners** - Faster builds

   ```yaml
   # GitHub Actions
   runs-on: ${{ matrix.os }}
   strategy:
     matrix:
       os: [ubuntu-latest, macos-latest-xlarge] # AMD64 and ARM64
   ```

2. **Cache by architecture** - Separate caches

   ```yaml
   - uses: actions/cache@v4
     with:
       key: cache-${{ runner.arch }}-${{ hashFiles('**/Cargo.lock') }}
   ```

3. **Parallel builds** - Use matrix
   ```yaml
   strategy:
     matrix:
       platform: [linux/amd64, linux/arm64]
   ```

## Resources

- [Docker Multi-Arch Documentation](https://docs.docker.com/build/building/multi-platform/)
- [Docker Buildx Documentation](https://docs.docker.com/buildx/working-with-buildx/)
- [GitHub Actions: Setup QEMU](https://github.com/docker/setup-qemu-action)
- [GitHub Actions: Setup Buildx](https://github.com/docker/setup-buildx-action)

## FAQ

**Q: Do I need to do anything special to use multi-arch images?**

A: No! Docker automatically pulls the correct architecture for your machine.

**Q: Can I build on Apple Silicon and deploy to Linux AMD64?**

A: Yes! Either:

1. Build multi-arch and push both variants
2. Or specify `--platform linux/amd64` when building

**Q: Why are ARM64 builds slower than AMD64?**

A: Two reasons:

1. Fewer apt mirror servers for ARM64 packages
2. If building on AMD64 machine, QEMU emulation is slow

**Q: Should I always build multi-arch?**

A: For local development: No, build native.
For publishing: Yes, so everyone can use your images.

**Q: How do I know which architecture I'm running?**

A: Check with:

```bash
uname -m
# arm64 or aarch64 = ARM64
# x86_64 = AMD64
```

**Q: Can I use Rosetta for ARM64 on Intel Mac?**

A: Don't confuse this with Rosetta. Rosetta runs ARM64 _binaries_ on Intel. We're talking about Docker _images_. Use native Docker images for best performance.
