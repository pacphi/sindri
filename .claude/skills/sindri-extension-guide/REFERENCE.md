# Sindri Extension Field Reference

Complete field-by-field reference for `extension.yaml` files.

## Metadata Section

| Field          | Type   | Required | Pattern/Values      | Description                 |
| -------------- | ------ | -------- | ------------------- | --------------------------- |
| `name`         | string | Yes      | `^[a-z][a-z0-9-]*$` | Lowercase with hyphens only |
| `version`      | string | Yes      | `^\d+\.\d+\.\d+$`   | Semantic versioning         |
| `description`  | string | Yes      | 10-200 chars        | Brief description           |
| `category`     | enum   | Yes      | See below           | Extension category          |
| `author`       | string | No       | -                   | Author name                 |
| `homepage`     | uri    | No       | Valid URI           | Project homepage            |
| `dependencies` | array  | No       | Extension names     | Required extensions         |

**Category Values:**

- `base` - Core system components
- `agile` - Project management tools (Jira, Linear)
- `language` - Programming language runtimes
- `dev-tools` - Development tools
- `infrastructure` - Cloud/container tools
- `ai` - AI/ML tools
- `database` - Database servers
- `monitoring` - Observability tools
- `mobile` - Mobile SDKs
- `desktop` - GUI environments
- `utilities` - General tools

## Requirements Section

| Field       | Type            | Required | Description                  |
| ----------- | --------------- | -------- | ---------------------------- |
| `domains`   | array[hostname] | No       | DNS names for network access |
| `diskSpace` | integer         | No       | Minimum MB required          |
| `secrets`   | array[string]   | No       | Required credentials/tokens  |

## Install Section

| Field    | Type | Required | Values                                             | Description         |
| -------- | ---- | -------- | -------------------------------------------------- | ------------------- |
| `method` | enum | Yes      | `mise`, `apt`, `binary`, `npm`, `script`, `hybrid` | Installation method |

### mise Method

```yaml
install:
  method: mise
  mise:
    configFile: mise.toml # Path to mise config
    reshim: true # Rebuild shims (default: false)
```

| Field        | Type    | Required | Description                      |
| ------------ | ------- | -------- | -------------------------------- |
| `configFile` | string  | Yes      | Path to mise.toml config file    |
| `reshim`     | boolean | No       | Rebuild mise shims after install |

### apt Method

```yaml
install:
  method: apt
  apt:
    repositories:
      - name: repo-name
        key: https://example.com/gpg-key
        url: https://example.com/repo
        suite: jammy
        component: stable
    packages:
      - package1
      - package2
```

| Field                      | Type          | Required | Description             |
| -------------------------- | ------------- | -------- | ----------------------- |
| `repositories`             | array         | No       | APT repositories to add |
| `repositories[].name`      | string        | Yes      | Repository identifier   |
| `repositories[].key`       | uri           | Yes      | GPG key URL             |
| `repositories[].url`       | uri           | Yes      | Repository base URL     |
| `repositories[].suite`     | string        | Yes      | Distribution codename   |
| `repositories[].component` | string        | Yes      | Repository component    |
| `packages`                 | array[string] | Yes      | APT packages to install |

### binary Method

```yaml
install:
  method: binary
  binary:
    url: https://example.com/tool.tar.gz
    extract: tar.gz
    destination: ~/.local/bin/tool
    github:
      repo: owner/repo
      asset: tool-linux-amd64.tar.gz
```

| Field          | Type   | Required | Values                  | Description                |
| -------------- | ------ | -------- | ----------------------- | -------------------------- |
| `url`          | uri    | No\*     | -                       | Direct download URL        |
| `extract`      | enum   | No       | `tar.gz`, `zip`, `none` | Archive format             |
| `destination`  | string | Yes      | -                       | Target path (supports `~`) |
| `github.repo`  | string | No\*     | `owner/repo`            | GitHub repository          |
| `github.asset` | string | No       | -                       | Release asset name pattern |

\*Either `url` or `github.repo` is required

### npm Method

```yaml
install:
  method: npm
  npm:
    packages:
      - typescript
      - eslint@8.0.0
    global: true
```

| Field      | Type          | Required | Description                          |
| ---------- | ------------- | -------- | ------------------------------------ |
| `packages` | array[string] | Yes      | NPM packages (with optional version) |
| `global`   | boolean       | No       | Install globally (default: true)     |

### script Method

```yaml
install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 300
```

