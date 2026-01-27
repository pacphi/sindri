# ADR 036: Build-Time Image Metadata Caching

**Status**: Accepted
**Date**: 2026-01-27
**Deciders**: Core Team
**Related**: [ADR-034: Image Handling Consistency Framework](034-image-handling-consistency-framework.md)

## Context

The `sindri image list` command queries the GitHub Container Registry (GHCR) to display available Sindri container images. This operation requires authentication even for public packages due to GHCR API policies.

### Authentication Challenge

GitHub Container Registry has different authentication requirements for different operations:

- **Image Pulling**: No authentication required for public packages (`docker pull ghcr.io/pacphi/sindri:v3.0.0`)
- **API Tag Listing**: Requires authentication even for public packages (`GET /v2/{repo}/tags/list` returns 401 without auth)

This creates a poor user experience:

```bash
❯ sindri image list
Error: Failed to list tags

Caused by:
    Registry API error (401 Unauthorized): {"errors":[{"code":"UNAUTHORIZED","message":"authentication required"}]}
```

### Current User Experience Problems

1. **Authentication Barrier**: Users must generate and set `GITHUB_TOKEN` to use `sindri image list`
2. **Token Management**: Users must:
   - Visit https://github.com/settings/tokens
   - Create classic Personal Access Token (fine-grained not supported)
   - Grant `read:packages` scope
   - Export as environment variable
3. **Repeated Failures**: Every user hits authentication error on first use
4. **Documentation Burden**: Requires extensive setup instructions

### Use Case Analysis

**Primary users of `sindri image list`**:

1. **New users** exploring available versions (no token setup yet)
2. **CI/CD systems** checking for updates (environment-based tokens)
3. **Offline users** working in restricted networks
4. **Quick reference** users who just want to see recent versions

**Observed usage patterns**:

- Users rarely need ALL tags (100+ versions)
- Recent 5-10 versions cover 95% of use cases
- Version list doesn't change frequently (releases every few weeks)
- Token setup friction causes abandonment

### Requirements

1. **Zero-friction first use**: `sindri image list` should work without setup
2. **Fresh data when available**: Use live data if `GITHUB_TOKEN` present
3. **Graceful degradation**: Fall back to cached data when no token
4. **Clear error messages**: Guide users for token issues (expired, invalid)
5. **Reasonable staleness**: Warn when cache is outdated
6. **Small binary size**: Minimize embedded data impact

## Decision

### Build-Time Metadata Embedding

**Embed the last 5 image tags** into the CLI binary at compile time, with a 120-day staleness threshold.

### Architecture

```
┌─────────────────────────────────────────────────┐
│ Build Time (GitHub Actions with GITHUB_TOKEN)  │
├─────────────────────────────────────────────────┤
│ 1. build.rs script runs                         │
│ 2. Detects GITHUB_TOKEN environment variable    │
│ 3. Fetches last 5 tags from GHCR               │
│ 4. Generates image_metadata.json               │
│ 5. Embeds JSON in OUT_DIR                      │
│ 6. include_str!() at compile time              │
└─────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────┐
│ Runtime (User's machine)                        │
├─────────────────────────────────────────────────┤
│ sindri image list                               │
│   ↓                                             │
│ GITHUB_TOKEN present?                           │
│   ├─ YES: Fetch live (preferred)               │
│   │   ├─ Success: Display fresh data           │
│   │   ├─ 401: "Invalid/expired token" error    │
│   │   └─ Other: Warn, fall back to cache       │
│   └─ NO: Use embedded cache                     │
│       ├─ Cache empty: "No data" error           │
│       ├─ Cache >120 days: Warning, display      │
│       └─ Cache valid: Display                   │
└─────────────────────────────────────────────────┘
```

### Configuration Constants

```rust
// v3/crates/sindri-image/src/types.rs

impl CachedImageMetadata {
    /// Maximum number of versions to cache at build time
    pub const MAX_CACHED_VERSIONS: usize = 5;

    /// Time-to-live in days before cache is considered stale
    pub const TTL_DAYS: i64 = 120;
}
```

**Rationale**:

- **5 versions**: Covers recent releases (3-6 months), small binary impact (~2KB)
- **120 days**: 4 months gives users time to update CLI without constant warnings

### Data Structure

