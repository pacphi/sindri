# Support File Version Handling

> **Feature**: Dynamic support file fetching with comprehensive version format support
> **Status**: Implementation Ready
> **Last Updated**: 2026-02-05

## Overview

This document demonstrates how Sindri v3 handles all semantic version formats when fetching support files (`common.sh`, `compatibility-matrix.yaml`, `extension-source.yaml`) from GitHub.

---

## Supported Version Formats

The `semver` crate provides full Semantic Versioning 2.0.0 support. Here are all supported formats:

### 1. **Stable Releases**

```rust
// Version: 3.0.0
let version = Version::parse("3.0.0").unwrap();
let tag = format!("v{}", version);
// Result: "v3.0.0"
// GitHub URL: https://raw.githubusercontent.com/pacphi/sindri/v3.0.0/v3/common.sh
```

### 2. **Pre-release Versions**

#### Alpha Releases

```rust
// Version: 3.0.0-alpha.18
let version = Version::parse("3.0.0-alpha.18").unwrap();
let tag = format!("v{}", version);
// Result: "v3.0.0-alpha.18"
// GitHub URL: https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.18/v3/common.sh
```

#### Beta Releases

```rust
// Version: 3.0.0-beta.3
let version = Version::parse("3.0.0-beta.3").unwrap();
let tag = format!("v{}", version);
// Result: "v3.0.0-beta.3"
// GitHub URL: https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-beta.3/v3/common.sh
```

#### Release Candidates

```rust
// Version: 3.0.0-rc.1
let version = Version::parse("3.0.0-rc.1").unwrap();
let tag = format!("v{}", version);
// Result: "v3.0.0-rc.1"
// GitHub URL: https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-rc.1/v3/common.sh
```

#### Development Builds

```rust
// Version: 3.0.0-dev
let version = Version::parse("3.0.0-dev").unwrap();
let tag = format!("v{}", version);
// Result: "v3.0.0-dev"
// GitHub URL: https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-dev/v3/common.sh
```

### 3. **Build Metadata** (Automatically Stripped)

```rust
// Version: 3.0.0+20240115
let version = Version::parse("3.0.0+20240115").unwrap();
// semver automatically strips build metadata
let tag = format!("v{}", version);
// Result: "v3.0.0" (build metadata removed)
// GitHub URL: https://raw.githubusercontent.com/pacphi/sindri/v3.0.0/v3/common.sh
```

**Why stripped?**: Build metadata doesn't affect version precedence in Semantic Versioning 2.0.0.

### 4. **Complex Pre-release Identifiers**

```rust
// Version: 3.0.0-alpha.18.special.build
let version = Version::parse("3.0.0-alpha.18.special.build").unwrap();
let tag = format!("v{}", version);
// Result: "v3.0.0-alpha.18.special.build"
```

---

## Implementation: `SupportFileManager`

### **Core Version Handling**

```rust
use semver::Version;

pub struct SupportFileManager {
    cli_version: Version,
    // ... other fields
}

impl SupportFileManager {
    /// Get current CLI version from Cargo.toml
    fn get_cli_version() -> Result<Version> {
        Version::parse(env!("CARGO_PKG_VERSION"))
            .context("Failed to parse CLI version")
    }

    /// Build GitHub tag from version
    pub fn build_tag(&self) -> String {
        format!("v{}", self.cli_version)
    }

    /// Build GitHub raw URL for a file
    fn build_github_url(&self, tag: &str, repo_path: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            self.repo_owner, self.repo_name, tag, repo_path
        )
    }
}
```

### **Version Comparison**

```rust
/// Check if support files need updating
pub async fn needs_update(&self) -> Result<bool> {
    let metadata = self.load_metadata().await?;

    // Parse stored version
    let stored_version = Version::parse(&metadata.cli_version)?;

    // Compare versions (handles all formats)
    Ok(stored_version != self.cli_version)
}
```

---

## URL Construction Examples

### **Input**: CLI Version from `Cargo.toml`

```toml
[package]
version = "3.0.0-alpha.18"
```

### **Output**: GitHub URLs

| File                          | URL                                                                                            |
| ----------------------------- | ---------------------------------------------------------------------------------------------- |
| **common.sh**                 | `https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.18/v3/common.sh`                 |
| **compatibility-matrix.yaml** | `https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.18/v3/compatibility-matrix.yaml` |
| **extension-source.yaml**     | `https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.18/v3/extension-source.yaml`     |

---

## Version Metadata Tracking

### **Metadata File**: `~/.sindri/.support-files-metadata.yaml`

```yaml
# Current state
cli_version: "3.0.0-alpha.18"
fetched_at: "2026-02-05T14:30:00Z"
source: github
github_tag: "v3.0.0-alpha.18"
```

### **After Upgrade**: `sindri` upgraded to `3.0.0-alpha.19`

```yaml
# Automatically updated
cli_version: "3.0.0-alpha.19"
fetched_at: "2026-02-05T15:00:00Z"
source: github
github_tag: "v3.0.0-alpha.19"
```

---

## Upgrade Lifecycle

### **Scenario 1: Major Version Upgrade**

```
Current:  3.0.0-alpha.18 → Upgrade to: 3.0.0
```

**Flow:**

1. Docker image updated with sindri v3.0.0
2. Container starts → `entrypoint.sh` runs
3. Checks metadata: `3.0.0-alpha.18` != `3.0.0` ✅ Update needed
4. Fetches from: `https://raw.githubusercontent.com/.../v3.0.0/v3/*.yaml`
5. Updates metadata to `3.0.0`