| Field     | Type    | Required | Default | Description                       |
| --------- | ------- | -------- | ------- | --------------------------------- |
| `path`    | string  | Yes      | -       | Script path relative to extension |
| `timeout` | integer | No       | 300     | Timeout in seconds                |

### hybrid Method

```yaml
install:
  method: hybrid
  hybrid:
    steps:
      - method: apt
        apt: { packages: [build-essential] }
      - method: script
        script: { path: scripts/install.sh }
```

| Field              | Type   | Required | Description                |
| ------------------ | ------ | -------- | -------------------------- |
| `steps`            | array  | Yes      | Ordered installation steps |
| `steps[].method`   | enum   | Yes      | Any install method         |
| `steps[].{method}` | object | Yes      | Method-specific config     |

## Configure Section

### Templates

```yaml
configure:
  templates:
    - source: templates/config.template
      destination: ~/.config/tool/config.yaml
      mode: overwrite
```

| Field         | Type   | Required | Values                                           | Description                           |
| ------------- | ------ | -------- | ------------------------------------------------ | ------------------------------------- |
| `source`      | string | Yes      | -                                                | Template path (relative to extension) |
| `destination` | string | Yes      | -                                                | Target path (supports `~`)            |
| `mode`        | enum   | No       | `overwrite`, `append`, `merge`, `skip-if-exists` | How to apply template                 |

### Environment Variables

```yaml
configure:
  environment:
    - key: MY_VAR
      value: $HOME/.tool
      scope: bashrc
```

| Field   | Type   | Required | Values                         | Description                             |
| ------- | ------ | -------- | ------------------------------ | --------------------------------------- |
| `key`   | string | Yes      | -                              | Variable name                           |
| `value` | string | Yes      | -                              | Value (supports `$HOME`, `$PATH`, etc.) |
| `scope` | enum   | Yes      | `bashrc`, `profile`, `session` | Where to set variable                   |

**Scope Values:**

- `bashrc` - Set in `~/.bashrc` (interactive shells)
- `profile` - Set in `~/.profile` (login shells)
- `session` - Set for current session only

## Validate Section

### Command Validation

```yaml
validate:
  commands:
    - name: node
      versionFlag: --version
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"
```

| Field             | Type   | Required | Default     | Description           |
| ----------------- | ------ | -------- | ----------- | --------------------- |
| `name`            | string | Yes      | -           | Command to execute    |
| `versionFlag`     | string | No       | `--version` | Flag to get version   |
| `expectedPattern` | string | No       | -           | Regex to match output |

### mise Validation

```yaml
validate:
  mise:
    tools:
      - node
      - python
    minToolCount: 2
```

| Field          | Type          | Required | Description            |
| -------------- | ------------- | -------- | ---------------------- |
| `tools`        | array[string] | Yes      | Tools to verify        |
| `minToolCount` | integer       | No       | Minimum tools required |

### Script Validation

```yaml
validate:
  script:
    path: scripts/validate.sh
    timeout: 60
```

| Field     | Type    | Required | Default | Description            |
| --------- | ------- | -------- | ------- | ---------------------- |
| `path`    | string  | Yes      | -       | Validation script path |
| `timeout` | integer | No       | 60      | Timeout in seconds     |

## Remove Section

```yaml
remove:
  confirmation: true
  mise:
    removeConfig: true
    tools: [node, python]
  apt:
    packages: [docker-ce]
    purge: false
  script:
    path: scripts/uninstall.sh
    timeout: 120
  paths:
    - ~/.config/tool
    - ~/.local/share/tool
```

| Field               | Type          | Required | Default | Description               |
| ------------------- | ------------- | -------- | ------- | ------------------------- |
| `confirmation`      | boolean       | No       | true    | Require user confirmation |
| `mise.removeConfig` | boolean       | No       | true    | Remove mise config        |
| `mise.tools`        | array[string] | No       | -       | Tools to remove           |
| `apt.packages`      | array[string] | No       | -       | APT packages to remove    |
| `apt.purge`         | boolean       | No       | false   | Use `apt purge`           |
| `script.path`       | string        | No       | -       | Uninstall script          |
| `script.timeout`    | integer       | No       | 120     | Script timeout            |
| `paths`             | array[string] | No       | -       | Paths to delete           |

## Upgrade Section

```yaml
upgrade:
  strategy: automatic
  mise:
    upgradeAll: true
    tools: [node]
  apt:
    packages: [docker-ce]
    updateFirst: true
  script:
    path: scripts/upgrade.sh
    timeout: 300
```