```rust
// v3/crates/sindri-image/src/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedImageMetadata {
    /// When this cache was generated (ISO 8601 timestamp)
    pub generated_at: String,
    /// Registry hostname
    pub registry: String,
    /// Repository path
    pub repository: String,
    /// Cached tags (limited to MAX_CACHED_VERSIONS)
    pub tags: Vec<CachedTagInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTagInfo {
    /// Tag name (e.g., "v3.0.0")
    pub tag: String,
    /// Image digest
    pub digest: String,
    /// Creation timestamp (ISO 8601)
    pub created: String,
}
```

### Build Script Implementation

```rust
// v3/crates/sindri/build.rs

#[tokio::main]
async fn main() {
    // Existing build metadata...

    // Cache image metadata if GITHUB_TOKEN available
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        eprintln!("📦 Fetching image metadata from GHCR...");

        match fetch_and_cache_metadata(&token).await {
            Ok(json) => {
                fs::write(concat!(env!("OUT_DIR"), "/image_metadata.json"), json)?;
                eprintln!("✅ Cached image metadata successfully");
            }
            Err(e) => {
                eprintln!("⚠️  Failed to fetch: {}", e);
                write_empty_cache()?; // Don't fail build
            }
        }
    } else {
        eprintln!("ℹ️  No GITHUB_TOKEN, creating empty cache");
        write_empty_cache()?;
    }
}

async fn fetch_and_cache_metadata(token: &str) -> Result<String> {
    let client = RegistryClient::new("ghcr.io").with_token(token);

    // Fetch and sort tags
    let mut tags = client.list_tags("pacphi/sindri").await?;
    tags.sort_by_semver_desc();

    // Take first 5, fetch manifests
    let recent: Vec<_> = tags.into_iter().take(5).collect();
    let mut cached_tags = Vec::new();

    for tag in recent {
        let manifest = client.get_manifest("pacphi/sindri", &tag).await?;
        cached_tags.push(CachedTagInfo {
            tag,
            digest: manifest.config.digest,
            created: Utc::now().to_rfc3339(),
        });
    }

    let cache = CachedImageMetadata {
        generated_at: Utc::now().to_rfc3339(),
        registry: "ghcr.io".to_string(),
        repository: "pacphi/sindri".to_string(),
        tags: cached_tags,
    };

    Ok(serde_json::to_string_pretty(&cache)?)
}
```

### Runtime Fallback Strategy

```rust
// v3/crates/sindri/src/commands/image.rs

async fn list(args: ImageListArgs) -> Result<()> {
    let github_token = std::env::var("GITHUB_TOKEN").ok();

    // Strategy: Live fetch preferred, cache fallback
    let tags = if let Some(token) = github_token {
        info!("Fetching latest images from registry");

        match fetch_live_tags(&args.registry, &repository, &token).await {
            Ok(tags) => tags,
            Err(e) if e.to_string().contains("401") => {
                // Invalid/expired token
                return Err(anyhow!(
                    "Authentication failed (401 Unauthorized).\n\n\
                     Your GITHUB_TOKEN may be invalid or expired.\n\
                     Generate new: https://github.com/settings/tokens\n\
                     Required scope: read:packages\n\n\
                     Then: export GITHUB_TOKEN=ghp_your_token_here"
                ));
            }
            Err(e) => {
                // Network or other error - fall back to cache
                warn!("Failed to fetch: {}", e);
                warn!("Falling back to cached image data...");
                load_cached_tags()?
            }
        }
    } else {
        info!("No GITHUB_TOKEN found, using cached data");
        load_cached_tags()?
    };

    if tags.is_empty() {
        return Err(anyhow!(
            "No image data available.\n\n\
             This can happen because:\n\
             1. CLI built without cached metadata\n\
             2. No GITHUB_TOKEN set for live fetching\n\n\
             To fix:\n\
             - Set: export GITHUB_TOKEN=ghp_your_token\n\
             - Or: Update CLI to latest version\n\n\
             Generate token: https://github.com/settings/tokens"
        ));
    }

    display_tags(&tags, &args)?;
    Ok(())
}

fn load_cached_tags() -> Result<Vec<String>> {
    const EMBEDDED: &str = include_str!(concat!(env!("OUT_DIR"), "/image_metadata.json"));

    let cache: CachedImageMetadata = serde_json::from_str(EMBEDDED)?;

    // Check staleness
    if cache.is_stale() {
        warn!(
            "⚠️  Cache is {} days old (TTL: {} days)",
            cache.age_days(),
            CachedImageMetadata::TTL_DAYS
        );
        warn!("   Consider updating CLI or setting GITHUB_TOKEN");
    }

    Ok(cache.tags.iter().map(|t| t.tag.clone()).collect())
}
```

