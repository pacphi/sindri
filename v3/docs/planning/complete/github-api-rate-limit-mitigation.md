# GitHub API Rate Limit Mitigation Plan

## Executive Summary

This document outlines strategies for mitigating GitHub API rate limiting issues that affect mise-based tool installation in Sindri. The primary issue manifests when mise's `aqua` backend attempts to fetch release information from GitHub's API, which has strict unauthenticated rate limits (60 requests/hour) that are frequently exceeded during CI runs.

**Root Cause:** mise's aqua backend has a known bug (#5418) where it doesn't properly use `GITHUB_TOKEN` for authentication, causing requests to be treated as unauthenticated even when a token is available.

**Impact:** Extension installation failures during CI, particularly affecting the `python` extension which uses `aqua:astral-sh/uv`.

---

## Table of Contents

1. [Problem Analysis](#1-problem-analysis)
2. [Current Mitigations](#2-current-mitigations)
3. [Proposed Solutions](#3-proposed-solutions)
4. [Implementation Recommendations](#4-implementation-recommendations)
5. [Testing Strategy](#5-testing-strategy)
6. [Decision Matrix](#6-decision-matrix)

---

## 1. Problem Analysis

### 1.1 GitHub API Rate Limits

| Authentication        | Rate Limit     | Reset Period | Use Case                |
| --------------------- | -------------- | ------------ | ----------------------- |
| Unauthenticated       | 60 requests    | Per hour     | Public API access       |
| Authenticated (token) | 5,000 requests | Per hour     | CI/CD workflows         |
| GitHub Actions        | 1,000 requests | Per hour     | GITHUB_TOKEN in Actions |

### 1.2 Failure Scenario

The following error was observed in CI run [#21085230557](https://github.com/pacphi/sindri/actions/runs/21085230557):

```
mise ERROR Failed to install aqua:astral-sh/uv@0.9: HTTP status client error
(403 rate limit exceeded) for url
(https://api.github.com/repos/astral-sh/uv/releases/tags/0.9.26)
```

**Timeline:**

1. DevPod workspace creation initiated on kind cluster
2. Extension initialization triggered `python` extension
3. mise attempted to install `uv` via aqua backend
4. GitHub API returned 403 (rate limit exceeded)
5. Retry logic attempted 3 times with exponential backoff
6. All retries failed due to sustained rate limiting
7. Extension installation marked as failed (4/5 succeeded)
8. DevPod deployment failed

### 1.3 Affected Extensions

Extensions using mise's aqua backend for GitHub-hosted tools:

| Extension   | Tool               | Backend           | Risk Level |
| ----------- | ------------------ | ----------------- | ---------- |
| python      | uv                 | aqua:astral-sh/uv | **High**   |
| infra-tools | k9s, kustomize, yq | asdf (mitigated)  | Low        |
| github-cli  | gh                 | aqua              | Medium     |
| cloud-tools | various            | mixed             | Medium     |

### 1.4 Known mise Bug

**Issue:** [mise #5418](https://github.com/jdx/mise/issues/5418) - aqua backend doesn't properly use GITHUB_TOKEN

**Status:** Open/Unresolved

**Workaround:** Use alternative backends (ubi, asdf, cargo) that properly handle authentication.

---

## 2. Current Mitigations

Sindri already implements several rate limit mitigations:

### 2.1 Serial Installation

```toml
# docker/lib/extensions/mise-config/install.sh
[settings]
jobs = 1  # Prevents concurrent API requests
```

### 2.2 Extended Timeouts

```toml
[settings]
http_timeout = "180s"
fetch_remote_versions_timeout = "180s"
```

### 2.3 Retry Logic with Exponential Backoff

```bash
# cli/extension-manager-modules/executor.sh
retry_command 3 300 mise install
# 3 attempts, 300s timeout, exponential backoff (2s → 4s → 8s + jitter)
```

### 2.4 asdf Backend for Problematic Tools

```toml
# docker/lib/extensions/infra-tools/mise.toml
# Use asdf backends to avoid mise aqua GITHUB_TOKEN bug (#5418)
"asdf:looztra/asdf-k9s" = "0.50"
"asdf:Banno/asdf-kustomize" = "5.7.1"
"asdf:sudermanjr/asdf-yq" = "4.50"
```

### 2.5 GITHUB_TOKEN Propagation

```bash
# cli/extension-manager-modules/executor.sh
if [[ -n "${GITHUB_TOKEN:-}" ]]; then
  export MISE_GITHUB_TOKEN="${GITHUB_TOKEN}"
fi
```

---

## 3. Proposed Solutions

### 3.1 Option A: Use `ubi` Backend

**Description:** Replace aqua backend with ubi (Universal Binary Installer) which properly handles GITHUB_TOKEN authentication.

**Implementation:**

```toml
# docker/lib/extensions/python/mise.toml
[tools]
python = "3.13"
# Use ubi backend to avoid aqua GitHub API rate limits (mise bug #5418)
"ubi:astral-sh/uv" = "0.9"

[env]
PYTHONDONTWRITEBYTECODE = "1"
PYTHONUNBUFFERED = "1"
```

**Pros:**

- Minimal code change
- Proper token authentication
- Consistent with mise ecosystem
- Binary downloads (fast)

**Cons:**

- Still uses GitHub API (with auth)
- Requires GITHUB_TOKEN to be set

### 3.2 Option B: Use `cargo` Backend

**Description:** Install uv from crates.io instead of GitHub releases.

**Implementation:**

```toml
# docker/lib/extensions/python/mise.toml
[tools]
python = "3.13"
# Install from crates.io to avoid GitHub API entirely
"cargo:uv" = "0.9"

[env]
PYTHONDONTWRITEBYTECODE = "1"
PYTHONUNBUFFERED = "1"
```

**Pros:**

- Completely avoids GitHub API
- No rate limit concerns
- No token required

**Cons:**

- Requires Rust toolchain
- Significantly slower (compiles from source)
- Larger disk space during build
- May fail on low-memory systems

### 3.3 Option C: Use `asdf` Backend

**Description:** Use an asdf plugin for uv if available.

**Implementation:**

```toml
# docker/lib/extensions/python/mise.toml
[tools]
python = "3.13"
"asdf:b0o/asdf-uv" = "0.9"

[env]
PYTHONDONTWRITEBYTECODE = "1"
PYTHONUNBUFFERED = "1"
```

**Pros:**

- Consistent with infra-tools pattern
- Often handles auth better than aqua

**Cons:**

- Depends on third-party asdf plugin maintenance
- May still use GitHub API internally
- Plugin quality varies

### 3.4 Option D: Pre-bake in Docker Image

**Description:** Install uv during Docker image build, eliminating runtime GitHub API calls.

**Implementation:**

```dockerfile
# Dockerfile (add to build stage)
# Install uv globally to avoid runtime GitHub API calls
RUN curl -LsSf https://astral.sh/uv/install.sh | sh && \
    mv ~/.local/bin/uv /usr/local/bin/ && \
    mv ~/.local/bin/uvx /usr/local/bin/
```

And modify extension to skip uv installation:

```toml
# docker/lib/extensions/python/mise.toml
[tools]
python = "3.13"
# uv is pre-installed in base image

[env]
PYTHONDONTWRITEBYTECODE = "1"
PYTHONUNBUFFERED = "1"
```

**Pros:**

- Most reliable - no runtime downloads
- Fastest startup time
- No rate limit concerns
- Works offline

**Cons:**

- Version tied to image build
- Larger base image
- Requires image rebuild to update uv
- Diverges from declarative extension model

### 3.5 Option E: Direct URL Download

**Description:** Use mise's URL backend to download from a direct URL, bypassing GitHub API.

**Implementation:**

```toml
# docker/lib/extensions/python/mise.toml
[tools]
python = "3.13"
# Direct download bypasses GitHub API
"ubi:astral-sh/uv[exe=uv]" = "0.9.26"

[env]
PYTHONDONTWRITEBYTECODE = "1"
PYTHONUNBUFFERED = "1"
```

**Pros:**

- Avoids API version lookup
- Direct binary download

**Cons:**

- Requires pinning exact version
- Less flexible version management

### 3.6 Option F: GitHub Token Caching/Pooling

**Description:** Implement token rotation or caching at the CI level to maximize rate limit headroom.

**Implementation:**

```yaml
# .github/workflows/ci.yml
env:
  # Use a PAT with higher limits for mise downloads
  MISE_GITHUB_TOKEN: ${{ secrets.MISE_GITHUB_PAT }}
```

**Pros:**

- Works with existing code
- No extension changes needed

**Cons:**

- Doesn't fix aqua backend bug
- Requires additional secret management
- PAT has security implications

---

## 4. Implementation Recommendations

### 4.1 Recommended Approach: Option A (ubi backend)

**Rationale:**

1. **Minimal change** - Single line modification to mise.toml
2. **Proven pattern** - ubi backend works correctly with GITHUB_TOKEN
3. **Consistent** - Follows mise best practices
4. **Reversible** - Easy to change if mise fixes bug #5418

### 4.2 Implementation Steps

1. **Update python extension mise.toml:**

   ```toml
   [tools]
   python = "3.13"
   "ubi:astral-sh/uv" = "0.9"
   ```

2. **Verify GITHUB_TOKEN propagation in CI:**

   ```yaml
   # .github/workflows/test-provider.yml
   env:
     GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
   ```

3. **Test locally:**

   ```bash
   GITHUB_TOKEN=$(gh auth token) ../../../v2/cli/extension-manager install python
   ```

4. **Update extension documentation** to note backend choice.

### 4.3 Long-term Considerations

| Timeframe   | Action                                              |
| ----------- | --------------------------------------------------- |
| Immediate   | Implement Option A (ubi backend)                    |
| Short-term  | Monitor mise #5418 for upstream fix                 |
| Medium-term | Consider Option D (pre-bake) for critical tools     |
| Long-term   | Evaluate if aqua backend is fixed, revert if stable |

---

## 5. Testing Strategy

### 5.1 Local Testing

```bash
# Test with rate-limited scenario (no token)
unset GITHUB_TOKEN
../../../v2/cli/extension-manager install python

# Test with authenticated scenario
export GITHUB_TOKEN=$(gh auth token)
../../../v2/cli/extension-manager install python
```

### 5.2 CI Testing

1. Run full CI pipeline on PR
2. Monitor `Test devpod-k8s` job specifically
3. Verify python extension installs successfully
4. Check logs for any rate limit warnings

### 5.3 Stress Testing

```bash
# Simulate concurrent installations (rate limit stress)
for i in {1..5}; do
  ../../../v2/cli/extension-manager install python &
done
wait
```

---

## 6. Decision Matrix

| Option           | Effort | Reliability | Speed   | Offline | Recommended         |
| ---------------- | ------ | ----------- | ------- | ------- | ------------------- |
| A. ubi backend   | Low    | High        | Fast    | No      | **Yes**             |
| B. cargo backend | Low    | High        | Slow    | No      | No                  |
| C. asdf backend  | Low    | Medium      | Fast    | No      | Maybe               |
| D. Pre-bake      | Medium | Highest     | Fastest | Yes     | For critical tools  |
| E. Direct URL    | Low    | High        | Fast    | No      | For pinned versions |
| F. Token pooling | Medium | Medium      | Fast    | No      | No                  |

---

## Appendix A: Related Issues

- [mise #5418](https://github.com/jdx/mise/issues/5418) - aqua backend GITHUB_TOKEN issue
- [GitHub Actions Run #21085230557](https://github.com/pacphi/sindri/actions/runs/21085230557) - Original failure

## Appendix B: Affected Files

- `docker/lib/extensions/python/mise.toml` - Primary fix location
- `docker/lib/extensions/infra-tools/mise.toml` - Reference implementation
- `cli/extension-manager-modules/executor.sh` - Token propagation
- `.github/workflows/test-provider.yml` - CI token configuration

## Appendix C: Monitoring

To detect rate limit issues early:

```bash
# Check current rate limit status
curl -s -H "Authorization: Bearer $GITHUB_TOKEN" \
  https://api.github.com/rate_limit | jq '.rate'
```

Expected output:

```json
{
  "limit": 5000,
  "used": 42,
  "remaining": 4958,
  "reset": 1737100800
}
```
