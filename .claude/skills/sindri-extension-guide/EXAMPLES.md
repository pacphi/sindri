# Sindri Extension Examples

Real examples from the Sindri codebase demonstrating different extension patterns.

## 1. Language Runtime (mise-based)

**Pattern:** Simple language runtime installation via mise
**Use case:** Node.js, Python, Go, Rust, Ruby

```yaml
# docker/lib/extensions/nodejs/extension.yaml
---
metadata:
  name: nodejs
  version: 1.0.0
  description: Node.js LTS via mise
  category: language
  dependencies: []

requirements:
  domains:
    - registry.npmjs.org
    - nodejs.org
  diskSpace: 600

install:
  method: mise
  mise:
    configFile: mise.toml
    reshimAfterInstall: true

configure:
  environment:
    - key: NODE_ENV
      value: development
      scope: bashrc

validate:
  commands:
    - name: node
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"
    - name: npm

remove:
  mise:
    removeConfig: true
    tools: [nodejs]

bom:
  tools:
    - name: node
      version: dynamic
      source: mise
      type: runtime
      license: MIT
      homepage: https://nodejs.org
      purl: pkg:generic/nodejs
    - name: npm
      version: dynamic
      source: mise
      type: package-manager
      license: Artistic-2.0
      homepage: https://www.npmjs.com
      purl: pkg:npm/npm
```

**Key points:**

- Uses `mise` method with a `mise.toml` config file
- Validates both `node` and `npm` commands
- Sets `NODE_ENV` environment variable
- BOM tracks installed tools with their sources

---

## 2. Infrastructure Tool (apt-based)

**Pattern:** System package installation via APT repositories
**Use case:** Docker, system tools requiring root installation

```yaml
# docker/lib/extensions/docker/extension.yaml
---
metadata:
  name: docker
  version: 1.0.0
  description: Docker Engine and Compose
  category: infrastructure
  dependencies: []

requirements:
  domains:
    - download.docker.com
    - hub.docker.com
  diskSpace: 1000

install:
  method: apt
  apt:
    repositories:
      - gpgKey: https://download.docker.com/linux/ubuntu/gpg
        sources: >-
          deb [arch=amd64] https://download.docker.com/linux/ubuntu
          jammy stable
    packages:
      - docker-ce
      - docker-ce-cli
      - containerd.io
      - docker-compose-plugin

configure:
  environment:
    - key: DOCKER_BUILDKIT
      value: "1"
      scope: bashrc

validate:
  commands:
    - name: docker
      expectedPattern: "Docker version \\d+\\.\\d+\\.\\d+"

remove:
  confirmation: true
  apt:
    packages:
      - docker-ce
      - docker-ce-cli
      - containerd.io
      - docker-compose-plugin

bom:
  tools:
    - name: docker
      version: dynamic
      source: apt
      type: server
      license: Apache-2.0
      homepage: https://www.docker.com
      purl: pkg:deb/ubuntu/docker-ce
      cpe: cpe:2.3:a:docker:docker:*:*:*:*:*:*:*:*
```

**Key points:**

- Uses `apt` method with custom repository and GPG key
- Large disk requirement (1000MB)
- Requires confirmation before removal
- Includes CPE for vulnerability scanning

---

## 3. Development Tools with Dependencies

**Pattern:** Tools that depend on another extension
**Use case:** Language-specific development tools

```yaml
# docker/lib/extensions/nodejs-devtools/extension.yaml
---
metadata:
  name: nodejs-devtools
  version: 2.0.0
  description: >-
    TypeScript, ESLint, Prettier, and Node.js development tools via mise
    npm backend
  category: dev-tools
  author: Sindri Team
  dependencies:
    - nodejs

requirements:
  domains:
    - registry.npmjs.org
  diskSpace: 200
  secrets:
    - perplexity_api_key

install:
  method: mise
  mise:
    configFile: mise.toml
    reshimAfterInstall: true

configure:
  templates:
    - source: prettierrc.template
      destination: ~/templates/.prettierrc
      mode: overwrite
    - source: eslintrc.template
      destination: ~/templates/.eslintrc.json
      mode: overwrite
    - source: tsconfig.template
      destination: ~/templates/tsconfig.json
      mode: overwrite

validate:
  commands:
    - name: tsc
      expectedPattern: "Version \\d+\\.\\d+\\.\\d+"
    - name: ts-node
    - name: prettier
    - name: eslint
    - name: nodemon
    - name: goalie
  mise:
    tools:
      - npm:typescript
      - npm:prettier
      - npm:eslint
    minToolCount: 3

remove:
  mise:
    removeConfig: true
    tools:
      - npm:typescript
      - npm:ts-node
      - npm:nodemon
      - npm:prettier
      - npm:eslint
      - npm:@typescript-eslint/parser
      - npm:@typescript-eslint/eslint-plugin
      - npm:goalie
      - npm:research-swarm
  paths:
    - ~/templates/.prettierrc
    - ~/templates/.eslintrc.json
    - ~/templates/tsconfig.json

upgrade:
  strategy: automatic
  mise:
    upgradeAll: true

bom:
  tools:
    - name: typescript
      version: dynamic
      source: npm
      type: compiler
      license: Apache-2.0
      homepage: https://www.typescriptlang.org
      purl: pkg:npm/typescript
    - name: prettier
      version: dynamic
      source: npm
      type: cli-tool
      license: MIT
      homepage: https://prettier.io
      purl: pkg:npm/prettier
    - name: eslint
      version: dynamic
      source: npm
      type: cli-tool
      license: MIT
      homepage: https://eslint.org
      purl: pkg:npm/eslint
```

