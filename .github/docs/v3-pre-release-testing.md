# V3 Pre-Release Testing Strategy

## Overview

The V3 pre-release testing workflow validates **CI release candidates** before they are promoted to versioned releases. This ensures all extensions work correctly with the exact image that will be released.

## Key Concepts

### CI Release Candidate vs Released Image

**Important Distinction:**

- **CI Candidate**: Image built and tested by CI workflow (`ci-passed-{SHA}` or `ci-{SHA}`)
- **Released Image**: CI candidate promoted by retagging (e.g., `v3.0.0`, `v3-latest`, `latest`)

The release process **does NOT rebuild images**. It promotes CI candidates by:

1. Pulling the candidate: `ghcr.io/pacphi/sindri:ci-passed-{SHA}`
2. Retagging with version tags
3. Signing with Cosign
4. Generating SBOM
5. Publishing GitHub release

### Testing Philosophy

**Test individual extensions, not profiles** because:

- **Profiles are compositions**: `fullstack` = nodejs + python + docker + ...
- **Extensions are atomic units**: Each extension should work independently
- **Real-world usage**: Users often opt to install a specific extension, and at other times, profiles
- **Granular failure reporting**: Know exactly which extension failed

## Workflow Architecture

```
┌─────────────┐
│  Developer  │
│   commits   │
│   to main   │
└──────┬──────┘
       │
       v
┌─────────────────────────────────┐
│  CI Workflow (ci-v3.yml)        │
├─────────────────────────────────┤
│  1. Build Sindri binary         │
│  2. Build Docker image          │
│  3. Tag: ci-{SHA}               │
│  4. Run tests                   │
│  5. If pass: ci-passed-{SHA}    │
└──────┬──────────────────────────┘
       │
       │  CI Release Candidate Ready
       │
       v
┌─────────────────────────────────┐
│  Pre-Release Testing            │
│  (v3-pre-release-test.yml)      │
├─────────────────────────────────┤
│  Input: Commit SHA (or main)    │
│  1. Find ci-passed-{SHA}        │
│  2. Test ALL extensions         │
│  3. Test on selected providers  │
│  4. Report results              │
└──────┬──────────────────────────┘
       │
       │  ✅ All Tests Pass
       │
       v
┌─────────────────────────────────┐
│  Release (release-v3.yml)       │
│  Trigger: git tag v3.x.x        │
├─────────────────────────────────┤
│  1. Find ci-passed-{SHA}        │
│  2. Pull candidate              │
│  3. Retag with versions         │
│  4. Sign with Cosign            │
│  5. Generate SBOM               │
│  6. Create GitHub release       │
└─────────────────────────────────┘
```

## Workflow Details

### 1. CI Workflow (`ci-v3.yml`)

**Purpose**: Build and test every commit

**Output Images:**

- `ghcr.io/pacphi/sindri:ci-{FULL_SHA}` - Always created
- `ghcr.io/pacphi/sindri:ci-passed-{FULL_SHA}` - Only if tests pass

**What Gets Tested:**

- Rust format, clippy, tests
- Docker image build
- Security scan (Trivy)
- K8s integration tests (kind, k3d)

**Example:**

```bash
# Commit: a2ae248...
# CI builds: ghcr.io/pacphi/sindri:ci-a2ae248...
# Tests pass: ghcr.io/pacphi/sindri:ci-passed-a2ae248...
```

### 2. Pre-Release Workflow (`v3-pre-release-test.yml`)

**Purpose**: Validate CI candidate with extensive extension testing

**Trigger**: Manual (`workflow_dispatch`)

**Inputs:**

- `commit-sha`: Specific commit to test (default: latest main)
- `providers`: Which providers to test (default: `docker,k3d,fly`)
- `filter-heavy`: Exclude heavy extensions (default: false - test everything)
- `max-parallel`: Parallelism control (default: 2)

**Process:**

#### Step 1: Resolve CI Release Candidate

