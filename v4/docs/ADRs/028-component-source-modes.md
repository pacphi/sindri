## ADR-028: Component Source Modes for Development and Air-Gap

**Status:** Proposed
**Date:** 2026-04-28
**Deciders:** sindri-dev team
**Related:** ADR-003 (OCI-only registry distribution), ADR-014 (signed registries),
ADR-016 (registry tag cadence), ADR-018 (per-target lockfiles), DDD-02 (Registry domain),
DDD-03 (Resolver domain)

## Context

ADR-003 established OCI as the **only** production registry transport. That decision
remains correct for distribution: content addressability, cosign signing, mirror tooling,
and offline layout are all native to OCI. It is, however, an over-rotation when read as
"OCI is the only way bytes ever enter the resolver."

In practice the v4 resolver needs to accept components from sources that are intentionally
*not* the production OCI registry:

1. **Inner-loop authoring.** A maintainer editing `components/<name>/component.yaml` on
   their workstation needs `sindri resolve` / `sindri apply` to see those edits without a
   publish round-trip.
2. **Cross-machine preview.** A reviewer on another machine should be able to point at a
   feature branch of the registry repository and exercise the full pipeline before
   anything is pushed to GHCR or signed.
3. **Release candidates.** A `rc-` cut must be reachable to early adopters before cosign
   signing of the final monthly tag is performed.
4. **Air-gapped CI / offline developer images.** A pre-resolved closure must be packable
   into a portable artifact (OCI layout directory or tarball) and consumable from disk
   with no outbound network.
5. **Per-component overrides.** During investigation of a regression, a maintainer may
   want to substitute *one* component from a local checkout while pulling everything else
   from the production OCI registry.

`registry:local:/path/to/registry-dir` (mentioned in ADR-003 §"Local development loader")
covers (1) but not (2)–(5), and was never specified as a first-class abstraction. The v3
"resolve from GitHub" path covered (2) and (3) but was dropped without a replacement
when ADR-003 collapsed transport options.

The cost of leaving these modes unspecified is high: maintainers either build ad-hoc
shell scripts that bypass the resolver (losing admission gates, lockfile reproducibility,
SBOM emission) or push half-baked artifacts to the production registry to test them.
Both undermine the supply-chain story ADR-014 is trying to establish.

## Decision

**Generalize the registry as a list of `RegistrySource` entries with explicit types and
priority order.** The resolver consumes a `Vec<RegistrySource>` rather than a single OCI
URL. Lookups walk the list in declared order; first match wins. The lockfile records
which source satisfied each component so that `sindri apply` is reproducible per
environment.

OCI remains the production default and the only mode for which signing is mandatory.
Other modes are gated by per-source policy.

### Source types

```yaml
# v4 sindri.yaml (excerpt)
registry:
  sources:
    - type: local-path
      path: ./local-overrides
      scope: [my-component]                  # optional name filter

    - type: git
      url: https://github.com/org/components-staging
      ref: my-feature-branch                 # branch, tag, or sha
      subdir: registry                       # optional; defaults to repo root

    - type: oci
      url: oci://ghcr.io/sindri-dev/registry-core
      tag: 2026.05

    - type: local-oci                        # OCI layout on disk; the bundle/air-gap mode
      path: ./vendor/registry-core
```

Four source types map cleanly to the five use cases above:

| Source type   | Use cases  | Trust default                                       |
| ------------- | ---------- | --------------------------------------------------- |
| `local-path`  | (1), (5)   | Unsigned; advisory                                  |
| `git`         | (2), (3)   | Unsigned unless `--require-signed`                  |
| `oci`         | production | Signed (cosign required); fail-closed on signature  |
| `local-oci`   | (4)        | Inherits the signing state of the layout it points at |

A `RegistrySource` is the new name; `Registry` (the aggregate from DDD-02) is now an
ordered composition of one or more sources plus shared cache and trust state.

### Resolver ordering and `scope:`

Sources are consulted in declared order. The first source that yields a `ComponentEntry`
for the requested component id wins. The optional `scope:` field on a source restricts
that source to the listed component names; other components fall through to the next
source. This composes cleanly with collections: a maintainer can override a single
component out of a fifty-component closure while everything else continues to come from
production OCI.

The lockfile records the source per resolved component:

