# Extension Authoring Guide

This guide covers creating extensions for Sindri. For architectural background on the capability system, see [ADR-001: Extension Capabilities System](architecture/adr/ADR-001-extension-capabilities-system.md).

## Creating a New Extension

### 1. Create Directory Structure

```bash
mkdir -p v2/docker/lib/extensions/myext/{templates,scripts}
```

### 2. Create extension.yaml

```yaml
metadata:
  name: myext
  version: 1.0.0
  description: My custom extension
  category: dev-tools # base, language, dev-tools, infrastructure, ai, etc.
  dependencies: [] # List other required extensions

requirements:
  domains: # Required DNS domains
    - example.com
  diskSpace: 100 # Required disk space in MB
  validationTimeout: 30 # Optional: seconds to wait for validation (default: 30)
  secrets: # Optional secrets
    - MY_API_KEY

install:
  method: <method> # Choose installation method

configure: # Optional configuration
  templates:
    - source: templates/config.tmpl
      destination: ~/.myext/config
      mode: overwrite # or: append
  environment:
    - key: MYEXT_HOME
      value: ~/.myext
      scope: bashrc

validate: # Validation checks
  commands:
    - name: myext
      expectedPattern: "myext v\\d+\\.\\d+"

remove: # Cleanup instructions
  confirmation: true
  paths:
    - ~/.myext
```

### 3. Installation Methods

#### Method: mise

For tools available via mise/asdf plugins:

```yaml
install:
  method: mise
  mise:
    configFile: mise.toml
    reshimAfterInstall: true
```

Create `mise.toml`:

```toml
[tools]
mytool = "1.0"  # Pin to specific version for reliability

[env]
MYTOOL_HOME = "~/.mytool"
```

#### Method: apt

For system packages. The extension manager uses modern GPG keyring handling
(stores keys in `/etc/apt/keyrings/` with `signed-by` option, avoiding
deprecated `apt-key`):

```yaml
install:
  method: apt
  apt:
    repositories:
      - gpgKey: https://example.com/key.gpg
        sources: "deb [arch=amd64] https://example.com/ubuntu jammy stable"
    packages:
      - mypackage
      - mypackage-cli
```

> **Note**: The `gpgKey` URL is downloaded and stored in
> `/etc/apt/keyrings/<extension>.gpg`. The sources line is automatically
> updated to include `signed-by=/etc/apt/keyrings/<extension>.gpg`.

#### Method: binary

For downloading binaries:

```yaml
install:
  method: binary
  binary:
    downloads:
      - name: mytool
        source:
          url: https://github.com/owner/repo/releases/download/v1.0.0/mytool_linux_amd64
        destination: /workspace/bin
        extract: false # Set true for archives
```

#### Method: npm

For Node.js packages:

```yaml
install:
  method: npm
  npm:
    packages:
      - "@myorg/mytool"
      - "another-tool@^2.0.0"
```

#### Method: script

For custom installation:

```yaml
install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 600
```

#### Method: hybrid

For complex installations:

```yaml
install:
  method: hybrid
  steps:
    - method: apt
      apt:
        packages: [build-essential]
    - method: script
      script:
        path: scripts/compile.sh
    - method: binary
      binary:
        downloads:
          - name: mytool
            source:
              url: https://example.com/mytool
```

### 4. Add to Registry

Update `v2/docker/lib/registry.yaml`:

```yaml
extensions:
  myext:
    category: dev-tools
    description: My custom extension
    dependencies: []
    protected: false
```

### 5. Test Extension

```bash
# Build Docker image
docker build -t sindri:test -f Dockerfile .

# Run container
docker run -it -v sindri-workspace:/workspace sindri:test

# Inside container
extension-manager install myext
extension-manager validate myext
```

## Domain Requirements

Extensions should declare all external domains they need to access during installation.
This enables pre-flight DNS checks and helps users understand network requirements.

### Declaring Domains

```yaml
requirements:
  domains:
    - registry.npmjs.org # npm package registry
    - nodejs.org # Node.js binaries
    - github.com # GitHub releases
```

### Guidelines

1. **List all domains** accessed during installation (package registries, binary downloads)
2. **Use base domains** when possible (e.g., `github.com` covers `raw.githubusercontent.com`)
3. **Exclude transient domains** that are only used at runtime, not installation
4. **Keep domains current** - update if install scripts change

### Validation

Domains are validated in CI and can be checked locally:

