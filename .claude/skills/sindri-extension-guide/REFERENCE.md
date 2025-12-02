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
# docker/lib/registry.yaml
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
