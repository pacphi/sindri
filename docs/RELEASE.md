# Release Process

This guide explains how to create and publish new releases for Sindri.

> **📚 Related Documentation**:
>
> - [Changelog Management Guide](CHANGELOG_MANAGEMENT.md) - Detailed changelog structure and automation
> - [Contributing Guide](../CONTRIBUTING.md) - Development workflow and conventions
> - [Version-Specific Changelogs](../CHANGELOG.md) - Navigate to v1, v2, or v3 changelogs

## Overview

Sindri uses **automated releases** triggered by Git tags. After the April 2026 reorg, all release workflows live on `main` and are dispatched by tag pattern.

| Tag pattern | Workflow on `main`                       | Source branch | Artifact type                              |
| ----------- | ---------------------------------------- | ------------- | ------------------------------------------ |
| `v2.*.*`    | `.github/workflows/release-v2.yml`       | `v2`          | Multi-distro Docker images + GHCR + cosign |
| `v3.*.*`    | `.github/workflows/release-v3.yml`       | `v3`          | Rust binaries (cargo-dist) + Docker        |
| `v4.*.*`    | `.github/workflows/release-v4.yml` (uses `_release-cargo-dist.yml`) | `v4` | Rust binaries (cargo-dist) |

When you push a version tag, GitHub Actions automatically:

