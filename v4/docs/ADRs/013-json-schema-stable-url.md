# ADR-013: JSON Schema Publication at Stable URL

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

Editing `sindri.yaml` by hand (or even with `sindri edit`) is error-prone without IDE
autocompletion. Building a VS Code extension is expensive. However, YAML Language Server
support (present in VS Code, Cursor, Helix, Neovim, IntelliJ) is free if a JSON Schema
is published at a stable URL and referenced from the file.

## Decision

Publish JSON Schemas at `https://schemas.sindri.dev/v4/` from day one:

| Schema             | URL                                                 |
| ------------------ | --------------------------------------------------- |
| BOM manifest       | `https://schemas.sindri.dev/v4/bom.json`            |
| Install policy     | `https://schemas.sindri.dev/v4/policy.json`         |
| Component          | `https://schemas.sindri.dev/v4/component.json`      |
| Registry index     | `https://schemas.sindri.dev/v4/registry-index.json` |
| Per-target schemas | `https://schemas.sindri.dev/v4/targets/{kind}.json` |

`sindri init` and `sindri edit` auto-prepend the YAML language-server pragma to
every file they generate:

```yaml
# yaml-language-server: $schema=https://schemas.sindri.dev/v4/bom.json
apiVersion: sindri.dev/v4
kind: BillOfMaterials
...
```

`sindri edit --schema` prints the path to the local schema copy (for air-gapped setups).

### Versioning

Schemas are versioned as part of the `apiVersion` field (`sindri.dev/v4`). A v4.1 that
adds new optional fields is backward-compatible and does not require a new `apiVersion`.
Breaking schema changes require `sindri.dev/v5`.

## Consequences

**Positive**

- Autocomplete and inline error squiggles in any YAML-LSP-enabled editor at zero
  incremental cost beyond schema maintenance.
- Reduces "why did validate fail?" support questions.

**Negative / Risks**

- Schema must stay in sync with the Rust type definitions. Mitigation: generate the
  JSON Schema from `schemars` derive macros on the Rust types in `sindri-core`.

## References

- Research: `09-imperative-ux.md` §6
