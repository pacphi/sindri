# Container Image Handling Consistency Across Providers

## Executive Summary

This document defines a comprehensive strategy to standardize image handling behavior across all container-based deployment providers in Sindri V3, ensuring consistency, flexibility, and alignment with industry best practices.

**Status**: Planning (Ready for Implementation)
**Last Updated**: 2026-01-27
**Version**: 1.0.0
**Author**: Analysis conducted via thorough codebase research and industry best practices review

### Implementation Status

| Phase | Description                              | Status      | ADR                                                                                                                                                     |
| ----- | ---------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | ADR Publication                          | 🟢 COMPLETE | [ADR-034](../../architecture/adr/034-image-handling-consistency-framework.md), [ADR-035](../../architecture/adr/035-dockerfile-path-standardization.md) |
| 1     | Add image_config Support (All Providers) | ⚪ PENDING  | ADR-034                                                                                                                                                 |
| 2     | Dockerfile Path Standardization          | ⚪ PENDING  | ADR-035                                                                                                                                                 |
| 3     | Activate Docker Build Support            | ⚪ PENDING  | ADR-034                                                                                                                                                 |
| 4     | Add Fly Image Override                   | ⚪ PENDING  | ADR-034                                                                                                                                                 |
| 5     | Documentation Updates                    | ⚪ PENDING  | ADR-034                                                                                                                                                 |
| 6     | Config Generation Template Enhancements  | ⚪ PENDING  | [ADR-028](../../architecture/adr/028-config-init-template-generation.md)                                                                                |
| 7     | Architecture Documentation Update        | ⚪ PENDING  | ADR-034, ADR-035                                                                                                                                        |

**Progress**: 1/8 phases complete (12.5%)

**Status Legend**:

- 🟢 COMPLETE - Implementation finished and merged
- 🟡 IN PROGRESS - Actively being implemented
- ⚪ PENDING - Not yet started
- 🔴 BLOCKED - Blocked by dependencies or issues

**Next Steps**:

1. Review and accept ADR-034 and ADR-035
2. Begin Phase 1 implementation (image_config support)
3. Create feature branch for implementation

### Key Findings

1. **Inconsistent Behavior**: None of the providers use the structured `image_config` field despite schema definition
2. **Unused Capabilities**: Docker provider has build functionality marked as dead code
3. **Fly Ignores Images**: Fly provider always builds from Dockerfile, ignoring `image` field entirely
4. **Path Inconsistencies**: Dockerfile paths vary (Fly: `v3/Dockerfile`, E2B/DevPod: `./Dockerfile`)
5. **Opportunity**: Industry best practices support both pre-built images AND Dockerfile builds

### Proposed Solution Summary

| Provider       | Current        | Proposed                           | Change Required               |
| -------------- | -------------- | ---------------------------------- | ----------------------------- |
| **Docker**     | Pre-built only | Pre-built OR build from Dockerfile | ⚠️ Activate build support     |
| **Fly**        | Build only     | Pre-built OR build from Dockerfile | ⚠️ Add image override         |
| **DevPod**     | Smart (both)   | Keep current behavior              | ✅ No change needed           |
| **E2B**        | Build only     | Pre-built OR build from Dockerfile | ⚠️ Add image template support |
| **Kubernetes** | Pre-built only | Keep current behavior              | ✅ No change needed           |

### Implementation Phases

| Phase | Description                                | Priority | Effort | Impact                                  |
| ----- | ------------------------------------------ | -------- | ------ | --------------------------------------- |
| 1     | Add `image_config` support (all providers) | HIGH     | Medium | All providers gain semver, verification |
| 2     | Standardize Dockerfile paths               | MEDIUM   | Low    | Consistency                             |
| 3     | Activate Docker build support              | HIGH     | Low    | Improved local dev workflow             |
| 4     | Add Fly image override                     | HIGH     | Medium | Enables CI/CD workflows                 |
| 5     | Documentation updates                      | HIGH     | Medium | User clarity                            |

---

## Table of Contents