| Field             | Type          | Required | Values/Default                                       | Description               |
| ----------------- | ------------- | -------- | ---------------------------------------------------- | ------------------------- |
| `strategy`        | enum          | No       | `automatic`, `manual`, `none` (default: `automatic`) | Upgrade approach          |
| `mise.upgradeAll` | boolean       | No       | true                                                 | Upgrade all mise tools    |
| `mise.tools`      | array[string] | No       | -                                                    | Specific tools to upgrade |
| `apt.packages`    | array[string] | No       | -                                                    | APT packages to upgrade   |
| `apt.updateFirst` | boolean       | No       | true                                                 | Run `apt update` first    |
| `script.path`     | string        | No       | -                                                    | Upgrade script            |
| `script.timeout`  | integer       | No       | 300                                                  | Script timeout            |

## BOM (Bill of Materials) Section

### Tools

```yaml
bom:
  tools:
    - name: node
      version: "20.0.0"
      source: mise
      type: runtime
      license: MIT
      homepage: https://nodejs.org
      downloadUrl: https://nodejs.org/dist/v20.0.0/node-v20.0.0-linux-x64.tar.gz
      checksum:
        algorithm: sha256
        value: abc123...
      purl: pkg:npm/node@20.0.0
      cpe: cpe:2.3:a:nodejs:node.js:20.0.0
```

| Field                | Type   | Required | Values                                                     | Description                     |
| -------------------- | ------ | -------- | ---------------------------------------------------------- | ------------------------------- |
| `name`               | string | Yes      | -                                                          | Tool name                       |
| `version`            | string | Yes      | semver or `dynamic`                                        | Tool version                    |
| `source`             | enum   | Yes      | `mise`, `apt`, `npm`, `binary`, `script`, `github-release` | Installation source             |
| `type`               | enum   | Yes      | See below                                                  | Component type                  |
| `license`            | string | No       | SPDX identifier                                            | License (e.g., MIT, Apache-2.0) |
| `homepage`           | uri    | No       | -                                                          | Project URL                     |
| `downloadUrl`        | uri    | No       | -                                                          | Download URL                    |
| `checksum.algorithm` | enum   | No       | `sha256`, `sha512`, `md5`                                  | Hash algorithm                  |
| `checksum.value`     | string | No       | -                                                          | Hash value                      |
| `purl`               | string | No       | Package URL format                                         | SBOM package URL                |
| `cpe`                | string | No       | CPE format                                                 | Vulnerability scanning ID       |

**Type Values:**

- `runtime` - Language runtime
- `compiler` - Compiler
- `package-manager` - Package manager
- `cli-tool` - Command-line tool
- `library` - Library
- `framework` - Framework
- `database` - Database
- `server` - Server
- `utility` - Utility

### Files

```yaml
bom:
  files:
    - path: ~/.config/tool/config.yaml
      type: config
      checksum:
        algorithm: sha256
        value: def456...
```

| Field      | Type   | Required | Values                                          | Description                       |
| ---------- | ------ | -------- | ----------------------------------------------- | --------------------------------- |
| `path`     | string | Yes      | -                                               | File path (relative to workspace) |
| `type`     | enum   | Yes      | `config`, `binary`, `library`, `script`, `data` | File type                         |
| `checksum` | object | No       | -                                               | Optional hash verification        |

## Registry Entry Format

```yaml
# v2/docker/lib/registry.yaml
extensions:
  my-extension:
    category: dev-tools
    description: Short description for listing
    dependencies:
      - nodejs
      - python
    protected: false
```

| Field          | Type          | Required | Description                           |
| -------------- | ------------- | -------- | ------------------------------------- |
| `category`     | enum          | Yes      | Must match extension.yaml category    |
| `description`  | string        | Yes      | Short description for listings        |
| `dependencies` | array[string] | No       | Other extension names                 |
| `protected`    | boolean       | No       | Prevent modification (default: false) |

## Capabilities Section (Optional)

**IMPORTANT:** Capabilities are optional and only needed for extensions requiring project initialization, authentication, hooks, or MCP integration. Most extensions (nodejs, python, docker) don't need this section.

### Project-Init Capability

```yaml
capabilities:
  project-init:
    enabled: true
    commands:
      - command: "mytool init --force"
        description: "Initialize mytool"
        requiresAuth: anthropic
        conditional: false
    state-markers:
      - path: ".mytool"
        type: directory
        description: "Config directory"
    validation:
      command: "mytool --version"
      expectedPattern: "^\\d+\\.\\d+"
      expectedExitCode: 0
```

