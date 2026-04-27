# Authoring a Sindri Component Manifest

This document explains how to write a `component.yaml` for the Sindri v4 registry. It is aimed at maintainers who want to publish a tool as a first-class Sindri component, and at contributors who are adding components to `registry-core`. You will finish this guide having written a complete, lint-clean manifest from scratch.

For design rationale, consult the ADRs cross-linked throughout this document.

---

## Concepts Before You Write

### One tool, one component ([ADR-002](architecture/adr/002-atomic-component-unit.md))

Each `component.yaml` wraps exactly one logical tool and one install backend. If your tool requires two operations — for example, installing a system package and then running a configure script — model them as two separate components connected with a `dependsOn` edge.

### Backend-addressed install ([ADR-004](architecture/adr/004-backend-addressed-manifest-syntax.md))

The backend (`mise`, `npm`, `binary`, `script`, etc.) is explicit in the manifest and in the user's `sindri.yaml`. There is no silent auto-pick.

### Policy gates ([ADR-008](architecture/adr/008-install-policy-subsystem.md))

During `sindri resolve`, every component passes through four admission gates:

1. **Platform eligibility** — the current host matches at least one entry in `platforms:`.
2. **Policy eligibility** — the resolved policy allows the license, source, and capabilities.
3. **Dependency closure** — every transitive dependency also passes gates 1 and 2.
4. **Capability trust** — `collision_handling` and `project_init` from third-party registries are gated by the trust policy.

### Cross-platform coverage ([ADR-009](architecture/adr/009-cross-platform-backend-coverage.md))

Declare every platform your component supports. Resolution fails loudly for undeclared platforms rather than falling back silently. Prefer typed backends (`mise`, `brew`, `winget`) over `script` wherever possible.

### Script lifecycle contract ([ADR-024](architecture/adr/024-script-component-lifecycle-contract.md))

If your component uses the `script` backend, the CLI injects `SINDRI_COMPONENT_VERSION` before every lifecycle script. Your `install.sh` must implement version-aware idempotency using the `at_version` helper.

---

## Component Manifest Schema

The canonical schema is at [`v4/schemas/component.json`](../schemas/component.json). A minimal valid manifest:

```yaml
apiVersion: sindri.dev/v4
kind: Component
metadata:
  name: my-tool
  version: "1.0.0"
  description: "A great tool"
  license: MIT
  homepage: https://example.com/my-tool
  tags:
    - utility

platforms:
  - os: linux
    arch: x86_64
  - os: macos
    arch: aarch64

install:
  binary:
    url_template: "https://example.com/releases/my-tool-{version}-{os}-{arch}.tar.gz"
    install_path: "~/.local/bin/my-tool"
    checksums:
      linux-x86_64: "sha256:abcdef1234567890..."
      macos-aarch64: "sha256:abcdef0987654321..."

depends_on: []
```

---

## Field Reference

### `metadata`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | Yes | Lowercase, hyphens only. Must be unique within a registry. |
| `version` | string | Yes | The tool version this component installs, not the manifest version. Use exact semver or the tool's own versioning scheme. |
| `description` | string | Yes | One sentence, under 80 characters. |
| `license` | string | Yes | SPDX identifier (e.g. `MIT`, `Apache-2.0`, `GPL-3.0-only`). Used by policy gates. |
| `homepage` | string | No | Upstream project URL. |
| `tags` | string[] | No | Free-form tags for `sindri search`. |

### `platforms`

List of `{ os, arch }` pairs the component supports. Every platform a user might install on must be listed explicitly.

Valid `os` values: `linux`, `macos`, `windows`.
Valid `arch` values: `x86_64`, `aarch64`.

```yaml
platforms:
  - os: linux
    arch: x86_64
  - os: linux
    arch: aarch64
  - os: macos
    arch: aarch64
  - os: windows
    arch: x86_64
```

Note: macOS `x86_64` (Intel) is intentionally out of scope for v4.0 (Apple-deprecated host). See [ADR-009](architecture/adr/009-cross-platform-backend-coverage.md).

### `install`

The install block declares how the tool is installed. Exactly one of the sub-blocks must be present in `default:`. Per-platform `overrides:` override the default on matching platforms.

#### `mise` backend

```yaml
install:
  mise:
    tools:
      node: "22.0.0"
```

The `tools` map is passed directly to `mise install`. The key is the mise plugin name; the value is the version.

#### `binary` backend

```yaml
install:
  binary:
    url_template: "https://github.com/{repo}/releases/download/v{version}/tool-{os}-{arch}.tar.gz"
    install_path: "~/.local/bin/my-tool"
    checksums:
      linux-x86_64: "sha256:..."
      linux-aarch64: "sha256:..."
      macos-aarch64: "sha256:..."
      windows-x86_64: "sha256:..."
```

Template variables: `{version}`, `{os}` (lowercase), `{arch}` (lowercase).

Binary components **must** declare checksums for every supported platform. `sindri registry lint` enforces this.

#### `npm` backend

```yaml
install:
  npm:
    package: "@anthropic-ai/claude-code"
    version: "1.0.0"
    global: true
```

#### `script` backend