```yaml
jobs:
  resolve-ci-image:
    # Finds ci-passed-{SHA} or falls back to ci-{SHA}
    outputs:
      ci_image: ghcr.io/pacphi/sindri:ci-passed-a2ae248...
      image_source: ci-passed # or ci-tag
      commit_sha: a2ae248...
```

#### Step 2: Test All Extensions Individually

```yaml
jobs:
  test-extensions:
    uses: ./.github/workflows/v3-extension-test.yml
    with:
      selection-mode: all # CRITICAL: Test all extensions
      sindri-image: ${{ needs.resolve-ci-image.outputs.ci_image }}
      providers: docker,k3d,fly
```

**How It Works:**

1. `v3-extension-test.yml` discovers ALL extensions (not profile-based)
2. Generates provider-specific matrices
3. Delegates to provider workflows (docker, k3d, fly, etc.)
4. Each provider uses the **same CI candidate image** passed via `sindri-image`

#### Step 3: Generate Report

- Lists which extensions passed/failed
- Shows which providers succeeded
- Provides next steps (tag for release or fix failures)

### 3. Release Workflow (`release-v3.yml`)

**Purpose**: Promote CI candidate to versioned release

**Trigger**: Git tag (`v3.*.*`)

**Process:**

```bash
# 1. Tag is created
git tag v3.0.0
git push origin v3.0.0

# 2. Release workflow finds the tag's commit SHA
SHA=$(git rev-parse v3.0.0)  # e.g., a2ae248...

# 3. Pulls the CI candidate
docker pull ghcr.io/pacphi/sindri:ci-passed-a2ae248...

# 4. Retags for release
docker tag ghcr.io/pacphi/sindri:ci-passed-a2ae248... ghcr.io/pacphi/sindri:v3.0.0
docker tag ghcr.io/pacphi/sindri:ci-passed-a2ae248... ghcr.io/pacphi/sindri:v3
docker tag ghcr.io/pacphi/sindri:ci-passed-a2ae248... ghcr.io/pacphi/sindri:v3-latest
docker tag ghcr.io/pacphi/sindri:ci-passed-a2ae248... ghcr.io/pacphi/sindri:latest

# 5. Pushes all tags
docker push ghcr.io/pacphi/sindri:v3.0.0
docker push ghcr.io/pacphi/sindri:v3
docker push ghcr.io/pacphi/sindri:v3-latest
docker push ghcr.io/pacphi/sindri:latest

# 6. Signs image by digest (cosign 3.x, keyless)
cosign sign --yes ghcr.io/pacphi/sindri@sha256:<digest>

# 7. Attests build provenance (SLSA)
actions/attest-build-provenance → pushed to registry

# 8. Generates SBOM
anchore/sbom-action → sbom.spdx.json
cosign attach sbom --sbom sbom.spdx.json ghcr.io/pacphi/sindri@sha256:<digest>
```

## Extension Testing Strategy

### Why Individual Extensions?

**Problem with profile-based testing:**

```yaml
# profiles.yaml
fullstack:
  extensions:
    - nodejs
    - python
    - docker
    - postgresql
    - redis
```

If `fullstack` profile passes, we know the **combination** works, but:

- ❌ Don't know if `nodejs` works independently
- ❌ Don't know if `python` has issues
- ❌ Can't identify which specific extension failed
- ❌ Users often install extensions individually, not profiles

**Solution: Individual extension testing**

```yaml
selection-mode: all # NOT profile-based
```

This tests:

- `sindri extension install nodejs` ✅ or ❌
- `sindri extension install python` ✅ or ❌
- `sindri extension install docker` ✅ or ❌
- ... (every extension in registry)

### Test Matrix

For each extension × provider combination:

```bash
# Example: nodejs extension on docker provider
1. sindri extension install nodejs --yes
2. sindri extension validate nodejs
3. sindri extension remove nodejs --yes
```

**Providers tested:**

- **docker**: Local container testing
- **k3d**: Kubernetes (lightweight)
- **fly**: Edge deployment (if secrets available)
- **devpod**: Cloud-based devcontainers (optional, requires cloud credentials)
- **packer**: VM image builds (optional, VM-based not containers, requires cloud credentials)

