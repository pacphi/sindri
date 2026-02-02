# Homebrew Distribution Plan

## Status: Ready for Implementation

This plan describes how to distribute Sindri v3 via Homebrew, allowing macOS and Linux users to install with `brew install pacphi/sindri/sindri`.

## Problem Statement

Direct binary downloads on macOS trigger "unidentified developer" warnings. Homebrew provides a trusted distribution channel without requiring an Apple Developer account.

## Solution Overview

Create a Homebrew tap repository (`pacphi/homebrew-sindri`) with a formula that:

- Supports macOS (ARM64 and x86_64) and Linux (ARM64 and x86_64)
- Auto-updates when new releases are published
- Includes SHA256 verification for security

## Benefits

- **Trusted by users**: Homebrew is the standard package manager for macOS
- **No Apple Developer account required**: Works immediately
- **Automatic updates**: Formula updates on each release
- **Multi-platform**: Supports macOS and Linux
- **Simple tap command**: `brew tap pacphi/sindri` (follows Homebrew convention)

## Implementation

### Step 1: Create Homebrew Tap Repository

Create new GitHub repository: `pacphi/homebrew-sindri`

Directory structure:

```
homebrew-sindri/
├── Formula/
│   └── sindri.rb
├── README.md
└── .github/
    └── workflows/
        └── update-formula.yml
```

### Step 2: Formula File

**File: `Formula/sindri.rb`**

```ruby
class Sindri < Formula
  desc "Multi-cloud development environment orchestrator (Rust)"
  homepage "https://github.com/pacphi/sindri"
  version "3.0.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/pacphi/sindri/releases/download/v#{version}/sindri-v#{version}-macos-aarch64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM64"
    else
      url "https://github.com/pacphi/sindri/releases/download/v#{version}/sindri-v#{version}-macos-x86_64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/pacphi/sindri/releases/download/v#{version}/sindri-v#{version}-linux-aarch64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    else
      url "https://github.com/pacphi/sindri/releases/download/v#{version}/sindri-v#{version}-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    end
  end

  def install
    bin.install "sindri"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/sindri --version")
  end
end
```

### Step 3: Auto-Update Workflow

**File: `.github/workflows/update-formula.yml`**

```yaml
name: Update Formula

on:
  repository_dispatch:
    types: [release-published]
  workflow_dispatch:
    inputs:
      version:
        description: "Release version (e.g., 3.0.0)"
        required: true

jobs:
  update-formula:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6

      - name: Get version
        id: version
        run: |
          if [ -n "${{ github.event.inputs.version }}" ]; then
            VERSION="${{ github.event.inputs.version }}"
          else
            VERSION="${{ github.event.client_payload.version }}"
          fi
          echo "version=$VERSION" >> $GITHUB_OUTPUT

      - name: Download releases and calculate checksums
        id: checksums
        run: |
          VERSION="${{ steps.version.outputs.version }}"
          BASE_URL="https://github.com/pacphi/sindri/releases/download/v${VERSION}"

          for platform in macos-aarch64 macos-x86_64 linux-aarch64 linux-x86_64; do
            FILE="sindri-v${VERSION}-${platform}.tar.gz"
            curl -sLO "${BASE_URL}/${FILE}"
            SHA=$(sha256sum "$FILE" | awk '{print $1}')
            echo "${platform}=${SHA}" >> $GITHUB_OUTPUT
            echo "SHA256 for $platform: $SHA"
          done

      - name: Update formula
        run: |
          VERSION="${{ steps.version.outputs.version }}"

          cat > Formula/sindri.rb << 'EOF'
          class Sindri < Formula
            desc "Multi-cloud development environment orchestrator (Rust)"
            homepage "https://github.com/pacphi/sindri"
            version "VERSION_PLACEHOLDER"
            license "Apache-2.0"

            on_macos do
              if Hardware::CPU.arm?
                url "https://github.com/pacphi/sindri/releases/download/v#{version}/sindri-v#{version}-macos-aarch64.tar.gz"
                sha256 "SHA_MACOS_ARM64"
              else
                url "https://github.com/pacphi/sindri/releases/download/v#{version}/sindri-v#{version}-macos-x86_64.tar.gz"
                sha256 "SHA_MACOS_X86_64"
              end
            end

            on_linux do
              if Hardware::CPU.arm?
                url "https://github.com/pacphi/sindri/releases/download/v#{version}/sindri-v#{version}-linux-aarch64.tar.gz"
                sha256 "SHA_LINUX_ARM64"
              else
                url "https://github.com/pacphi/sindri/releases/download/v#{version}/sindri-v#{version}-linux-x86_64.tar.gz"
                sha256 "SHA_LINUX_X86_64"
              end
            end

            def install
              bin.install "sindri"
            end

            test do
              assert_match version.to_s, shell_output("#{bin}/sindri --version")
            end
          end
          EOF

          sed -i "s/VERSION_PLACEHOLDER/${VERSION}/g" Formula/sindri.rb
          sed -i "s/SHA_MACOS_ARM64/${{ steps.checksums.outputs.macos-aarch64 }}/g" Formula/sindri.rb
          sed -i "s/SHA_MACOS_X86_64/${{ steps.checksums.outputs.macos-x86_64 }}/g" Formula/sindri.rb
          sed -i "s/SHA_LINUX_ARM64/${{ steps.checksums.outputs.linux-aarch64 }}/g" Formula/sindri.rb
          sed -i "s/SHA_LINUX_X86_64/${{ steps.checksums.outputs.linux-x86_64 }}/g" Formula/sindri.rb

      - name: Commit and push
        run: |
          VERSION="${{ steps.version.outputs.version }}"
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/sindri.rb
          git diff --staged --quiet || git commit -m "Update sindri to v${VERSION}"
          git push
```