### **Scenario 2: Pre-release Iteration**

```
Current:  3.0.0-alpha.18 → Upgrade to: 3.0.0-alpha.19
```

**Flow:**

1. Docker image updated with sindri v3.0.0-alpha.19
2. Container starts → version mismatch detected
3. Fetches from: `https://raw.githubusercontent.com/.../v3.0.0-alpha.19/v3/*.yaml`
4. Updates metadata to `3.0.0-alpha.19`

### **Scenario 3: Hotfix with Build Metadata**

```
Current:  3.0.0 → Rebuild: 3.0.0+hotfix-20240115
```

**Flow:**

1. Docker image rebuilt with `3.0.0+hotfix-20240115`
2. `semver` strips build metadata → `3.0.0`
3. Checks metadata: `3.0.0` == `3.0.0` ✅ No update needed
4. Reuses existing support files

---

## Edge Cases Handled

### **1. Invalid Version String**

```rust
// Input: "invalid-version"
match Version::parse("invalid-version") {
    Ok(v) => { /* Use version */ },
    Err(e) => {
        // Fall back to bundled files
        return self.update_from_bundled().await;
    }
}
```

### **2. GitHub Tag Doesn't Exist**

```rust
// Trying to fetch: v3.0.0-alpha.99 (doesn't exist)
match self.fetch_from_github(&tag, file).await {
    Ok(content) => { /* Success */ },
    Err(e) => {
        warn!("Failed to fetch from GitHub: {}", e);
        // Fall back to bundled files
        self.copy_bundled(file, &dest_path).await?;
    }
}
```

### **3. Network Unavailable**

```rust
// reqwest::get() fails (no network)
Err(NetworkError) => {
    warn!("Network unavailable, using bundled files");
    self.update_from_bundled().await?;
}
```

### **4. Version Downgrade**

```rust
// Current: 3.0.0-alpha.19 → Downgrade to: 3.0.0-alpha.18
let needs_update = stored_version != self.cli_version;
// Result: true (any version mismatch triggers update)
```

---

## Testing Matrix

### **Unit Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_version() {
        let v = Version::parse("3.0.0").unwrap();
        assert_eq!(format!("v{}", v), "v3.0.0");
    }

    #[test]
    fn test_alpha_version() {
        let v = Version::parse("3.0.0-alpha.18").unwrap();
        assert_eq!(format!("v{}", v), "v3.0.0-alpha.18");
    }

    #[test]
    fn test_beta_version() {
        let v = Version::parse("3.0.0-beta.3").unwrap();
        assert_eq!(format!("v{}", v), "v3.0.0-beta.3");
    }

    #[test]
    fn test_rc_version() {
        let v = Version::parse("3.0.0-rc.1").unwrap();
        assert_eq!(format!("v{}", v), "v3.0.0-rc.1");
    }

    #[test]
    fn test_dev_version() {
        let v = Version::parse("3.0.0-dev").unwrap();
        assert_eq!(format!("v{}", v), "v3.0.0-dev");
    }

    #[test]
    fn test_build_metadata_stripped() {
        let v = Version::parse("3.0.0+20240115").unwrap();
        assert_eq!(format!("v{}", v), "v3.0.0");
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("3.0.0-alpha.18").unwrap();
        let v2 = Version::parse("3.0.0-alpha.19").unwrap();
        assert_ne!(v1, v2);
        assert!(v1 < v2);
    }

    #[test]
    fn test_build_metadata_equality() {
        let v1 = Version::parse("3.0.0").unwrap();
        let v2 = Version::parse("3.0.0+hotfix").unwrap();
        // Build metadata is stripped, so they're equal
        assert_eq!(v1, v2);
    }
}
```

---

## CLI Usage

### **Check current support file version**

```bash
$ sindri extension support-files status
Support Files Status:
  CLI Version:    3.0.0-alpha.18
  Files Version:  3.0.0-alpha.18
  Source:         github
  Fetched At:     2026-02-05 14:30:00 UTC
  Status:         ✓ Up-to-date
```

### **Force update support files**

```bash
$ sindri extension support-files update --force
Updating support files for CLI v3.0.0-alpha.19...
✓ Fetched common.sh from GitHub
✓ Fetched compatibility-matrix.yaml from GitHub
✓ Fetched extension-source.yaml from GitHub
Support files updated successfully (3 files from GitHub)
```

### **Update from bundled (offline mode)**

```bash
$ sindri extension support-files update --bundled
Updating support files from bundled sources (offline mode)...
✓ Copied common.sh from bundled
✓ Copied compatibility-matrix.yaml from bundled
✓ Copied extension-source.yaml from bundled
Support files updated successfully (3 files from bundled)
```

---

## Advantages of This Approach

1. ✅ **Full Semantic Versioning Support**: Handles all SemVer 2.0.0 formats
2. ✅ **Automatic Version Matching**: Files always match CLI version
3. ✅ **Zero-downtime Upgrades**: Update files without image rebuild
4. ✅ **Offline Capable**: Falls back to bundled files
5. ✅ **Transparent**: Clear metadata tracking
6. ✅ **Type-safe**: Rust's type system prevents version parsing errors
7. ✅ **Testable**: Comprehensive unit test coverage

---

## Related Documentation

- [`support_files.rs`](../../../crates/sindri-extensions/src/support_files.rs) - Implementation
- [Semantic Versioning 2.0.0](https://semver.org/) - Specification
- [semver crate](https://docs.rs/semver/) - Rust semver library
- [SOURCING_MODES.md](SOURCING_MODES.md) - Extension loading modes
