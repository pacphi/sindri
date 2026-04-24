# ADR-002: Atomic Component Replaces Extension

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3's `extension.yaml` is a capable but coarse-grained unit. Three problems stand out:

1. **Bundle extensions.** `ai-toolkit` installs Fabric + Codex + Gemini + Droid + Grok as one
   indivisible unit. A user who only wants Codex must install all five. Same for
   `cloud-tools` (7 cloud CLIs) and `infra-tools` (14 tools).

2. **Hybrid install method.** `InstallMethod::Hybrid` exists because some extensions need
   both a distro package manager _and_ a post-install script. This conflates "install" with
   "configure" and requires bespoke dispatch logic.

3. **The term "extension".** v3 docs, subcommands, and user muscle-memory use "extension".
   The term is overloaded across many tools. "Component" matches SBOM vocabulary, the
   `dependsOn` model, and the devcontainer-features shape that inspires this design.

## Decision

### 1. Rename: Extension → Component

Replace `extension.yaml` and `kind: Extension` with `component.yaml` and `kind: Component`
everywhere in the v4 CLI, schema, registry, and documentation. The rename is a clean break
(v4 is a deliberate breaking change; no migration path).

CLI subcommands change accordingly:

- `sindri extension install` → `sindri add <backend>:<name>` + `sindri apply`
- `sindri extension list` → `sindri ls`
- etc. (see ADR-011)

### 2. Atomicity principle

Each component wraps **one logical tool** via **one install backend**. If a tool requires
two operations (e.g., `apt:docker-ce` + config script), they become two components with
a `dependsOn` edge:

```yaml
# docker-post-install/component.yaml
kind: Component
metadata: { name: docker-post-install }
dependsOn:
  - apt:docker-ce
install:
  script:
    install.sh: "..." # sets storage-driver, DinD config, user group
```

The bundle extensions decompose:

| v3 Bundle     | v4 atomic components                                                                            |
| ------------- | ----------------------------------------------------------------------------------------------- |
| `ai-toolkit`  | `npm:codex@openai`, `npm:claude-code`, `npm:gemini-cli@google`, `binary:fabric`, `binary:droid` |
| `cloud-tools` | `binary:aws-cli`, `mise:gcloud`, `binary:flyctl`, `binary:doctl`, … (7 separate)                |
| `infra-tools` | `mise:terraform`, `binary:kubectl`, `binary:helm`, `binary:k9s`, … (14 separate)                |

The "install the whole set" UX is preserved via meta-component collections (ADR-006).

### 3. Component manifest shape (v4)

```yaml
apiVersion: sindri.dev/v4
kind: Component
metadata:
  name: nodejs
  version: "22.11.0"
  category: languages
  license: Apache-2.0
  homepage: https://nodejs.org
  backend: mise

platforms:
  - linux-x86_64
  - linux-aarch64
  - macos-aarch64
  - windows-x86_64
  - windows-aarch64

options:
  corepack:
    type: bool
    default: true

dependsOn:
  - mise-config

install:
  default:
    mise:
      tools: ["node@{{ version }}"]
      reshim: true
  overrides:
    macos: { brew: { package: node } }

validate:
  commands:
    - name: node
      versionFlag: "--version"
      expectedPattern: "v22\\.11\\.0"

# Capabilities unchanged from v3:
capabilities:
  hooks: { ... }
  project-init: { ... }
  collision-handling: { ... }
  mcp: { ... }
```

**Key diffs from v3 `extension.yaml`:**

| v3 field                                       | v4 change                                                        |
| ---------------------------------------------- | ---------------------------------------------------------------- |
| `install.method` enum                          | Replaced by one backend block (`mise:`, `apt:`, `binary:`, etc.) |
| `InstallMethod::Hybrid`                        | Deleted. Compose two atomic components via `dependsOn`.          |
| `bom.tools[]`                                  | Deleted. SBOM is emitted by the resolver (ADR-007).              |
| `metadata.version` (extension package version) | Replaced by OCI content digest.                                  |
| `metadata.distros[]`                           | Replaced by `platforms:` list (ADR-009).                         |

### 4. OCI addressing

Components live in OCI registry artifacts (ADR-003). Each component version is
content-addressed by the digest of its `component.yaml` blob. The `metadata.version` field
in v3 (the extension's _own_ version, not the tool's version) is eliminated — OCI digests
provide the integrity and addressability that field was trying to supply.

## Consequences

**Positive**

- Users choose exactly which tools to install. No accidental installs.
- `Hybrid` dispatch logic removed from the executor.
- Collections (ADR-006) are now just components, not a separate type.
- SBOM generation is simpler (ADR-007).

**Negative / Risks**

- Bundle extensions (`ai-toolkit` etc.) must be decomposed before v4.0 ships. Scope is
  known: three bundles → ~26 atomic components + 3 meta-components.
- Component authors must learn the new format. Mitigated by tooling (`sindri registry lint`)
  and the familiarity of the YAML structure.

## Alternatives rejected

- **Keep `extension.yaml`, add a BOM layer (Alternative B from research).** The compatibility
  matrix survives in this scenario; backend choice stays implicit; duplicate pinning stays.
  Rejected: doesn't solve the user's core complaints.

## References

- Research: `01-current-state.md` §7, `03-proposal-primary.md` §3, `04-alternatives.md`
- Open question resolved: Q12 (rename "extension" → "component")