## Developer Workflow Integration

### How This Relates to `make v3-cycle-fast`

```makefile
v3-cycle-fast:
  @sindri destroy --config $(CONFIG) -f
  @make v3-docker-build-fast       # Builds local image
  @make v3-install                 # Installs sindri CLI
  @sindri deploy --config $(CONFIG)
```

**Key Differences:**

| Aspect         | make v3-cycle-fast          | Pre-Release Testing       |
| -------------- | --------------------------- | ------------------------- |
| **Purpose**    | Local development iteration | Release validation        |
| **Image**      | Local build from source     | CI candidate              |
| **Extensions** | Profile-based (config file) | Individual (all)          |
| **Duration**   | 3-5 minutes                 | 30-60 minutes             |
| **Providers**  | Single (from CONFIG)        | Multiple (docker,k3d,fly) |
| **Frequency**  | Every code change           | Before release            |

### When to Use Each

**Use `make v3-cycle-fast`:**

- Developing features
- Testing configuration changes
- Quick validation cycles
- Single extension debugging

**Use pre-release testing:**

- Before tagging a release
- After major refactors
- Before announcing new version
- Validating CI candidates

## Running Pre-Release Tests

### Via GitHub Actions UI

1. Go to Actions → "v3: Pre-Release Tests"
2. Click "Run workflow"
3. Configure:
   - **Commit SHA**: Leave empty for latest main, or specify SHA
   - **Providers**: `docker,k3d,fly` (default)
   - **Filter heavy**: false (test everything for releases)
   - **Max parallel**: 2 (conservative for stability)
4. Click "Run workflow"

### Via GitHub CLI

```bash
# Test latest main commit
gh workflow run v3-pre-release-test.yml

# Test specific commit
gh workflow run v3-pre-release-test.yml \
  -f commit-sha=a2ae248b3c1d5e6f7a8b9c0d1e2f3a4b5c6d7e8f

# Test with specific providers
gh workflow run v3-pre-release-test.yml \
  -f providers=docker,k3d

# Conservative settings (faster)
gh workflow run v3-pre-release-test.yml \
  -f filter-heavy=true \
  -f max-parallel=4
```

## Interpreting Results

### Success Scenario

```
✅ Pre-Release Test Summary
├─ Commit: a2ae248...
├─ Candidate: ghcr.io/pacphi/sindri:ci-passed-a2ae248...
├─ Status: ✅ Passed CI Tests
└─ Results:
   ├─ Docker: 47/47 extensions passed
   ├─ k3d: 47/47 extensions passed
   └─ Fly: 45/47 extensions passed (2 skipped - no secrets)

Next Steps:
  git tag v3.0.0
  git push origin v3.0.0
```

### Failure Scenario

```
❌ Pre-Release Test Summary
├─ Commit: a2ae248...
├─ Candidate: ghcr.io/pacphi/sindri:ci-passed-a2ae248...
├─ Status: ✅ Passed CI Tests
└─ Results:
   ├─ Docker: 46/47 extensions passed (1 failed)
   │  └─ ❌ ollama: installation timeout
   ├─ k3d: 47/47 extensions passed
   └─ Fly: 45/47 extensions passed

Next Steps:
  1. Review ollama extension logs
  2. Fix installation timeout issue
  3. Commit fix to main
  4. Wait for CI to build new candidate
  5. Re-run pre-release tests
```

## Cost & Time Considerations

### Estimated Duration

**With default settings** (`docker,k3d,fly`, no filter-heavy, max-parallel=2):

- ~50 extensions
- 3 providers
- 2 parallel jobs per provider
- 3 minutes avg per extension

**Time**: ~40-60 minutes

### GitHub Actions Cost

**Free tier:**

- 2,000 minutes/month for public repos
- 3,000 minutes/month for Pro accounts

**One pre-release run**: ~60 minutes

**Recommendation**: Run pre-release tests 1-2 times per release cycle (not on every commit)

### Optimization Strategies

1. **Filter heavy extensions** for initial validation:

   ```bash
   gh workflow run v3-pre-release-test.yml -f filter-heavy=true
   ```

