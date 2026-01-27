# ADR 035: Dockerfile Path Standardization

**Status**: Superseded
**Date**: 2026-01-27
**Superseded By**: [ADR-037: Image Naming and Tagging Strategy](037-image-naming-and-tagging-strategy.md)
**Deciders**: Core Team
**Related**: [ADR-034: Image Handling Consistency Framework](034-image-handling-consistency-framework.md)

---

**SUPERSEDED NOTICE**: This ADR has been superseded by a fundamental architectural change. Instead of searching for user-provided Dockerfiles, Sindri now clones its own GitHub repository to get the official Sindri v3 Dockerfile and build context. See [ADR-037](037-image-naming-and-tagging-strategy.md) for the new approach.

---

## Context

Container-based providers that support building from Dockerfile have inconsistent expectations for where the Dockerfile is located:

### Current Inconsistencies

| Provider   | Dockerfile Path | Location                             |
| ---------- | --------------- | ------------------------------------ |
| **Fly**    | `v3/Dockerfile` | Hardcoded in fly.toml template       |
| **DevPod** | `./Dockerfile`  | base_dir (project root parent)       |
| **E2B**    | `./Dockerfile`  | base_dir (current working directory) |

### Problems

1. **User Confusion**: Users must know provider-specific paths
2. **Duplication**: May need multiple Dockerfiles (v3/Dockerfile AND ./Dockerfile)
3. **Template Lock-in**: Fly's hardcoded path in template is inflexible
4. **Discoverability**: No clear convention for where to place Dockerfile

### Example User Pain

```bash
# User creates Dockerfile for Docker provider
echo "FROM ubuntu:22.04" > ./Dockerfile
sindri deploy --provider docker  # ✅ Works

# Now tries Fly
sindri deploy --provider fly
# ❌ Error: v3/Dockerfile not found

# User confused, creates second Dockerfile
mkdir v3
cp Dockerfile v3/Dockerfile
sindri deploy --provider fly  # ✅ Works but duplicate files
```

## Decision

### Standard Search Order

All providers that support Dockerfile builds will search for Dockerfiles in this **priority order**:

```
1. ./Dockerfile            # Project root (default)
2. ./v3/Dockerfile         # Sindri v3 specific (fallback)
3. ./deploy/Dockerfile     # Deploy-specific (fallback)
```

**Rationale**:

- `./Dockerfile` is Docker ecosystem convention (matches `docker build` default)
- `./v3/Dockerfile` provides backward compatibility for Fly users
- `./deploy/Dockerfile` allows separation of build config from dev environment

### Shared Helper Function

Create a **shared utility** in `sindri-providers/src/utils.rs`:

```rust
/// Find Dockerfile using standard search paths
///
/// Searches in priority order:
/// 1. ./Dockerfile (project root - Docker convention)
/// 2. ./v3/Dockerfile (Sindri v3 specific - backward compat)
/// 3. ./deploy/Dockerfile (deploy-specific)
///
/// Returns the first existing Dockerfile path, or None if not found
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

### Provider Updates

**Docker Provider** (`docker.rs`):

```rust
// Current:
let dockerfile = "./Dockerfile";  // Assumed

// New:
let dockerfile = find_dockerfile()
    .ok_or_else(|| anyhow!("No Dockerfile found in search paths"))?;
```

**Fly Provider** (`fly.rs`):

```rust
// Current (template):
[build]
dockerfile = "v3/Dockerfile"  # Hardcoded

// New (template):
[build]
dockerfile = "{{ dockerfile_path }}"  # Dynamic

// Code:
let dockerfile_path = find_dockerfile()
    .unwrap_or(PathBuf::from("v3/Dockerfile"))  // Backward compat default
    .display()
    .to_string();
```

**DevPod Provider** (`devpod.rs`):

```rust
// Current:
let dockerfile = base_dir.join("Dockerfile");

// New:
let dockerfile = find_dockerfile()
    .ok_or_else(|| anyhow!("No Dockerfile found in search paths"))?;
```

**E2B Provider** (`e2b.rs`):

```rust
// Current:
let dockerfile_path = base_dir.join("Dockerfile");
if !dockerfile_path.exists() {
    return Err(anyhow!("Dockerfile not found at {}", dockerfile_path.display()));
}

// New:
let dockerfile_path = find_dockerfile()
    .ok_or_else(|| anyhow!(
        "No Dockerfile found. Searched: ./Dockerfile, ./v3/Dockerfile, ./deploy/Dockerfile"
    ))?;
```

### Error Messages

Provide **clear, actionable errors** when no Dockerfile is found:

```
Error: No Dockerfile found

Searched locations:
  • ./Dockerfile (project root - recommended)
  • ./v3/Dockerfile (Sindri v3 specific)
  • ./deploy/Dockerfile (deploy-specific)

Create a Dockerfile or specify a pre-built image:
  deployment:
    image: ghcr.io/myorg/app:v1.0.0
