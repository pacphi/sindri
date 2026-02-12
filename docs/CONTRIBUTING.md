# Contributing to Sindri

Thank you for your interest in contributing to Sindri! This guide will help you get started.

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the technical merit of contributions
- Help others learn and grow

## Development Versions

Sindri has two active development versions:

- **V2** (Bash/Docker): The original implementation using shell scripts, Docker containers, and pnpm tooling. See [V2 Development](#v2-development) below.
- **V3** (Rust CLI): The next-generation CLI rewritten in Rust as a cargo workspace. See [V3 Development](#v3-development) below.

Choose the section below that matches your work.

---

## V2 Development

## Development Setup

### Prerequisites

- Docker 20.10+
- Node.js 18+ (for pnpm scripts)
- pnpm 8+
- yq 4+
- Optional: flyctl (for Fly.io testing)

### Initial Setup

```bash
# Clone repository
git clone https://github.com/pacphi/sindri
cd sindri

# Install dependencies
pnpm install

# Validate installation
pnpm validate
```

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/my-new-feature
```

### 2. Make Changes

Follow the project structure:

- **Extensions:** `v2/docker/lib/extensions/<name>/extension.yaml`
- **CLI tools:** `cli/`
- **Deployment adapters:** `v2/deploy/adapters/`
- **Documentation:** `docs/`
- **Tests:** `test/unit/yaml/`

### 3. Validate Changes

```bash
# Run all validations
pnpm validate

# Specific validations
pnpm lint:yaml          # YAML linting
pnpm lint:shell         # Shell script linting
pnpm lint:md            # Markdown linting
```

### 4. Test Changes

```bash
# Run tests
pnpm test

# Test specific extension
./v2/cli/extension-manager validate <extension-name>

# Test Docker build
pnpm build
```

### 5. Commit Changes

Follow conventional commit format:

```bash
git commit -m "feat(extensions): add postgresql extension"
git commit -m "fix(docker): resolve volume permission issue"
git commit -m "docs(readme): update quickstart guide"
```

**Commit types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Test updates
- `refactor`: Code refactoring
- `chore`: Maintenance tasks

### 6. Push and Create PR

```bash
git push origin feature/my-new-feature
```

Create pull request on GitHub with:

- Clear title and description
- Reference any related issues
- Include screenshots/examples if applicable

## Adding Extensions

### Extension Development Process

1. **Create extension directory:**

   ```bash
   mkdir -p v2/docker/lib/extensions/myext
   ```

2. **Create `extension.yaml`:**

   ```yaml
   metadata:
     name: myext
     version: 1.0.0
     description: My extension
     category: dev-tools

   requirements:
     diskSpace: 100

   install:
     method: mise
     mise:
       configFile: mise.toml

   validate:
     commands:
       - name: mytool
         expectedPattern: "v\\d+"
   ```

3. **Add to registry:**

   ```yaml
   # v2/docker/lib/registry.yaml
   extensions:
     myext:
       category: dev-tools
       description: My extension
   ```

4. **Validate:**

   ```bash
   ./cli/extension-manager validate myext
   ```

5. **Test in Docker:**

   ```bash
   pnpm build
   docker run -it sindri:local
   extension-manager install myext
   ```

See: [Extension Authoring Guide](../v2/docs/EXTENSION_AUTHORING.md)

## Testing Guidelines

### Unit Tests

Located in `test/unit/`:

```bash
pnpm test:unit
```

### Integration Tests

Located in `.github/scripts/`:

```bash
pnpm test:integration
```

### Extension Tests

Test all extensions:

```bash
pnpm test:extensions
```

Test specific extension locally:

```bash
./v2/cli/extension-manager validate <extension-name>
```

### Manual Testing

1. Build image:

   ```bash
   pnpm build
   ```

2. Run container:

   ```bash
   docker run -it -v sindri-test:/workspace sindri:local
   ```

3. Install extensions:

   ```bash
   extension-manager install myext
   ```

4. Validate:

   ```bash
   extension-manager validate myext
   ```

## Code Style

### Shell Scripts

- Use `#!/usr/bin/env bash`
- Include `set -euo pipefail`
- Source `v2/docker/lib/common.sh` for shared functions
- Use functions from common.sh:
  - `print_status`, `print_success`, `print_warning`, `print_error`
- Pass `shellcheck -S warning`

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/../common.sh"

main() {
    print_status "Starting task..."
    # Implementation
    print_success "Task completed"
}

main "$@"
```

### YAML Files

- 2-space indentation
- Pass `yamllint --strict`
- Validate against schemas in `v2/docker/lib/schemas/`

### Markdown

- Pass `markdownlint`
- Format with `prettier`
- Include code examples
- Link to related documentation

### Link Validation

Validate internal and external links in markdown files:

```bash
# Check internal markdown links
find docs -name "*.md" -o -name "README.md" | while read file; do
  echo "Checking: $file"
  grep -oP '\[.*?\]\(\K[^)]+(?=\))' "$file" | while read link; do
    # Skip external URLs and anchors
    if [[ ! "$link" =~ ^(https?://|mailto:|#) ]]; then
      link_path="${link%%#*}"
      if [[ -n "$link_path" ]]; then
        dir=$(dirname "$file")
        if [[ "$link_path" =~ ^/ ]]; then
          target=".$link_path"
        else
          target="$dir/$link_path"
        fi
        if [[ ! -e "$target" ]]; then
          echo "❌ BROKEN: $file -> $link"
        fi
      fi
    fi
  done
done

# Check external URLs (requires markdown-link-check)
pnpm add -g markdown-link-check
find docs -name "*.md" | xargs -n1 markdown-link-check -q

# Automated CI check
# Links are automatically validated on PR via .github/workflows/check-links.yml
```

**CI Integration:**

- Internal links checked on every PR
- External URLs checked weekly (scheduled)
- Anchor links validated for correctness

---

## V3 Development

### Prerequisites

- Rust 1.93+ (install via [rustup](https://rustup.rs/))
- cargo (comes with Rust)
- Optional: [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) for code coverage (`rustup component add llvm-tools-preview && cargo install cargo-llvm-cov`)

### Setup

```bash
cd v3
cargo build
cargo test --workspace
```

### Workspace Structure

V3 is organized as a Cargo workspace with 12 crates:

| Crate               | Description                                      |
| ------------------- | ------------------------------------------------ |
| `sindri`            | CLI binary entry point (clap-based)              |
| `sindri-core`       | Shared types, configuration, and utilities       |
| `sindri-providers`  | Cloud provider adapters (Docker, Fly.io, DevPod) |
| `sindri-extensions` | Extension registry, resolution, and lifecycle    |
| `sindri-secrets`    | Secrets management (encryption, vaults)          |
| `sindri-backup`     | Backup and restore operations                    |
| `sindri-projects`   | Project scaffolding and management               |
| `sindri-doctor`     | Environment diagnostics and health checks        |
| `sindri-clusters`   | Kubernetes cluster management                    |
| `sindri-image`      | Container image management                       |
| `sindri-packer`     | Image packing and distribution                   |
| `sindri-update`     | Self-update mechanism                            |

### Development Workflow

```bash
# Build all crates
cargo build

# Run all tests
cargo test --workspace

# Test a specific crate
cargo test -p sindri-core

# Lint all crates
cargo clippy --workspace

# Format all code
cargo fmt --all

# Check formatting without writing
cargo fmt --all -- --check

# Build in release mode
cargo build --release

# Code coverage (requires cargo-llvm-cov)
cargo llvm-cov --workspace --html --output-dir coverage/
```

### V3 Testing Guidelines

- Unit tests go in `#[cfg(test)] mod tests` blocks within each source file
- Use `tempfile::TempDir` for tests that touch the filesystem
- Use `#[tokio::test]` for async tests
- Use `serial_test::serial` when tests share global state (e.g., environment variables)
- Run a single test by name: `cargo test -p sindri-core test_name`

Example test structure:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_loads_defaults() {
        let tmp = TempDir::new().unwrap();
        // ...
    }

    #[tokio::test]
    async fn test_provider_connect() {
        // ...
    }

    #[test]
    #[serial_test::serial]
    fn test_env_var_handling() {
        // ...
    }
}
```

### V3 Code Style

- **Error handling:** `anyhow::Result` for application-level errors, `thiserror` for library crate error types
- **Async runtime:** tokio; use `#[async_trait]` for async trait methods
- **CLI parsing:** clap derive macros (`#[derive(Parser)]`, `#[derive(Subcommand)]`)
- **Serialization:** serde with `serde_yaml_ng` (not the deprecated `serde_yaml`)
- **Path handling:** `camino::Utf8PathBuf` for UTF-8 guaranteed paths
- **Formatting:** Standard `rustfmt` defaults; run `cargo fmt --all` before committing
- **Linting:** All code must pass `cargo clippy --workspace` with no warnings

### V3 Extension Development

V3 uses a different extension system from V2. Extensions are defined as YAML manifests with JSON Schema validation. For full details, see the [V3 Extension Guide](../v3/docs/EXTENSIONS.md).

### V3 Documentation

V3-specific documentation lives in `v3/docs/`. Key guides:

- [CLI Reference](../v3/docs/CLI.md)
- [Architecture](../v3/docs/ARCHITECTURE.md)
- [Getting Started](../v3/docs/GETTING_STARTED.md)
- [Extensions](../v3/docs/EXTENSIONS.md)
- [Configuration](../v3/docs/CONFIGURATION.md)
- [Schema Reference](../v3/docs/SCHEMA.md)

---

## Shared Guidelines

The following sections apply to both V2 and V3 development.

## Documentation

### Documentation Structure

- **README.md** - Teaser with links to detailed docs
- **docs/** - Detailed documentation
- **CLAUDE.md** - Developer guide for Claude Code
- **Extension README** - Per-extension documentation (optional)

### Documentation Naming Conventions

All documentation files must follow these naming standards:

| Document Type                 | Naming Pattern                  | Example                                               |
| ----------------------------- | ------------------------------- | ----------------------------------------------------- |
| Core documentation            | `UPPER_CASE_UNDERSCORE.md`      | `GETTING_STARTED.md`, `EXTENSION_AUTHORING.md`        |
| Extension documentation       | `UPPER-CASE-HYPHEN.md`          | `NODEJS-DEVTOOLS.md`, `AI-TOOLKIT.md`                 |
| Architecture Decision Records | `NNN-kebab-case-description.md` | `001-extension-system.md`, `021-ci-workflow-split.md` |

**Directory organization:**

- Version-specific docs go under `v2/docs/` or `v3/docs/`
- Shared/version-agnostic docs go under `docs/shared/`
- Migration and comparison guides go under `docs/shared/migration/`

**Rationale:**

- `UPPER_CASE` signals these are project documentation (not code)
- Consistent naming enables automated validation via CI
- Clear separation prevents version confusion

### Version Tagging Requirements

All version-specific documentation **must** include a version header at the top of the file:

```markdown
# Document Title

> This documentation applies to **Sindri V2**

---

(rest of content)
```

Or for V3:

```markdown
# Document Title

> This documentation applies to **Sindri V3**

---

(rest of content)
```

**When to use version tags:**

- Any document describing V2-only or V3-only features
- Installation/setup guides specific to a version
- Extension documentation (V2 and V3 have different extension systems)
- Architecture documentation (different implementations)

**When NOT to use version tags:**

- Migration guides (they cover both versions)
- Comparison guides (they cover both versions)
- IDE integration docs (version-agnostic)
- This contributing guide (applies to all versions)

### Cross-Reference Guidelines

When linking between documentation files:

**Use relative paths:**

```markdown
<!-- Good - relative path -->

See [My Doc](./MY_DOC.md)

<!-- Bad - absolute path -->

See [My Doc](/path/to/MY_DOC.md)
```

**Always verify links exist:**

```bash
# Run link checker before committing
pnpm lint:md

# Or manually check
find docs -name "*.md" -exec grep -l "broken-link" {} \;
```

**Update references when moving files:**

1. Before moving: search for all references to the file
2. Move the file
3. Update all references to the new location
4. Run link checker to verify

```bash
# Find all references to a file
grep -r "EXTENSIONS.md" docs/ v2/docs/ v3/docs/
```

**Cross-version references:**

When linking from V2 docs to V3 docs (or vice versa), use explicit version paths:

```markdown
<!-- From v2/docs/SOME_DOC.md linking to v3 -->

For V3 docs, see [V3 Doc](../../v3/docs/SOME_DOC.md)

<!-- From v3/docs/SOME_DOC.md linking to v2 -->

For V2 docs, see [V2 Doc](../../v2/docs/SOME_DOC.md)
```

### Writing Documentation

- **Be concise** - Users scan, not read
- **Use examples** - Show, don't just tell
- **Link liberally** - Connect related topics
- **Keep updated** - Update docs with code changes

### Documentation Checklist

When adding features:

- [ ] Update README.md if user-facing
- [ ] Add/update docs/ files
- [ ] Update CLAUDE.md if architecture changes
- [ ] Add examples if applicable
- [ ] Update schema files

## CI/CD

### GitHub Actions Workflows

- **ci.yml** - Main CI orchestrator with unified provider testing
- **validate-yaml.yml** - Comprehensive YAML schema validation
- **v2-test-provider.yml** - Full test suite per provider (CLI + extensions + integration)
- **release.yml** - Release automation

### Unified Provider Testing

The CI runs **identical tests on every selected provider**, ensuring consistent quality:

```text
FOR EACH provider in [docker, fly, devpod-aws, devpod-do, ...]:
  ├─> Deploy infrastructure
  ├─> Run sindri-test.sh (inside container):
  │   ├─> Quick: CLI validation
  │   ├─> Extension: Single extension lifecycle
  │   └─> Profile: Profile lifecycle
  └─> Cleanup
```

This catches provider-specific bugs that would be missed by Docker-only testing.

### Pre-Commit Checks

Locally run before pushing:

```bash
pnpm validate
pnpm test
```

### CI Expectations

All PRs must:

- Pass validation (shellcheck, yamllint, markdownlint)
- Pass all tests
- Not introduce new linting warnings
- Include documentation updates

## Release Process

Sindri uses **automated releases** triggered by Git tags. For detailed instructions, see [RELEASE.md](RELEASE.md).

### Quick Release

```bash
# 1. Ensure all changes are committed and pushed
git push origin main

# 2. Create and push a version tag
git tag v1.1.0
git push origin v1.1.0
```

That's it! The workflow handles:

- Validating semantic version tags
- Generating changelog from commits
- Building and pushing Docker image to GHCR
- Creating GitHub Release with assets
- Updating CHANGELOG.md automatically

### Changelog Automation

**Important:** CHANGELOG.md is auto-generated by the release workflow. **Do not edit manually.**

For proper changelog categorization, use [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix                    | Category      |
| ------------------------- | ------------- |
| `feat:`                   | Features      |
| `fix:`                    | Bug Fixes     |
| `docs:`                   | Documentation |
| `deps:`                   | Dependencies  |
| `perf:`                   | Performance   |
| `refactor:`               | Refactoring   |
| `test:`                   | Tests         |
| `chore:`, `ci:`, `style:` | Maintenance   |

**Examples:**

```bash
git commit -m "feat: add postgresql extension"
git commit -m "fix(docker): resolve volume permission issue"
git commit -m "docs(readme): update quickstart guide"
```

See [RELEASE.md](RELEASE.md) for complete release documentation.

## Getting Help

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - Questions and community support
- **Documentation** - Comprehensive guides in `docs/`

## Recognition

Contributors are recognized in:

- GitHub contributors page
- CHANGELOG.md for significant contributions
- README.md acknowledgments section

Thank you for contributing to Sindri!