- Validates the tag format and version
- Generates a changelog from commit messages (filtered by the version's source dir)
- Updates the version-specific CHANGELOG.md (`v2/CHANGELOG.md`, `v3/CHANGELOG.md`, or `v4/CHANGELOG.md`)
- Validates version consistency across files
- Builds and publishes Docker images to GHCR (v2 / v3) and/or Rust binaries (v3 / v4)
- Signs container images with cosign and attaches SLSA build provenance
- Generates and attaches an SBOM
- Creates release assets
- Publishes a GitHub Release
- Updates documentation (for stable releases)

> **Branch requirement**: You must tag on the version's source branch (e.g., `git tag v3.1.5` while on the `v3` branch). The release workflow checks out the source branch's `vN/RELEASE_NOTES.md` for the release body.

## Quick Release

For maintainers who want to quickly cut a release:

```bash
# 1. Switch to the version's source branch and pull latest
git checkout v3              # or v2 / v4
git pull --ff-only origin v3

# 2. Confirm CI is green for this branch
gh run list --branch v3 --limit 1

# 3. Create and push a version tag (matches release-v3.yml's tag pattern)
git tag v3.1.5
git push origin v3.1.5

# 4. Monitor the release at:
#    https://github.com/pacphi/sindri/actions/workflows/release-v3.yml
```

The tag pattern (`v2.*.*` / `v3.*.*` / `v4.*.*`) determines which release workflow fires.

## Post-Reorg Release Walkthrough (first release on each line)

After the April 2026 reorg, the first release of each version line should be treated as a verification run. Use the lowest-stakes upcoming release (e.g., a v4 alpha) and walk through the steps below, capturing any deviation as a follow-up issue.

### 1. Pre-flight

```bash
# On the source branch:
git checkout v4
git pull --ff-only origin v4

# Confirm CI shim is green
gh run list --branch v4 --limit 5

# Confirm no in-flight PRs to v4 that should land first
gh pr list --base v4 --state open

# Inspect what the release notes will reference
cat v4/RELEASE_NOTES.md     # release-v4.yml uses this for the GitHub Release body
```

### 2. Tag and push

```bash
# Tag format is enforced by release-v{N}.yml's validate-tag job.
# v4 example (alpha):
git tag v4.0.0-alpha.1 -m "v4.0.0-alpha.1: first post-reorg dry run"
git push origin v4.0.0-alpha.1
```

The push triggers `release-v4.yml` on `main` (matched by the `v4.*.*` tag pattern).

### 3. Watch the workflow

```bash
# List release runs:
gh run list --workflow release-v4.yml --limit 3
# Tail the most recent run:
gh run watch
```

Expected jobs (v4 example):

| Job              | Purpose                                                              |
| ---------------- | -------------------------------------------------------------------- |
| `validate-tag`   | Confirms tag matches `^v4\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$`         |
| `build`          | Calls `_release-cargo-dist.yml` to build cross-platform binaries     |
| `publish`        | Creates the GitHub Release with `v4/RELEASE_NOTES.md` as body        |

For v2 / v3, additional jobs run (changelog generation, Docker image build + GHCR push, cosign signing, SBOM, doc updates for stable releases).

### 4. Verify artifacts and signatures

After the workflow completes:

```bash
# Confirm the GitHub Release exists
gh release view v4.0.0-alpha.1

# Confirm release assets attached (binaries + checksums)
gh release view v4.0.0-alpha.1 --json assets -q '.assets[].name'

# v2 / v3: verify GHCR image
docker pull ghcr.io/pacphi/sindri:3.1.5

# v2 / v3: verify cosign signature (keyless)
cosign verify ghcr.io/pacphi/sindri:3.1.5 \
  --certificate-identity-regexp 'https://github.com/pacphi/sindri' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com'

# v2 / v3: verify SLSA build provenance
gh attestation verify oci://ghcr.io/pacphi/sindri:3.1.5 --owner pacphi
```

### 5. Confirm release notes link to the right branch

`release-v4.yml`'s publish job checks out `ref: v4` and uses `v4/RELEASE_NOTES.md` for the release body. Read the rendered release page and confirm:

- Body content matches the source-branch RELEASE_NOTES.md
- Generated changelog (if any) covers commits since the previous v4 tag
- "Source branch" link points to `v4`, not `main`

### 6. Capture timing for the runbook

Record:

- Total wall time (validate-tag → publish complete)
- Per-job duration (helpful for spotting regressions later)
- Any retries / re-runs and the underlying cause

Update the [Release Schedule](#release-schedule) section if the observed cadence informs future planning.

## Rollback procedure (post-reorg)

If `release-vN.yml` fails after publishing artifacts:

```bash
# 1. Mark the GitHub Release as draft so users don't pull a partial release
gh release edit v3.1.5 --draft

# 2. (Optional) Untag the GHCR image so it can't be pulled by tag
#    Requires write access to packages.
gh api -X DELETE \
  /user/packages/container/sindri/versions/<VERSION_ID>

# 3. Delete the tag locally and remotely
git tag -d v3.1.5
git push origin :refs/tags/v3.1.5

# 4. Fix the underlying issue (do NOT reuse the version number)
# 5. Re-tag with a bumped patch (v3.1.6) and re-run

# Never delete a published, in-use release without coordinating with users.
```

If only the documentation step failed (stable releases): the Release and image are intact; re-run only the failed job:

```bash
gh run rerun <run-id> --failed
```

## Detailed Release Process

### Prerequisites

Before creating a release, ensure:

- [ ] All tests are passing (check GitHub Actions)
- [ ] Documentation is up to date
- [ ] Breaking changes are documented
- [ ] Security scans are clean
- [ ] All PRs for this release are merged

### Step 1: Determine Version Number

Use [Semantic Versioning](https://semver.org/) (MAJOR.MINOR.PATCH):

- **MAJOR** (v2.0.0): Breaking changes to APIs or workflows
- **MINOR** (v1.1.0): New features, backward-compatible
- **PATCH** (v1.0.1): Bug fixes, security updates

Examples:

- Adding a new extension → MINOR version bump
- Fixing a bug in CLI → PATCH version bump
- Changing the extension API → MAJOR version bump

### Step 2: Update Version References (Optional)

The automation will handle most version updates, but you may want to manually update:

```bash
# Check for hardcoded version references
grep -r "v[0-9]\+\.[0-9]\+\.[0-9]\+" docs/
```

### Step 3: Create and Push Git Tag

#### For Stable Releases

```bash
# Create the tag
git tag v1.2.3

# Push to trigger the release workflow
git push origin v1.2.3
```

#### For Pre-releases

Use pre-release identifiers for alpha, beta, or release candidate versions:

```bash
# Alpha release
git tag v1.2.3-alpha.1
git push origin v1.2.3-alpha.1

# Beta release
git tag v1.2.3-beta.1
git push origin v1.2.3-beta.1

# Release candidate
git tag v1.2.3-rc.1
git push origin v1.2.3-rc.1
```

**Pre-release behavior:**

- Marked as "Pre-release" on GitHub
- Not set as the "latest" release
- Documentation is not updated
- Good for testing new features with early adopters

### Step 4: Monitor the Release Workflow

1. Go to [GitHub Actions](https://github.com/pacphi/sindri/actions)
2. Watch the version-specific workflow (`v2: Release`, `v3: Release`, or `v4: Release`)
3. Verify the expected jobs complete successfully:
   - `validate-tag` — semver/tag-pattern validation
   - `generate-changelog` (v2/v3 only)
   - `build` — binaries (v3/v4 via cargo-dist) and/or Docker image (v2/v3)
   - `sign` / `attest` — cosign + SLSA provenance (v2/v3 image releases)
   - `publish` — create the GitHub Release with attached assets
   - `update-docs` (stable releases only)

### Step 5: Verify the Release

After the workflow completes:

1. **Check the Release Page**: Visit <https://github.com/pacphi/sindri/releases>
2. **Verify Release Assets**:
   - `install.sh` - Installation script
   - `QUICK_REFERENCE.md` - Quick reference guide
3. **Review Changelog**: Ensure generated changelog is accurate
4. **Verify Docker Image**: Check GHCR for the new image tag

```bash
# Pull and verify the released image
docker pull ghcr.io/pacphi/sindri:1.2.3
docker run --rm ghcr.io/pacphi/sindri:1.2.3 --version
```

## Tag Format Requirements

Tags must follow this pattern:

```text
v[MAJOR].[MINOR].[PATCH](-[PRERELEASE])?
```

**Valid tags:**

- `v1.0.0` - Stable release
- `v1.2.3` - Stable release
- `v2.0.0-alpha.1` - Alpha pre-release
- `v1.5.0-beta.2` - Beta pre-release
- `v1.0.0-rc.1` - Release candidate

**Invalid tags:**

- `1.0.0` - Missing 'v' prefix
- `v1.0` - Missing patch version
- `release-1.0.0` - Wrong prefix
- `v1.0.0-SNAPSHOT` - Invalid pre-release format

## Changelog Generation

The automation generates changelogs from commit messages. For best results, use [Conventional Commits](https://www.conventionalcommits.org/).

### Commit Message Format

```text
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Commit Types

Commits are automatically categorized:

- `feat:` or `feat(scope):` → **Features** section
- `fix:` or `fix(scope):` → **Bug Fixes** section
- `docs:` or `docs(scope):` → **Documentation** section
- `deps:` → **Dependencies** section
- `perf:` → **Performance** section
- `refactor:` → **Refactoring** section
- `test:` → **Tests** section
- `chore:`, `ci:`, `style:` → **Maintenance** section
- Other types → **Other Changes** section

### Examples

```bash
# Feature
git commit -m "feat: add Ruby extension support"
git commit -m "feat(extensions): add Python data science stack"

# Bug fix
git commit -m "fix: resolve SSH key permission issues"
git commit -m "fix(ci): make health check CI-mode aware"

# Documentation
git commit -m "docs: update QUICKSTART with new extension system"
git commit -m "docs(api): document extension manager commands"

# Dependencies
git commit -m "deps: update Node.js to LTS v22"

# Other
git commit -m "chore: clean up temporary files"
git commit -m "style: format shell scripts"
```

## What Gets Automated

### Automated Changelog

The workflow automatically:

1. Compares current tag with previous tag
2. Extracts all commits since last release
3. Categorizes by commit type
4. Generates formatted changelog with:
   - Features section
   - Bug Fixes section
   - Documentation section
   - Dependencies section
   - Performance section
   - Refactoring section
   - Tests section
   - Maintenance section
   - Other Changes section
5. Adds installation instructions
6. Includes full diff link

### CHANGELOG.md Updates

For both stable and pre-releases:

- Adds new version section to CHANGELOG.md
- Preserves existing changelog entries
- Commits and pushes updates back to main branch

**Important:** CHANGELOG.md is auto-generated. Do not edit manually.

### Docker Image Build

The workflow builds and publishes Docker images:

- Tagged with version (e.g., `ghcr.io/pacphi/sindri:1.2.3`)
- Tagged with semver patterns (`1.2`, `1`)
- Includes OCI labels for version tracking

### Release Assets

Files are automatically created and attached:

#### install.sh

- Quick installation script
- Downloads specific version
- Validates prerequisites
- Provider-agnostic deployment options

#### QUICK_REFERENCE.md

- Common commands
- Setup instructions
- Documentation links

### Documentation Updates (Stable Releases Only)

For stable releases (not pre-releases):

- Updates version references in README.md
- Version badges auto-update via shields.io
- Commits documentation changes to main branch

## Rollback and Recovery

### Delete a Tag Locally and Remotely

```bash
# Delete local tag
git tag -d v1.2.3

# Delete remote tag
git push origin :refs/tags/v1.2.3
```

### Delete a Release on GitHub

1. Go to <https://github.com/pacphi/sindri/releases>
2. Click the release to delete
3. Click "Delete" button
4. Confirm deletion

### Fix a Bad Release

If a release has issues:

1. **Delete the release and tag** (see above)
2. **Fix the issues** in your code
3. **Create a new patch version** with the fixes:

   ```bash
   git tag v1.2.4
   git push origin v1.2.4
   ```

Never reuse a version number that has already been published.

## Release Checklist

Use this checklist for each release:

### Pre-Release

- [ ] All tests passing on main branch
- [ ] All planned PRs merged
- [ ] Documentation reviewed and updated
- [ ] Breaking changes documented (if any)
- [ ] Security vulnerabilities addressed
- [ ] Version number decided (MAJOR.MINOR.PATCH)
- [ ] Commit messages follow conventional format

### Release

- [ ] Tag created with correct format
- [ ] Tag pushed to GitHub
- [ ] Workflow started successfully
- [ ] All workflow jobs completed

### Post-Release

- [ ] Release visible on GitHub releases page
- [ ] Release assets present (install.sh, etc.)
- [ ] Docker image available on GHCR
- [ ] Changelog accurate and complete
- [ ] CHANGELOG.md updated in repository
- [ ] Documentation updated (stable releases)
- [ ] Installation tested from release artifacts
- [ ] Community notified (if applicable)

## Versioning Strategy

### Patch Releases (v1.0.x)

Create patch releases for:

- Bug fixes
- Security updates
- Documentation corrections
- Minor script improvements

**Example:**

```bash
git tag v1.0.1
git push origin v1.0.1
```

### Minor Releases (v1.x.0)

Create minor releases for:

- New extensions
- New features
- Backward-compatible enhancements
- Tool updates

**Example:**

```bash
git tag v1.1.0
git push origin v1.1.0
```

### Major Releases (vx.0.0)

Create major releases for:

- Breaking API changes
- Incompatible extension system changes
- Major architectural changes
- Workflow breaking changes

**Example:**

```bash
git tag v2.0.0
git push origin v2.0.0
```

### Pre-releases

Use pre-releases for:

- Testing new features
- Early access for contributors
- Release candidates before stable

**Alpha** - Early development, unstable:

```bash
git tag v1.2.0-alpha.1
git push origin v1.2.0-alpha.1
```

**Beta** - Feature complete, testing needed:

```bash
git tag v1.2.0-beta.1
git push origin v1.2.0-beta.1
```

**Release Candidate** - Stable, final testing:

```bash
git tag v1.2.0-rc.1
git push origin v1.2.0-rc.1
```

## Troubleshooting

### Workflow Fails

If the release workflow fails:

1. Check the [Actions tab](https://github.com/pacphi/sindri/actions)
2. Review the failed job logs
3. Fix the issue
4. Delete the tag and recreate:

   ```bash
   git tag -d v1.2.3
   git push origin :refs/tags/v1.2.3
   # Fix the issue
   git tag v1.2.3
   git push origin v1.2.3
   ```

### Tag Already Exists

```bash
# If you need to move a tag
git tag -d v1.2.3
git push origin :refs/tags/v1.2.3
git tag v1.2.3 <commit-sha>
git push origin v1.2.3
```

### Changelog Not Generated

The workflow generates changelog from commits. Ensure:

- Commits exist between tags
- Commit messages are properly formatted
- Previous tag exists and is valid

### Release Not Marked as Latest

Only stable releases (without pre-release suffix) are marked as "latest":

- `v1.2.3` → Marked as latest
- `v1.2.3-beta.1` → Not marked as latest

### Docker Image Not Published

Check that:

- GHCR authentication succeeded
- Docker build completed without errors
- Image tags are valid

## Best Practices

### Before Each Release

1. **Run Local Validation**

   ```bash
   pnpm validate
   pnpm test
   ```

2. **Test Deployment**

   ```bash
   pnpm build
   docker run -it sindri:local
   ```

3. **Review Recent Commits**

   ```bash
   git log $(git describe --tags --abbrev=0)..HEAD --oneline
   ```

### After Each Release

1. **Announce the Release** (for significant releases)
   - Update project README if needed
   - Post in discussions
   - Share with community

2. **Monitor for Issues**
   - Watch GitHub issues
   - Monitor deployment reports
   - Respond to user feedback

3. **Plan Next Release**
   - Review roadmap
   - Prioritize features
   - Update milestones

## Release Schedule

Sindri follows a rolling release model:

- **Patch releases**: As needed for bug fixes and security updates
- **Minor releases**: When new features are ready and tested
- **Major releases**: When breaking changes are necessary

There is no fixed schedule. Releases happen when:

1. Sufficient changes have accumulated
2. All tests pass
3. Documentation is current
4. No blocking issues exist

## Questions?

- Review [CONTRIBUTING.md](../CONTRIBUTING.md) for development workflow
- Check [GitHub Issues](https://github.com/pacphi/sindri/issues) for known problems
- Start a [Discussion](https://github.com/pacphi/sindri/discussions) for questions
- Contact maintainers for release-specific questions
