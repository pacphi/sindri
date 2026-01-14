# Contributing to Sindri

Thank you for your interest in contributing to Sindri! This guide will help you get started.

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the technical merit of contributions
- Help others learn and grow

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

- **Extensions:** `docker/lib/extensions/<name>/extension.yaml`
- **CLI tools:** `cli/`
- **Deployment adapters:** `deploy/adapters/`
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
./cli/extension-manager validate <extension-name>

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
   mkdir -p docker/lib/extensions/myext
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
   # docker/lib/registry.yaml
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

See: [Extension Authoring Guide](EXTENSION_AUTHORING.md)

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
./cli/extension-manager validate <extension-name>
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
- Source `docker/lib/common.sh` for shared functions
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
- Validate against schemas in `docker/lib/schemas/`

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

## Documentation

### Documentation Structure

- **README.md** - Teaser with links to detailed docs
- **docs/** - Detailed documentation
- **CLAUDE.md** - Developer guide for Claude Code
- **Extension README** - Per-extension documentation (optional)

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
- **test-provider.yml** - Full test suite per provider (CLI + extensions + integration)
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