### Error Messages

**Empty Cache + No Token**:

```
No image data available.

This can happen because:
1. The CLI was built without cached image metadata
2. No GITHUB_TOKEN is set for live fetching

To fix:
- Set GITHUB_TOKEN: export GITHUB_TOKEN=ghp_your_token_here
- Or update to the latest CLI version with cached metadata

Generate token at: https://github.com/settings/tokens (requires 'read:packages' scope)
```

**Invalid/Expired Token**:

```
Authentication failed (401 Unauthorized).

Your GITHUB_TOKEN may be invalid or expired.
Please generate a new token at: https://github.com/settings/tokens
Required scope: read:packages

Then set it: export GITHUB_TOKEN=ghp_your_token_here
```

**Stale Cache Warning**:

```
⚠️  Cached image data is 150 days old (cache TTL: 120 days)
   Consider updating the CLI or setting GITHUB_TOKEN for latest data
```

## Consequences

### Positive

1. **Zero-Friction First Use**: New users can run `sindri image list` immediately
2. **Offline Support**: Works without network access (using cache)
3. **CI/CD Friendly**: No mandatory token setup for basic commands
4. **Cost Reduction**: Fewer GHCR API calls (reduced rate limit pressure)
5. **Better UX**: Clear, actionable error messages for token issues
6. **Small Impact**: ~2KB binary size increase for 5 cached versions
7. **Graceful Degradation**: Live → Cache → Clear Error (3-tier fallback)

### Negative

1. **Stale Data**: Users without tokens see versions as of build time
2. **Build Complexity**: Requires authenticated builds in CI/CD
3. **Two Code Paths**: Live fetch vs cache loading (more testing)
4. **Version Lag**: New releases not visible until CLI updated

### Trade-offs

| Aspect                 | Without Caching              | With Caching (This ADR)                |
| ---------------------- | ---------------------------- | -------------------------------------- |
| First-use experience   | ❌ Authentication error      | ✅ Works immediately                   |
| Data freshness         | ✅ Always current (if token) | ⚠️ Up to 120 days old (no token)       |
| Binary size            | ✅ Minimal (~30MB)           | ⚠️ +2KB (~30.002MB)                    |
| Build complexity       | ✅ Simple (no external deps) | ⚠️ Requires GITHUB_TOKEN in CI         |
| Offline support        | ❌ None                      | ✅ Works with cache                    |
| Error guidance         | ⚠️ Generic auth error        | ✅ Specific, actionable messages       |
| GHCR API rate limits   | ⚠️ Every user hits registry  | ✅ Reduced (cache + warnings)          |
| New release visibility | ✅ Immediate (if token)      | ⚠️ Delayed until CLI update (no token) |
| User control           | ❌ Must setup token          | ✅ Optional token for fresh data       |
| Testing complexity     | ✅ Single code path          | ⚠️ Two paths (live + cache)            |
| CI/CD setup            | ✅ Standard build            | ⚠️ Must inject GITHUB_TOKEN secret     |
| Token rotation         | ⚠️ User responsibility       | ⚠️ User + CI (for builds)              |

### Risks & Mitigation

**Risk**: Users rely on stale cache and miss important updates
**Mitigation**:

- 120-day warning threshold
- Display cache age in output
- Encourage setting GITHUB_TOKEN in docs

**Risk**: Build fails if GITHUB_TOKEN invalid in CI
**Mitigation**:

- Build script writes empty cache on fetch failure
- Build succeeds, users see "no data" error at runtime with clear instructions

**Risk**: Users confused about why they see different versions than other users
**Mitigation**:

- Document caching behavior in help text
- Show "(cached)" indicator when displaying cached data
- Add `--no-cache` flag to force live fetch

**Risk**: Binary size growth with more cached versions
**Mitigation**:

- Limit to 5 versions (2KB)
- Monitor binary size in CI
- Consider compression if needed

## Implementation

### Files Changed

```
v3/crates/sindri/Cargo.toml                   (+ build-dependencies)
v3/crates/sindri/build.rs                     (+ fetch logic, ~150 lines)
v3/crates/sindri-image/src/types.rs           (+ CachedImageMetadata, ~80 lines)
v3/crates/sindri-image/src/lib.rs             (+ exports)
v3/crates/sindri/src/commands/image.rs        (refactor list(), ~120 lines)
.github/workflows/release.yml                 (ensure GITHUB_TOKEN set)
```