### Step 4: Tap README

**File: `README.md`**

```markdown
# Homebrew Tap for Sindri

This tap provides Homebrew formulas for [Sindri](https://github.com/pacphi/sindri), a multi-cloud development environment orchestrator.

## Installation

\`\`\`bash
brew tap pacphi/sindri
brew install sindri
\`\`\`

## Updating

\`\`\`bash
brew upgrade sindri
\`\`\`

## Available Formulas

| Formula | Description                                                 |
| ------- | ----------------------------------------------------------- |
| sindri  | Multi-cloud development environment orchestrator (v3, Rust) |
```

### Step 5: Update Main Repository Release Workflow

**Add to `.github/workflows/release-v3.yml` (in create-release job)**

```yaml
- name: Trigger Homebrew tap update
  if: success()
  uses: peter-evans/repository-dispatch@v3
  with:
    token: ${{ secrets.HOMEBREW_TAP_TOKEN }}
    repository: pacphi/homebrew-sindri
    event-type: release-published
    client-payload: '{"version": "${{ needs.validate-tag.outputs.version }}"}'
```

### Step 6: Create GitHub Secret

Create a Personal Access Token (PAT) with `repo` scope:

1. Go to https://github.com/settings/tokens
2. Generate new token (classic) with `repo` scope
3. Add as secret `HOMEBREW_TAP_TOKEN` in the main sindri repository

### Step 7: Update Documentation

Add to main README and release notes:

```markdown
## Installation

### Homebrew (macOS/Linux) - Recommended

\`\`\`bash
brew tap pacphi/sindri
brew install sindri
\`\`\`

### Direct Download

See the [releases page](https://github.com/pacphi/sindri/releases) for platform-specific binaries.
```

## Verification

After first release with Homebrew integration:

```bash
# Test installation
brew tap pacphi/sindri
brew install sindri
sindri --version

# Validate formula
brew audit --strict pacphi/sindri/sindri

# Test formula (runs the test block)
brew test sindri
```

## Timeline

1. **Create tap repository**: `pacphi/homebrew-sindri`
2. **Add initial formula**: With placeholder SHAs
3. **Add workflow**: For auto-updates
4. **Configure secret**: `HOMEBREW_TAP_TOKEN` in main repo
5. **Update release workflow**: Add repository_dispatch step
6. **Test**: With next v3 release

## Sources

- [How to Publish your Rust project on Homebrew](https://federicoterzi.com/blog/how-to-publish-your-rust-project-on-homebrew/)
- [Automatically maintaining Homebrew formulas](https://til.simonwillison.net/homebrew/auto-formulas-github-actions)
- [Homebrew Releaser GitHub Action](https://github.com/marketplace/actions/homebrew-releaser)
- [SpectralOps Rust CI Release Template](https://github.com/SpectralOps/rust-ci-release-template)