```

### Backward Compatibility

**Fly users with existing `v3/Dockerfile`**:

- ✅ Still works (searched as second priority)
- No migration required

**DevPod/E2B users with `./Dockerfile`**:

- ✅ Still works (searched as first priority)
- No migration required

**New users**:

- Use `./Dockerfile` (conventional path)
- Auto-discovered by all providers

## Consequences

### Positive

1. **Consistency**: All providers search same paths in same order
2. **Convention**: Aligns with Docker ecosystem (./Dockerfile)
3. **Backward Compatible**: Existing Fly `v3/Dockerfile` still works
4. **Flexibility**: Users can organize as `./deploy/Dockerfile` if desired
5. **Clear Errors**: Helpful messages when Dockerfile not found
6. **Code Reuse**: Shared utility eliminates duplication

### Negative

1. **Multiple Candidate Paths**: Could be confusing if user has multiple Dockerfiles
2. **Priority Ambiguity**: User might not know which Dockerfile is used
3. **Testing**: Need to test all three paths for each provider

### Risks & Mitigation

**Risk**: User has both `./Dockerfile` and `./v3/Dockerfile`, confused which is used
**Mitigation**: Log which Dockerfile was found: `Using Dockerfile: ./Dockerfile`

**Risk**: User expects different path (e.g., `./docker/Dockerfile`)
**Mitigation**: Clear error message shows searched paths

**Risk**: Breaking change for users expecting specific paths
**Mitigation**: All current paths (`.` and `v3/`) are searched, so no breaks

## Alternatives Considered

### Alternative 1: Single Hardcoded Path

**Option**: All providers use `./Dockerfile` only

**Rejected**: Breaks backward compatibility for Fly users with `v3/Dockerfile`

### Alternative 2: Configurable Dockerfile Path

**Option**: Add `dockerfile_path` config field:

```yaml
deployment:
  provider: docker
  dockerfile_path: ./custom/Dockerfile
```

**Rejected**: Adds complexity, most users don't need it. Can add later if requested.

### Alternative 3: Provider-Specific Paths (Current)

**Option**: Keep existing inconsistent paths

**Rejected**: User confusion, duplication, inconsistent experience

### Alternative 4: Search All Subdirectories

**Option**: Recursively search for any `Dockerfile`

**Rejected**: Too broad, could find wrong file (e.g., `node_modules/Dockerfile`)

## Implementation

### Phase 1: Create Shared Helper

- File: `v3/crates/sindri-providers/src/utils.rs`
- Function: `find_dockerfile() -> Option<PathBuf>`
- Tests: Unit tests for search logic

### Phase 2: Update Providers

- Docker: Use `find_dockerfile()` in build logic
- Fly: Update template to use `{{ dockerfile_path }}`, compute in code
- DevPod: Replace `base_dir.join("Dockerfile")` with `find_dockerfile()`
- E2B: Replace hardcoded check with `find_dockerfile()`

### Phase 3: Update Error Messages

- Standardize "No Dockerfile found" errors across providers
- List searched paths in error message
- Suggest alternatives (use pre-built image)

### Phase 4: Documentation

- Update CONFIGURATION.md with Dockerfile path section
- Update provider docs with Dockerfile location guidance
- Add migration note for Fly users (v3/Dockerfile still works)

### Phase 5: Testing

- Unit tests: `find_dockerfile()` with various file layouts
- Integration tests: Deploy with Dockerfile in each location
- Regression tests: Fly with `v3/Dockerfile` still works

## Testing Strategy

### Unit Tests (`utils_tests.rs`)

```rust
#[test]
fn test_find_dockerfile_priority() {
    // Create ./Dockerfile and ./v3/Dockerfile
    // Verify ./Dockerfile is returned (first priority)
}

#[test]
fn test_find_dockerfile_fallback() {
    // Only create ./v3/Dockerfile
    // Verify ./v3/Dockerfile is returned
}

#[test]
fn test_find_dockerfile_none() {
    // No Dockerfile in any location
    // Verify None is returned
}
```

### Integration Tests

```bash
# Test 1: Docker with ./Dockerfile
echo "FROM ubuntu:22.04" > ./Dockerfile
sindri deploy --provider docker
# Verify: Uses ./Dockerfile

# Test 2: Fly with v3/Dockerfile (backward compat)
mkdir v3
echo "FROM ubuntu:22.04" > ./v3/Dockerfile
sindri deploy --provider fly
# Verify: Uses ./v3/Dockerfile

# Test 3: Priority (both exist)
echo "FROM ubuntu:22.04" > ./Dockerfile
echo "FROM ubuntu:22.04" > ./v3/Dockerfile
sindri deploy --provider docker
# Verify: Uses ./Dockerfile (logs "Using Dockerfile: ./Dockerfile")
```

## References

**Docker Ecosystem Convention**:

- Docker CLI: `docker build` defaults to `./Dockerfile`
- Docker Compose: `build.dockerfile` defaults to `./Dockerfile`
- Buildpacks: Expect `./Dockerfile` by convention

**Code Changes**:

- `v3/crates/sindri-providers/src/utils.rs` - New helper
- `v3/crates/sindri-providers/src/docker.rs` - Use helper
- `v3/crates/sindri-providers/src/fly.rs` - Use helper + template update
- `v3/crates/sindri-providers/src/devpod.rs` - Use helper
- `v3/crates/sindri-providers/src/e2b.rs` - Use helper
- `v3/crates/sindri-providers/src/templates/fly.toml.tera` - Dynamic path

**Related ADRs**:

- [ADR-034: Image Handling Consistency Framework](034-image-handling-consistency-framework.md) - Overall consistency strategy