```yaml
install:
  script:
    install_sh: "install.sh"
    uninstall_sh: "uninstall.sh"
    validate_sh: "validate.sh"
    upgrade_sh: "upgrade.sh"
```

Every script receives `SINDRI_COMPONENT_VERSION` from the CLI (see [ADR-024](architecture/adr/024-script-component-lifecycle-contract.md)). Scripts must implement version-aware idempotency.

#### Per-platform overrides

```yaml
install:
  default:
    binary:
      url_template: "..."
  overrides:
    macos-aarch64:
      brew:
        package: my-tool
    linux-x86_64:
      apt:
        packages:
          - my-tool
```

### `depends_on`

List of component addresses (in `backend:name` format) that must be installed before this component. The resolver builds a DAG from all `depends_on` edges and topologically sorts the install order.

```yaml
depends_on:
  - mise-config
  - binary:gh
```

### `options`

Typed user-configurable options exposed in `sindri.yaml`.

```yaml
options:
  enable_corepack:
    type: bool
    default: true
  extra_flags:
    type: string
    default: ""
```

### `capabilities`

Optional capabilities block for post-install integration.

```yaml
capabilities:
  hooks:
    pre_install:
      - command: "echo 'about to install'"
    post_install:
      - command: "node --version"
    pre_project_init:
      - command: "corepack enable"
    post_project_init:
      - command: "npm install"

  project_init:
    - name: "Install dependencies"
      command: "npm install"
      workdir: "."
      priority: 10

  collision_handling:
    path_prefix: "nodejs/"   # must start with `{name}/`
    on_conflict: skip        # skip | stop | proceed

  mcp:
    server_command: "my-tool mcp serve"
    protocol: "stdio"
```

**Collision handling path prefix rule:** the `path_prefix` field must start with `{component-name}/`. Core registry components may also use `:shared`. See [ADR-008](architecture/adr/008-install-policy-subsystem.md) Gate 4 and [ADR-024](architecture/adr/024-script-component-lifecycle-contract.md).

### `validate`

Commands run by `sindri validate` and `sindri doctor --components` to assert the tool is correctly installed.

```yaml
validate:
  commands:
    - name: node
      version_flag: "--version"
      expected_pattern: "v22\\."
```

---

## Worked Example: `gh` CLI via Binary Backend

The following is a complete, lint-clean component manifest for the GitHub CLI (`gh`).

```yaml
apiVersion: sindri.dev/v4
kind: Component
metadata:
  name: gh
  version: "2.67.0"
  description: "GitHub CLI — work with GitHub from the command line"
  license: MIT
  homepage: https://cli.github.com
  tags:
    - developer-tools
    - github
    - vcs

platforms:
  - os: linux
    arch: x86_64
  - os: linux
    arch: aarch64
  - os: macos
    arch: aarch64
  - os: windows
    arch: x86_64

install:
  default:
    binary:
      url_template: >-
        https://github.com/cli/cli/releases/download/v{version}/gh_{version}_{os}_{arch_alias}.tar.gz
      install_path: "~/.local/bin/gh"
      checksums:
        linux-x86_64:   "sha256:aaaa..."
        linux-aarch64:  "sha256:bbbb..."
        macos-aarch64:  "sha256:cccc..."
        windows-x86_64: "sha256:dddd..."
  overrides:
    macos-aarch64:
      brew:
        package: gh
    linux-x86_64:
      apt:
        packages:
          - gh

depends_on: []

validate:
  commands:
    - name: gh
      version_flag: "--version"
      expected_pattern: "gh version 2\\.67"

capabilities:
  hooks:
    post_install:
      - command: "gh --version"
```

---

## Publishing to registry-core

1. Create a directory under `v4/registry-core/components/<name>/`.
2. Write `component.yaml` following this guide.
3. Run the linter: `sindri registry lint v4/registry-core/components/<name>/component.yaml`.
4. Add an entry to `v4/registry-core/index.yaml` with the component name, backend, latest version, license, and OCI reference.
5. Open a PR. The CI workflow (`ci-v4.yml`) will re-run the linter and schema validation.
6. After merge, the registry-core publish workflow (`registry-core-publish.yml`, see [ADR-016](architecture/adr/016-registry-tag-cadence.md)) creates a patch tag and cosign-signs the new OCI artifact (see [ADR-014](architecture/adr/014-signed-registries-cosign.md)).

### OCI reference format

```
ghcr.io/sindri-dev/registry-core/<name>:<version>
```

For collections:

```
ghcr.io/sindri-dev/registry-core/collections/<name>:<version>
```

---

## Common Lint Errors

| Code | Cause | Fix |
|------|-------|-----|
| `LINT_EMPTY_PLATFORMS` | `platforms:` is an empty list | Add at least one platform entry |
| `LINT_MISSING_LICENSE` | `metadata.license` is empty or missing | Add a valid SPDX identifier |
| `LINT_MISSING_CHECKSUMS` | `binary` component has no checksums | Run `sindri registry fetch-checksums` |
| `LINT_COLLISION_PREFIX` | `collision_handling.path_prefix` does not start with `{name}/` | Prefix must be `{component-name}/` |
| `PARSE_ERROR` | YAML is malformed | Check indentation and quoting |

Run `sindri registry lint --json` for machine-readable output in CI.