| Field                         | Type    | Required | Values                                  | Description                        |
| ----------------------------- | ------- | -------- | --------------------------------------- | ---------------------------------- |
| `enabled`                     | boolean | Yes      | -                                       | Enable project initialization      |
| `commands[].command`          | string  | Yes      | -                                       | Command to execute                 |
| `commands[].description`      | string  | Yes      | -                                       | Human-readable description         |
| `commands[].requiresAuth`     | enum    | No       | `anthropic`, `openai`, `github`, `none` | Auth required before running       |
| `commands[].conditional`      | boolean | No       | default: false                          | Only run if conditions met         |
| `state-markers[].path`        | string  | Yes      | -                                       | File/directory path                |
| `state-markers[].type`        | enum    | Yes      | `directory`, `file`, `symlink`          | Type of marker                     |
| `state-markers[].description` | string  | No       | -                                       | Description of marker              |
| `validation.command`          | string  | Yes      | -                                       | Command to validate initialization |
| `validation.expectedPattern`  | string  | No       | -                                       | Regex pattern for validation       |
| `validation.expectedExitCode` | integer | No       | default: 0                              | Expected exit code                 |

### Auth Capability

```yaml
capabilities:
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
        description: "CLI-based features"
```

| Field                        | Type          | Required | Values                                    | Description                    |
| ---------------------------- | ------------- | -------- | ----------------------------------------- | ------------------------------ |
| `provider`                   | enum          | Yes      | `anthropic`, `openai`, `github`, `custom` | Auth provider                  |
| `required`                   | boolean       | No       | default: false                            | Block without auth             |
| `methods`                    | array[enum]   | No       | `api-key`, `cli-auth`                     | Accepted auth methods          |
| `envVars`                    | array[string] | No       | -                                         | Environment variables to check |
| `validator.command`          | string        | No       | -                                         | Command to validate auth       |
| `validator.expectedExitCode` | integer       | No       | default: 0                                | Expected exit code             |
| `features[].name`            | string        | Yes      | -                                         | Feature name                   |
| `features[].requiresApiKey`  | boolean       | Yes      | -                                         | Whether feature needs API key  |
| `features[].description`     | string        | Yes      | -                                         | Feature description            |

### Hooks Capability

```yaml
capabilities:
  hooks:
    pre-install:
      command: "echo 'Pre-install'"
      description: "Pre-installation checks"
    post-install:
      command: "mytool --version"
      description: "Verify installation"
    pre-project-init:
      command: "mytool doctor"
      description: "Health check"
    post-project-init:
      command: "echo 'Done'"
      description: "Completion message"
```

| Field                           | Type   | Required | Description            |
| ------------------------------- | ------ | -------- | ---------------------- |
| `pre-install.command`           | string | Yes      | Command before install |
| `pre-install.description`       | string | No       | Hook description       |
| `post-install.command`          | string | Yes      | Command after install  |
| `post-install.description`      | string | No       | Hook description       |
| `pre-project-init.command`      | string | Yes      | Command before init    |
| `pre-project-init.description`  | string | No       | Hook description       |
| `post-project-init.command`     | string | Yes      | Command after init     |
| `post-project-init.description` | string | No       | Hook description       |

### MCP Capability

```yaml
capabilities:
  mcp:
    enabled: true
    server:
      command: "npx"
      args:
        - "-y"
        - "@mytool/mcp"
        - "start"
      env:
        MYTOOL_MODE: "mcp"
    tools:
      - name: "mytool-action"
        description: "Perform action"
```

| Field                 | Type          | Required | Description             |
| --------------------- | ------------- | -------- | ----------------------- |
| `enabled`             | boolean       | Yes      | Enable MCP server       |
| `server.command`      | string        | Yes      | Command to start server |
| `server.args`         | array[string] | No       | Command arguments       |
| `server.env`          | object        | No       | Environment variables   |
| `tools[].name`        | string        | Yes      | Tool name               |
| `tools[].description` | string        | Yes      | Tool description        |

### Features Capability (V3+ Extensions)

```yaml
capabilities:
  features:
    core:
      daemon_autostart: true
      flash_attention: true
      unified_config: true
    swarm:
      default_topology: hierarchical-mesh
      consensus_algorithm: raft
    llm:
      default_provider: anthropic
      load_balancing: false
    advanced:
      sona_learning: false
      security_scanning: true
      claims_system: false
      plugin_system: true
    mcp:
      transport: stdio
```

