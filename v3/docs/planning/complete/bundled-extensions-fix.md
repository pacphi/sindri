# Bundled Extensions Support - Implementation Summary

## Problem

When building Sindri from source with `buildFromSource.enabled: true`, the v3 CLI was trying to fetch extensions from GitHub (`sindri/sindri-extensions` repository) which doesn't exist yet. This caused extension installation to fail with:

```
Failed to fetch compatibility matrix
Error: Failed to parse compatibility matrix
Caused by: missing field `schema_version`
```

## Root Cause

1. Extensions were bundled at `/opt/sindri/extensions` in Dockerfile.dev
2. `compatibility-matrix.yaml` was NOT copied to the image
3. `ExtensionDistributor` always attempted to fetch from GitHub
4. No logic to detect "bundled mode" and use local files

## Solution

Implemented three fixes to support bundled extensions:

### Fix 1: Bundle compatibility-matrix.yaml in Dockerfile.dev

**File:** `v3/Dockerfile.dev`

Added compatibility matrix to bundled files:

```dockerfile
COPY --chown=${DEVELOPER_USER}:${DEVELOPER_USER} v3/compatibility-matrix.yaml /opt/sindri/compatibility-matrix.yaml
```

### Fix 2: Support bundled compatibility matrix

**File:** `v3/crates/sindri-extensions/src/distribution.rs`

Modified `get_compatibility_matrix()` to:

1. Check if `SINDRI_EXT_HOME` environment variable is set (indicates bundled mode)
2. If set, load from `/opt/sindri/compatibility-matrix.yaml`
3. Fall back to cache/GitHub only if bundled file not found

```rust
pub async fn get_compatibility_matrix(&self) -> Result<CompatibilityMatrix> {
    // Check for bundled mode (build-from-source with extensions at /opt/sindri)
    if let Ok(ext_home) = std::env::var("SINDRI_EXT_HOME") {
        let bundled_path = std::path::PathBuf::from(&ext_home)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("/opt/sindri"))
            .join("compatibility-matrix.yaml");

        if bundled_path.exists() {
            debug!("Using bundled compatibility matrix from {:?}", bundled_path);
            let content = fs::read_to_string(&bundled_path).await?;
            return serde_yaml_ng::from_str(&content)?;
        }
    }

    // Fall back to cache/GitHub...
}
```

### Fix 3: Support bundled extension installation

**File:** `v3/crates/sindri-extensions/src/distribution.rs`

Added two helper methods:

1. `get_bundled_extension_dir()` - Check if extension exists in SINDRI_EXT_HOME
2. `get_bundled_extension_version()` - Get version of bundled extension

Modified `install()` to:

1. Check for bundled extensions first
2. Use bundled extension if available and compatible
3. Fall back to GitHub download only if not bundled

```rust
// In install method:
let ext_dir = if let Some(bundled_dir) = self.get_bundled_extension_dir(name).await? {
    info!("Using bundled extension from {:?}", bundled_dir);
    bundled_dir
} else {
    // Download from GitHub...
};
```

## Detection Logic

**Bundled Mode Detection:**

- Checks for `SINDRI_EXT_HOME` environment variable
- In Dockerfile.dev: `ENV SINDRI_EXT_HOME=/opt/sindri/extensions`
- Extensions at: `/opt/sindri/extensions/<extension-name>/`
- Compatibility matrix at: `/opt/sindri/compatibility-matrix.yaml`

**Fallback to GitHub:**

- Only when `SINDRI_EXT_HOME` is not set
- Or when bundled files don't exist
- Maintains backward compatibility with production images

## Testing

### Manual Test in Container

```bash
# Build and deploy with buildFromSource
sindri deploy

# Connect to container
sindri connect

# Inside container, verify bundled mode
echo $SINDRI_EXT_HOME  # Should show: /opt/sindri/extensions
ls /opt/sindri/        # Should show: extensions/ registry.yaml profiles.yaml compatibility-matrix.yaml

# Install an extension
sindri extension install jvm

# Verify it uses bundled extension (check logs for "Using bundled extension")
```

### Expected Behavior

**Before Fix:**

```
⬢ [Docker] ❯ sindri extension install jvm
ℹ Installing extension: jvm
✗ Failed to install jvm: Failed to fetch compatibility matrix
Error: Failed to fetch compatibility matrix
Caused by:
    0: Failed to parse compatibility matrix
    1: missing field `schema_version`
```

**After Fix:**

```
⬢ [Docker] ❯ sindri extension install jvm
ℹ Installing extension: jvm
✓ Using bundled extension from /opt/sindri/extensions/jvm
✓ Successfully installed jvm 1.0.0
```

## Files Modified

1. `v3/Dockerfile.dev` - Added compatibility-matrix.yaml to bundled files
2. `v3/crates/sindri-extensions/src/distribution.rs` - Added bundled mode support

## Compatibility

- **Bundled mode** (buildFromSource): Uses `/opt/sindri` files
- **Production mode** (pre-built image): Falls back to GitHub/cache
- **Development mode**: Works with both approaches
- **Air-gapped deployments**: Fully supported with bundled mode

## Future Enhancements

1. Add validation that bundled extension versions match compatibility matrix
2. Support mixed mode (some bundled, some downloaded)
3. Add CLI flag to force GitHub download even in bundled mode
4. Improve logging to clearly indicate bundled vs. downloaded extensions