```bash
# Validate format and check for duplicates
extension-manager validate-domains myext

# Also perform DNS resolution checks
extension-manager --check-dns validate-domains myext

# Validate all extensions
extension-manager validate-domains
```

The validation checks:

- **Format** - Valid hostname syntax (fails build if invalid)
- **Duplicates** - No duplicate entries (fails build if found)
- **DNS Resolution** - Domains resolve (warning only, optional)
- **Undeclared** - Domains in scripts not in YAML (warning only, heuristic)

## Best Practices

1. **Keep it simple** - Use existing methods when possible
2. **Validate thoroughly** - Add comprehensive validation checks
3. **Document requirements** - Be explicit about domains and disk space
4. **Handle errors** - Scripts should exit on failure
5. **Clean uninstall** - Remove all artifacts in remove section
6. **Use templates** - For configuration files that need customization
7. **Respect dependencies** - List all required extensions
8. **Test locally** - Validate in Docker before deploying

## Available Categories

- **base** - Core system components
- **language** - Programming language runtimes
- **dev-tools** - Development tools and utilities
- **infrastructure** - Cloud and container tools
- **ai** - AI/ML frameworks and tools
- **database** - Database clients and tools
- **monitoring** - Observability tools
- **mobile** - Mobile development SDKs
- **utilities** - General purpose tools

## Validation Patterns

```yaml
validate:
  # Check command exists and version
  commands:
    - name: mytool
      versionFlag: "--version" # default
      expectedPattern: "\\d+\\.\\d+\\.\\d+"

  # Check mise tools
  mise:
    tools: [mytool]
    minToolCount: 1

  # Custom validation
  script:
    path: scripts/validate.sh
```

### Validation Timeout

Some tools (especially JVM-based like Scala, or tools that initialize on first
run) may take longer than the default 30-second timeout. Set a custom timeout
in the requirements section:

```yaml
requirements:
  validationTimeout: 60 # Seconds (default: 30)
```

Use longer timeouts for:

- JVM tools (Java, Scala, Kotlin) - recommend 60s
- Tools that download on first run - recommend 30s
- CLI tools that initialize config on first run - recommend 30s

## Environment Variables

Extensions can set environment variables:

```yaml
configure:
  environment:
    - key: MYTOOL_HOME
      value: "~/.mytool"
      scope: bashrc      # or: profile, session

    - key: PATH
      value: "~/.mytool/bin:\$PATH"
      scope: bashrc
```

## Template Processing

Templates support basic variable substitution:

```yaml
configure:
  templates:
    - source: templates/config.yml.tmpl
      destination: ~/.mytool/config.yml
      mode: overwrite
      variables:
        - key: WORKSPACE
          value: /workspace
        - key: USER
          value: developer
```

## Extension Authoring Checklist

Use this checklist when creating or updating extensions to ensure completeness and quality.

### Required Components

- [ ] **extension.yaml** - Complete extension definition with all required fields
- [ ] **Registry Entry** - Added to `v2/docker/lib/registry.yaml` with category and description
- [ ] **Category Assignment** - Valid category from `categories.yaml`
- [ ] **Install Method** - One of: `mise`, `script`, `apt`, `npm`, `binary`, `hybrid`
- [ ] **Validation Commands** - At least one command to verify installation
- [ ] **Dependencies** - All required extensions listed (if any)

### Installation Methods Coverage

Check that your installation method is properly configured:

- [ ] **mise**: `mise.toml` file exists and is valid
- [ ] **script**: `install.sh` script exists, is executable, and handles errors
- [ ] **apt**: All package names are correct and available in Ubuntu repos
- [ ] **npm**: Package names are correct and published to npm
- [ ] **binary**: Download URLs are stable and checksums verified
- [ ] **hybrid**: Multiple methods work independently

### Documentation

- [ ] **Description** - Clear, concise description (10-200 characters)
- [ ] **Requirements** - Disk space estimated accurately
- [ ] **Domains** - All network domains declared in `requirements.domains`
- [ ] **Dependencies** - Dependency relationships documented
- [ ] **Optional: README** - Extension-specific README in `resources/` (for complex extensions)

### Bill of Materials (BOM)

- [ ] **BOM Section** - Added to `extension.yaml`
- [ ] **Tool Entries** - All installed tools listed with versions
- [ ] **Source Attribution** - Correct source (mise, apt, script) for each tool
- [ ] **License Info** - Software licenses documented (optional but recommended)
- [ ] **Homepage** - Project homepage URLs included (optional but recommended)