| Field                        | Type    | Values                                                       | Default           | Description              |
| ---------------------------- | ------- | ------------------------------------------------------------ | ----------------- | ------------------------ |
| `core.daemon_autostart`      | boolean | -                                                            | true              | Auto-start daemon        |
| `core.flash_attention`       | boolean | -                                                            | true              | Enable flash attention   |
| `core.unified_config`        | boolean | -                                                            | true              | Use unified config       |
| `swarm.default_topology`     | enum    | `hierarchical-mesh`, `hierarchical`, `mesh`, `ring`          | hierarchical-mesh | Default swarm topology   |
| `swarm.consensus_algorithm`  | enum    | `raft`, `paxos`, `gossip`, `crdt`, `byzantine`               | raft              | Consensus algorithm      |
| `llm.default_provider`       | enum    | `anthropic`, `openai`, `google`, `cohere`, `local`, `custom` | anthropic         | Default LLM provider     |
| `llm.load_balancing`         | boolean | -                                                            | false             | Enable load balancing    |
| `advanced.sona_learning`     | boolean | -                                                            | false             | Enable SONA learning     |
| `advanced.security_scanning` | boolean | -                                                            | false             | Enable security scanning |
| `advanced.claims_system`     | boolean | -                                                            | false             | Enable work claims       |
| `advanced.plugin_system`     | boolean | -                                                            | true              | Enable plugin system     |
| `mcp.transport`              | enum    | `stdio`, `http`, `websocket`                                 | stdio             | MCP transport protocol   |

## Environment Variables Available

These variables are available in scripts and templates:

| Variable           | Value                           | Description            |
| ------------------ | ------------------------------- | ---------------------- |
| `$HOME`            | `/alt/home/developer`           | User home directory    |
| `$WORKSPACE`       | `/alt/home/developer/workspace` | Workspace directory    |
| `$DOCKER_LIB`      | `/docker/lib`                   | Immutable system files |
| `$ALT_HOME`        | `/alt/home/developer`           | Alternative home path  |
| `$MISE_DATA_DIR`   | `$HOME/.local/share/mise`       | mise data directory    |
| `$MISE_CONFIG_DIR` | `$HOME/.config/mise`            | mise config directory  |
| `$MISE_CACHE_DIR`  | `$HOME/.cache/mise`             | mise cache directory   |
| `$MISE_STATE_DIR`  | `$HOME/.local/state/mise`       | mise state directory   |

## Docs Section

The `docs` section provides structured documentation for the extension that can be rendered in markdown format via `sindri extension docs <name>`. This section is **required** for all extensions.

```yaml
docs:
  title: "Node.js"
  overview: |
    Node.js LTS via mise with pnpm package manager. Provides a complete
    Node.js development environment with modern package management.
  last-updated: "2026-01-26"
  usage:
    - section: "Running Node.js"
      examples:
        - description: "Check version"
          code: |
            node --version
        - description: "Run a JavaScript file"
          code: |
            node app.js
    - section: "Package Management"
      examples:
        - description: "Install dependencies"
          code: |
            pnpm install
  related:
    - name: nodejs-devtools
      description: "TypeScript, ESLint, and Prettier"
    - name: playwright
      description: "Browser automation"
```

| Field          | Type   | Required | Description                                     |
| -------------- | ------ | -------- | ----------------------------------------------- |
| `title`        | string | Yes      | Display title for the extension                 |
| `overview`     | string | Yes      | Multi-line description of the extension         |
| `last-updated` | string | Yes      | ISO date (YYYY-MM-DD) of last documentation update |
| `usage`        | array  | No       | Usage examples grouped by section               |
| `related`      | array  | No       | Related extensions                              |

### Usage Sub-fields

| Field      | Type   | Required | Description                    |
| ---------- | ------ | -------- | ------------------------------ |
| `section`  | string | Yes      | Section name for grouping      |
| `examples` | array  | Yes      | Code examples for this section |

**Example Fields:**

| Field         | Type   | Required | Description              |
| ------------- | ------ | -------- | ------------------------ |
| `description` | string | Yes      | What the example does    |
| `code`        | string | Yes      | Shell commands (literal) |

### Related Sub-fields

| Field         | Type   | Required | Description                    |
| ------------- | ------ | -------- | ------------------------------ |
| `name`        | string | Yes      | Extension name                 |
| `description` | string | Yes      | Brief description of extension |
