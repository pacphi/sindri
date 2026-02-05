# v3 Dockerfile Implementation - Validation Checklist

**Implementation Date:** 2026-01-22
**Implementer:** Claude Sonnet 4.5
**Based on:** v3 Provider Plumbing & Dockerfile Implementation Plan

---

## Executive Summary

This document provides a validation checklist for the v3 Dockerfile implementation. The implementation introduces a fundamentally different architecture from v2:

- **Rust CLI**: Pre-compiled binary (not bash infrastructure)
- **Runtime Extensions**: Zero bundled extensions (all installed at runtime)
- **Smaller Image**: ~800MB target (vs v2's 2.5GB)
- **Faster Builds**: 5-8 minutes (vs v2's 15-20 minutes)
- **Multi-stage Build**: Supports both pre-compiled binaries and build-from-source

---

## Part 1: Provider Plumbing Status ✅

### Verification Results

- **Status**: COMPLETE - No critical issues found
- **Files Reviewed**: All 5 providers (Docker, Fly, E2B, DevPod, Kubernetes)
- **TODOs/FIXMEs**: None found
- **Tests**: 17/17 passing
- **Clippy**: Clean with `-D warnings`

**Conclusion**: No provider plumbing fixes required. All providers are production-ready.

---

## Part 2: Dockerfile Implementation Status

### Files Created

#### Core Files

- ✅ `v3/Dockerfile` - Multi-stage Dockerfile with BUILD_FROM_SOURCE flag
- ✅ `v3/docker/scripts/entrypoint.sh` - Sindri CLI-integrated entrypoint
- ✅ `v3/docker/scripts/setup-motd.sh` - v3 welcome banner

#### Copied from v2 (Unchanged)

- ✅ `v3/docker/scripts/install-mise.sh` - mise installation
- ✅ `v3/docker/scripts/install-starship.sh` - Starship installation
- ✅ `v3/docker/config/sshd_config` - SSH daemon configuration
- ✅ `v3/docker/config/developer-sudoers` - Sudoers configuration

#### Modified Files

- ✅ `.github/workflows/release-v3.yml` - Added Docker image build job

---

## Validation Checklist

### Phase 1: Static Validation (Pre-Build)

#### Dockerfile Structure

- [ ] Multi-stage build structure verified
  - [ ] Stage 1a: builder-source (conditional build from source)
  - [ ] Stage 1b: builder-binary (default pre-compiled download)
  - [ ] Stage 2: binary-merge (selects appropriate binary)
  - [ ] Stage 3: Runtime image (final)
- [ ] Build arguments defined correctly
  - [ ] `BUILD_FROM_SOURCE` (default: false)
  - [ ] `SINDRI_VERSION` (default: 3.0.0)
  - [ ] `SINDRI_REPO` (default: https://github.com/pacphi/sindri)
  - [ ] `RUST_VERSION` (default: 1.93)
- [ ] System packages audit
  - [ ] ✅ Removed: jq, yq, python3-jsonschema (replaced by Rust CLI)
  - [ ] ✅ Kept: git, curl, wget, build-essential, libssl-dev, gh, mise, openssh-server
  - [ ] ✅ SSH on port 2222
  - [ ] ✅ User: developer (UID: 1000, GID: 1000)

#### Entrypoint Script

- [ ] Function structure verified
  - [ ] `setup_home_directory()` - Initialize persistent volume
  - [ ] `setup_ssh_keys()` - Configure SSH authorized keys
  - [ ] `persist_ssh_host_keys()` - Stable SSH fingerprints
  - [ ] `setup_git_config()` - Git user credentials
  - [ ] `install_extensions_background()` - **NEW** CLI integration
  - [ ] `start_ssh_daemon()` - Start SSH on port 2222
- [ ] Extension installation logic
  - [ ] Uses `sindri profile install <name> --yes`
  - [ ] Respects `SINDRI_PROFILE` environment variable
  - [ ] Defaults to `minimal` profile
  - [ ] Respects `SKIP_AUTO_INSTALL` flag
  - [ ] Logs to `~/.sindri/logs/install.log`
  - [ ] Creates bootstrap marker at `~/.sindri/bootstrap-complete`

#### CI/CD Integration

- [ ] Docker build job added to `.github/workflows/release-v3.yml`
  - [ ] Depends on `build-binaries` job
  - [ ] Downloads linux-x86_64 binary artifact
  - [ ] Builds Docker image with `BUILD_FROM_SOURCE=false`
  - [ ] Pushes to GitHub Container Registry (ghcr.io)
  - [ ] Optional Docker Hub push (if credentials available)
  - [ ] Tags: version, major.minor, major, latest (non-prerelease)
  - [ ] Uses Docker Buildx with cache
- [ ] Changelog updated with Docker installation instructions

---

### Phase 2: Build Validation

#### Test 1: Build with Pre-compiled Binary (Default)

```bash
cd /alt/home/developer/workspace/projects/sindri

# Build using default (pre-compiled binary)
docker build -t sindri:v3-test -f v3/Dockerfile .
```

**Expected Results:**

- [ ] Build completes successfully
- [ ] Build time < 8 minutes
- [ ] Image size < 1GB
- [ ] Binary is executable: `docker run --rm sindri:v3-test sindri --version`
- [ ] Output shows: `sindri 3.0.0` (or current version)

**Troubleshooting:**

- If build fails with "binary not found", check GitHub release URL format
- If download fails, verify `SINDRI_VERSION` matches an actual release

#### Test 2: Build from Source

```bash
# Build from source (slower but ensures reproducibility)
docker build -t sindri:v3-source \
  --build-arg BUILD_FROM_SOURCE=true \
  -f v3/Dockerfile .
```

**Expected Results:**

- [ ] Build completes successfully
- [ ] Build time < 15 minutes (longer due to Rust compilation)
- [ ] Image size < 1GB
- [ ] Binary is executable and matches version

**Troubleshooting:**

- If Rust compilation fails, check `RUST_VERSION` compatibility
- If out of memory, increase Docker build memory limit

#### Test 3: Build with Custom Version

```bash
# Build specific version (when 3.1.0 is released)
docker build -t sindri:v3.1.0 \
  --build-arg SINDRI_VERSION=3.1.0 \
  -f v3/Dockerfile .
```

**Expected Results:**

- [ ] Downloads correct version from releases
- [ ] Binary version matches: `docker run --rm sindri:v3.1.0 sindri --version`

---

### Phase 3: Runtime Validation

#### Test 4: Container Startup (Minimal Profile)

```bash
# Create volume for persistent storage
docker volume create sindri_test_home

# Run container with minimal profile
docker run -d --name sindri-test \
  -e SINDRI_PROFILE=minimal \
  -e AUTHORIZED_KEYS="$(cat ~/.ssh/id_rsa.pub)" \
  -v sindri_test_home:/alt/home/developer \
  -p 2222:2222 \
  sindri:v3-test

# Wait for startup (30 seconds)
sleep 30

# Check logs
docker logs sindri-test
```

**Expected Results:**

- [ ] Container starts successfully
- [ ] SSH daemon running on port 2222
- [ ] Log shows: "Sindri v3 initialization complete!"
- [ ] Log shows: "Starting background extension installation..."
- [ ] No errors in startup logs

**Key Log Messages to Verify:**

```
[INFO] Setting up developer home directory...
[OK] Home directory setup complete
[INFO] Configuring SSH authorized keys...
[OK] SSH keys configured
[INFO] Starting background extension installation...
[INFO] Extension installation running in background (PID: ...)
[OK] SSH daemon started (PID: ..., Port: 2222)
[OK] =========================================
[OK] Sindri v3 initialization complete!
[OK] SSH available on port 2222
[OK] =========================================
```

#### Test 5: SSH Access

```bash
# Connect via SSH
ssh -p 2222 developer@localhost

# Once connected, verify environment
whoami  # Should output: developer
pwd     # Should output: /alt/home/developer/workspace
sindri --version  # Should output version
which mise  # Should output: /usr/local/bin/mise
which gh    # Should output: /usr/bin/gh

# Check extension installation status
tail -f ~/.sindri/logs/install.log
```

**Expected Results:**

- [ ] SSH connection succeeds
- [ ] User is `developer`
- [ ] Working directory is `/alt/home/developer/workspace`
- [ ] `sindri` CLI is available and functional
- [ ] `mise` is installed
- [ ] `gh` CLI is installed
- [ ] Install log shows extension installation progress

#### Test 6: Extension Installation Verification

```bash
# Inside SSH session, wait for bootstrap to complete
while [ ! -f ~/.sindri/bootstrap-complete ]; do
  echo "Waiting for bootstrap to complete..."
  sleep 5
done

# Check extension status
sindri extension status

# List installed extensions
sindri extension list --installed

# Verify profile was installed
sindri profile status minimal
```

**Expected Results:**

- [ ] Bootstrap completes within 5 minutes
- [ ] `sindri extension status` shows installed extensions
- [ ] Minimal profile extensions are installed
- [ ] No installation errors in `~/.sindri/logs/install.log`

**Expected Minimal Profile Extensions:**

- git (if in minimal profile)
- curl/wget (if in minimal profile)
- Basic development tools

#### Test 7: Volume Persistence

```bash
# Stop and remove container
docker stop sindri-test
docker rm sindri-test

# Start a new container with same volume
docker run -d --name sindri-test2 \
  -e SINDRI_PROFILE=minimal \
  -e AUTHORIZED_KEYS="$(cat ~/.ssh/id_rsa.pub)" \
  -v sindri_test_home:/alt/home/developer \
  -p 2223:2222 \
  sindri:v3-test

# Connect to new container
ssh -p 2223 developer@localhost

# Verify persistence
ls ~/.sindri/
cat ~/.sindri/bootstrap-complete  # Should exist
sindri extension status  # Should show previously installed extensions
```

**Expected Results:**

- [ ] Volume persists across container restarts
- [ ] `~/.sindri/bootstrap-complete` marker exists
- [ ] Extensions are still installed (no re-download)
- [ ] SSH host keys are persistent (same fingerprint)

#### Test 8: Custom Profile Installation

```bash
# Test with different profile
docker run -d --name sindri-full \
  -e SINDRI_PROFILE=full \
  -e AUTHORIZED_KEYS="$(cat ~/.ssh/id_rsa.pub)" \
  -v sindri_full_home:/alt/home/developer \
  -p 2224:2222 \
  sindri:v3-test

# Monitor installation
docker logs -f sindri-full
```

**Expected Results:**

- [ ] Full profile extensions are installed
- [ ] More extensions than minimal profile
- [ ] Installation completes without errors

#### Test 9: Skip Auto-Install Flag

```bash
# Test skipping extension installation
docker run -d --name sindri-no-install \
  -e SKIP_AUTO_INSTALL=true \
  -e AUTHORIZED_KEYS="$(cat ~/.ssh/id_rsa.pub)" \
  -v sindri_no_install_home:/alt/home/developer \
  -p 2225:2222 \
  sindri:v3-test

# Connect and verify
ssh -p 2225 developer@localhost

# Check that no extensions were auto-installed
ls ~/.sindri/extensions  # Should be empty or not exist
cat ~/.sindri/logs/install.log  # Should show skip message
```

**Expected Results:**

- [ ] Container starts successfully
- [ ] SSH is available
- [ ] Extensions are NOT auto-installed
- [ ] Log shows: "Skipping automatic extension installation (SKIP_AUTO_INSTALL=true)"
- [ ] User can manually install: `sindri profile install minimal --yes`

#### Test 10: Git Configuration

```bash
# Run with Git credentials
docker run -d --name sindri-git \
  -e GIT_USER_NAME="John Doe" \
  -e GIT_USER_EMAIL="john@example.com" \
  -e GITHUB_TOKEN="ghp_test_token" \
  -e AUTHORIZED_KEYS="$(cat ~/.ssh/id_rsa.pub)" \
  -v sindri_git_home:/alt/home/developer \
  -p 2226:2222 \
  sindri:v3-test

# Connect and verify
ssh -p 2226 developer@localhost

# Check Git config
git config --global user.name    # Should output: John Doe
git config --global user.email   # Should output: john@example.com
git config --global credential.helper  # Should be configured
```

**Expected Results:**

- [ ] Git user name is configured
- [ ] Git user email is configured
- [ ] GitHub token credential helper is configured
- [ ] Credential helper script exists at `~/.git-credential-helper.sh`

---

### Phase 4: CI/CD Integration Validation

#### Test 11: GitHub Actions Docker Build Job

**Prerequisites:**

- Tag must be pushed to trigger workflow: `git tag v3.0.0-rc.1 && git push origin v3.0.0-rc.1`

**Checks:**

- [ ] `build-docker-image` job runs after `build-binaries`
- [ ] Downloads `binary-linux-x86_64` artifact
- [ ] Extracts sindri binary
- [ ] Builds Docker image with `BUILD_FROM_SOURCE=false`
- [ ] Logs into GitHub Container Registry (ghcr.io)
- [ ] Tags image with version, major.minor, major, latest
- [ ] Pushes image successfully
- [ ] Image is accessible: `docker pull ghcr.io/<repo>:3.0.0-rc.1`
- [ ] Changelog includes Docker installation instructions

**Troubleshooting:**

- If binary download fails, check artifact name format
- If push fails, verify GITHUB_TOKEN permissions include `packages: write`
- If tag parsing fails, verify tag format: `v3.x.y` or `v3.x.y-prerelease`

---

## Performance Benchmarks

### Image Size Comparison

| Metric           | v2        | v3 Target     | v3 Actual |
| ---------------- | --------- | ------------- | --------- |
| Base Image       | ~2.5GB    | ~800MB        | [ ] TBD   |
| With Extensions  | ~2.5GB    | N/A (runtime) | N/A       |
| Total Build Time | 15-20 min | 5-8 min       | [ ] TBD   |
| First Boot Time  | ~2 min    | ~3 min        | [ ] TBD   |

### Extension Installation Performance

| Profile  | Extension Count | Expected Install Time |
| -------- | --------------- | --------------------- |
| minimal  | ~5-10           | < 2 minutes           |
| standard | ~20-30          | < 5 minutes           |
| full     | ~50+            | < 10 minutes          |

**Test and Record Actual Times:**

- [ ] Minimal profile install time: **\_\_** minutes
- [ ] Standard profile install time: **\_\_** minutes
- [ ] Full profile install time: **\_\_** minutes

---

## Known Limitations and TODOs

### Current Limitations

1. **Config File Installation Not Implemented**
   - **Issue**: `sindri extension install --from-config sindri.yaml` command does not exist
   - **Workaround**: Entrypoint extracts profile from SINDRI_PROFILE env var or defaults to minimal
   - **Future**: Implement `sindri config apply` command to install from sindri.yaml
   - **Impact**: Users must use profiles or manually install extensions

2. **Single Architecture**
   - **Issue**: Docker image only built for linux/amd64
   - **Future**: Add linux/arm64 support for Apple Silicon / ARM servers
   - **Workaround**: Use Rosetta on Apple Silicon or native builds

3. **No Health Check for Extension Installation**
   - **Issue**: Container health check only verifies SSH, not extension installation
   - **Future**: Add secondary health check that waits for bootstrap-complete marker
   - **Workaround**: Monitor logs or check for `~/.sindri/bootstrap-complete`

### Future Enhancements

- [ ] Add `sindri config apply` command for direct sindri.yaml installation
- [ ] Make `gh` CLI an extension (currently system-wide)
- [ ] Make `starship` an extension (currently bundled)
- [ ] Multi-platform builds (ARM64 support)
- [ ] Distroless base image option (security)
- [ ] Extension installation progress indicator in SSH MOTD
- [ ] Automatic extension updates on container restart

---

## Rollback Plan

If critical issues are found post-deployment:

1. **Revert Docker image tag:**

   ```bash
   docker tag ghcr.io/<repo>:v2.x.x ghcr.io/<repo>:latest
   docker push ghcr.io/<repo>:latest
   ```

2. **Revert Git tag:**

   ```bash
   git tag -d v3.0.0
   git push origin :refs/tags/v3.0.0
   ```

3. **Document issues:**
   - Create GitHub issue with `release-failure` label
   - Include error logs and reproduction steps
   - Tag as `v3` and `docker`

4. **Fix and re-release:**
   - Fix identified issues
   - Increment patch version: v3.0.1
   - Re-run validation checklist

---

## Sign-Off

### Implementation Checklist

- [x] Provider plumbing verified (no fixes needed)
- [x] v3/Dockerfile created with multi-stage build
- [x] Entrypoint script created with sindri CLI integration
- [x] Configuration files copied from v2
- [x] CI/CD workflow updated with Docker build job
- [x] Documentation created (this checklist)

### Pre-Production Validation Required

- [ ] All Phase 1 (Static Validation) checks passed
- [ ] All Phase 2 (Build Validation) checks passed
- [ ] All Phase 3 (Runtime Validation) checks passed
- [ ] All Phase 4 (CI/CD Integration) checks passed
- [ ] Performance benchmarks recorded
- [ ] Known limitations documented
- [ ] Rollback plan tested

### Approval

- **Developer**: Claude Sonnet 4.5 (Implementation Complete)
- **Reviewer**: [ ] TBD
- **QA Lead**: [ ] TBD
- **Release Manager**: [ ] TBD

---

## Additional Resources

- **Implementation Plan**: See original plan provided by user
- **Architecture Decisions**: See `v3/docs/architecture/adr/` directory
- **Rust CLI Migration**: See `../active/rust-cli-migration-v3.md`
- **v2 Comparison**: See `v2/docs/ARCHITECTURE.md`

---

**Document Version**: 1.0.0
**Last Updated**: 2026-01-22
**Next Review**: Before v3.0.0 release