2. **Test fewer providers** initially:

   ```bash
   gh workflow run v3-pre-release-test.yml -f providers=docker
   ```

3. **Increase parallelism** (if GitHub runner capacity allows):

   ```bash
   gh workflow run v3-pre-release-test.yml -f max-parallel=4
   ```

4. **Staged testing**:
   - **Stage 1**: docker only, filter-heavy=true (~15 min)
   - **Stage 2**: docker+k3d, filter-heavy=false (~30 min)
   - **Stage 3**: all providers (~60 min, final validation)

## Release Cadence Recommendation

### Stable Releases (v3.x.0)

```
Week 1-2: Development
  ├─ make v3-cycle-fast (developers iterate)
  └─ CI tests run on every commit

Week 3: Pre-Release Testing
  ├─ Stage 1: Quick validation (docker, filter-heavy)
  ├─ Fix any failures
  ├─ Stage 2: Full validation (docker+k3d, all extensions)
  └─ Stage 3: Multi-provider (docker+k3d+fly)

Week 4: Release
  ├─ Tag: v3.1.0
  └─ Release workflow promotes candidate
```

### Prereleases (v3.x.0-beta.N)

```
Development Sprint:
  ├─ CI tests on every commit
  └─ Pre-release test before each beta tag

Beta 1:
  ├─ Pre-release test (docker only)
  └─ Tag: v3.1.0-beta.1

Beta 2:
  ├─ Pre-release test (docker+k3d)
  └─ Tag: v3.1.0-beta.2

RC (Release Candidate):
  ├─ Full pre-release test (all providers)
  └─ Tag: v3.1.0-rc.1

Final:
  ├─ Final pre-release test
  └─ Tag: v3.1.0
```

## Troubleshooting

### "No CI Image Found"

**Problem**: Pre-release workflow can't find CI candidate

**Solutions:**

1. Check if CI ran on the commit:

   ```bash
   gh run list --workflow=ci-v3.yml --commit=a2ae248...
   ```

2. Wait for CI to complete

3. Verify image exists:

   ```bash
   docker pull ghcr.io/pacphi/sindri:ci-passed-a2ae248...
   ```

4. Check retention policy (images expire after 90 days)

### Extension Test Failures

**Problem**: Some extensions fail during testing

**Debug:**

1. Download test artifacts from GitHub Actions
2. Review `install.log`, `validate.log`, `remove.log`
3. Reproduce locally:
   ```bash
   sindri deploy --config minimal.yaml
   sindri extension install <failed-extension>
   ```

### Provider-Specific Failures

**Problem**: Extension works on docker but fails on k3d/fly

**Common causes:**

- Resource constraints (k3d has lower memory limits)
- Network differences (k3d networking vs docker)
- Provider-specific requirements

**Debug:**

- Compare provider resource limits in `v3-matrix-generator.yml`
- Check extension `requirements.memory` in `extension.yaml`
- Review provider-specific logs

## Future Enhancements

### Possible Additions

1. **Bundled vs Runtime Testing**:
   - Test extensions baked into image (buildFromSource)
   - Test extensions installed at runtime (current behavior)
   - Validate both workflows

2. **Performance Benchmarks**:
   - Measure extension installation time
   - Track memory usage
   - Compare across providers

3. **Upgrade Testing**:
   - Test upgrading from v3.0.0 → v3.1.0
   - Validate extension compatibility

4. **Integration Testing**:
   - Test extension combinations (e.g., nodejs + postgresql)
   - Validate inter-extension dependencies

5. **Scheduled Runs**:
   - Nightly pre-release tests on main
   - Weekly comprehensive tests (all providers)

## References

- CI Workflow: `.github/workflows/ci-v3.yml`
- Release Workflow: `.github/workflows/release-v3.yml`
- Pre-Release Workflow: `.github/workflows/v3-pre-release-test.yml`
- Extension Test Workflow: `.github/workflows/v3-extension-test.yml`
- Provider Workflows: `.github/workflows/v3-provider-*.yml`