```yaml
- id: mise:nodejs
  version: 22.0.0
  source:
    type: oci
    url: oci://ghcr.io/sindri-dev/registry-core
    tag: 2026.05
  blob_digest: sha256:…
- id: script:my-component
  version: 0.0.0-local
  source:
    type: local-path
    path: ./local-overrides
  blob_digest: sha256:…    # computed locally over the file bytes
```

`sindri apply` re-reads the lockfile's recorded source for each component, so a lockfile
resolved against a `local-path` source is *not* magically reproducible on another
machine — by design, because the user opted in to that mode. `sindri lock --strict-oci`
fails if any resolved component used a non-OCI source, gating production deployments.

### Trust and signing per source

Signing is no longer a global gate. ADR-014's "registries must be signed" rule is
preserved for **`oci` sources**, which are the production transport. Other source types
default to advisory verification:

| Source       | Default verification                                 | Override                       |
| ------------ | ---------------------------------------------------- | ------------------------------ |
| `local-path` | None                                                 | n/a                            |
| `git`        | None; commit sha recorded in lockfile for traceability | `--require-signed` rejects unsigned commits when configured with a key |
| `oci`        | cosign signature required (fail-closed)              | `--no-verify` per ADR-014      |
| `local-oci`  | Inherits original artifact's cosign signature if present | `--require-signed` enforces |

`sindri lock --strict-oci` is the production gate: it fails the resolve if any source
used was not `oci` with a verified signature. CI pipelines that produce shippable
lockfiles MUST pass `--strict-oci`. Inner-loop development MUST NOT.

### CLI surface

Two new verbs make the source modes practical without ad-hoc tooling:

- **`sindri registry serve [--root ./components] [--addr 127.0.0.1:5000]`** —
  spins up an ephemeral local OCI registry over a components directory. Lets a
  maintainer test the full cosign + content-addressing path locally without pushing
  anywhere. Implemented as a thin wrapper over the existing OCI client codepath (the
  registry implementation is `oci-distribution-spec` v1.1).
- **`sindri registry prefetch <oci-ref> [--target air-gap.tar | --layout ./oci-layout]`** —
  resolves the full closure of a registry tag once and writes either a portable tarball
  or an OCI image layout directory. The output is consumed by a `local-oci` source.
  Air-gapped CI runs `sindri registry prefetch` once on a connected build host, ships
  the artifact, and runs `sindri apply` offline.

A third verb already specified by ADR-011 is unaffected: `sindri registry add` keeps its
existing semantics for `oci` sources and gains `--type git|local-path|local-oci` flags.

### What is *not* changing

- ADR-003's claim that OCI is the only **production** transport. `oci` remains the only
  source type allowed under `--strict-oci`.
- ADR-014's signing requirements for the `sindri/core` registry. The trust model
  (hardcoded public key for sindri/core, explicit `sindri registry trust` for third
  parties) applies to OCI sources unchanged.
- ADR-016's tag cadence. Source-mode policy is orthogonal to tag immutability; an `oci`
  source pointing at `2026.05` continues to fail CI if that tag is republished.
- The lockfile schema is **extended**, not replaced. Lockfiles produced before this
  change are read by treating an absent `source:` block as `{type: oci, ...}` reconstructed
  from the historical `registry:` field.

### Bundle mode (the v3 "all-in-image" pattern)

The fourth source type, `local-oci`, is the canonical answer to v3's bundled-registry
ergonomics. The OCI image layout spec already describes the on-disk format; `oras` and
`sigstore-rs` both read it directly. Sindri prescribes no new format here — `local-oci`
points at a directory in OCI image layout v1.1, period.

Bundle workflow:

```bash
# Build host (online):
sindri registry prefetch oci://ghcr.io/sindri-dev/registry-core:2026.05 \
    --layout ./vendor/registry-core

# Air-gapped host (offline):
sindri lock --target air-gap         # registry source is local-oci ./vendor/...
sindri apply --target air-gap
```

### Why this is not a back-door around ADR-003

ADR-003 says: "OCI is the only production transport." That is preserved.
This ADR says: "Development and air-gap workflows need additional, clearly-typed
sources." Neither weakens the supply-chain story:

- The default sindri.yaml the `init` command writes contains a single `oci` source
  pointing at `sindri/core`. Users opt into other source types deliberately.