- [Current State Analysis](#current-state-analysis)
  - [Implementation Comparison](#implementation-comparison)
  - [Code Locations](#code-locations)
  - [Key Inconsistencies](#key-inconsistencies)
- [Industry Best Practices](#industry-best-practices)
  - [Fly.io Best Practices](#flyio-best-practices)
  - [DevPod Best Practices](#devpod-best-practices)
  - [E2B Best Practices](#e2b-best-practices)
  - [Kubernetes Best Practices](#kubernetes-best-practices)
- [Proposed Consistent Behavior](#proposed-consistent-behavior)
  - [Image Resolution Priority](#image-resolution-priority)
  - [Provider-Specific Build Support](#provider-specific-build-support)
- [Detailed Recommendations](#detailed-recommendations)
  - [Docker Provider](#1-docker-provider)
  - [Fly Provider](#2-fly-provider)
  - [DevPod Provider](#3-devpod-provider)
  - [E2B Provider](#4-e2b-provider)
  - [Kubernetes Provider](#5-kubernetes-provider)
- [Implementation Plan](#implementation-plan)
  - [Phase 0: ADR Publication](#phase-0-adr-publication)
  - [Phase 1: Add image_config Support](#phase-1-add-image_config-support-all-providers)
  - [Phase 2: Dockerfile Path Standardization](#phase-2-dockerfile-path-standardization)
  - [Phase 3: Activate Docker Build Support](#phase-3-activate-docker-build-support)
  - [Phase 4: Add Fly Image Override](#phase-4-add-fly-image-override)
  - [Phase 5: Documentation Updates](#phase-5-documentation-updates)
  - [Phase 6 Config Generation Template Enhancements](#phase-6-config-generation-template-enhancements) <!-- markdownlint-disable-line MD051 -->
  - [Phase 7: Architecture Documentation Update](#phase-7-architecture-documentation-update)
- [File Changes Summary](#file-changes-summary)
- [Testing Strategy](#testing-strategy)
- [Success Criteria](#success-criteria)
- [References](#references)

---

## Current State Analysis

### Implementation Comparison

Based on thorough analysis of provider implementations in `v3/crates/sindri-providers/src/*.rs`:

#### **Docker Provider** (`docker.rs`)

**Image Field Handling:**

- Reads `deployment.image` from config (line 725): `file.deployment.image.as_deref().unwrap_or("sindri:latest")`
- Uses value directly in docker-compose template as `image: {{ image }}`
- Default fallback: `"sindri:latest"`

**Image Config Handling:**

- ❌ Does NOT use `image_config` at all
- ❌ No structured image configuration support

**Dockerfile Build Support:**

- ❌ Has `build_image()` method (lines 291-323) marked as `#[allow(dead_code)]` - **NOT CURRENTLY USED**
- Method supports: tag, dockerfile path, context directory, force rebuild flag
- Uses `docker build -t <tag> -f <dockerfile> <context>` command
- **Current Behavior**: Expects pre-built image, does NOT auto-build from Dockerfile

**Key Finding**: Unused build capability exists and could be activated.

---

#### **Fly Provider** (`fly.rs`)

**Image Field Handling:**

- Reads `deployment.image` from config (line 109): `file.deployment.image.as_deref().unwrap_or("sindri:latest")`
- Stored in `FlyDeployConfig.image` but **NOT directly used** in fly.toml

**Image Config Handling:**

- ❌ Does NOT use `image_config`

**Dockerfile Build Support:**

- ✅ **ALWAYS BUILDS FROM DOCKERFILE** via `flyctl deploy` (line 338-360)
- Uses `flyctl_deploy()` method with `--ha=false --wait-timeout 600`
- Optional `--no-cache` flag when `rebuild=true` (force flag)
- fly.toml template hardcodes: `dockerfile = "v3/Dockerfile"` (line 11)
- Build happens server-side during `flyctl deploy`
- No local Docker build required

**Key Finding**: Image field is completely ignored; always builds from Dockerfile.

---

#### **DevPod Provider** (`devpod.rs`)

**Image Field Handling:**

- DevPod uses **devcontainer.json** which has `"image": "{{ image }}"`
- Image is set via `generate_devcontainer()` method (lines 374-398)
- Can use explicit image tag OR build from Dockerfile

**Image Config Handling:**

- ❌ Does NOT use `image_config`

**Dockerfile Build Support:**

- ✅ **ACTIVELY USED** via `build_image()` method (lines 232-254)
- Builds locally using `docker build -t <tag> -f <dockerfile> <context>`
- Smart behavior in `prepare_image()` (lines 312-371):
  - **Local K8s (kind/k3d)**: Builds and loads into cluster
  - **Cloud providers (AWS/GCP/Azure)**: Builds, pushes to registry
  - **Docker provider**: Uses Dockerfile directly in devcontainer
- Requires `buildRepository` for cloud providers (line 347-352)
- Supports Docker login with credentials from env/files

**Key Finding**: ALREADY OPTIMAL - supports both pre-built images AND smart Dockerfile builds.

---

#### **E2B Provider** (`e2b.rs`)

**Image Field Handling:**

- ❌ Does NOT use `deployment.image` field

**Image Config Handling:**

- ❌ Does NOT use `image_config`

**Dockerfile Build Support:**

- ✅ **MANDATORY** via `build_template()` method (lines 308-361)
- Reads Dockerfile from `base_dir.join("Dockerfile")` (line 209)
- Generates E2B-specific Dockerfile (`e2b.Dockerfile`) with additional env vars (lines 201-271)
- Adds E2B-specific configuration:
  - `ENV E2B_PROVIDER=true`
  - `ENV INSTALL_PROFILE`, `CUSTOM_EXTENSIONS`, etc.
  - Sets `WORKDIR /alt/home/developer/workspace`
  - Sets `USER developer`
- Builds template using E2B CLI: `e2b template build --name <alias> --dockerfile e2b.Dockerfile`
- Build happens remotely on E2B infrastructure (2-5 minutes)
- Build triggered when:
  - `buildOnDeploy=true`
  - `force=true`
  - Template doesn't exist
  - `reuseTemplate=false`

**Key Finding**: Always requires Dockerfile; no pre-built image support.

---

#### **Kubernetes Provider** (`kubernetes.rs`)

**Image Field Handling:**

- Reads `deployment.image` from config (line 166): `file.deployment.image.as_deref().unwrap_or("sindri:latest")`
- Uses directly in k8s-deployment.yaml: `image: {{ image }}`
- Default fallback: `"sindri:latest"`

**Image Config Handling:**

- ❌ Does NOT use `image_config`

**Dockerfile Build Support:**

- ❌ **NO LOCAL BUILD** - image pull only
- Has `load_image_to_cluster()` method (lines 213-250) for local clusters:
  - **kind**: `kind load docker-image <image> --name <cluster>`
  - **k3d**: `k3d image import <image> -c <cluster>`
  - Assumes image already exists locally
- For remote clusters: Image must be pushed to accessible registry
- Creates `ImagePullSecret` for private registries (lines 469-540)
- Detects registry from image name and uses `~/.docker/config.json`

**Key Finding**: Correctly enforces pre-built images only (aligned with K8s best practices).

---

### Code Locations

#### **Deploy Methods**

```
- Docker:      v3/crates/sindri-providers/src/docker.rs:446-543
- Fly:         v3/crates/sindri-providers/src/fly.rs:416-491
- DevPod:      v3/crates/sindri-providers/src/devpod.rs:633-702
- E2B:         v3/crates/sindri-providers/src/e2b.rs:539-689
- Kubernetes:  v3/crates/sindri-providers/src/kubernetes.rs:626-767
```

#### **Image Resolution Logic**

```
- Docker:      Line 725 (plan method)
- Fly:         Line 109 (get_fly_config)
- DevPod:      Lines 312-371 (prepare_image)
- E2B:         Lines 201-271 (generate_e2b_dockerfile)
- Kubernetes:  Line 166 (get_k8s_config)
```

#### **Dockerfile Detection/Build Logic**

```
- Docker:      Lines 291-323 (build_image - UNUSED)
- Fly:         Lines 338-361 (flyctl_deploy - hardcoded path)
- DevPod:      Lines 232-254 (build_image), 318 (Dockerfile detection)
- E2B:         Lines 209-219 (Dockerfile detection), 308-361 (build_template)
- Kubernetes:  Lines 213-250 (load_image_to_cluster - no build)
```

#### **Template Generation**

```
- Docker:      templates/docker-compose.yml.tera (line 7: image: {{ image }})
- Fly:         templates/fly.toml.tera (line 11: dockerfile = "v3/Dockerfile")
- DevPod:      templates/devcontainer.json.tera (line 3: "image": "{{ image }}")
- E2B:         templates/e2b.toml.tera (line 7: dockerfile = "e2b.Dockerfile")
- Kubernetes:  templates/k8s-deployment.yaml.tera (line 25: image: {{ image }})
```

---

### Key Inconsistencies

1. **Image Config Usage**: NONE of the providers use the structured `image_config` field despite it being defined in the schema
2. **Default Image**: Docker and Kubernetes use `"sindri:latest"` default, but Fly stores it without using it
3. **Build Support Discrepancy**:
   - Docker has unused build capability
   - Fly always builds but ignores image field
   - DevPod conditionally builds based on provider type
   - E2B always builds and transforms Dockerfile
   - Kubernetes never builds
4. **Dockerfile Path Inconsistency**:
   - Fly: Hardcoded `v3/Dockerfile`
   - DevPod: Expects `Dockerfile` at base_dir (project root parent)
   - E2B: Expects `Dockerfile` at base_dir (current working directory)
5. **Image Field Relevance**:
   - Docker/Kubernetes: Critical (specifies which image to use)
   - DevPod: Conditional (only used if not building)
   - Fly: Ignored (always builds from Dockerfile)
   - E2B: Not used at all (always builds from Dockerfile)

---

## Industry Best Practices

### Fly.io Best Practices

**Sources:**

- [Deploy with a Dockerfile · Fly Docs](https://fly.io/docs/languages-and-frameworks/dockerfile/)
- [fly deploy · Fly Docs](https://fly.io/docs/flyctl/deploy/)
- [deploy from docker image vs local docker file - Fly.io Community](https://community.fly.io/t/deploy-from-docker-image-vs-local-docker-file/24349)

**Key Insights:**

1. **Priority Order**: If an image is specified, either with the `--image` option or in the `[build]` section of fly.toml, Fly.io uses that image, **regardless of the presence of a Dockerfile** in the working directory.

   Fallback order:
   - Pre-built image (via `--image` flag or fly.toml `[build]` section)
   - `[build]` section in fly.toml
   - `--dockerfile` flag path
   - Local Dockerfile in working directory

2. **Optimization Strategy**: "Build once, deploy many times"
   - Use `fly deploy --build-only --push` to build and push image
   - Then use `fly deploy --image <registry/image:tag>` to deploy without rebuilding
   - Significantly faster deployments when image doesn't change

3. **Best Practices**:
   - Use multi-stage builds to reduce image size
   - Use `fly secrets` for runtime secrets
   - Choose minimal base images (Alpine, slim variants)
   - For debugging, use `fly deploy --build-local` to build with local Docker daemon

**Implication**: Fly.io **explicitly supports both** pre-built images AND Dockerfile builds, with image taking precedence. Our implementation should align with this.

---

### DevPod Best Practices

**Sources:**

- [Prebuild a Workspace | DevPod docs](https://devpod.sh/docs/developing-in-workspaces/prebuild-a-workspace)
- [How DevPod Builds Workspaces | DevPod docs](https://devpod.sh/docs/how-it-works/building-workspaces)
- [Speeding up Dev Containers with Pre-built Images](https://www.daytona.io/dotfiles/speeding-up-dev-containers-with-pre-built-images)

**Key Insights:**

1. **Prebuild Strategy**: DevPod generates a hash in the form of `devpod-HASH` from the devcontainer.json configuration and uses this as a tag for the created docker image. When starting a workspace, DevPod searches for this tag in configured registries and uses it instead of building if found.

2. **Performance Benefits**: Pre-built images reduce startup time dramatically:
   - Without prebuilds: 4-5 minutes (building complex environments)
   - With prebuilds: <10 seconds (image pull)
   - Initial pull (1.8GB image on 1 Gbps): <1 minute

3. **Automatic Local Caching**: If you use the Docker provider, DevPod builds your dev container image directly onto your machine. The next time you bring up your workspace using the Docker provider, it won't need to re-build the image.

4. **Team Consistency**: Maintaining a base image centrally ensures consistent environments across all projects and team members. Updates propagate automatically without repetitive config management.

**Implication**: DevPod's smart caching and prebuild system demonstrates the value of supporting both pre-built images (for speed/consistency) AND local builds (for customization).

---

### E2B Best Practices

**Sources:**

- [Sandbox Template - E2B](https://e2b.dev/docs/sandbox-template)
- [SDK Reference - E2B CLI](https://e2b.dev/docs/sdk-reference/cli/v1.0.9/template)

**Key Insights:**

1. **Template Building**: E2B converts Dockerfiles into sandbox templates with template IDs. The build command looks for `e2b.Dockerfile` or `Dockerfile` in the root directory by default.

2. **Image Requirements**: Only Debian-based images (Debian, Ubuntu, or E2B images) are supported.

3. **Build Options**:
   - `-d, --dockerfile`: Specify path to Dockerfile
   - `-n, --name`: Specify sandbox template name (lowercase, letters/numbers/dashes/underscores)
   - `-c, --cmd`: Specify command executed when sandbox starts

4. **Build System 2.0**: E2B's newer Build System 2.0 offers a simpler approach where you don't need Dockerfiles, extra config files, or manual CLI build commands - you just write code.

**Implication**: E2B's template system is Dockerfile-centric, but future versions may move away from this. Consider adding pre-built image support for flexibility.

---

### Kubernetes Best Practices

**Sources:**

- [Kubernetes best practices: Small Container Images | Google Cloud](https://cloud.google.com/blog/products/containers-kubernetes/kubernetes-best-practices-how-and-why-to-build-small-container-images)
- [Building Docker images in Kubernetes | Snyk](https://snyk.io/blog/building-docker-images-kubernetes/)
- [27+ Kubernetes Deployment Best Practices | Zeet](https://zeet.co/blog/kubernetes-deployment-best-practices)

**Key Insights:**

1. **Build Separation**: Container images **should NOT be rebuilt** as they move through different pipeline stages. Rebuilding can introduce differences that may cause production failures or accidentally add untested code. **Best practice: Build once and promote along environments.**

2. **Immutable Deployments**: Use **unique image digests** as tags to ensure containers always use the same version. Avoid the `latest` tag in production environments as it makes determining the image version difficult.

3. **Trusted Sources**: Container images should come from trusted sources like official repositories or verified vendors. Organizations often prefer to approve containers first and use internal registries, allowing only images from these registries to be deployed in clusters.

4. **Image Size Optimization**:
   - Alpine images can be 10X smaller than base images
   - Smaller images pull faster, reducing deployment time
   - Use multi-stage builds to exclude build dependencies from final image
   - Beginner mistake: Using base images with 80% unused packages/libraries

5. **CI/CD Integration**: Container images should be built in CI/CD pipelines, NOT during Kubernetes deployment. The deployment phase should only pull and run pre-built, tested images.

**Implication**: Kubernetes provider should NEVER build images during deployment - this aligns with current implementation and should be preserved.

---

## Proposed Consistent Behavior

### Image Resolution Priority

All providers should follow this **standardized priority order**:

```
1. deployment.image_config.digest     → Immutable, production-safe
2. deployment.image_config.tag_override → Explicit tag (e.g., v3.0.0-beta.1)
3. deployment.image_config.version    → Semantic version constraint (e.g., ^3.0.0)
4. deployment.image                   → Legacy field, full image reference
5. Local Dockerfile                   → Build on-demand (provider-dependent)
6. Default fallback                   → ghcr.io/pacphi/sindri:latest
```

This order balances:

- **Production safety**: Digest-pinned images are immutable
- **Flexibility**: Tag override for specific versions
- **Convenience**: Semver constraints for automatic updates
- **Backward compatibility**: Legacy `image` field still works
- **Local development**: Dockerfile builds when no image specified
- **Sensible defaults**: Official Sindri image as last resort

---

### Provider-Specific Build Support

| Provider       | Build from Dockerfile? | When to Build?                            | Override with image/image_config?         |
| -------------- | ---------------------- | ----------------------------------------- | ----------------------------------------- |
| **Docker**     | ✅ **YES (activate)**  | When no image specified OR `force=true`   | ✅ Yes - skip build if image provided     |
| **Fly**        | ✅ YES (keep)          | When no image specified OR `force=true`   | ✅ **YES (NEW)** - use image if provided  |
| **DevPod**     | ✅ YES (keep)          | Smart: cloud=build+push, local=dockerfile | ✅ Yes (already works)                    |
| **E2B**        | ✅ YES (keep)          | Always (template system)                  | ✅ **YES (NEW)** - use image for template |
| **Kubernetes** | ❌ NO (keep)           | Never - CI/CD builds only                 | ✅ Yes (already works)                    |

**Rationale**:

- **Docker**: Activate unused build capability for better local dev workflow
- **Fly**: Add image override to support CI/CD workflows (build in CI, deploy via Sindri)
- **DevPod**: Already optimal, no changes needed
- **E2B**: Consider image-based templates (if E2B CLI supports it)
- **Kubernetes**: Correctly enforces pre-built images per best practices

---

## Detailed Recommendations

### 1. Docker Provider

**Current Issue**: Has `build_image()` method marked `#[allow(dead_code)]` - not used.

**Proposed Behavior**:

```yaml
# Scenario A: Pre-built image (skip build)
deployment:
  provider: docker
  image: ghcr.io/myorg/app:v1.0.0  # Uses this, no build

# Scenario B: Build from Dockerfile
deployment:
  provider: docker
  # No image specified - builds from ./Dockerfile

# Scenario C: Force rebuild
deployment:
  provider: docker
  image: myapp:latest
# sindri deploy --force  → Rebuilds even if image exists
```

**Implementation**:

1. **Activate `build_image()` method** - Remove `#[allow(dead_code)]` annotation

2. **Add build logic in `deploy()` method**:

   ```rust
   // docker.rs:446 - in deploy() method
   let image = if let Some(img) = config.inner().deployment.image.as_deref() {
       // Use specified image
       img.to_string()
   } else if let Some(dockerfile) = find_dockerfile() {
       // Build from Dockerfile
       let tag = format!("{}:latest", config.name());
       self.build_image(&tag, dockerfile.to_str().unwrap(), ".", opts.force)?;
       tag
   } else {
       // Default
       "ghcr.io/pacphi/sindri:latest".to_string()
   };
   ```

3. **Add Dockerfile detection helper**:
   ```rust
   fn find_dockerfile() -> Option<PathBuf> {
       let candidates = vec!["./Dockerfile", "./v3/Dockerfile", "./deploy/Dockerfile"];
       candidates.iter()
           .map(PathBuf::from)
           .find(|p| p.exists())
   }
   ```

**Benefits**:

- Local dev workflow without pushing images
- Consistent with DevPod and E2B
- Faster iteration cycles
- Supports both pre-built and local build scenarios

**Files Modified**:

- `v3/crates/sindri-providers/src/docker.rs`

---

### 2. Fly Provider

**Current Issue**: Always builds from Dockerfile, ignores `image` field entirely.

**Proposed Behavior**:

```yaml
# Scenario A: Pre-built image (SKIP build)
deployment:
  provider: fly
  image: ghcr.io/myorg/app:v1.0.0  # Fly uses this, skips Dockerfile

# Scenario B: Build from Dockerfile (current behavior)
deployment:
  provider: fly
  # No image - Fly builds from v3/Dockerfile
```

**Implementation**:

1. **Add `flyctl_deploy_image()` method**:

   ```rust
   async fn flyctl_deploy_image(
       &self,
       image: &str,
       config: &FlyDeployConfig,
       _rebuild: bool,
   ) -> Result<()> {
       let mut args = vec!["deploy", "--image", image, "--ha=false"];

       if let Some(org) = &config.organization {
           args.push("--org");
           args.push(org);
       }

       let status = Command::new("flyctl")
           .args(&args)
           .stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit())
           .status()
           .await?;

       if !status.success() {
           return Err(anyhow!("flyctl deploy failed"));
       }

       Ok(())
   }
   ```

2. **Update `deploy()` to check for pre-built image**:

   ```rust
   async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
       let fly_config = self.get_fly_config(config);

       // Check for pre-built image
       let use_prebuilt = config.inner().deployment.image.is_some()
                          || config.inner().deployment.image_config.is_some();

       if use_prebuilt {
           // Resolve image (use shared resolver)
           let image = config.resolve_image().await?;
           info!("Deploying pre-built image: {}", image);

           // Deploy using pre-built image
           self.flyctl_deploy_image(&image, &fly_config, opts.force).await?;
       } else {
           // Current behavior: build from Dockerfile
           info!("Building from Dockerfile: v3/Dockerfile");
           self.flyctl_deploy(&fly_config, opts.force).await?;
       }

       // ... rest of deploy logic
   }
   ```

**Benefits**:

- Faster deploys when using pre-built images (no remote build - 2-5 minutes saved)
- Aligns with Fly.io best practice: "build once, deploy many times"
- Supports CI/CD workflows (build in CI, deploy via Sindri)
- Maintains backward compatibility (no image = build from Dockerfile)

**Files Modified**:

- `v3/crates/sindri-providers/src/fly.rs`

---

### 3. DevPod Provider

**Current Status**: ✅ **ALREADY OPTIMAL** - no changes needed

**Current Behavior**:

- Uses pre-built image when specified
- Builds from Dockerfile for cloud providers (with registry push)
- Uses Dockerfile directly in devcontainer for Docker provider
- Auto-caches builds locally

**Rationale**: DevPod already follows all best practices and supports both scenarios intelligently.

---

### 4. E2B Provider

**Current Issue**: Always requires Dockerfile, cannot use pre-built images.

**Proposed Behavior**:

```yaml
# Scenario A: Pre-built image template (NEW)
deployment:
  provider: e2b
  image: ghcr.io/myorg/sindri-custom:v1.0.0  # E2B creates template from this

# Scenario B: Build template from Dockerfile (current)
deployment:
  provider: e2b
  # Builds template from ./Dockerfile
```

**Implementation** (conditional on E2B CLI support):

1. **Research E2B CLI capabilities**:
   - Check if `e2b template build` supports `--base-image` or similar flag
   - Alternative: Use `--from-image` or registry pull + template creation

2. **Update `build_template()` to support images**:

   ```rust
   async fn build_template(
       &self,
       config: &SindriConfig,
       e2b_config: &E2bDeployConfig,
       _force: bool,
   ) -> Result<()> {
       info!("Building E2B template: {}", &e2b_config.template_alias);

       // Check for pre-built image
       let use_image = config.inner().deployment.image.is_some()
                       || config.inner().deployment.image_config.is_some();

       if use_image {
           let image = config.resolve_image().await?;
           info!("Creating E2B template from pre-built image: {}", image);

           // Use E2B CLI with image (if supported)
           // e2b template build --from-image <image> --name <alias>
           // OR pull image and convert to template

           todo!("Implement image-based template creation");
       } else {
           // Current behavior: generate e2b.Dockerfile
           self.generate_e2b_dockerfile(config, e2b_config, &self.output_dir)?;
           self.generate_e2b_toml(config, e2b_config, &self.output_dir)?;

           // Build template using E2B CLI
           // ... existing build logic
       }
   }
   ```

**Benefits**:

- Reuse existing Sindri images
- Faster template builds (no Dockerfile transformation needed)
- Consistency with other providers

**Research Needed**:

- ⚠️ Verify E2B CLI supports image-based template creation
- Check E2B API for programmatic template creation from images

**Files Modified** (if supported):

- `v3/crates/sindri-providers/src/e2b.rs`

---

### 5. Kubernetes Provider

**Current Status**: ✅ **ALREADY CORRECT** - no changes needed

**Current Behavior**: Image pull only - **CORRECT per K8s best practices**.

**Rationale**: Kubernetes should NOT build images during deployment. Images should be built in CI/CD pipelines and pulled from registries.

For local clusters (kind/k3d), current behavior is correct:

- Expects pre-built image locally
- Uses `kind load` / `k3d import` to load into cluster

---

## Implementation Plan

### Phase 0: ADR Publication

**Priority**: HIGH
**Effort**: Low
**Impact**: Document architectural decisions for review and approval
**Status**: 🟢 COMPLETE

**Objective**: Publish Architecture Decision Records documenting the key decisions for image handling consistency.

**ADRs Created**:

1. **ADR-034: Image Handling Consistency Framework**
   - File: `v3/docs/architecture/adr/034-image-handling-consistency-framework.md`
   - Status: Proposed
   - Content:
     - Current inconsistencies across providers
     - Industry best practices analysis
     - Standardized image resolution priority (6-level hierarchy)
     - Provider-specific build support matrix
     - Implementation phases overview

2. **ADR-035: Dockerfile Path Standardization**
   - File: `v3/docs/architecture/adr/035-dockerfile-path-standardization.md`
   - Status: Proposed
   - Content:
     - Current path inconsistencies
     - Standard search order: `./Dockerfile` → `./v3/Dockerfile` → `./deploy/Dockerfile`
     - Shared `find_dockerfile()` utility
     - Error message standards
     - Backward compatibility strategy

**Next Steps**:

1. Review ADR-034 and ADR-035 with core team
2. Gather feedback and iterate if needed
3. Accept ADRs before proceeding with implementation
4. Update ADR README with new entries

**Success Criteria**:

- ✅ ADR-034 published and documented
- ✅ ADR-035 published and documented
- ⏳ ADRs reviewed by core team
- ⏳ ADRs accepted (status changed to "Accepted")

---

### Phase 1: Add `image_config` Support (All Providers)

**Priority**: HIGH
**Effort**: Medium
**Impact**: All providers gain semantic versioning, signature verification, provenance attestation

**Objective**: Enable all providers to use the structured `image_config` field instead of just the legacy `image` field.

**Implementation**:

1. **Update all provider `deploy()` methods** to use `config.resolve_image().await?` instead of reading `deployment.image` directly:

   ```rust
   // Current (all providers except DevPod):
   let image = file.deployment.image.as_deref().unwrap_or("sindri:latest");

   // New:
   let image = config.resolve_image().await?;
   ```

2. **Files to modify**:
   - `v3/crates/sindri-providers/src/docker.rs` (line 725 in `plan()`)
   - `v3/crates/sindri-providers/src/fly.rs` (line 109 in `get_fly_config()`)
   - `v3/crates/sindri-providers/src/kubernetes.rs` (line 166 in `get_k8s_config()`)
   - `v3/crates/sindri-providers/src/e2b.rs` (check if image field is used)

3. **Leverage existing resolver**: The `resolve_image()` method in `v3/crates/sindri-core/src/config/loader.rs:185-297` already implements the full priority chain.

**Testing**:

```yaml
# Test configuration
deployment:
  provider: docker # Test with each provider
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
    verify_signature: true
    verify_provenance: true
```

**Expected Behavior**:

- Image resolution follows priority order (digest > tag_override > version > legacy image > Dockerfile > default)
- Signature verification runs if configured
- Provenance verification runs if configured

**Success Criteria**:

- All providers resolve images using `resolve_image()`
- Semantic versioning works (`^3.0.0` resolves to latest 3.x.x)
- Image verification works when enabled
- Legacy `image` field still works (backward compatibility)

---

### Phase 2: Dockerfile Path Standardization

**Priority**: MEDIUM
**Effort**: Low
**Impact**: Consistency across providers

**Objective**: Unify Dockerfile path detection across all providers.

**Current Inconsistencies**:

- Fly: Hardcoded `v3/Dockerfile`
- DevPod: `./Dockerfile` (base_dir = project root parent)
- E2B: `./Dockerfile` (base_dir = current working directory)

**Proposed Standard**:

```
Search order:
1. ./Dockerfile               # Project root (default)
2. ./v3/Dockerfile            # Sindri v3 specific (fallback)
3. ./deploy/Dockerfile        # Deploy-specific (fallback)
```

**Implementation**:

1. **Create shared helper function** in `v3/crates/sindri-providers/src/utils.rs`:

   ```rust
   /// Find Dockerfile using standard search paths
   pub fn find_dockerfile() -> Option<PathBuf> {
       let candidates = vec![
           "./Dockerfile",
           "./v3/Dockerfile",
           "./deploy/Dockerfile",
       ];

       candidates.iter()
           .map(PathBuf::from)
           .find(|p| p.exists())
   }
   ```

2. **Update providers to use shared function**:
   - Docker: Use `find_dockerfile()` in build logic
   - Fly: Replace hardcoded `v3/Dockerfile` in template with dynamic path
   - DevPod: Use `find_dockerfile()` instead of `base_dir.join("Dockerfile")`
   - E2B: Use `find_dockerfile()` instead of `base_dir.join("Dockerfile")`

3. **Update Fly template** (`v3/crates/sindri-providers/src/templates/fly.toml.tera`):

   ```toml
   # Before:
   [build]
   dockerfile = "v3/Dockerfile"

   # After:
   [build]
   dockerfile = "{{ dockerfile_path }}"
   ```

**Files Modified**:

- `v3/crates/sindri-providers/src/utils.rs` (new helper)
- `v3/crates/sindri-providers/src/docker.rs`
- `v3/crates/sindri-providers/src/fly.rs`
- `v3/crates/sindri-providers/src/devpod.rs`
- `v3/crates/sindri-providers/src/e2b.rs`
- `v3/crates/sindri-providers/src/templates/fly.toml.tera`

**Success Criteria**:

- All providers search same paths in same order
- Dockerfile at `./Dockerfile` works for all providers
- Backward compatibility: `v3/Dockerfile` still works for Fly

---

### Phase 3: Activate Docker Build Support

**Priority**: HIGH
**Effort**: Low
**Impact**: Improved local development workflow

**Objective**: Enable Docker provider to build images from Dockerfile when no image is specified.

**Implementation**:

1. **Remove dead code annotation** (`v3/crates/sindri-providers/src/docker.rs:291`):

   ```rust
   // Before:
   #[allow(dead_code)]
   fn build_image(...)

   // After:
   fn build_image(...)
   ```

2. **Add build logic in `deploy()` method** (around line 446):

   ```rust
   async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
       let file = config.inner();
       let name = &file.name;

       // Resolve image: use specified OR build OR default
       let image = if file.deployment.image.is_some() || file.deployment.image_config.is_some() {
           // Use resolve_image() for full image_config support
           config.resolve_image().await?
       } else if let Some(dockerfile) = find_dockerfile() {
           // Build from Dockerfile
           let tag = format!("{}:latest", name);
           info!("No image specified, building from {}", dockerfile.display());
           self.build_image(&tag, dockerfile.to_str().unwrap(), ".", opts.force)?;
           tag
       } else {
           // Default fallback
           info!("No image or Dockerfile found, using default");
           "ghcr.io/pacphi/sindri:latest".to_string()
       };

       // Continue with existing deploy logic using resolved image
       // ...
   }
   ```

3. **Update `build_image()` to return `Result<String>`** (image tag):

   ```rust
   fn build_image(
       &self,
       tag: &str,
       dockerfile: &str,
       context: &str,
       force: bool,
   ) -> Result<String> {
       info!("Building Docker image: {}", tag);

       let mut args = vec!["build", "-t", tag, "-f", dockerfile];
       if force {
           args.push("--no-cache");
       }
       args.push(context);

       let status = Command::new("docker")
           .args(&args)
           .status()?;

       if !status.success() {
           return Err(anyhow!("Docker build failed"));
       }

       Ok(tag.to_string())
   }
   ```

**Testing**:

```bash
# Test A: Pre-built image (skip build)
cat > sindri.yaml << EOF
deployment:
  provider: docker
  image: ghcr.io/pacphi/sindri:v3.0.0
EOF
sindri deploy  # Should pull image, not build

# Test B: Build from Dockerfile
cat > sindri.yaml << EOF
deployment:
  provider: docker
EOF
echo "FROM ubuntu:22.04" > Dockerfile
sindri deploy  # Should build from Dockerfile

# Test C: Force rebuild
sindri deploy --force  # Should rebuild even if image exists
```

**Files Modified**:

- `v3/crates/sindri-providers/src/docker.rs`

**Success Criteria**:

- Pre-built images skip build
- Dockerfile builds when no image specified
- `--force` flag triggers rebuild
- Build failures are properly reported

---

### Phase 4: Add Fly Image Override

**Priority**: HIGH
**Effort**: Medium
**Impact**: Enables CI/CD workflows (build in CI, deploy via Sindri)

**Objective**: Allow Fly provider to deploy pre-built images instead of always building from Dockerfile.

**Implementation**:

1. **Add `flyctl_deploy_image()` method** (`v3/crates/sindri-providers/src/fly.rs`):

   ```rust
   /// Deploy using a pre-built image (skip Dockerfile build)
   async fn flyctl_deploy_image(
       &self,
       image: &str,
       config: &FlyDeployConfig,
   ) -> Result<()> {
       info!("Deploying pre-built image to Fly.io: {}", image);

       let mut args = vec![
           "deploy",
           "--image", image,
           "--ha=false",
           "--wait-timeout", "600",
       ];

       if let Some(org) = &config.organization {
           args.push("--org");
           args.push(org);
       }

       let status = Command::new("flyctl")
           .args(&args)
           .stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit())
           .status()
           .await?;

       if !status.success() {
           return Err(anyhow!("flyctl deploy failed"));
       }

       Ok(())
   }
   ```

2. **Update `deploy()` to check for pre-built image** (around line 416):

   ```rust
   async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
       let fly_config = self.get_fly_config(config);
       let name = config.name();

       // Check for pre-built image
       let file = config.inner();
       let use_prebuilt = file.deployment.image.is_some()
                          || file.deployment.image_config.is_some();

       if use_prebuilt {
           // Deploy using pre-built image
           let image = config.resolve_image().await?;
           info!("Using pre-built image: {}", image);
           self.flyctl_deploy_image(&image, &fly_config).await?;
       } else {
           // Current behavior: build from Dockerfile
           info!("No image specified, building from Dockerfile");
           self.flyctl_deploy(&fly_config, opts.force).await?;
       }

       // Continue with existing connection/status logic
       // ...
   }
   ```

3. **Update fly.toml template** (conditional):
   - If using Dockerfile: Include `[build]` section
   - If using pre-built image: Omit `[build]` section (Fly ignores it when using `--image`)

**Testing**:

```bash
# Test A: Pre-built image (skip build)
cat > sindri.yaml << EOF
deployment:
  provider: fly
  image: ghcr.io/pacphi/sindri:v3.0.0
EOF
sindri deploy  # Should deploy image, skip Dockerfile build

# Test B: Build from Dockerfile (current behavior)
cat > sindri.yaml << EOF
deployment:
  provider: fly
EOF
sindri deploy  # Should build from v3/Dockerfile

# Test C: CI/CD workflow
# In CI: docker build -t ghcr.io/myorg/app:v1.0.0 && docker push ...
# Then deploy:
cat > sindri.yaml << EOF
deployment:
  provider: fly
  image: ghcr.io/myorg/app:v1.0.0
EOF
sindri deploy  # Should deploy without rebuilding
```

**Files Modified**:

- `v3/crates/sindri-providers/src/fly.rs`

**Success Criteria**:

- Pre-built images deploy without building (saves 2-5 minutes)
- Dockerfile builds when no image specified (backward compatible)
- `flyctl deploy --image` is used correctly
- Deployment succeeds on Fly.io

---

### Phase 5: Documentation Updates

**Priority**: HIGH
**Effort**: Medium
**Impact**: User clarity and adoption

**Objective**: Document new image handling behavior and resolution priority across all providers.

**Files to Update**:

1. **`v3/docs/CONFIGURATION.md`**
   - Add "Image vs Dockerfile Priority" section after line 170
   - Add build support table for each provider
   - Update `deployment.image` description to clarify it's respected by all providers
   - Update `deployment.image_config` to explain resolution priority

2. **`v3/docs/IMAGE_MANAGEMENT.md`**
   - Add section on image resolution priority (after line 265)
   - Document provider-specific build behavior
   - Add examples for each scenario (pre-built, Dockerfile, force rebuild)

3. **`v3/docs/providers/DOCKER.md`**
   - Add section on Dockerfile build support
   - Document when builds happen vs when images are pulled
   - Add examples

4. **`v3/docs/providers/FLY.md`**
   - Add section on image override capability
   - Document CI/CD workflow (build in CI, deploy via Sindri)
   - Add examples comparing build vs pre-built scenarios

5. **`v3/docs/providers/DEVPOD.md`**
   - Document current smart build behavior (already optimal)
   - Explain prebuild strategy and caching

6. **`v3/docs/providers/E2B.md`**
   - Update if image-based templates are supported
   - Otherwise, document Dockerfile-only requirement

7. **`v3/docs/providers/KUBERNETES.md`**
   - Reinforce that K8s does NOT build images (best practice)
   - Document local cluster image loading (kind/k3d)

**New Section for CONFIGURATION.md**:

````markdown
## Image vs Dockerfile Priority

Sindri follows this priority order for image resolution across all providers:

1. **image_config.digest** - Immutable production deployments (SHA256 digest)
2. **image_config.tag_override** - Explicit tag override (e.g., `v3.0.0-beta.1`)
3. **image_config.version** - Semantic version constraint (e.g., `^3.0.0`)
4. **image** - Legacy full image reference (e.g., `ghcr.io/org/app:v1.0.0`)
5. **Local Dockerfile** - Build on-demand (provider-dependent)
6. **Default** - `ghcr.io/pacphi/sindri:latest`

### Build Support by Provider

| Provider   | Builds from Dockerfile? | When?                       | Override with Image?        |
| ---------- | ----------------------- | --------------------------- | --------------------------- |
| Docker     | ✅ Yes                  | When no image specified     | ✅ Yes                      |
| Fly        | ✅ Yes                  | When no image specified     | ✅ Yes                      |
| DevPod     | ✅ Yes                  | Cloud providers (with push) | ✅ Yes                      |
| E2B        | ✅ Yes                  | Always (template system)    | ❌ No (Dockerfile required) |
| Kubernetes | ❌ No                   | Never - use CI/CD           | ✅ Yes (image required)     |

### Examples

#### Pre-built Image (All Providers)

```yaml
deployment:
  provider: docker # or fly, devpod, kubernetes
  image: ghcr.io/myorg/app:v1.0.0
# Deploys this image directly, no build
```
````

#### Semantic Versioning (All Providers)

```yaml
deployment:
  provider: docker # or fly, devpod, kubernetes
  image_config:
    registry: ghcr.io/myorg/app
    version: "^1.0.0" # Resolves to latest 1.x.x
    verify_signature: true
# Resolves version, verifies signature, then deploys
```

#### Build from Dockerfile (Docker, Fly, DevPod, E2B)

```yaml
deployment:
  provider: docker # or fly, devpod, e2b
  # No image specified - builds from ./Dockerfile
```

#### Immutable Digest (Production)

```yaml
deployment:
  provider: kubernetes
  image_config:
    registry: ghcr.io/myorg/app
    digest: sha256:abc123...
# Deploys exact immutable image (best for production)
```

````

**Success Criteria**:
- All providers documented with image handling behavior
- Priority order clearly explained
- Examples provided for each scenario
- CI/CD workflows documented (Fly)
- Best practices emphasized (K8s: no builds)

---

### Phase 6 Config Generation Template Enhancements

**Priority**: HIGH
**Effort**: Medium
**Impact**: Improved user onboarding and configuration clarity
**Status**: ⚪ PENDING

**Objective**: Enhance `sindri config init` templates to document all image deployment variants with comprehensive examples and explanations.

**Problem Statement**:

When users run `sindri config init`, the generated `sindri.yaml` should serve as both configuration AND documentation, explaining all supported deployment methods with clear examples.

**Current Gaps**:
1. No documentation of image handling variants (legacy `image` vs new `image_config`)
2. No explanation of what happens when no image is specified
3. Missing examples for all supported `image_config` attributes
4. No guidance on when to use pre-built images vs Dockerfile builds
5. Provider-specific behavior not clearly explained

**Enhancements Required**:

1. **Update `sindri.yaml.tera` template** (`v3/crates/sindri-core/src/templates/sindri.yaml.tera`):

   Add comprehensive image handling section with examples:

   ```yaml
   deployment:
     provider: {{ provider }}

     # Image Deployment Options (choose one):

     # Option 1: Pre-built image (legacy - simple)
     # image: ghcr.io/myorg/app:v1.0.0

     # Option 2: Semantic versioning with verification (recommended)
     # image_config:
     #   registry: ghcr.io/myorg/app
     #   version: "^1.0.0"  # Resolves to latest 1.x.x
     #   verify_signature: true  # Verify with cosign
     #   verify_provenance: true  # Verify SLSA provenance
     #   pull_policy: IfNotPresent  # Always | IfNotPresent | Never

     # Option 3: Explicit tag override
     # image_config:
     #   registry: ghcr.io/myorg/app
     #   tag_override: v1.0.0-beta.1

     # Option 4: Immutable digest (production)
     # image_config:
     #   registry: ghcr.io/myorg/app
     #   digest: sha256:abc123...

     # Option 5: Build from local Dockerfile
     # (Leave image/image_config commented out)
     # Sindri will build from ./Dockerfile if no image specified
     # Supported providers: docker, fly, devpod, e2b

     {% if provider == "kubernetes" %}
     # Note: Kubernetes does NOT support building from Dockerfile
     # You MUST specify a pre-built image for this provider
     {% endif %}
````

2. **Provider-Specific Sections**:

   Add conditional explanations based on provider:

   ```yaml
   {% if provider == "fly" %}
   # Fly.io Image Deployment:
   # - Pre-built image: Skips build, deploys in ~30 seconds
   # - Dockerfile build: Builds server-side, takes 2-5 minutes
   # - Default Dockerfile path: ./Dockerfile or ./v3/Dockerfile
   {% endif %}

   {% if provider == "docker" %}
   # Docker Image Deployment:
   # - Pre-built image: Pulls from registry
   # - Dockerfile build: Builds locally with `docker build`
   # - Default Dockerfile path: ./Dockerfile
   {% endif %}

   {% if provider == "devpod" %}
   # DevPod Image Deployment:
   # - Pre-built image: Uses directly in devcontainer
   # - Dockerfile build: Builds and optionally pushes to registry
   # - Cloud providers require buildRepository config
   {% endif %}

   {% if provider == "e2b" %}
   # E2B Image Deployment:
   # - Dockerfile build: REQUIRED (builds E2B template)
   # - Pre-built images: Not currently supported
   # - Default Dockerfile path: ./Dockerfile
   {% endif %}

   {% if provider == "kubernetes" %}
   # Kubernetes Image Deployment:
   # - Pre-built image: REQUIRED (pulls from registry)
   # - Dockerfile build: NOT supported (build in CI/CD)
   # - Best practice: Use immutable digests in production
   {% endif %}
   ```

3. **Image Resolution Priority Documentation**:

   Add section explaining the resolution order:

   ```yaml
   # Image Resolution Priority Order:
   # 1. image_config.digest (immutable)
   # 2. image_config.tag_override (explicit tag)
   # 3. image_config.version (semver constraint)
   # 4. image (legacy full reference)
   # 5. Local Dockerfile (provider-dependent)
   # 6. Default: ghcr.io/pacphi/sindri:latest
   ```

4. **Complete Example Matrix**:

   For each provider, show practical examples:

   ```yaml
   # Example 1: Development (build locally)
   # deployment:
   #   provider: docker
   #   # No image - builds from ./Dockerfile

   # Example 2: CI/CD (build in CI, deploy here)
   # deployment:
   #   provider: fly
   #   image: ghcr.io/myorg/app:${CI_COMMIT_SHA}

   # Example 3: Production (immutable deployment)
   # deployment:
   #   provider: kubernetes
   #   image_config:
   #     registry: ghcr.io/myorg/app
   #     digest: sha256:abc123...
   ```

**Implementation Steps**:

1. Update `ConfigInitContext` in `v3/crates/sindri-core/src/templates/context.rs`:
   - Add `supports_dockerfile_build: bool`
   - Add `requires_prebuilt_image: bool`
   - Add `default_dockerfile_path: String`

2. Update `sindri.yaml.tera` with enhanced image handling section

3. Add provider capability matrix to template context:

   ```rust
   let supports_dockerfile_build = matches!(provider,
       Provider::Docker | Provider::Fly | Provider::DevPod | Provider::E2B
   );
   let requires_prebuilt_image = matches!(provider, Provider::Kubernetes);
   ```

4. Update unit tests in `v3/crates/sindri-core/src/templates/mod.rs`

5. Integration test: `sindri config init` for each provider

**Files Modified**:

- `v3/crates/sindri-core/src/templates/sindri.yaml.tera` (~150 lines added)
- `v3/crates/sindri-core/src/templates/context.rs` (~20 lines)
- `v3/crates/sindri-core/src/templates/mod.rs` (tests)

**Success Criteria**:

- Generated config files are self-documenting
- All image deployment variants explained with examples
- Provider-specific behavior clearly documented
- Users understand when to use pre-built vs Dockerfile builds
- No confusion about image resolution priority
- Examples align with best practices (K8s: no builds, Fly: CI/CD workflow)

**Related**:

- [ADR-028: Config Init Template Generation](../../architecture/adr/028-config-init-template-generation.md)
- [ADR-034: Image Handling Consistency Framework](../../architecture/adr/034-image-handling-consistency-framework.md)

---

### Phase 7: Architecture Documentation Update

**Priority**: HIGH
**Effort**: Low
**Impact**: Permanent architectural record of image handling consistency
**Status**: ⚪ PENDING
**Dependencies**: Phases 1-6 must be complete

**Objective**: Update `v3/docs/ARCHITECTURE.md` to reflect the image handling consistency framework once all implementation phases are complete.

**Problem Statement**:

The ARCHITECTURE.md document is the primary reference for Sindri v3's architectural decisions and component design. After implementing the image handling consistency framework, it must be updated to:

1. Document the standardized image resolution priority across all providers
2. Explain the Dockerfile path standardization
3. Reference the new ADRs (034, 035)
4. Update provider descriptions with build capabilities

**Updates Required**:

1. **Provider Abstraction Layer Section** (Lines ~40-62):

   Update to include image handling capabilities:

   ```markdown
   ### 2. Provider Abstraction Layer (`sindri-providers`)

   Async trait-based abstraction supporting five deployment providers with **standardized image handling**.

   **Supported Providers**:

   - **Docker**: Local container-based development with Docker Compose v2
     - **Image Handling**: Pre-built images OR local Dockerfile builds
   - **Fly.io**: Edge deployment with Fly Machines API
     - **Image Handling**: Pre-built images OR server-side Dockerfile builds
   - **DevPod**: Multi-backend development (Kubernetes, AWS, Docker)
     - **Image Handling**: Smart builds (cloud=build+push, local=dockerfile)
   - **E2B**: Ephemeral cloud sandboxes for AI agents
     - **Image Handling**: Dockerfile-based template builds (required)
   - **Kubernetes**: Production cluster deployments
     - **Image Handling**: Pre-built images only (no builds)

   **Image Resolution Priority** (all providers):

   1. `image_config.digest` - Immutable (production-safe)
   2. `image_config.tag_override` - Explicit tag
   3. `image_config.version` - Semantic version constraint
   4. `image` - Legacy full reference
   5. Local Dockerfile - Build on-demand (provider-dependent)
   6. Default - `ghcr.io/pacphi/sindri:latest`

   **Dockerfile Path Standardization**: All providers search in priority order:

   - `./Dockerfile` (project root - default)
   - `./v3/Dockerfile` (Sindri v3 specific - fallback)
   - `./deploy/Dockerfile` (deploy-specific - fallback)

   See [ADR-034](architecture/adr/034-image-handling-consistency-framework.md) and [ADR-035](architecture/adr/035-dockerfile-path-standardization.md) for details.
   ```

2. **Configuration Management Section** (Lines ~90-102):

   Add image configuration details:

   ```markdown
   ### 4. Configuration Management (`sindri-core`)

   Type-safe configuration loading, validation, and schema enforcement with **structured image handling**.

   **Key Types**:

   - `SindriConfig`: Root configuration from sindri.yaml
   - `DeploymentConfig`: Provider-agnostic deployment settings with `image_config` support
   - `ImageConfig`: Structured image configuration with semver resolution, verification
   - `ExtensionConfig`: Extension system configuration
   - `SecretsConfig`: Multi-source secret definitions

   **Image Configuration Features**:

   - Semantic versioning with constraint resolution (e.g., `^3.0.0`)
   - Image signature verification via cosign
   - SLSA provenance attestation verification
   - Immutable digest pinning for production
   - Pull policy control (Always, IfNotPresent, Never)

   See `v3/crates/sindri-core/src/types/config_types.rs:53-98` for `ImageConfig` definition.
   ```

3. **ADR Tracking Section**:

   Add new ADRs to the appropriate section:

   ```markdown
   ### Provider Framework & Configuration

   | ADR                                                                 | Title                                | Status   |
   | ------------------------------------------------------------------- | ------------------------------------ | -------- |
   | [002](architecture/adr/002-provider-abstraction-layer.md)           | Provider Abstraction Layer           | Accepted |
   | [003](architecture/adr/003-template-based-configuration.md)         | Template-Based Configuration (Tera)  | Accepted |
   | [034](architecture/adr/034-image-handling-consistency-framework.md) | Image Handling Consistency Framework | Accepted |
   | [035](architecture/adr/035-dockerfile-path-standardization.md)      | Dockerfile Path Standardization      | Accepted |
   ```

4. **Add New Subsection** (after Provider Abstraction Layer):

   ```markdown
   #### Image Resolution & Verification

   **Image Resolution** (`sindri-core/src/config/loader.rs:185-297`):

   - 6-level priority chain from digest to default
   - Semantic version constraint resolution via `sindri-image` crate
   - Registry API integration for tag enumeration
   - GitHub token support for private registries

   **Image Verification** (`sindri-image/src/verify.rs`):

   - Cosign signature verification
   - SLSA provenance attestation validation
   - Certificate identity and OIDC issuer validation
   - SBOM (Software Bill of Materials) fetching

   **Dockerfile Discovery** (`sindri-providers/src/utils.rs`):

   - Standardized path search across all providers
   - Priority order: `./Dockerfile` → `./v3/Dockerfile` → `./deploy/Dockerfile`
   - Clear error messages with searched paths

   This architecture enables consistent image handling across all deployment providers while maintaining provider-specific optimization strategies.
   ```

**Implementation Steps**:

1. Read current `v3/docs/ARCHITECTURE.md` to understand structure
2. Update Provider Abstraction Layer section with image handling details
3. Update Configuration Management section with `ImageConfig` details
4. Add ADR-034 and ADR-035 to ADR tracking section
5. Add new "Image Resolution & Verification" subsection
6. Update "Last Updated" date to completion date
7. Review for consistency with implemented changes

**Files Modified**:

- `v3/docs/ARCHITECTURE.md` (~100 lines modified/added)

**Success Criteria**:

- ARCHITECTURE.md accurately reflects image handling consistency framework
- Provider capabilities clearly documented (build vs pre-built only)
- Image resolution priority clearly explained
- Dockerfile path standardization documented
- New ADRs referenced in tracking section
- Code references point to correct line numbers
- Document serves as accurate architectural reference

**Related**:

- [ADR-034: Image Handling Consistency Framework](../../architecture/adr/034-image-handling-consistency-framework.md)
- [ADR-035: Dockerfile Path Standardization](../../architecture/adr/035-dockerfile-path-standardization.md)
- Phase 5: Documentation Updates (includes CONFIGURATION.md)

**Note**: CONFIGURATION.md updates are covered in Phase 5 (Documentation Updates), which includes adding the "Image vs Dockerfile Priority" section with build support table and examples.

---

## File Changes Summary

### Core Changes

| File                                           | Changes                                    | Lines | Priority |
| ---------------------------------------------- | ------------------------------------------ | ----- | -------- |
| `v3/crates/sindri-providers/src/docker.rs`     | Remove dead code, add build logic          | ~50   | HIGH     |
| `v3/crates/sindri-providers/src/fly.rs`        | Add image override method, update deploy   | ~80   | HIGH     |
| `v3/crates/sindri-providers/src/devpod.rs`     | Use `find_dockerfile()` helper             | ~5    | MEDIUM   |
| `v3/crates/sindri-providers/src/e2b.rs`        | Use `find_dockerfile()`, add image support | ~30   | MEDIUM   |
| `v3/crates/sindri-providers/src/kubernetes.rs` | Use `resolve_image()`                      | ~5    | HIGH     |
| `v3/crates/sindri-providers/src/utils.rs`      | Add `find_dockerfile()` helper             | ~15   | MEDIUM   |

### Template Changes

| File                                                     | Changes                 | Lines | Priority |
| -------------------------------------------------------- | ----------------------- | ----- | -------- |
| `v3/crates/sindri-providers/src/templates/fly.toml.tera` | Dynamic dockerfile path | ~2    | MEDIUM   |

### Documentation Changes

| File                              | Changes                                 | Lines | Priority |
| --------------------------------- | --------------------------------------- | ----- | -------- |
| `v3/docs/CONFIGURATION.md`        | Add image priority section, build table | ~80   | HIGH     |
| `v3/docs/IMAGE_MANAGEMENT.md`     | Add resolution priority section         | ~50   | HIGH     |
| `v3/docs/ARCHITECTURE.md`         | Update provider sections, add ADRs      | ~100  | HIGH     |
| `v3/docs/providers/DOCKER.md`     | Add build support section               | ~40   | MEDIUM   |
| `v3/docs/providers/FLY.md`        | Add image override section              | ~40   | MEDIUM   |
| `v3/docs/providers/DEVPOD.md`     | Document smart build behavior           | ~30   | LOW      |
| `v3/docs/providers/E2B.md`        | Update build requirements               | ~20   | LOW      |
| `v3/docs/providers/KUBERNETES.md` | Reinforce no-build policy               | ~20   | LOW      |

### Config Template Changes

| File                                                   | Changes                          | Lines | Priority |
| ------------------------------------------------------ | -------------------------------- | ----- | -------- |
| `v3/crates/sindri-core/src/templates/sindri.yaml.tera` | Add image handling documentation | ~150  | HIGH     |
| `v3/crates/sindri-core/src/templates/context.rs`       | Add image capability flags       | ~20   | MEDIUM   |

**Total Estimated Changes**: ~720 lines across 17 files

---

## Testing Strategy

### Unit Tests

1. **Image Resolution Tests** (`v3/crates/sindri-core/tests/config_tests.rs`)

   ```rust
   #[test]
   fn test_image_resolution_priority() {
       // Test digest takes precedence
       // Test tag_override over version
       // Test version resolution
       // Test legacy image field
       // Test default fallback
   }
   ```

2. **Docker Build Tests** (`v3/crates/sindri-providers/tests/docker_tests.rs`)

   ```rust
   #[test]
   fn test_docker_build_when_no_image() {
       // Create test Dockerfile
       // Deploy without image specified
       // Verify build was triggered
       // Verify image tag is correct
   }

   #[test]
   fn test_docker_skip_build_with_image() {
       // Deploy with image specified
       // Verify build was NOT triggered
       // Verify specified image was used
   }
   ```

3. **Fly Image Override Tests** (`v3/crates/sindri-providers/tests/fly_tests.rs`)

   ```rust
   #[test]
   fn test_fly_image_override() {
       // Deploy with image specified
       // Verify flyctl deploy --image was called
       // Verify build was NOT triggered
   }

   #[test]
   fn test_fly_dockerfile_build() {
       // Deploy without image
       // Verify flyctl deploy was called (no --image flag)
       // Verify build was triggered
   }
   ```

4. **Dockerfile Path Tests** (`v3/crates/sindri-providers/tests/utils_tests.rs`)
   ```rust
   #[test]
   fn test_find_dockerfile_priority() {
       // Create test Dockerfiles at different paths
       // Verify ./Dockerfile takes precedence
       // Verify v3/Dockerfile is fallback
       // Verify deploy/Dockerfile is last resort
   }
   ```

### Integration Tests

1. **End-to-End Docker Deployment**

   ```bash
   # Setup
   mkdir test-docker-build
   cd test-docker-build
   cat > sindri.yaml << EOF
   version: "3.0"
   name: test-docker-build
   deployment:
     provider: docker
   extensions:
     profile: minimal
   EOF

   cat > Dockerfile << EOF
   FROM ubuntu:22.04
   RUN apt-get update && apt-get install -y curl
   EOF

   # Test
   sindri deploy

   # Verify
   docker images | grep test-docker-build
   docker ps | grep test-docker-build
   ```

2. **End-to-End Fly Deployment with Image**

   ```bash
   # Setup
   mkdir test-fly-image
   cd test-fly-image
   cat > sindri.yaml << EOF
   version: "3.0"
   name: test-fly-image
   deployment:
     provider: fly
     image: ghcr.io/pacphi/sindri:v3.0.0
   extensions:
     profile: minimal
   EOF

   # Test
   sindri deploy

   # Verify
   flyctl status | grep test-fly-image
   # Should NOT show build logs
   ```

3. **End-to-End Image Config Resolution**

   ```bash
   # Setup
   cat > sindri.yaml << EOF
   version: "3.0"
   name: test-image-config
   deployment:
     provider: docker
     image_config:
       registry: ghcr.io/pacphi/sindri
       version: "^3.0.0"
       verify_signature: true
   extensions:
     profile: minimal
   EOF

   # Test
   sindri deploy

   # Verify
   # Should resolve to latest 3.x.x version
   # Should verify signature
   # Should deploy successfully
   ```

### Manual Testing Checklist

- [ ] Docker: Pre-built image skips build
- [ ] Docker: No image triggers Dockerfile build
- [ ] Docker: Force flag triggers rebuild
- [ ] Fly: Pre-built image skips Dockerfile build
- [ ] Fly: No image triggers Dockerfile build
- [ ] DevPod: Smart build behavior works (cloud vs local)
- [ ] E2B: Dockerfile build works (or image template if implemented)
- [ ] Kubernetes: Image pull works, no builds attempted
- [ ] All providers: `image_config.version` resolves correctly
- [ ] All providers: Signature verification works
- [ ] All providers: Provenance verification works
- [ ] All providers: Digest pinning works
- [ ] Dockerfile found at `./Dockerfile`, `./v3/Dockerfile`, `./deploy/Dockerfile`

---

## Success Criteria

### Functional Requirements

✅ **Image Resolution Consistency**

- All providers use `config.resolve_image().await?`
- Priority order respected: digest > tag_override > version > image > Dockerfile > default
- Legacy `image` field works for backward compatibility

✅ **Build Support**

- Docker: Builds from Dockerfile when no image specified
- Fly: Supports both pre-built images AND Dockerfile builds
- DevPod: Maintains current smart build behavior
- E2B: Template builds work (with optional image support)
- Kubernetes: No builds (correct per best practices)

✅ **Dockerfile Path Standardization**

- All providers search: `./Dockerfile` → `./v3/Dockerfile` → `./deploy/Dockerfile`
- Consistent behavior across providers

✅ **Image Config Features**

- Semantic versioning works (`^3.0.0` resolves to latest 3.x.x)
- Signature verification works when enabled
- Provenance verification works when enabled
- Digest pinning works for immutable deployments

### Non-Functional Requirements

✅ **Performance**

- Pre-built images skip build time (2-5 minutes saved)
- Local builds cached appropriately (Docker, DevPod)
- No performance regression for existing workflows

✅ **Backward Compatibility**

- Existing configs continue to work
- Legacy `image` field still supported
- Dockerfile builds work where they worked before

✅ **Documentation**

- All changes documented
- Examples provided for each scenario
- Migration guide for users

✅ **Testing**

- Unit tests for all new functionality
- Integration tests for end-to-end workflows
- Manual testing checklist completed

### User Experience

✅ **Clarity**

- Users understand when builds happen vs when images are pulled
- Error messages are clear (e.g., "No image or Dockerfile found")
- Documentation explains tradeoffs (pre-built vs build)

✅ **Flexibility**

- Users can choose pre-built images for speed
- Users can build locally for customization
- CI/CD workflows supported (build in CI, deploy via Sindri)

✅ **Consistency**

- Same configuration works similarly across providers
- Dockerfile paths predictable
- Image resolution behavior predictable

---

## References

### Industry Best Practices

**Fly.io**

- [Deploy with a Dockerfile · Fly Docs](https://fly.io/docs/languages-and-frameworks/dockerfile/)
- [fly deploy · Fly Docs](https://fly.io/docs/flyctl/deploy/)
- [Deploy an app · Fly Docs](https://fly.io/docs/launch/deploy/)
- [deploy from docker image vs local docker file - Fly.io Community](https://community.fly.io/t/deploy-from-docker-image-vs-local-docker-file/24349)
- [Working with Docker on Fly.io · Fly Docs](https://fly.io/docs/blueprints/working-with-docker/)
- [Using base images for faster deployments · Fly Docs](https://fly.io/docs/blueprints/using-base-images-for-faster-deployments/)

**DevPod**

- [Prebuild a Workspace | DevPod docs](https://devpod.sh/docs/developing-in-workspaces/prebuild-a-workspace)
- [How DevPod Builds Workspaces | DevPod docs](https://devpod.sh/docs/how-it-works/building-workspaces)
- [devcontainer.json | DevPod docs](https://devpod.sh/docs/developing-in-workspaces/devcontainer-json)
- [Speeding up Dev Containers with Pre-built Images](https://www.daytona.io/dotfiles/speeding-up-dev-containers-with-pre-built-images)
- [Ultimate Guide to Dev Containers](https://www.daytona.io/dotfiles/ultimate-guide-to-dev-containers)

**E2B**

- [Sandbox Template - E2B](https://e2b.dev/docs/sandbox-template)
- [SDK Reference - E2B CLI](https://e2b.dev/docs/sdk-reference/cli/v1.0.9/template)
- [e2b template build](https://e2b.dev/docs/sdk-reference/cli/v1.0.2/template)
- [Introducing Build System 2.0 — E2B Blog](https://e2b.dev/blog/introducing-build-system-2-0)

**Kubernetes**

- [Kubernetes best practices: Small Container Images | Google Cloud](https://cloud.google.com/blog/products/containers-kubernetes/kubernetes-best-practices-how-and-why-to-build-small-container-images)
- [Building Docker images in Kubernetes | Snyk](https://snyk.io/blog/building-docker-images-kubernetes/)
- [27+ Kubernetes Deployment Best Practices | Zeet](https://zeet.co/blog/kubernetes-deployment-best-practices)
- [10 Kubernetes Best Practices to Get Started](https://www.densify.com/kubernetes-tools/kubernetes-best-practices/)
- [Kubernetes Best Practices For 2025](https://www.cloudzero.com/blog/kubernetes-best-practices/)
- [Best practices for CI/CD to GKE | Google Cloud](https://cloud.google.com/kubernetes-engine/docs/concepts/best-practices-continuous-integration-delivery-kubernetes)

### Internal Documentation

**Current Configuration**

- `v3/docs/CONFIGURATION.md` - Configuration reference
- `v3/docs/IMAGE_MANAGEMENT.md` - Image management guide
- `v3/docs/SCHEMA.md` - Schema documentation

**Provider Documentation**

- `v3/docs/providers/DOCKER.md` - Docker provider
- `v3/docs/providers/FLY.md` - Fly provider
- `v3/docs/providers/DEVPOD.md` - DevPod provider
- `v3/docs/providers/E2B.md` - E2B provider
- `v3/docs/providers/KUBERNETES.md` - Kubernetes provider

**Code References**

- `v3/crates/sindri-core/src/config/loader.rs:185-297` - Image resolution logic
- `v3/crates/sindri-core/src/types/config_types.rs` - Config type definitions
- `v3/crates/sindri-image/src/resolver.rs` - Version resolution
- `v3/crates/sindri-image/src/verify.rs` - Image verification

---

## Appendix: Decision Log

### Decision 1: Activate Docker Build Support

**Date**: 2026-01-27
**Decision**: Activate unused `build_image()` method in Docker provider
**Rationale**: Improves local dev workflow, aligns with DevPod/E2B behavior, method already exists
**Alternatives Considered**: Leave as-is (rejected - unused capability), remove method (rejected - useful)

### Decision 2: Add Fly Image Override

**Date**: 2026-01-27
**Decision**: Support pre-built images in Fly provider
**Rationale**: Aligns with Fly.io best practices, enables CI/CD workflows, saves 2-5 minutes per deploy
**Alternatives Considered**: Leave as-is (rejected - inconsistent), remove Dockerfile builds (rejected - breaks existing workflows)

### Decision 3: Standardize Dockerfile Paths

**Date**: 2026-01-27
**Decision**: Use `./Dockerfile` → `./v3/Dockerfile` → `./deploy/Dockerfile` search order
**Rationale**: Reduces confusion, aligns with Docker/DevPod/E2B conventions
**Alternatives Considered**: Single path (rejected - less flexible), configurable path (rejected - too complex)

### Decision 4: Keep Kubernetes Pre-built Only

**Date**: 2026-01-27
**Decision**: Do NOT add build support to Kubernetes provider
**Rationale**: Aligns with K8s best practices (build in CI/CD, not during deploy)
**Alternatives Considered**: Add build support (rejected - anti-pattern for K8s)

### Decision 5: Keep DevPod Unchanged

**Date**: 2026-01-27
**Decision**: No changes to DevPod provider
**Rationale**: Already implements optimal behavior (smart builds, caching, both scenarios supported)
**Alternatives Considered**: Standardize with others (rejected - would be a regression)

---

**End of Document**
