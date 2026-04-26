# ADR-003: OCI-Only Registry Distribution

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 bundles a single `registry.yaml` (git file) and a `compatibility-matrix.yaml` alongside
the CLI binary. This has several weaknesses:

- Registry updates require a CLI release.
- No content-addressability: checking out the same git sha doesn't guarantee the same
  checksum for downloaded binaries.
- No signing: there is no cosign/sigstore layer verifying the registry.
- Corporate forks are impossible without forking the entire CLI repository.
- No offline mirror story.

Two transport options were evaluated: OCI and git.

## Decision

**OCI is the only production registry transport.** A `registry:local` loader supports
development workflows (component authoring and local smoke-tests before publishing).

### Why OCI over git

| Concern                | OCI                                              | Git                                  |
| ---------------------- | ------------------------------------------------ | ------------------------------------ |
| Content-addressability | Native (manifest digest)                         | Not without explicit tagging by hash |
| Signing                | cosign/sigstore, SLSA provenance                 | Requires GPG+webhooks setup          |
| Corporate mirrors      | Standard OCI mirror (`skopeo copy`, Harbor, ECR) | Fork-and-ssh                         |
| Offline                | OCI layout spec (`oci://path/to/dir`)            | Git clone + no-network flag          |
| Cache invalidation     | Manifest digest comparison (cheap)               | `git fetch` + diff (slower)          |
| Immutable tags         | Enforced by registry policy + CI                 | Convention only                      |
| Codebase complexity    | `oras` / standard OCI client                     | Custom git-fetch + parse layer       |

### Registry artifact structure

```
oci://ghcr.io/sindri-dev/registry-core:2026.04
├── manifest.json            (OCI artifact manifest)
├── signatures/              (cosign signatures)
└── layers/ (tarball):
    ├── index.yaml           (lightweight catalog; one entry per component/collection)
    ├── components/          (atomic components — one dir per tool, no backend prefix)
    │   ├── nodejs/
    │   │   └── component.yaml
    │   └── python/
    │       └── component.yaml
    ├── collections/         (meta-components — sibling of components/)
    │   └── anthropic-dev/
    │       └── component.yaml
    └── checksums/
        └── sha256sums
```

### Directory naming conventions

Component directories use the **simple name** from `metadata.name` — no backend prefix.
The backend is already encoded in the `install.*` block of `component.yaml` and in the
`backend` field of `index.yaml`. Duplicating it in the directory name is noisy and was
eliminated in the initial implementation sprint.

Collections live in a top-level `collections/` directory **sibling to** `components/`.
This makes `ls registry-core/collections/` a zero-ambiguity discovery path and avoids
polluting the atomic-component namespace with `collection-*` dirs.

OCI image references follow the same convention:

- Atomic: `ghcr.io/sindri-dev/registry-core/{name}:{version}` (e.g., `…/nodejs:22.0.0`)
- Collection: `ghcr.io/sindri-dev/registry-core/collections/{name}:{version}` (e.g., `…/collections/anthropic-dev:2026.04`)

### Tag semantics (decided per ADR-016)

- `YYYY.MM` — immutable monthly tag.
- `YYYY.MM.N` — patch tag for component-version additions between monthlies.
- `:latest` / `:stable` — rolling pointers.

### Local development loader

`registry:local:/path/to/registry-dir` allows a maintainer to `sindri add` from a
local checkout of a registry repo without publishing to GHCR. Registry CI runs the
same validation checks via `sindri registry lint`.

### Air-gapped / offline

Deferred to v4.1 for a full spec (open question Q9), but the mechanism is standard:
`skopeo copy oci://ghcr.io/sindri-dev/registry-core:2026.04 oci://my-host/registry-core:2026.04`.
`sindri resolve --offline` uses the cached index and fails loudly if the cache is stale.

### Auth for private registries

OCI auth uses the standard Docker credential store (`docker/config.json` or platform
keychain). `sindri registry add acme oci://ghcr.io/acme/registry-internal:v7` reads
existing Docker auth; no separate credential mechanism.

## Consequences

**Positive**

- Immutable registry tags guarantee reproducibility: same tag → same install on any
  machine at any time.
- Cosign signing (ADR-014) plugs directly into the OCI manifest digest.
- Corporate private registries are trivially supported via any OCI-compliant host.
- CLI binary no longer bundles registry data; registry and CLI version independently.

**Negative / Risks**

- Air-gapped users require an OCI mirror. Well-understood, but must be documented from
  day one.
- First `sindri resolve` requires network access to pull registry indices.
  Mitigated: index files are tiny (KB-range); caching with 24h TTL makes subsequent
  offline use comfortable.
- OCI toolchain dependency (`oras` or equivalent). Mitigated: `oras` is a stable CNCF
  project; the OCI client can be embedded as a Rust library.

## Alternatives rejected

- **Git-hosted registries.** Easy to author (push a tag) but lack content-addressability
  and require bespoke caching. Rejected: the security and reproducibility trade-offs are
  unacceptable.
- **Embedded bootstrap registry.** Bundling a core registry inside the CLI binary for
  offline bootstrap was discussed. Rejected: complicates the release pipeline and couples
  the CLI and registry versions. Open question Q2 resolved as: "No embedded registry;
  first `sindri resolve` pulls."

## References

- Research: `03-proposal-primary.md` §4, `05-open-questions.md` Q1–Q2, `10-registry-lifecycle.md` §1
