# mise Distribution Plan

## Status: Planned — pending release pipeline integration

This plan describes how to distribute Sindri v3 via [mise](https://mise.jdx.dev/), the polyglot runtime manager, allowing users to install with `mise use sindri`.

## Problem Statement

Users want a simple, cross-platform way to install and manage Sindri versions. mise is increasingly popular among developers (23k+ GitHub stars, 950+ tools) and provides:

- Unified tool management across languages and platforms
- Version pinning per-project via `.mise.toml`
- Automatic version switching when entering project directories
- Checksum verification for security

## Solution Overview

Add Sindri v3 to the [mise registry](https://github.com/jdx/mise/blob/main/registry.toml) using the GitHub backend, which:

- Downloads release assets directly from GitHub releases
- Supports all platforms (Linux, macOS, Windows)
- Includes checksum verification
- Requires no separate registry maintenance

## Backend Selection (2026 Best Practices)

| Backend    | Pros                                                    | Cons                                           | Recommendation     |
| ---------- | ------------------------------------------------------- | ---------------------------------------------- | ------------------ |
| **aqua**   | Best security (Cosign/SLSA), mise reimplements natively | Requires separate PR to aquaproj/aqua-registry | Future enhancement |
| **github** | Simple, auto-detects assets, good security              | Less verification than aqua                    | **Start here**     |
| **asdf**   | Legacy support                                          | No longer accepted for new tools               | Not recommended    |

**Decision**: Use `github` backend initially. Consider aqua-registry submission later for enhanced supply chain security.

## Benefits

- **Cross-platform**: Works on Linux, macOS, and Windows
- **Version management**: Users can install specific versions or latest
- **Project-local**: Teams can pin versions in `.mise.toml`
- **No Apple Developer needed**: Works immediately (unlike direct downloads on macOS)
- **Trusted distribution**: mise handles verification

## Implementation

### Step 1: Ensure Checksums in Release

Already implemented in release workflow. Each release includes:

- `checksums.txt` - SHA256 checksums for all binaries
- Individual `.sha256` files per asset

### Step 2: Submit PR to mise Registry

**File to modify**: `registry.toml` in [jdx/mise](https://github.com/jdx/mise)

```toml
[tools.sindri]
description = "Multi-cloud development environment orchestrator"
backends = ["github:pacphi/sindri"]
test = ["sindri --version", "{{version}}"]

[tools.sindri.github]
# Asset naming pattern: sindri-v{version}-{platform}.tar.gz
# Examples:
#   sindri-v3.0.0-linux-x86_64.tar.gz
#   sindri-v3.0.0-macos-aarch64.tar.gz
#   sindri-v3.0.0-windows-x86_64.zip
version_prefix = "v"
```

### Step 3: PR Submission Process

1. Fork [jdx/mise](https://github.com/jdx/mise)
2. Edit `registry.toml` to add sindri entry
3. Create PR with title: `registry: add sindri (github:pacphi/sindri)`
4. GitHub Actions will auto-generate additional config
5. Wait for maintainer review

### Step 4: Update Documentation

After PR is merged, update main README:

```markdown
## Installation

### mise (Recommended)

\`\`\`bash

# Install latest version

mise use -g sindri

# Install specific version

mise use -g sindri@3.0.0

# Per-project version

mise use sindri@3.0.0 # Creates .mise.toml
\`\`\`

### Homebrew (macOS/Linux)

\`\`\`bash
brew tap pacphi/sindri
brew install sindri
\`\`\`

### Direct Download

See the [releases page](https://github.com/pacphi/sindri/releases).
```

## User Experience

After mise registry PR is merged:

```bash
# Global installation
mise use -g sindri
sindri --version
# Sindri 3.0.0 (rustc 1.93.0)

# Project-specific version
cd my-project
mise use sindri@3.0.0
cat .mise.toml
# [tools]
# sindri = "3.0.0"

# Update to latest
mise upgrade sindri
```

## Verification

mise automatically verifies:

1. **Asset authenticity** - Downloaded from GitHub releases
2. **Checksums** - SHA256 verification via checksums.txt
3. **Version matching** - Ensures correct version installed

Users can verify manually:

```bash
# Check installed version
mise ls sindri

# Verify binary
sindri --version
```

## Future Enhancements

### Phase 2: Aqua Registry (Optional)

For enhanced supply chain security, submit to [aquaproj/aqua-registry](https://github.com/aquaproj/aqua-registry):

1. Adds Cosign signature verification
2. Adds SLSA provenance checking
3. Requires `cmdx s pacphi/sindri` generated config

**File structure** (in aquaproj/aqua-registry):

```
pkgs/pacphi/sindri/
├── pkg.yaml          # Test versions
├── registry.yaml     # Package config
└── scaffold.yaml     # Optional generation config
```

**Example registry.yaml**:

```yaml
packages:
  - type: github_release
    repo_owner: pacphi
    repo_name: sindri
    asset: sindri-v{{.Version}}-{{.OS}}-{{.Arch}}.tar.gz
    format: tar.gz
    files:
      - name: sindri
    supported_envs:
      - linux/amd64
      - linux/arm64
      - darwin/amd64
      - darwin/arm64
      - windows/amd64
    replacements:
      amd64: x86_64
      arm64: aarch64
      darwin: macos
    checksum:
      type: github_release
      asset: checksums.txt
      algorithm: sha256
```

After aqua-registry PR is merged, update mise registry entry:

```toml
[tools.sindri]
backends = [
  "aqua:pacphi/sindri",   # Preferred (Cosign/SLSA)
  "github:pacphi/sindri", # Fallback
]
```

## Timeline

1. **Now**: Checksums added to release workflow
2. **Next**: Submit PR to jdx/mise registry
3. **After merge**: Update README with mise instructions
4. **Future**: Consider aqua-registry for enhanced security

## Sources

- [mise Getting Started](https://mise.jdx.dev/getting-started.html)
- [mise Registry](https://mise.jdx.dev/registry.html)
- [mise GitHub Backend](https://mise.jdx.dev/dev-tools/backends/github.html)
- [mise Aqua Backend](https://mise.jdx.dev/dev-tools/backends/aqua.html)
- [mise 2026 Updates](https://github.com/jdx/mise/discussions/7727)
- [Contributing to aqua-registry](https://aquaproj.github.io/docs/products/aqua-registry/contributing/)