### Configuration

- [ ] **Templates** - Template files exist in extension directory if referenced
- [ ] **Environment Variables** - All required env vars documented
- [ ] **Scope** - Environment scope appropriate (`bashrc`, `profile`, or `session`)

### Upgrade Strategy

- [ ] **Upgrade Section** - Defined with appropriate strategy
- [ ] **Strategy** - Correct strategy: `automatic`, `manual`, `reinstall`, `in-place`, or `none`
- [ ] **Upgrade Script** - If using script strategy, script exists and works
- [ ] **mise Upgrades** - If using mise, upgrade configuration specified
- [ ] **Timeout** - Reasonable timeout set (default: 600s)

### Removal/Cleanup

- [ ] **Remove Section** - Defined with cleanup paths
- [ ] **Paths** - All installed files/directories listed for removal
- [ ] **mise Cleanup** - If using mise, removal configuration specified
- [ ] **Confirmation** - Removal confirmation enabled unless intentionally disabled
- [ ] **Test Removal** - Removal tested and verified clean

### Validation & Testing

- [ ] **Schema Validation** - Extension passes `./cli/extension-manager validate <name>`
- [ ] **Local Test** - Tested in local Docker environment
- [ ] **Installation Test** - Fresh install works without errors
- [ ] **Validation Test** - Validation commands return expected output
- [ ] **Dependency Test** - Dependencies install in correct order
- [ ] **Removal Test** - Extension removes cleanly
- [ ] **Domain Validation** - All domains in `requirements.domains` are accessible

### Advanced Features (Optional)

- [ ] **Custom Validation** - mise validation or custom script if needed
- [ ] **Secrets Integration** - Uses secrets from `requirements.secrets` if applicable
- [ ] **GPU Requirements** - GPU configuration if extension requires GPU
- [ ] **Extended BOM** - Additional BOM fields (downloadUrl, checksum, purl, cpe) for security
- [ ] **Optional Metadata** - Author, homepage, license in metadata section

### Code Quality

- [ ] **shellcheck** - All bash scripts pass `shellcheck -S warning`
- [ ] **yamllint** - extension.yaml passes `yamllint --strict`
- [ ] **Error Handling** - Scripts use `set -euo pipefail` and handle errors
- [ ] **Common Functions** - Scripts source `common.sh` and use provided functions
- [ ] **Exit Codes** - Scripts exit with appropriate codes (0=success, non-zero=failure)

### Final Checks

- [ ] **No Hardcoded Paths** - Uses `$HOME`, `$WORKSPACE`, environment variables
- [ ] **idempotency** - Extension can be installed multiple times safely
- [ ] **No Breaking Changes** - Backwards compatible with previous versions (or version bumped)
- [ ] **CI Passes** - All CI checks pass (YAML validation, shellcheck, tests)
- [ ] **Documentation Updated** - CLAUDE.md, CONFIGURATION.md updated if needed

### Pre-Submission Checklist

Before submitting your extension:

```bash
# 1. Validate YAML syntax
./v2/cli/extension-manager validate <extension-name>

# 2. Test local installation
./v2/cli/extension-manager install <extension-name>

# 3. Verify validation works
./v2/cli/extension-manager status <extension-name>

# 4. Test removal
./v2/cli/extension-manager remove <extension-name>

# 5. Reinstall to verify idempotency
./v2/cli/extension-manager install <extension-name>

# 6. Run full validation
pnpm validate

# 7. Test in clean container
pnpm build && docker run -it sindri:local
```

### Common Issues to Avoid

- ❌ **Missing dependencies** - Causes installation failures
- ❌ **Undeclared domains** - Triggers security warnings
- ❌ **Wrong install method** - Use mise for version-managed tools
- ❌ **Missing validation** - Extension appears installed but doesn't work
- ❌ **Incomplete removal** - Leaves artifacts behind
- ❌ **Hardcoded paths** - Breaks in different environments
- ❌ **No error handling** - Silent failures
- ❌ **Wrong category** - Makes extension hard to find
- ❌ **Inadequate testing** - Fails in production

### Getting Help

- **Documentation** - Read [SCHEMA.md](SCHEMA.md) for complete reference
- **Examples** - Browse `v2/docker/lib/extensions/` for working examples
- **Claude Code Skill** - Use the sindri-extension-guide skill for guidance
- **Validation Errors** - Run with `-v` flag for detailed error messages
