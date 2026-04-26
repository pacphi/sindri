# ADR-004: Backend-Addressed Manifest Syntax (`backend:tool@version`)

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3's extension manifest has no concept of "which package manager installs this?" — the
install method is buried inside each `extension.yaml`. Users have no way to express "I
want kubectl installed via mise, not via binary download."

The prior-art survey identified **mise**'s `[tools]` TOML syntax as the cleanest
"same tool, pick your source" idiom: `"backend:name" = "version"`.

## Decision

Adopt the `backend:name` map-key idiom for `sindri.yaml` `components:`.

### Syntax

Map key is `backend:name` (or `backend:name@qualifier` for scoped names). Value is a
version string, range, or options object.

```yaml
components:
  # backend:name: "version"
  mise:nodejs: "22.11.0"
  mise:python: "3.14.0"

  # Exact version pinned
  binary:aws-cli: "2.17.21"

  # Range (resolved to exact in sindri.lock)
  mise:golang: ">=1.24,<2"

  # Scoped npm package with qualifier
  npm:codex@openai: "2.3.1"

  # Options object
  mise:ruby:
    version: "3.3.6"
    options:
      yjit: true

  # Collection (meta-component)
  collection:anthropic-dev: "2026.04"
```

### Backend identifiers

First-class backends in v4.0:

| Backend      | Platform(s)                                 | Purpose                                       |
| ------------ | ------------------------------------------- | --------------------------------------------- |
| `mise`       | Linux, macOS, Windows                       | Language runtimes via mise                    |
| `apt`        | Linux (Debian/Ubuntu)                       | System packages                               |
| `dnf`        | Linux (Fedora/RHEL)                         | System packages                               |
| `zypper`     | Linux (openSUSE)                            | System packages                               |
| `pacman`     | Linux (Arch)                                | System packages                               |
| `apk`        | Linux (Alpine)                              | System packages                               |
| `brew`       | macOS (primary), Linux (Linuxbrew optional) | macOS-native tools                            |
| `winget`     | Windows                                     | Windows package manager                       |
| `scoop`      | Windows                                     | Dev-tool-oriented Windows PM                  |
| `binary`     | All                                         | Direct GitHub release / URL download          |
| `npm`        | All                                         | npm global packages                           |
| `pipx`       | All                                         | Python CLI tools                              |
| `cargo`      | All                                         | Rust-published CLIs                           |
| `go-install` | All                                         | Go-published CLIs                             |
| `script`     | All                                         | Escape hatch (bash + ps1 variants)            |
| `collection` | Virtual                                     | Meta-component (no install; only `dependsOn`) |

### User choice is explicit; no auto-pick

`mise:python` and `apt:python3` are different map keys. If a user writes `python` without
a backend prefix, `sindri validate` returns schema error `ADM_MISSING_BACKEND`. There is
no silent auto-selection. Open question Q6 resolved as: "backend prefix is authoritative;
no auto-pick mode."

### Version ranges allowed in manifest; exact required in lockfile

Open question Q5 resolved: `sindri.yaml` may contain semver ranges (`">=22,<23"`);
`sindri resolve` expands to exact versions and writes them to `sindri.lock`. `sindri
install` fails if `sindri.lock` is absent or stale (exit code 5).

### Map ordering

`sindri.yaml` uses a map. When install order matters (rare: component A's configure
step must finish before component B's), components may declare `order:` in their
`component.yaml`, which the DAG-resolution step in `sindri resolve` respects.
Open question Q4 resolved: map keys with optional `order:` override.

## Consequences

**Positive**

- User intent is unambiguous: they see exactly which backend installs each tool.
- Same tool across different backends produces different map keys, avoiding silent
  conflict (e.g., `mise:python` coexists with `apt:python3` if both are wanted).
- `backendOrder` preference (ADR-008) is a fallback when the user wants "give me
  the best backend for this OS" without spelling it out every time.

**Negative / Risks**

- Slightly more verbose than `nodejs: "22.11.0"`. Acceptable — the backend choice
  is material and worth making explicit.
- Users from mise/aqua backgrounds will find this familiar; net new users need to
  learn the idiom. Mitigated by `sindri init` wizard which writes entries for you.

## Alternatives rejected

- **List syntax** (`- backend: mise, name: nodejs, version: "22.11"`). Uglier; YAML lists
  don't model "one entry per component" as clearly as a map. Rejected (Q4).
- **Implicit backend lookup** (user writes `nodejs: "22"`, Sindri auto-picks). Hides
  backend choice, brings back the v3 problem. Rejected (Q6).

## References

- Research: `02-prior-art.md` §mise, `03-proposal-primary.md` §1, `05-open-questions.md` Q4–Q6
