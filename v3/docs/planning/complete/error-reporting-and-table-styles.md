# Enhanced Error Reporting and Table Styles

## Overview

This document describes the improvements made to profile installation error reporting and table visual styles in Sindri v3.

## Changes Summary

### 1. Enhanced Error Reporting

#### Problem
When profile installations failed, users only saw extension names without context about:
- What phase the failure occurred in (source resolution, download, install, validate)
- The actual error message
- Which source the extension came from (bundled, downloaded, local-dev)

#### Solution

**New Data Structures** (`v3/crates/sindri-extensions/src/profile.rs`):

```rust
/// Installation phase where an error occurred
pub enum InstallPhase {
    SourceResolution,  // Error finding extension
    Download,          // Error downloading from GitHub
    Install,           // Error during installation
    Validate,          // Error during validation
}

/// Failed extension with detailed information
pub struct FailedExtension {
    pub name: String,
    pub error: String,
    pub phase: InstallPhase,
    pub source: Option<String>,  // "bundled", "downloaded", or "local-dev"
}

/// Successfully installed extension information
pub struct InstalledExtension {
    pub name: String,
    pub version: String,
    pub source: String,  // Tracks where it came from
}
```

**Enhanced Display** (`v3/crates/sindri/src/commands/profile.rs`):

Before:
```
Failed extensions:
  - python
  - nodejs
```

After:
```
2 extension(s) failed to install:

  ✗ python
    Phase:  Install
    Source: downloaded
    Error:  Failed to execute install script: permission denied

  ✗ nodejs
    Phase:  Validate
    Source: bundled
    Error:  Validation command 'node --version' failed with exit code 1

Tip: Run with RUST_LOG=debug for detailed logs
```

#### Source Type Tracking

Extensions now track their source dynamically using `ExtensionSourceResolver`:

- **Bundled**: Extensions from `/opt/sindri/extensions` (Docker images)
- **Downloaded**: Extensions from `~/.sindri/extensions` (GitHub releases)
- **Local-dev**: Extensions from `v3/extensions` (development mode)

The source is no longer hard-coded but determined at runtime:

```rust
// Determine source dynamically
let source_type = resolver.find_source(ext_name)
    .map(|s| s.to_string())
    .unwrap_or_else(|| "github".to_string());

// Use dynamic source in manifest
self.manifest.mark_installed(name, &version, &source_type)?;
```

### 2. Modern Table Styles

#### Problem
Tables used ASCII box-drawing characters that were visually heavy:

```
+--------------------+-----------------+---------+-----------+---------------+
| name               | category        | version | installed | description   |
+--------------------+-----------------+---------+-----------+---------------+
| python             | language        | latest  | 3.13.0    | Python...     |
+--------------------+-----------------+---------+-----------+---------------+
```

#### Solution

Applied `Style::sharp()` from the `tabled` crate for cleaner, more compact output:

```
┌────────────────────┬─────────────────┬─────────┬───────────┬───────────────┐
│ name               │ category        │ version │ installed │ description   │
├────────────────────┼─────────────────┼─────────┼───────────┼───────────────┤
│ python             │ language        │ latest  │ 3.13.0    │ Python...     │
│ nodejs             │ language        │ latest  │ 20.0.0    │ Node.js...    │
└────────────────────┴─────────────────┴─────────┴───────────┴───────────────┘
```

**Why `sharp()` over `modern()`:**
- No dividers between data rows (only below header)
- More compact vertical spacing
- Easier to scan large lists
- Less visual clutter while maintaining clarity

**Files Modified:**
- `v3/crates/sindri/src/commands/extension.rs` - 4 tables updated
- `v3/crates/sindri/src/commands/profile.rs` - 2 tables updated
- `v3/crates/sindri/src/commands/k8s.rs` - 1 table updated

**Change Pattern:**

```rust
// Before
let table = Table::new(data);
println!("{}", table);

// After
use tabled::settings::Style;

let mut table = Table::new(data);
table.with(Style::sharp());
println!("{}", table);
```

## Benefits

### Error Reporting
1. **Clear Phase Identification**: Users immediately know where installation failed
2. **Source Awareness**: Users can distinguish between bundled (offline) and downloaded (online) failures
3. **Actionable Errors**: Full error messages help debug issues
4. **Better Support**: Users can provide detailed error reports

### Table Styles
1. **Reduced Visual Clutter**: Sharp style removes row dividers for cleaner output
2. **Better Readability**: No lines between rows makes scanning faster and easier
3. **Consistent Presentation**: All tables use the same sharp style
4. **Compact Display**: Takes less vertical space, more information visible at once

## Implementation Details

### Error Classification

Errors are automatically classified by analyzing error messages:

```rust
fn classify_error(error: &anyhow::Error) -> (InstallPhase, String) {
    let error_str = error.to_string().to_lowercase();

    let phase = if error_str.contains("not found") || error_str.contains("definition not loaded") {
        InstallPhase::SourceResolution
    } else if error_str.contains("download") || error_str.contains("fetch") {
        InstallPhase::Download
    } else if error_str.contains("validation") || error_str.contains("validate") {
        InstallPhase::Validate
    } else {
        InstallPhase::Install
    };

    (phase, error.to_string())
}
```

### Bundled vs Downloaded Mode

The system automatically detects the source mode:

```rust
// Bundled mode (Docker): SINDRI_EXT_HOME=/opt/sindri/extensions
if bundled.is_available("python") {
    source = "bundled"
}

// Downloaded mode: ~/.sindri/extensions
else if downloaded.is_available("python") {
    source = "downloaded"
}

// Development mode: v3/extensions
else if local_dev.is_available("python") {
    source = "local-dev"
}
```

## Testing

Compilation verified for:
- `sindri-extensions` package
- `sindri` CLI package

Tests updated to match new `ProfileInstallResult` structure.

## Alternative Table Styles

The `tabled` crate supports several styles. We chose `sharp()` for optimal readability, but other options include:

```rust
Style::sharp()    // Sharp corners, no row dividers (CURRENT)
Style::modern()   // Sharp corners with row dividers between every row
Style::rounded()  // Rounded corners ╭╮╰╯
Style::blank()    // No borders, space-separated
Style::ascii()    // Pure ASCII with + and |
```

**Comparison:**

| Style | Row Dividers | Compactness | Best For |
|-------|--------------|-------------|----------|
| `sharp()` ✅ | Header only | High | Large lists, quick scanning |
| `modern()` | Every row | Medium | Structured data with row emphasis |
| `rounded()` | Every row | Medium | Softer aesthetic |
| `blank()` | None | Very high | Minimal output |
| `ascii()` | Every row | Low | Terminal compatibility |

To change the style, modify the `.with(Style::sharp())` calls to use a different style.

## Future Enhancements

Potential improvements:
1. Add retry logic for download failures
2. Show progress indicators for each phase
3. Export detailed error logs to file
4. Add `--format` flag for table styles (modern, sharp, ascii)
5. Capture installation duration metrics per extension
