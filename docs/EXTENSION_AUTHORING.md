# Extension Authoring Guide

## Creating a New Extension

### 1. Create Directory Structure

```bash
mkdir -p docker/lib/extensions/myext/{templates,scripts}
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
mytool = "latest"

[env]
MYTOOL_HOME = "~/.mytool"
```

#### Method: apt

For system packages:

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

Update `docker/lib/registry.yaml`:

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