**Key points:**

- Depends on `nodejs` extension (installed first)
- Uses templates for configuration files
- Validates multiple commands
- Uses mise tool validation with minimum count
- Cleans up template files on removal
- Automatic upgrade strategy

---

## 4. Script-Based Installation

**Pattern:** Custom installation script for complex setups
**Use case:** Tools requiring custom installation logic

```yaml
# docker/lib/extensions/playwright/extension.yaml
---
metadata:
  name: playwright
  version: 2.0.0
  description: Playwright browser automation framework with Chromium
  category: dev-tools
  author: Sindri Team
  dependencies:
    - nodejs

requirements:
  domains:
    - registry.npmjs.org
    - playwright.azureedge.net
  diskSpace: 1000

install:
  method: script
  script:
    path: install.sh
    timeout: 900

configure:
  templates:
    - source: playwright-config.template
      destination: ~/playwright.config.ts
      mode: overwrite
    - source: test-spec.template
      destination: ~/tests/example.spec.ts
      mode: overwrite
    - source: tsconfig.template
      destination: ~/tsconfig.json
      mode: overwrite

validate:
  commands:
    - name: npx
      versionFlag: playwright --version

remove:
  paths:
    - ~/node_modules/playwright
    - ~/node_modules/@playwright
    - ~/.cache/ms-playwright

upgrade:
  strategy: manual
  script:
    path: upgrade.sh
    timeout: 600

bom:
  tools:
    - name: playwright
      version: dynamic
      source: npm
      type: framework
      license: Apache-2.0
      homepage: https://playwright.dev
      purl: pkg:npm/@playwright/test
```

**Key points:**

- Uses custom `install.sh` script with long timeout (900s)
- Custom validation flag: `npx playwright --version`
- Manual upgrade strategy with separate script
- Removes cache directories on uninstall

---

## 5. Extension WITH Capabilities (Project Initialization)

**Pattern:** Extension that initializes projects and requires authentication
**Use case:** AI tools, project management extensions

```yaml
# docker/lib/extensions/spec-kit/extension.yaml
---
metadata:
  name: spec-kit
  version: 1.0.0
  description: GitHub specification kit for AI-powered repository documentation
  category: dev-tools
  dependencies:
    - python

requirements:
  domains:
    - github.com
    - raw.githubusercontent.com
  diskSpace: 50

install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 60

validate:
  commands:
    - name: uvx
      expectedPattern: "uv \\d+\\.\\d+\\.\\d+"

# CAPABILITIES SECTION - NEW!
capabilities:
  project-init:
    enabled: true
    commands:
      - command: "uvx --from git+https://github.com/github/spec-kit.git specify init --here --force --ai claude --script sh"
        description: "Initialize GitHub spec-kit for AI-powered workflows"
        requiresAuth: none
        conditional: false

    state-markers:
      - path: ".github/spec.json"
        type: file
        description: "GitHub spec-kit configuration file"

    validation:
      command: "test -f .github/spec.json"
      expectedExitCode: 0

  hooks:
    post-project-init:
      command: "bash scripts/commit-spec-kit.sh"
      description: "Commit spec-kit initialization files"

bom:
  tools:
    - name: spec-kit
      version: dynamic
      source: github
      type: cli-tool
      license: MIT
      homepage: https://github.com/github/spec-kit
```

**Key points:**

