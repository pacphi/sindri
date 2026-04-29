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

---

## Auth declarations (ADR-026)

When your component requires a credential — an API token, OAuth flow, X.509
certificate, or SSH key — declare it under the top-level `auth:` block. The
field is additive and `#[serde(default)]`, so existing components without an
`auth:` block continue to work unchanged.

A typical bearer-token declaration looks like this:

```yaml
auth:
  tokens:
    - name: anthropic_api_key
      description: "Anthropic API key used by the Claude Code CLI."
      scope: runtime           # install | runtime | both (default: both)
      optional: false          # if true, install proceeds when no source binds
      audience: "urn:anthropic:api"
      redemption:
        kind: env-var
        env-name: ANTHROPIC_API_KEY
      discovery:
        env-aliases: [ANTHROPIC_API_KEY, CLAUDE_API_KEY]
```

Field semantics live in the ADR; this document focuses on the conventions you
should follow when filling the values in.

### `audience`

The `audience` field identifies the *logical resource* the credential
authenticates against. It is the RFC-9068 audience claim when the token is a
JWT; otherwise treat it as a free-form URL or vendor URN. The resolver
(ADR-027) uses audience matching to bind component requirements to target
capabilities, so consistency across components matters more than perfect
formal correctness.

Use the values in the table below. If you're adding a service not listed,
check the upstream OAuth / OIDC discovery document if one exists; otherwise
mint a sensible URL or `urn:` in the same style.

#### Canonical audience reference

| Provider                      | Audience                              | Used by (examples)                    |
| ----------------------------- | ------------------------------------- | ------------------------------------- |
| **AI providers**              |                                       |                                       |
| Anthropic                     | `urn:anthropic:api`                   | `claude-code`, `claudish`, `compahook`, `ruflo`, `claude-marketplace` |
| OpenAI                        | `https://api.openai.com`              | `codex`, `openclaw`                   |
| Google Generative Language    | `https://generativelanguage.googleapis.com` | `gemini-cli`                    |
| xAI                           | `https://api.x.ai`                    | `grok`                                |
| **Source forges**             |                                       |                                       |
| GitHub                        | `https://api.github.com`              | `gh`, `github-cli`, Go private modules |
| GitLab                        | `https://gitlab.com/api/v4`           | `glab`                                |
| Atlassian                     | `https://api.atlassian.com`           | `jira-mcp`                            |
| **Language registries (P2)**  |                                       |                                       |
| npm                           | `https://registry.npmjs.org`          | `nodejs` (private regs)               |
| PyPI                          | `https://pypi.org`                    | `python` (publish / private indexes)  |
| Maven Central / OSSRH         | `https://repo.maven.apache.org`       | `java` (`mvn deploy`)                 |
| crates.io                     | `https://crates.io`                   | `rust` (`cargo publish`)              |
| Go module proxy (best-guess)  | `https://api.github.com`              | `golang` (private modules via GitHub) |
| **Container registries (P2)** |                                       |                                       |
| Docker Hub                    | `https://hub.docker.com`              | `docker`                              |
| Supabase Management API       | `https://api.supabase.com`            | `supabase-cli`                        |
| **Cloud providers (P1)**      |                                       |                                       |
| AWS STS                       | `https://sts.amazonaws.com`           | `aws-cli`                             |
| Azure ARM                     | `https://management.azure.com`        | `azure-cli`                           |
| GCP                           | `https://www.googleapis.com`          | `gcloud`                              |
| IBM Cloud IAM                 | `https://iam.cloud.ibm.com`           | `ibmcloud`                            |
| Alibaba Cloud                 | `https://ecs.aliyuncs.com`            | `aliyun`                              |
| DigitalOcean                  | `https://api.digitalocean.com`        | `doctl`                               |
| Fly.io                        | `https://api.fly.io`                  | `flyctl`                              |

The list grows. If you migrate a new component and have to invent an audience
string, please open a PR adding a row here so the next author can reuse it.

### `redemption`

Internally-tagged on `kind` (Phase 0 schema). The three variants:

```yaml
# Inject as <ENV_NAME>=<value> into the target's apply env.
redemption:
  kind: env-var
  env-name: ANTHROPIC_API_KEY

# Write to a file path. mode defaults to 0600; persist defaults to false
# (file is deleted post-apply).
redemption:
  kind: file
  path: "/etc/sindri/cert.pem"
  mode: 0o600
  persist: false

# env-var pointing at a file (e.g. GOOGLE_APPLICATION_CREDENTIALS).
redemption:
  kind: env-file
  env-name: GOOGLE_APPLICATION_CREDENTIALS
  path: "/run/secrets/gcp.json"
```

### `optional`

`optional: true` means the install proceeds even if no source binds — the
tool installs in degraded mode and surfaces the missing credential at runtime
(usually as an error from the upstream tool). Use this for:

- Language registry tokens (`nodejs`, `python`, `rust`, `java`, `golang`):
  the toolchain installs fine; private-registry usage is the user's choice.
- Service tokens that public usage doesn't need (`docker` for unauthenticated
  pulls, `supabase-cli` for local dev without the Management API).
- Internal tokens for components that *can* run without them (e.g. `compahook`,
  `claudish`, `claude-marketplace`, `ruflo` declare ANTHROPIC_API_KEY as
  optional even though most users will set it).

`optional: false` means the resolver must bind a source — Gate 5 (Phase 2)
will deny the apply otherwise. Use this for:

- Provider API keys for AI assistants (`claude-code`, `codex`, `gemini-cli`).
- Required OAuth flows where the tool is inert without the token.

### `scope`

When the credential is needed: `install`, `runtime`, or `both` (default).
A token used only by `install.sh` is `install`; a runtime API key is
`runtime`; an authentication token used by both `install` *and* the
installed CLI defaults to `both`.

### `discovery`

Hints to the resolver (ADR-027 §"binding algorithm") about how to find a
source for the requirement automatically. The most common form is a list of
environment-variable aliases:

```yaml
discovery:
  env-aliases: [GITHUB_TOKEN, GH_TOKEN]
```

This lets the resolver recognise — without the user having to wire
`provides:` into a target — that an ambient `GITHUB_TOKEN` in the operator's
shell can satisfy this requirement.

## The `--auth` lint rule

`sindri registry lint --auth <path>` (or
`python3 tools/validate_registry.py --auth`) enables a warning-only rule that
fires on components in credentialed categories (tags: `cloud`, `ai`,
`ai-dev`, `mcp`) that lack an `auth:` block. The rule **never** fails the
build.

To opt out for a specific component, add this comment to the top of
`component.yaml` (must be in the first 8 lines):

```yaml
# sindri-lint: auth-not-required
metadata:
  name: my-component
  ...
```

Use the opt-out sparingly — usually a real `auth:` block is the right move.

## Migration phases

`auth:` declarations land in waves (see
[the implementation plan](plans/auth-aware-implementation-plan-2026-04-28.md)):

- **P0** (highest impact): provider API keys for AI assistants and source
  forges (`claude-code`, `codex`, `gemini-cli`, `gh`, `glab`, …).
- **P1**: cloud-provider CLIs and MCP servers.
- **P2**: language registry tokens (`nodejs`, `python`, `rust`, `java`,
  `golang`) and service-specific tokens (`docker`, `supabase-cli`). All
  marked `optional: true`.
- **P3**: internal Anthropic-team tools (`compahook`, `claudish`,
  `claude-marketplace`, `ruflo`). Marked `optional: true` — internal users
  escalate locally.

If you're adding a new component, jump straight to declaring the right
`auth:` block; you don't need to phase it.