- `--strict-oci` is a one-flag CI gate. Production pipelines flip it on once and
  forget it.
- Every component resolved from a non-OCI source is recorded as such in the lockfile,
  in the StatusLedger, and in the SBOM. There is no path by which an unsigned component
  silently lands in a strict-OCI deployment.

## Consequences

**Positive**

- Maintainers iterate on components without round-tripping through GHCR; review cycles
  collapse from minutes to seconds.
- Cross-machine PR previews are first-class — point a colleague's sindri.yaml at a git
  source on your fork and they exercise the same resolve→apply path the production user
  will exercise.
- Air-gapped CI gets a documented, reproducible path that does not require running
  `skopeo` outside of Sindri.
- Per-component overrides allow surgical bisection of regressions without rebuilding the
  whole registry.
- Trust policy is now explicit per-source rather than implicit per-transport, which is
  easier to reason about during audits.

**Negative / Risks**

- More source types means more code paths and more documentation surface. Mitigation:
  every source type goes through the same `RegistrySource` trait; the resolver is
  source-agnostic.
- The `--strict-oci` flag must be obvious in CI templates or maintainers will accidentally
  ship lockfiles resolved from `local-path` sources. Mitigation: ship a CI template that
  enables it; emit a loud warning at the top of every non-strict resolve report.
- A `git` source with a mutable ref (a branch name) is non-reproducible by definition.
  Mitigation: the lockfile records the resolved commit sha, not the ref. Re-running
  `sindri lock` against the branch is what produces drift; running `sindri apply` against
  the existing lockfile is reproducible.
- Test surface grows: each source type needs its own resolution tests. Mitigation:
  shared fixture harness; source-trait conformance tests.

## Alternatives rejected

- **Leave `registry:local:` as the only escape hatch.** Already insufficient for git
  preview, per-component override, and air-gap. Continuing to bolt features onto
  `registry:local:` re-creates the same overloaded interface this ADR replaces.
- **Push `git` resolution outside Sindri (use `git clone` then `registry:local:`).**
  Sacrifices reproducibility (the lockfile cannot record the commit sha if Sindri never
  saw the git URL) and forces every CI to wrap the same shell script.
- **Make signing a per-component declaration rather than per-source.** Doubles the
  schema surface, complicates `component.yaml`, and conflates "where did the bytes
  come from" with "are the bytes trusted." Source-level trust is simpler and matches
  how OCI registries already work.
- **Treat `local-oci` as a flag on `oci` (`url: file://...`).** Conflates two distinct
  resolution paths (network OCI client vs. on-disk layout reader). Separate types let
  the implementation share trait surface but pick the right backend per source.

## Open questions (deferred)

- **Q1.** Should `sindri registry prefetch` also bundle target-specific binaries (the
  `apply`-time downloads driven by ADR-010)? Probably yes, behind a `--with-binaries`
  flag — but the prefetch step then needs to know the target's `Platform`. Defer to
  v4.1; the minimum viable version of `prefetch` ships the registry artifact only.
- **Q2.** Should `git` source caching share storage with the OCI blob cache (DDD-02
  `~/.sindri/cache/registries/`)? Probably yes, namespaced as
  `~/.sindri/cache/registries/git/<sha>`, but that is an implementation detail of
  DDD-02 §Cache Model, not a source-modes decision.
- **Q3.** Is `--strict-oci` better expressed as a project-level setting in `sindri.yaml`
  (`registry.policy.strict_oci: true`) rather than a flag? Probably both — flag for
  CI override, project setting for "this repository's lockfiles must always be
  production-grade." Specify in the implementation plan, not here.

## References

- ADR-003 (OCI-only registry distribution) §"Local development loader"
- ADR-014 (Signed registries with cosign) §"Trust model"
- ADR-016 (Registry tag cadence)
- ADR-018 (Per-target lockfiles) — lockfile is the contract that records source per
  component
- DDD-02 (Registry domain) — `Registry` aggregate becomes a composition of sources
- DDD-03 (Resolver domain) — `Lockfile.ResolvedComponent` gains a `source:` field
- v3 prior art: the `resolve from GitHub` path, dropped without replacement when
  ADR-003 collapsed transport options
- OCI image layout spec v1.1 (referenced for `local-oci` on-disk format)
