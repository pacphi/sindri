# Sindri v3 — Multi-Distro Addendum

**Image Naming & Tag Strategy · Makefile Additions**

> Supplements: PRD & Technical Specification v1.0
> Version 1.0 · March 2026 · [pacphi/sindri](https://github.com/pacphi/sindri)

---

## Table of Contents

1. [Image Naming & Tag Convention](#1-image-naming--tag-convention)
   - [1.1 Design Principles](#11-design-principles)
   - [1.2 Full Tag Taxonomy](#12-full-tag-taxonomy)
   - [1.3 `release-v3.yml`: Updated `promote-image` Job](#13-release-v3yml-updated-promote-image-job)
   - [1.4 Updated `ci-v3.yml`: Distro-Tagged CI Builds](#14-updated-ci-v3yml-distro-tagged-ci-builds)
2. [Makefile Additions & Updates](#2-makefile-additions--updates)
   - [2.1 New Variables](#21-new-variables)
   - [2.2 Per-Distro Build Targets](#22-per-distro-build-targets)
   - [2.3 Updated Dev Cycle Targets](#23-updated-dev-cycle-targets)
   - [2.4 Distro Testing Targets](#24-distro-testing-targets)
   - [2.5 Updated Cache & Image Management](#25-updated-cache--image-management-targets)
   - [2.6 Updated `.PHONY` List and Help Text](#26-updated-phony-list-and-help-text)
   - [2.7 Summary of Changes to Existing Targets](#27-summary-of-changes-to-existing-targets)

---

## 1 Image Naming & Tag Convention

The current release pipeline produces a single set of tags all pointing to the same Ubuntu 24.04 image (`3.0.0`, `3.0`, `3`, `v3`, `v3-latest`, `latest`). With multi-distro support, the registry must carry three distinct image variants — and the tagging scheme must be backward-compatible so that existing users who pull `sindri:latest` continue to receive the Ubuntu image.

### 1.1 Design Principles

- **Backward compatibility** — All unqualified/floating tags (`latest`, `v3-latest`, `v3`, semver without suffix) continue to resolve to the Ubuntu image. No breaking change for existing users.

- **Explicit distro suffix** — Distro-specific images carry a `-ubuntu`, `-fedora`, or `-opensuse` suffix appended to every tag. This mirrors the convention used by official Docker Hub images (e.g. `node:20-bookworm`, `python:3.12-alpine`).

- **Floating distro aliases** — Short floating tags (`ubuntu`, `fedora`, `opensuse`) always point to the latest stable release of each variant — useful for `docker pull sindri:fedora` in CI scripts.

- **CI tags use SHA suffix** — CI images keep their existing `v3-ci-<sha>` and `v3-ci-passed-<sha>` scheme, extended with `v3-ci-<sha>-fedora` etc. when distro matrix builds run.

- **Base images** — Each distro variant gets its own base image tag (`base-ubuntu-3.0.0`, `base-fedora-3.0.0`, `base-opensuse-3.0.0`) to decouple their build cadences.

---

### 1.2 Full Tag Taxonomy

#### Stable Release (e.g. `v3.1.0`)

| Tag              | Distro        | Floating? | Notes                                             |
| ---------------- | ------------- | --------- | ------------------------------------------------- |
| `3.1.0`          | Ubuntu 24.04  | No        | Immutable versioned tag (backward-compat default) |
| `3.1.0-ubuntu`   | Ubuntu 24.04  | No        | Explicit distro + version (preferred for pinning) |
| `3.1.0-fedora`   | Fedora 41     | No        | Explicit distro + version                         |
| `3.1.0-opensuse` | openSUSE 15.6 | No        | Explicit distro + version                         |
| `3.1`            | Ubuntu 24.04  | Yes       | Minor float (backward-compat)                     |
| `3.1-ubuntu`     | Ubuntu 24.04  | Yes       | Minor float with distro                           |
| `3.1-fedora`     | Fedora 41     | Yes       | Minor float with distro                           |
| `3.1-opensuse`   | openSUSE 15.6 | Yes       | Minor float with distro                           |
| `3`              | Ubuntu 24.04  | Yes       | Major float (backward-compat)                     |
| `3-ubuntu`       | Ubuntu 24.04  | Yes       | Major float with distro                           |
| `3-fedora`       | Fedora 41     | Yes       | Major float with distro                           |
| `3-opensuse`     | openSUSE 15.6 | Yes       | Major float with distro                           |
| `v3`             | Ubuntu 24.04  | Yes       | v3 alias (backward-compat)                        |
| `v3-latest`      | Ubuntu 24.04  | Yes       | v3-latest alias (backward-compat)                 |
| `latest`         | Ubuntu 24.04  | Yes       | latest alias (backward-compat)                    |
| `ubuntu`         | Ubuntu 24.04  | Yes       | Short distro alias — always latest stable         |
| `fedora`         | Fedora 41     | Yes       | Short distro alias — always latest stable         |
| `opensuse`       | openSUSE 15.6 | Yes       | Short distro alias — always latest stable         |

#### Pre-release (e.g. `v3.1.0-alpha.3`)

| Tag                      | Distro        | Notes                                   |
| ------------------------ | ------------- | --------------------------------------- |
| `3.1.0-alpha.3`          | Ubuntu 24.04  | Immutable pre-release (backward-compat) |
| `3.1.0-alpha.3-ubuntu`   | Ubuntu 24.04  | Explicit distro                         |
| `3.1.0-alpha.3-fedora`   | Fedora 41     | Explicit distro                         |
| `3.1.0-alpha.3-opensuse` | openSUSE 15.6 | Explicit distro                         |

> **Note** — Pre-releases never update the floating (`latest`, `ubuntu`, `fedora`, `opensuse`) aliases. Only immutable versioned tags are pushed.

#### CI / Promotion Tags

| Tag                           | Description                                       |
| ----------------------------- | ------------------------------------------------- |
| `v3-ci-<sha>`                 | Ubuntu CI build for commit sha (existing pattern) |
| `v3-ci-<sha>-ubuntu`          | Ubuntu CI build — explicit distro suffix          |
| `v3-ci-<sha>-fedora`          | Fedora CI build for commit sha                    |
| `v3-ci-<sha>-opensuse`        | openSUSE CI build for commit sha                  |
| `v3-ci-passed-<sha>`          | Ubuntu promotion candidate (existing pattern)     |
| `v3-ci-passed-<sha>-ubuntu`   | Ubuntu promotion candidate — explicit             |
| `v3-ci-passed-<sha>-fedora`   | Fedora promotion candidate                        |
| `v3-ci-passed-<sha>-opensuse` | openSUSE promotion candidate                      |

#### Base Images (GHCR)

| Tag                    | Description                                           |
| ---------------------- | ----------------------------------------------------- |
| `base-ubuntu-3.0.0`    | Versioned Ubuntu base (Ubuntu 24.04 + Rust toolchain) |
| `base-ubuntu-latest`   | Floating Ubuntu base (always latest)                  |
| `base-fedora-3.0.0`    | Versioned Fedora base (Fedora 41 + Rust toolchain)    |
| `base-fedora-latest`   | Floating Fedora base                                  |
| `base-opensuse-3.0.0`  | Versioned openSUSE base (Leap 15.6 + Rust toolchain)  |
| `base-opensuse-latest` | Floating openSUSE base                                |
| `base-latest`          | Alias for `base-ubuntu-latest` (backward-compat)      |

---

### 1.3 `release-v3.yml`: Updated `promote-image` Job

The `promote-image` job must be split into three parallel matrix entries, one per distro. Each pulls the appropriate distro-suffixed CI image and applies the distro-aware tag set.

```yaml
# Job 5 (updated): promote each distro variant in parallel
promote-image:
  name: Promote — ${{ matrix.distro }}
  runs-on: ubuntu-latest
  needs: [validate-tag, verify-ci-image, verify-pre-release-passed]
  strategy:
    fail-fast: false
    matrix:
      distro: [ubuntu, fedora, opensuse]
  permissions:
    contents: read
    packages: write
  env:
    DISTRO: ${{ matrix.distro }}
    DOCKERHUB_USERNAME: ${{ secrets.DOCKERHUB_USERNAME }}
    DOCKERHUB_TOKEN: ${{ secrets.DOCKERHUB_TOKEN }}
  steps:
    - name: Login to GHCR
      uses: docker/login-action@v4
      with:
        registry: ghcr.io
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}

    - name: Login to Docker Hub (optional)
      if: env.DOCKERHUB_TOKEN != ''
      uses: docker/login-action@v4
      with:
        username: ${{ env.DOCKERHUB_USERNAME }}
        password: ${{ env.DOCKERHUB_TOKEN }}

    - name: Determine image list
      id: images
      run: |
        IMAGES="ghcr.io/${{ github.repository }}"
        if [[ -n "$DOCKERHUB_USERNAME" ]]; then
          IMAGES="${IMAGES}"$'\n'"${DOCKERHUB_USERNAME}/sindri"
        fi
        echo "list<<EOF" >> $GITHUB_OUTPUT
        echo "$IMAGES" >> $GITHUB_OUTPUT
        echo "EOF" >> $GITHUB_OUTPUT

    - name: Generate tags for ${{ matrix.distro }}
      id: meta
      uses: docker/metadata-action@v6
      with:
        images: ${{ steps.images.outputs.list }}
        tags: |
          # ── Versioned tags with distro suffix (all distros) ──────────────
          type=semver,pattern={{version}}-${{ matrix.distro }},value=v${{ needs.validate-tag.outputs.version }}
          type=semver,pattern={{major}}.{{minor}}-${{ matrix.distro }},value=v${{ needs.validate-tag.outputs.version }}
          type=semver,pattern={{major}}-${{ matrix.distro }},value=v${{ needs.validate-tag.outputs.version }},enable=${{ needs.validate-tag.outputs.is_prerelease == 'false' }}

          # ── Distro-only floating alias (stable only) ──────────────────
          type=raw,value=${{ matrix.distro }},enable=${{ needs.validate-tag.outputs.is_prerelease == 'false' }}

          # ── Ubuntu-only backward-compat tags (stable only) ────────────
          type=semver,pattern={{version}},value=v${{ needs.validate-tag.outputs.version }},enable=${{ matrix.distro == 'ubuntu' }}
          type=semver,pattern={{major}}.{{minor}},value=v${{ needs.validate-tag.outputs.version }},enable=${{ matrix.distro == 'ubuntu' }}
          type=semver,pattern={{major}},value=v${{ needs.validate-tag.outputs.version }},enable=${{ matrix.distro == 'ubuntu' && needs.validate-tag.outputs.is_prerelease == 'false' }}
          type=raw,value=v3,enable=${{ matrix.distro == 'ubuntu' && needs.validate-tag.outputs.is_prerelease == 'false' }}
          type=raw,value=v3-latest,enable=${{ matrix.distro == 'ubuntu' && needs.validate-tag.outputs.is_prerelease == 'false' }}
          type=raw,value=latest,enable=${{ matrix.distro == 'ubuntu' && needs.validate-tag.outputs.is_prerelease == 'false' }}
        labels: |
          org.opencontainers.image.title=Sindri v3 (${{ matrix.distro }})
          org.opencontainers.image.description=Multi-cloud development environment orchestrator (Rust) — ${{ matrix.distro }} variant
          org.opencontainers.image.version=${{ needs.validate-tag.outputs.version }}
          sindri.version=v3
          sindri.distro=${{ matrix.distro }}
          sindri.release.promoted-from=${{ needs.verify-ci-image.outputs.ci_image }}-${{ matrix.distro }}

    - name: Pull CI image for ${{ matrix.distro }}
      run: |
        SHA="${{ needs.verify-ci-image.outputs.commit_sha }}"
        # Prefer "passed" tag; fall back to bare CI tag
        CI_IMG="ghcr.io/${{ github.repository }}:v3-ci-passed-${SHA}-${{ matrix.distro }}"
        CI_FALLBACK="ghcr.io/${{ github.repository }}:v3-ci-${SHA}-${{ matrix.distro }}"

        if docker pull "$CI_IMG" 2>/dev/null; then
          echo "ci_image=$CI_IMG" >> $GITHUB_ENV
        elif docker pull "$CI_FALLBACK" 2>/dev/null; then
          echo "ci_image=$CI_FALLBACK" >> $GITHUB_ENV
          echo "::warning::Using unverified CI image for ${{ matrix.distro }}: $CI_FALLBACK"
        else
          echo "::error::No CI image found for distro=${{ matrix.distro }}, sha=${SHA}"
          echo "::error::Expected: $CI_IMG"
          exit 1
        fi

    - name: Push release tags for ${{ matrix.distro }}
      run: |
        TAGS="${{ steps.meta.outputs.tags }}"
        echo "Retagging ${ci_image} for release..."
        while IFS= read -r TAG; do
          [[ -z "$TAG" ]] && continue
          echo "  → $TAG"
          docker tag "${ci_image}" "$TAG"
        done <<< "$TAGS"

        while IFS= read -r TAG; do
          [[ -z "$TAG" ]] && continue
          if [[ "$TAG" =~ ^docker\.io/ ]] && [[ -z "$DOCKERHUB_USERNAME" ]]; then
            echo "  ⏭  Skipping $TAG (Docker Hub not configured)"
            continue
          fi
          docker push "$TAG" && echo "  ✅ $TAG" || echo "  ⚠  $TAG (push failed, non-fatal)"
        done <<< "$TAGS"

# ── verify-ci-image: also checks all three distro images exist ──────────────
verify-ci-image:
  # Add to existing outputs:
  outputs:
    ci_image: ${{ steps.verify.outputs.ci_image }} # ubuntu CI image (existing)
    image_digest: ${{ steps.verify.outputs.image_digest }}
    commit_sha: ${{ steps.get-sha.outputs.sha }}
    all_distros_available: ${{ steps.check-distros.outputs.all_distros_available }} # NEW
  steps:
    # ... (existing steps) ...
    - name: Check all distro CI images
      id: check-distros
      run: |
        SHA="${{ steps.get-sha.outputs.sha }}"
        ALL_OK=true
        for DISTRO in ubuntu fedora opensuse; do
          IMG="ghcr.io/${{ github.repository }}:v3-ci-passed-${SHA}-${DISTRO}"
          if docker pull "$IMG" 2>/dev/null; then
            echo "  ✅ ${DISTRO}: $IMG"
          else
            echo "  ⚠  ${DISTRO}: CI image not found (${IMG})"
            ALL_OK=false
          fi
        done
        echo "all_distros_available=${ALL_OK}" >> $GITHUB_OUTPUT
```

---

### 1.4 Updated `ci-v3.yml`: Distro-Tagged CI Builds

The `build-image` job becomes a 3-way matrix; `mark-passed` loops over all three distros.

```yaml
build-image:
  name: Build CI Image — ${{ matrix.distro }}
  runs-on: ubuntu-latest
  needs: rust-build
  strategy:
    fail-fast: false
    matrix:
      distro: [ubuntu, fedora, opensuse]
  permissions:
    contents: read
    packages: write
    id-token: write
    attestations: write
  steps:
    # ... (existing steps: checkout, download binary, setup-buildx, login) ...

    - name: Generate metadata for ${{ matrix.distro }}
      id: meta
      uses: docker/metadata-action@v6
      with:
        images: ghcr.io/${{ github.repository }}
        tags: |
          type=raw,value=v3-ci-${{ github.sha }}-${{ matrix.distro }}
        labels: |
          sindri.distro=${{ matrix.distro }}
          sindri.ci.run=${{ github.run_id }}

    - name: Build and push — ${{ matrix.distro }}
      id: build
      uses: docker/build-push-action@v7
      with:
        context: .
        file: v3/Dockerfile
        push: true
        tags: ${{ steps.meta.outputs.tags }}
        labels: ${{ steps.meta.outputs.labels }}
        platforms: linux/amd64
        no-cache: false
        pull: true
        cache-from: type=gha,scope=${{ matrix.distro }}
        cache-to: type=gha,scope=${{ matrix.distro }},mode=max
        build-args: |
          DISTRO=${{ matrix.distro }}
        provenance: mode=max
        sbom: true

# mark-passed: promote all three distros
mark-passed:
  name: Mark Promotion Candidates
  needs: [build-image, security-scan, k8s-cluster-lifecycle]
  if: github.event_name == 'push' && github.ref == 'refs/heads/main'
  runs-on: ubuntu-latest
  permissions:
    contents: read
    packages: write
  steps:
    - name: Login to GHCR
      uses: docker/login-action@v4
      with:
        registry: ghcr.io
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}

    - name: Tag all distros as promotion candidates
      run: |
        for DISTRO in ubuntu fedora opensuse; do
          SRC="ghcr.io/${{ github.repository }}:v3-ci-${{ github.sha }}-${DISTRO}"
          DST="ghcr.io/${{ github.repository }}:v3-ci-passed-${{ github.sha }}-${DISTRO}"
          docker pull "$SRC"
          docker tag "$SRC" "$DST"
          docker push "$DST"
          echo "✅ Marked: $DST"
        done
```

---

## 2 Makefile Additions & Updates

The Makefile requires four categories of change: new variables, new per-distro build targets, updated dev-cycle targets that accept a `DISTRO` parameter, and updated help text. All additions are backward-compatible — existing targets that don't pass `DISTRO` behave exactly as before.

---

### 2.1 New Variables

Add after the `VERSION` / `GIT_COMMIT` block:

```makefile
# ============================================================================
# Multi-Distro Configuration
# ============================================================================

# Default distro for local builds — matches Dockerfile ARG default.
# Override: make v3-docker-build DISTRO=fedora
DISTRO ?= ubuntu

# Validated distro values (guard used in distro-aware targets)
VALID_DISTROS := ubuntu fedora opensuse

# Version pins for distro base images (keep in sync with Dockerfile ARGs)
UBUNTU_VERSION   ?= 24.04
FEDORA_VERSION   ?= 41
OPENSUSE_VERSION ?= 15.6

# Computed local image name including distro suffix
# Examples: sindri:v3-ubuntu-local, sindri:v3-fedora-local
DISTRO_IMAGE_LOCAL     := sindri:v3-$(DISTRO)-local
DISTRO_IMAGE_VERSIONED := sindri:$(VERSION)-$(GIT_COMMIT)-$(DISTRO)

# Target registry (for remote push targets)
REGISTRY ?= ghcr.io/pacphi

# ─── Guard macro: aborts if DISTRO is not in VALID_DISTROS ──────────────────
define assert_valid_distro
	@if ! echo " $(VALID_DISTROS) " | grep -q " $(DISTRO) "; then \
		echo "$(RED)✗ Unknown DISTRO: $(DISTRO)$(RESET)"; \
		echo "  Valid values: $(VALID_DISTROS)"; \
		exit 1; \
	fi
endef
```

---

### 2.2 Per-Distro Build Targets

These replace the need to type `--build-arg DISTRO=...` manually. Each calls the shared `_v3-docker-build-impl` implementation.

```makefile
# ============================================================================
# V3 Per-Distro Docker Build Targets
# ============================================================================

# Internal implementation — do not call directly; use named distro targets.
.PHONY: _v3-docker-build-impl
_v3-docker-build-impl:
	$(call assert_valid_distro)
	@echo "$(BLUE)Building Sindri v3 Docker image [distro=$(DISTRO)]...$(RESET)"
	@echo "$(BLUE)→ Dockerfile: v3/Dockerfile$(RESET)"
	@echo "$(BLUE)→ Tags: $(DISTRO_IMAGE_LOCAL)$(RESET)"
	docker build \
		--build-arg DISTRO=$(DISTRO) \
		--build-arg UBUNTU_VERSION=$(UBUNTU_VERSION) \
		--build-arg FEDORA_VERSION=$(FEDORA_VERSION) \
		--build-arg OPENSUSE_VERSION=$(OPENSUSE_VERSION) \
		-t $(DISTRO_IMAGE_LOCAL) \
		-t $(DISTRO_IMAGE_VERSIONED) \
		-f $(V3_DIR)/Dockerfile \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ Build complete: $(DISTRO_IMAGE_LOCAL)$(RESET)"

# ── Convenience aliases ───────────────────────────────────────────────────────
.PHONY: v3-docker-build-ubuntu
v3-docker-build-ubuntu:
	@$(MAKE) _v3-docker-build-impl DISTRO=ubuntu

.PHONY: v3-docker-build-fedora
v3-docker-build-fedora:
	@$(MAKE) _v3-docker-build-impl DISTRO=fedora

.PHONY: v3-docker-build-opensuse
v3-docker-build-opensuse:
	@$(MAKE) _v3-docker-build-impl DISTRO=opensuse

# ── Build all three distros sequentially ─────────────────────────────────────
.PHONY: v3-docker-build-all
v3-docker-build-all: v3-docker-build-ubuntu v3-docker-build-fedora v3-docker-build-opensuse
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ All three distro images built:$(RESET)"
	@docker images --filter="reference=sindri:v3-*-local" \
		--format "table {{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"

# ── DISTRO-parameterised generic target (for CI scripts) ─────────────────────
#   Usage: make v3-docker-build DISTRO=fedora
.PHONY: v3-docker-build
v3-docker-build:
	@$(MAKE) _v3-docker-build-impl

# ─────────────────────────────────────────────────────────────────────────────
# Dev image (Dockerfile.dev — bundled extensions, from source)
# ─────────────────────────────────────────────────────────────────────────────

.PHONY: _v3-docker-build-dev-impl
_v3-docker-build-dev-impl:
	$(call assert_valid_distro)
	@echo "$(BLUE)Building Sindri v3 DEV image [distro=$(DISTRO)] from source...$(RESET)"
	@echo "$(BLUE)→ Dockerfile: v3/Dockerfile.dev  (~3-5 min)$(RESET)"
	docker build \
		--build-arg DISTRO=$(DISTRO) \
		--build-arg BASE_IMAGE=$(REGISTRY)/sindri:base-$(DISTRO)-latest \
		-t sindri:v3-$(DISTRO)-dev \
		-t sindri:$(VERSION)-$(GIT_COMMIT)-$(DISTRO)-dev \
		-f $(V3_DIR)/Dockerfile.dev \
		$(PROJECT_ROOT)
	@echo "$(GREEN)✓ Dev image complete: sindri:v3-$(DISTRO)-dev$(RESET)"

.PHONY: v3-docker-build-dev-ubuntu
v3-docker-build-dev-ubuntu:
	@$(MAKE) _v3-docker-build-dev-impl DISTRO=ubuntu

.PHONY: v3-docker-build-dev-fedora
v3-docker-build-dev-fedora:
	@$(MAKE) _v3-docker-build-dev-impl DISTRO=fedora

.PHONY: v3-docker-build-dev-opensuse
v3-docker-build-dev-opensuse:
	@$(MAKE) _v3-docker-build-dev-impl DISTRO=opensuse

.PHONY: v3-docker-build-dev
v3-docker-build-dev:
	@$(MAKE) _v3-docker-build-dev-impl

# ─────────────────────────────────────────────────────────────────────────────
# Base image builds (one per distro)
# ─────────────────────────────────────────────────────────────────────────────

.PHONY: _v3-docker-build-base-impl
_v3-docker-build-base-impl:
	$(call assert_valid_distro)
	@echo "$(BOLD)$(BLUE)Building v3 base image [distro=$(DISTRO)]...$(RESET)"
	@echo "Build time: ~15-20 min (arm64). Rebuild on Rust version change."
	docker build \
		--build-arg DISTRO=$(DISTRO) \
		--build-arg UBUNTU_VERSION=$(UBUNTU_VERSION) \
		--build-arg FEDORA_VERSION=$(FEDORA_VERSION) \
		--build-arg OPENSUSE_VERSION=$(OPENSUSE_VERSION) \
		-t sindri:base-$(DISTRO)-$(VERSION) \
		-t sindri:base-$(DISTRO)-latest \
		-f $(V3_DIR)/Dockerfile.base \
		$(V3_DIR)
	@echo "$(GREEN)✓ Base image built: sindri:base-$(DISTRO)-latest$(RESET)"

.PHONY: v3-docker-build-base-ubuntu
v3-docker-build-base-ubuntu:
	@$(MAKE) _v3-docker-build-base-impl DISTRO=ubuntu

.PHONY: v3-docker-build-base-fedora
v3-docker-build-base-fedora:
	@$(MAKE) _v3-docker-build-base-impl DISTRO=fedora

.PHONY: v3-docker-build-base-opensuse
v3-docker-build-base-opensuse:
	@$(MAKE) _v3-docker-build-base-impl DISTRO=opensuse

# Usage: make v3-docker-build-base DISTRO=fedora
.PHONY: v3-docker-build-base
v3-docker-build-base:
	@$(MAKE) _v3-docker-build-base-impl

.PHONY: v3-docker-build-base-all
v3-docker-build-base-all: v3-docker-build-base-ubuntu v3-docker-build-base-fedora v3-docker-build-base-opensuse
	@echo "$(GREEN)$(BOLD)✓ All three base images built$(RESET)"
```

---

### 2.3 Updated Dev Cycle Targets

The existing `v3-cycle-fast` and `v3-cycle-clean` targets accept an optional `DISTRO` parameter. Existing invocations without `DISTRO` default to `ubuntu`, preserving backward compatibility.

```makefile
# ============================================================================
# V3 Updated Dev Cycle Targets (DISTRO-aware)
# ============================================================================

# v3-cycle-fast: incremental build for one distro
# Usage: make v3-cycle-fast CONFIG=sindri.yaml
#        make v3-cycle-fast CONFIG=sindri.yaml DISTRO=fedora
.PHONY: v3-cycle-fast
v3-cycle-fast:
	@if [ -z "$(CONFIG)" ]; then \
		echo "$(RED)Error: CONFIG is required$(RESET)"; \
		echo "Usage: make v3-cycle-fast CONFIG=/path/to/sindri.yaml [DISTRO=ubuntu|fedora|opensuse]"; \
		exit 1; \
	fi
	$(call assert_valid_distro)
	@echo ""
	@echo "$(BOLD)$(GREEN)╔══════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(GREEN)║         V3 Fast Development Cycle [$(DISTRO)]                    ║$(RESET)"
	@echo "$(BOLD)$(GREEN)╚══════════════════════════════════════════════════════════════╝$(RESET)"
	@echo "$(BOLD)Mode:$(RESET) Incremental  $(BOLD)Distro:$(RESET) $(DISTRO)  $(BOLD)Time:$(RESET) ~3-5 min"
	@echo ""
	@$(MAKE) v3-cache-clear-soft
	@sindri destroy --config $(CONFIG) -f || true
	@$(MAKE) _v3-docker-build-dev-impl
	@$(MAKE) v3-install
	@sindri deploy --config $(CONFIG)
	@echo ""
	@echo "$(GREEN)✓ Fast cycle [$(DISTRO)] complete — Connect: sindri connect --config $(CONFIG)$(RESET)"

.PHONY: v3-cycle-clean
v3-cycle-clean:
	@if [ -z "$(CONFIG)" ]; then \
		echo "$(RED)Error: CONFIG is required$(RESET)"; exit 1; \
	fi
	$(call assert_valid_distro)
	@echo ""
	@echo "$(BOLD)$(YELLOW)╔══════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(YELLOW)║        V3 Clean Development Cycle [$(DISTRO)]                    ║$(RESET)"
	@echo "$(BOLD)$(YELLOW)╚══════════════════════════════════════════════════════════════╝$(RESET)"
	@echo "$(BOLD)Mode:$(RESET) Clean build  $(BOLD)Distro:$(RESET) $(DISTRO)  $(BOLD)Time:$(RESET) ~10-15 min"
	@echo ""
	@$(MAKE) v3-cache-clear-medium
	@sindri destroy --config $(CONFIG) -f || true
	@docker images --filter="reference=sindri:v3-$(DISTRO)*" \
		--format "{{.ID}}" | xargs docker rmi -f 2>/dev/null || true
	@$(MAKE) _v3-docker-build-dev-impl
	@$(MAKE) v3-install
	@sindri deploy --config $(CONFIG)
	@echo ""
	@echo "$(GREEN)✓ Clean cycle [$(DISTRO)] complete$(RESET)"
```

---

### 2.4 Distro Testing Targets

```makefile
# ============================================================================
# V3 Distro Smoke Tests (local — mirrors the CI distro matrix job)
# ============================================================================

# Run smoke test on a locally-built distro image.
# Usage: make v3-distro-test DISTRO=fedora
.PHONY: v3-distro-test
v3-distro-test:
	$(call assert_valid_distro)
	@IMG="$(DISTRO_IMAGE_LOCAL)"; \
	if ! docker image inspect "$$IMG" >/dev/null 2>&1; then \
		echo "$(YELLOW)Image not found: $$IMG$(RESET)"; \
		echo "Build first: make v3-docker-build DISTRO=$(DISTRO)"; \
		exit 1; \
	fi; \
	echo "$(BLUE)Running smoke tests for $(DISTRO)...$(RESET)"; \
	\
	echo "  [1/4] sindri --version"; \
	docker run --rm "$$IMG" sindri --version; \
	\
	echo "  [2/4] distro detection"; \
	DETECTED=$$(docker run --rm "$$IMG" /bin/bash -c \
		"source /docker/lib/pkg-manager.sh && detect_distro"); \
	if [ "$$DETECTED" != "$(DISTRO)" ]; then \
		echo "$(RED)FAIL: expected $(DISTRO), got $$DETECTED$(RESET)"; exit 1; \
	fi; \
	echo "$(GREEN)  ✓ Distro detection: $$DETECTED$(RESET)"; \
	\
	echo "  [3/4] architecture detection"; \
	ARCH=$$(docker run --rm "$$IMG" /bin/bash -c \
		"source /docker/lib/pkg-manager.sh && detect_arch"); \
	echo "$(GREEN)  ✓ Architecture: $$ARCH$(RESET)"; \
	\
	echo "  [4/4] starship --version"; \
	docker run --rm "$$IMG" starship --version; \
	echo "$(GREEN)  ✓ starship available$(RESET)"; \
	\
	echo "$(GREEN)$(BOLD)✓ All smoke tests passed for $(DISTRO)$(RESET)"

# Convenience aliases
.PHONY: v3-distro-test-ubuntu v3-distro-test-fedora v3-distro-test-opensuse
v3-distro-test-ubuntu:   ; @$(MAKE) v3-distro-test DISTRO=ubuntu
v3-distro-test-fedora:   ; @$(MAKE) v3-distro-test DISTRO=fedora
v3-distro-test-opensuse: ; @$(MAKE) v3-distro-test DISTRO=opensuse

# Build and test all three in sequence
.PHONY: v3-distro-test-all
v3-distro-test-all:
	@echo "$(BLUE)Building and testing all distros...$(RESET)"
	@for DISTRO in ubuntu fedora opensuse; do \
		$(MAKE) v3-docker-build DISTRO=$$DISTRO && \
		$(MAKE) v3-distro-test  DISTRO=$$DISTRO || exit 1; \
	done
	@echo "$(GREEN)$(BOLD)✓ All distros built and tested$(RESET)"

# ── pkg-manager.sh integration tests (Docker-based) ──────────────────────────
.PHONY: v3-pkg-manager-test
v3-pkg-manager-test:
	@echo "$(BLUE)Running pkg-manager.sh integration tests (Docker-based)...$(RESET)"
	@if ! command -v docker >/dev/null 2>&1; then \
		echo "$(RED)Docker is required to run these tests$(RESET)"; \
		exit 1; \
	fi
	$(V3_DIR)/tests/pkg-manager-test.sh
	@echo "$(GREEN)✓ pkg-manager.sh integration tests passed$(RESET)"
```

---

### 2.5 Updated Cache & Image Management Targets

```makefile
# ============================================================================
# V3 Updated Cache Targets
# ============================================================================

# Extended to show all distro-tagged images grouped by distro
.PHONY: v3-cache-status
v3-cache-status:
	@echo "$(BOLD)$(BLUE)╔══════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(BOLD)$(BLUE)║                     V3 Cache Status                               ║$(RESET)"
	@echo "$(BOLD)$(BLUE)╚══════════════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(BOLD)Base Images:$(RESET)"
	@docker images --filter="reference=sindri:base*" \
		--format "table {{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}" 2>/dev/null || true
	@echo ""
	@echo "$(BOLD)Local Build Images (by distro):$(RESET)"
	@for DISTRO in ubuntu fedora opensuse; do \
		echo "  [$$DISTRO]:"; \
		docker images --filter="reference=sindri:*$$DISTRO*" \
			--format "    {{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}" 2>/dev/null || true; \
	done
	@echo ""
	@echo "$(BOLD)BuildKit Cache (per distro scope):$(RESET)"
	@docker buildx du 2>/dev/null | head -20 || docker system df | grep "Build Cache" || echo "No cache data"
	@echo ""
	@echo "$(BOLD)Cargo Target:$(RESET)"
	@du -sh $(V3_DIR)/target 2>/dev/null || echo "Not built"

# Per-distro image cleanup — removes only images for one distro
# Usage: make v3-cache-clear-distro DISTRO=fedora
.PHONY: v3-cache-clear-distro
v3-cache-clear-distro:
	$(call assert_valid_distro)
	@echo "$(YELLOW)Removing all local images for distro=$(DISTRO)...$(RESET)"
	@docker images --filter="reference=sindri:*$(DISTRO)*" \
		--format "{{.ID}}" | sort -u | xargs docker rmi -f 2>/dev/null || true
	@echo "$(GREEN)✓ Removed all $(DISTRO) images$(RESET)"

# Updated v3-clean — accepts optional DISTRO for selective cleanup
# Usage: make v3-clean            (removes all non-base images, existing behaviour)
#        make v3-clean DISTRO=fedora  (removes only fedora images)
.PHONY: v3-clean
v3-clean:
	@echo "$(BLUE)Cleaning v3 Rust artifacts...$(RESET)"
	cd $(V3_DIR) && cargo clean
	@echo "$(BLUE)Cleaning Sindri repository caches...$(RESET)"
	@rm -rf ~/Library/Caches/sindri/repos 2>/dev/null || true
	@rm -rf ~/.cache/sindri/repos 2>/dev/null || true
	@if [ -n "$(DISTRO)" ] && [ "$(DISTRO)" != "." ]; then \
		echo "$(BLUE)Removing Docker images for distro=$(DISTRO)...$(RESET)"; \
		docker images --filter="reference=sindri:*$(DISTRO)*" \
			--format "{{.ID}}" | sort -u | xargs docker rmi -f 2>/dev/null || true; \
	else \
		echo "$(BLUE)Removing all non-base sindri Docker images...$(RESET)"; \
		docker images --filter="reference=sindri:*" --format "{{.ID}}\t{{.Tag}}" \
			| grep -v "base-" | awk '{print $$1}' \
			| xargs docker rmi -f 2>/dev/null || true; \
	fi
	@echo "$(BLUE)Clearing BuildKit build caches...$(RESET)"
	@docker builder prune --all --force 2>/dev/null || true
	@docker buildx prune --all --force 2>/dev/null || true
	@echo "$(GREEN)✓ v3 artifacts cleaned$(RESET)"
```

---

### 2.6 Updated `.PHONY` List and Help Text

Add to the `.PHONY` block:

```makefile
.PHONY: \
    v3-docker-build v3-docker-build-ubuntu v3-docker-build-fedora \
    v3-docker-build-opensuse v3-docker-build-all \
    v3-docker-build-dev v3-docker-build-dev-ubuntu v3-docker-build-dev-fedora \
    v3-docker-build-dev-opensuse \
    v3-docker-build-base v3-docker-build-base-ubuntu v3-docker-build-base-fedora \
    v3-docker-build-base-opensuse v3-docker-build-base-all \
    v3-distro-test v3-distro-test-ubuntu v3-distro-test-fedora \
    v3-distro-test-opensuse v3-distro-test-all \
    v3-pkg-manager-test \
    v3-cache-clear-distro \
    _v3-docker-build-impl _v3-docker-build-dev-impl _v3-docker-build-base-impl
```

Add to the `help` target (after the `v3-docker-build-fast` line):

```makefile
	@echo "$(BOLD)$(BLUE)═══ V3 Multi-Distro Build Targets ══════════════════════════════════$(RESET)"
	@echo "  v3-docker-build           - Build image (DISTRO=ubuntu|fedora|opensuse, default: ubuntu)"
	@echo "  v3-docker-build-ubuntu    - Build Ubuntu image locally"
	@echo "  v3-docker-build-fedora    - Build Fedora 41 image locally"
	@echo "  v3-docker-build-opensuse  - Build openSUSE Leap 15.6 image locally"
	@echo "  v3-docker-build-all       - Build all three distro images sequentially"
	@echo "  v3-docker-build-dev       - Build DEV image (DISTRO=..., from source)"
	@echo "  v3-docker-build-dev-ubuntu    - Build Ubuntu dev image"
	@echo "  v3-docker-build-dev-fedora    - Build Fedora dev image"
	@echo "  v3-docker-build-dev-opensuse  - Build openSUSE dev image"
	@echo "  v3-docker-build-base      - Build base image (DISTRO=...)"
	@echo "  v3-docker-build-base-all  - Build all three base images"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ V3 Distro Testing ════════════════════════════════════════════════$(RESET)"
	@echo "  v3-distro-test            - Smoke test local image (DISTRO=ubuntu|fedora|opensuse)"
	@echo "  v3-distro-test-ubuntu     - Smoke test Ubuntu local image"
	@echo "  v3-distro-test-fedora     - Smoke test Fedora local image"
	@echo "  v3-distro-test-opensuse   - Smoke test openSUSE local image"
	@echo "  v3-distro-test-all        - Build and test all three distros"
	@echo "  v3-pkg-manager-test       - Run pkg-manager.sh Docker-based integration tests"
	@echo ""
	@echo "$(BOLD)$(BLUE)═══ V3 Cache (distro-aware) ═════════════════════════════════════════$(RESET)"
	@echo "  v3-cache-status           - Show all distro images and cache usage"
	@echo "  v3-cache-clear-distro     - Remove images for one distro (DISTRO=...)"
	@echo "  v3-clean                  - Clean artifacts; add DISTRO= to target one distro"
```

---

### 2.7 Summary of Changes to Existing Targets

| Existing Target          | Change Type      | New Behaviour                                                   | Backward Compatible? |
| ------------------------ | ---------------- | --------------------------------------------------------------- | -------------------- |
| `v3-docker-build`        | Updated in-place | Accepts `DISTRO=ubuntu\|fedora\|opensuse`; defaults to `ubuntu` | ✓ Yes                |
| `v3-docker-build-latest` | Superseded       | Use `v3-docker-build-ubuntu` for local `ubuntu:latest` tag      | ✓ Yes (still works)  |
| `v3-docker-build-base`   | Updated in-place | Accepts `DISTRO=`; builds distro-specific base image            | ✓ Yes                |
| `v3-docker-build-fast`   | Superseded       | Use `v3-docker-build-dev DISTRO=ubuntu` instead                 | ✓ Yes (still works)  |
| `v3-cycle-fast`          | Updated in-place | Accepts `DISTRO=`; defaults to `ubuntu`                         | ✓ Yes                |
| `v3-cycle-clean`         | Updated in-place | Accepts `DISTRO=`; defaults to `ubuntu`                         | ✓ Yes                |
| `v3-cache-status`        | Updated in-place | Shows images grouped by distro                                  | ✓ Yes                |
| `v3-clean`               | Updated in-place | Accepts `DISTRO=`; without it behaves as before                 | ✓ Yes                |
| `v3-cache-clear-hard`    | Unchanged        | No change — nukes everything as before                          | ✓ Yes                |

> **Convention** — All distro-aware targets use the `assert_valid_distro` macro, which aborts with a clear error if an unrecognised value is passed (e.g. `DISTRO=alpine`). This prevents silent misbuilds.

---

_Sindri v3 Addendum: Image Naming & Makefile · v1.0 · March 2026 · pacphi/sindri_
