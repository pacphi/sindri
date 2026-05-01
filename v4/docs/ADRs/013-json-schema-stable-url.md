# ADR-013: JSON Schema Publication at Stable URL

**Status:** Accepted (transitional URL — see "Publication status" below)
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Publication status (added 2026-04-30, F-XCUT-02)

The canonical schema identity remains
`https://schemas.sindri.dev/v4/<name>.json` (encoded in each schema's
`$id`), but **fetched bytes currently resolve via**
`https://raw.githubusercontent.com/pacphi/sindri/v4/v4/schemas/<name>.json`
until the dedicated subdomain is stood up. `sindri init` and `sindri
edit` emit the transitional URL on both the YAML-LSP pragma and the
JSON-id pragma so YAML-LSP-aware editors fetch a real file.

The transitional base URL is overridable at build time via
`SINDRI_SCHEMA_BASE_URL` (consumed by `sindri-core::well_known`) so a
production build can flip to the canonical host without a code edit.

A follow-up tracking item exists for the `schemas.sindri.dev` host
setup; this ADR will lose the "transitional" qualifier when that host
serves the same content.

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
