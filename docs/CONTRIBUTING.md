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
- **Tests:** `.github/scripts/`

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

Test specific extension:

```bash
./.github/scripts/test-all-extensions.sh myext
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

- **validation.yml** - Code quality (shellcheck, yamllint, markdownlint)
- **integration.yml** - Integration tests
- **per-extension-tests.yml** - Extension validation
- **ci.yml** - Continuous integration

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

1. **Update version:**

   ```bash
   # Update version in package.json
   vim package.json
   ```

2. **Update CHANGELOG.md:**

   ```markdown
   ## [1.1.0] - 2025-11-21

   ### Added

   - New postgresql extension

   ### Fixed

   - Volume permission issue
   ```

3. **Create release tag:**

   ```bash
   git tag -a v1.1.0 -m "Release v1.1.0"
   git push origin v1.1.0
   ```

4. **GitHub Release:**
   - Automated via `.github/workflows/release.yml`
   - Includes Docker image build and publish

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