- Uses `capabilities.project-init` to run initialization commands
- State markers ensure idempotency (won't re-run if `.github/spec.json` exists)
- Post-project-init hook automatically commits generated files
- No authentication required (`requiresAuth: none`)

---

## 6. Advanced Extension WITH Full Capabilities

**Pattern:** Extension with project-init, auth, hooks, and MCP integration
**Use case:** Claude Flow V3, Agentic QE, advanced AI tools

```yaml
# docker/lib/extensions/claude-flow-v3/extension.yaml (simplified)
---
metadata:
  name: claude-flow-v3
  version: 3.0.0
  description: Next-gen multi-agent orchestration with 10x performance
  category: ai
  dependencies:
    - nodejs

install:
  method: mise
  mise:
    configFile: mise.toml

configure:
  environment:
    - key: CLAUDE_FLOW_VERSION
      value: "3"
      scope: bashrc
    - key: CF_SWARM_TOPOLOGY
      value: "hierarchical-mesh"
      scope: bashrc

validate:
  commands:
    - name: claude-flow
      expectedPattern: "^3\\.\\d+\\.\\d+"

# FULL CAPABILITIES - ALL FOUR TYPES!
capabilities:
  # 1. Project initialization
  project-init:
    enabled: true
    commands:
      - command: "claude-flow init --full"
        description: "Initialize Claude Flow v3"
        requiresAuth: anthropic
        conditional: false

      - command: "claude-flow swarm init --topology ${CF_SWARM_TOPOLOGY}"
        description: "Initialize UnifiedSwarmCoordinator"
        requiresAuth: none
        conditional: true

    state-markers:
      - path: ".claude"
        type: directory
        description: "Claude Code configuration"
      - path: ".claude/config.json"
        type: file
        description: "V3 unified config"

    validation:
      command: "claude-flow --version && claude-flow doctor --check"
      expectedPattern: "^3\\.\\d+\\.\\d+"

  # 2. Authentication (multi-method)
  auth:
    provider: anthropic
    required: false
    methods:
      - api-key
      - cli-auth
    envVars:
      - ANTHROPIC_API_KEY
    validator:
      command: "claude --version"
      expectedExitCode: 0
    features:
      - name: agent-spawn
        requiresApiKey: false
        description: "CLI-based agent spawning"
      - name: api-integration
        requiresApiKey: true
        description: "Direct API features"

  # 3. Lifecycle hooks
  hooks:
    post-install:
      command: "claude-flow --version | grep -q '^3\\.'"
      description: "Verify v3 installation"
    post-project-init:
      command: "echo 'Claude Flow v3 initialized - enjoy 10x performance!'"
      description: "v3 initialization complete"

  # 4. MCP server registration
  mcp:
    enabled: true
    server:
      command: "npx"
      args:
        - "-y"
        - "@claude-flow/cli@alpha"
        - "mcp"
        - "start"
      env:
        CLAUDE_FLOW_MCP_MODE: "1"
    tools:
      - name: "claude-flow-agent-spawn"
        description: "Spawn specialized agents"
      - name: "claude-flow-swarm-coordinate"
        description: "Coordinate multi-agent swarms"
      - name: "claude-flow-neural-sona"
        description: "SONA self-optimizing neural architecture"

bom:
  tools:
    - name: claude-flow
      version: "3.0.0-alpha"
      source: npm
      type: cli-tool
```

**Key points:**

- **Project-init**: Multiple commands (some conditional)
- **Auth**: Supports BOTH API key AND CLI auth (flexible for Max/Pro users)
- **Features**: Some features work without API key, others require it
- **Hooks**: Post-install and post-project-init lifecycle events
- **MCP**: Registers 15 tools with Claude Code via MCP server
- **State markers**: Prevents re-initialization if already configured

---

## Directory Structure Examples

### Minimal Extension (mise-based) - NO CAPABILITIES

```text
docker/lib/extensions/my-language/
├── extension.yaml
└── mise.toml
```

### Full Extension (script-based) - NO CAPABILITIES

```text
docker/lib/extensions/my-tool/
├── extension.yaml
├── mise.toml              # Optional
├── scripts/
│   ├── install.sh
│   ├── upgrade.sh
│   └── uninstall.sh
└── templates/
    ├── config.template
    └── rc.template
```

### Extension WITH Capabilities (project-init)

```text
docker/lib/extensions/spec-kit/
├── extension.yaml         # Includes capabilities section
├── scripts/
│   ├── install.sh
│   └── commit-spec-kit.sh # Post-project-init hook
└── templates/
    └── spec-template.json
```

---

## Registry Entry Examples

```yaml
# docker/lib/registry.yaml
extensions:
  # Language runtime - no dependencies
  nodejs:
    category: language
    description: Node.js LTS runtime
    dependencies: []
    protected: false

  # Dev tool - depends on language
  nodejs-devtools:
    category: dev-tools
    description: TypeScript, ESLint, Prettier
    dependencies:
      - nodejs
    protected: false

  # Infrastructure - standalone
  docker:
    category: infrastructure
    description: Docker Engine and Compose
    dependencies: []
    protected: false

  # Protected system extension
  mise-config:
    category: base
    description: mise package manager configuration
    dependencies: []
    protected: true
```

---

## mise.toml Examples

### Simple Language Tool

```toml
# docker/lib/extensions/nodejs/mise.toml
[tools]
node = "lts"
```

### Multiple npm Tools

```toml
# docker/lib/extensions/nodejs-devtools/mise.toml
# IMPORTANT: Use pinned major.minor versions instead of "latest"
# "latest" requires npm registry queries which can timeout and poison
# subsequent mise operations if they fail.
[tools]
"npm:typescript" = "5.9"
"npm:ts-node" = "10.9"
"npm:nodemon" = "3.1"
"npm:prettier" = "3.6"
"npm:eslint" = "9"
```

### Specific Versions

```toml
[tools]
python = "3.12"
"pipx:poetry" = "1.8.0"
```

---

## Install Script Example

```bash
#!/usr/bin/env bash
# docker/lib/extensions/playwright/scripts/install.sh
set -euo pipefail

echo "Installing Playwright..."

# Navigate to home directory
cd "$HOME"

# Initialize npm if needed
if [[ ! -f package.json ]]; then
    npm init -y
fi

# Install Playwright
npm install --save-dev @playwright/test

# Install browsers (Chromium only for size)
npx playwright install chromium
npx playwright install-deps chromium

echo "Playwright installed successfully"
```

---

## Template Example

```json
// docker/lib/extensions/nodejs-devtools/templates/eslintrc.template
{
  "root": true,
  "parser": "@typescript-eslint/parser",
  "plugins": ["@typescript-eslint"],
  "extends": ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
  "env": {
    "node": true,
    "es2022": true
  },
  "rules": {
    "no-unused-vars": "off",
    "@typescript-eslint/no-unused-vars": "error"
  }
}
```

---

## Validation Patterns

### Basic Version Check

```yaml
validate:
  commands:
    - name: node
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"
```

### Custom Version Flag

```yaml
validate:
  commands:
    - name: npx
      versionFlag: playwright --version
```

### Multiple Tools with mise

```yaml
validate:
  commands:
    - name: tsc
    - name: eslint
    - name: prettier
  mise:
    tools:
      - npm:typescript
      - npm:eslint
    minToolCount: 2
```

### Script Validation

```yaml
validate:
  script:
    path: scripts/validate.sh
    timeout: 60
```

---

## Understanding Capabilities

### When to Use Capabilities

**Use capabilities when your extension:**

1. **Needs project initialization** - Runs setup commands when creating a new project (e.g., `claude-flow init`, `spec-kit init`)
2. **Requires authentication** - Validates API keys or CLI authentication before running
3. **Has lifecycle hooks** - Needs to run commands pre/post install or project-init
4. **Provides MCP tools** - Registers as an MCP server for Claude Code integration

**Don't use capabilities when your extension:**

1. **Just installs tools** - Language runtimes (nodejs, python, go, rust)
2. **Provides development utilities** - Linters, formatters, build tools
3. **Installs system packages** - Docker, database clients, system tools

### Capability Examples in Sindri

| Extension          | Purpose          | Capabilities Used              | When to Use Similar Pattern           |
| ------------------ | ---------------- | ------------------------------ | ------------------------------------- |
| **nodejs**         | Node.js runtime  | None                           | Language runtimes, dev tools          |
| **docker**         | Docker Engine    | None                           | Infrastructure tools, system packages |
| **spec-kit**       | GitHub spec docs | project-init, hooks            | Project initialization without auth   |
| **claude-flow-v3** | Multi-agent AI   | project-init, auth, hooks, mcp | AI tools with authentication and MCP  |
| **agentic-qe**     | AI testing       | project-init, auth, hooks, mcp | AI tools requiring Anthropic API      |

### Multi-Method Authentication

Modern extensions support both API key and CLI authentication:

```yaml
capabilities:
  auth:
    provider: anthropic
    required: false
    methods:
      - api-key # Traditional API key in env var
      - cli-auth # CLI authentication (Max/Pro plan)
    features:
      - name: cli-features
        requiresApiKey: false # Works with CLI auth
      - name: api-features
        requiresApiKey: true # Requires API key
```

**Benefits:**

- Max/Pro users can use extensions without setting API keys
- Feature-level auth requirements (some features work without API key)
- Graceful degradation when API key unavailable