**Total**: ~350 lines added/modified across 6 files

### Testing Strategy

**Unit Tests**:

```rust
#[test]
fn test_cached_metadata_staleness() {
    let old_cache = CachedImageMetadata {
        generated_at: "2025-09-01T00:00:00Z".to_string(),
        // ... 150 days ago
    };
    assert!(old_cache.is_stale());
    assert_eq!(old_cache.age_days(), 150);
}

#[test]
fn test_cached_metadata_fresh() {
    let fresh_cache = CachedImageMetadata {
        generated_at: Utc::now().to_rfc3339(),
        // ...
    };
    assert!(!fresh_cache.is_stale());
}

#[test]
fn test_load_empty_cache() {
    // Mock empty cache
    let result = load_cached_tags();
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}
```

**Integration Tests**:

```bash
# Test 1: No token, cache present
$ unset GITHUB_TOKEN
$ sindri image list
Available images for ghcr.io:pacphi/sindri:
  pacphi/sindri:v3.0.5
  pacphi/sindri:v3.0.4
  # ...

# Test 2: Invalid token
$ export GITHUB_TOKEN=invalid_token
$ sindri image list
Error: Authentication failed (401 Unauthorized).
Your GITHUB_TOKEN may be invalid or expired.
# ...

# Test 3: Valid token
$ export GITHUB_TOKEN=$VALID_TOKEN
$ sindri image list
Available images for ghcr.io:pacphi/sindri:
  pacphi/sindri:v3.0.6  # Fresh data
  # ...
```

### CI/CD Updates

```yaml
# .github/workflows/release.yml

- name: Build Release Binary
  run: cargo build --release
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }} # For build-time caching

- name: Verify Cache Embedded
  run: |
    strings target/release/sindri | grep "generated_at" || {
      echo "Warning: Image metadata not embedded"
      exit 1
    }
```

### Documentation Updates

1. **README.md**: Mention zero-setup `sindri image list`
2. **INSTALLATION.md**: Note that `GITHUB_TOKEN` is optional for basic use
3. **TROUBLESHOOTING.md**: Add section on image list authentication
4. **CONFIGURATION.md**: Document caching behavior and TTL

## Alternatives Considered

### Alternative 1: Require Token for All Users

**Rejected**: High friction, poor first-use experience, many users abandon

### Alternative 2: Proxy Service

**Approach**: Host API service that caches tags and proxies requests

**Rejected**:

- Infrastructure costs
- Maintenance burden
- Single point of failure
- Privacy concerns (tracking requests)

### Alternative 3: Distribute Token with CLI

**Approach**: Embed read-only token in binary

**Rejected**:

- Security risk (token extraction)
- GitHub ToS violation
- Token rotation nightmare
- Can't revoke without new CLI release

### Alternative 4: Config File Caching

**Approach**: Cache tags in `~/.config/sindri/image_cache.json` at runtime

**Rejected**:

- Doesn't solve first-use problem
- Requires write permissions
- Cross-platform path complexity
- Cache invalidation logic needed

### Alternative 5: Larger Cache (20+ versions)

**Rejected**:

- Binary size impact (10KB+)
- Diminishing returns (5 versions covers 95% of use cases)
- Longer build times

### Alternative 6: Shorter TTL (30 days)

**Rejected**:

- Too aggressive warnings
- Users update CLI quarterly on average
- False sense of urgency

## References

**GitHub Documentation**:

- [Working with the Container registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry)
- [About permissions for GitHub Packages](https://docs.github.com/en/packages/learn-github-packages/about-permissions-for-github-packages)

**OCI Specification**:

- [OCI Distribution Spec v2 - Listing Tags](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#content-discovery)

**Code References**:

- `v3/crates/sindri/build.rs` - Build-time metadata fetching
- `v3/crates/sindri-image/src/types.rs:200-250` - Cached metadata types
- `v3/crates/sindri-image/src/registry.rs:37-71` - Registry client tag listing
- `v3/crates/sindri/src/commands/image.rs:24-150` - Image list command with fallback

**Industry Patterns**:

- **Homebrew**: Caches formula metadata in bottle builds
- **Rust Cargo**: Caches crates.io index locally with TTL
- **npm**: Caches package metadata with staleness warnings
- **kubectl**: Embeds API resource definitions, fetches live when available
