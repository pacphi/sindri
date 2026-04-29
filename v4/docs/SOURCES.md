# Sindri v4 Component Sources

This document explains the four registry source modes Sindri v4 supports —
`oci`, `local-path`, `git`, and `local-oci` — what each is for, when to reach
for it, and how `--strict-oci` decides which are allowed in production.

It is aimed at maintainers, platform engineers, and component authors trying
to decide how to wire their workflow. The decision record is
[ADR-028](ADRs/028-component-source-modes.md); the domain model is
[DDD-08](DDDs/08-registry-source-domain.md); the bytes-on-the-wire of OCI
itself live in [REGISTRY.md](REGISTRY.md).

If you only consume a published registry and have never asked yourself "how
do I test this component before it ships," you can stop reading after the
**TL;DR** — you want `oci`.

---

## TL;DR

| Source        | Format on disk            | Signed | Mutable | `--strict-oci` | Primary use case                          |
| ------------- | ------------------------- | ------ | ------- | -------------- | ----------------------------------------- |
| `oci`         | OCI registry (network)    | Yes    | No      | **Pass**       | Production. Default. The only one CI must trust. |
| `local-path`  | Loose `component.yaml` files | No   | Yes     | Fail           | Component author inner loop.              |
| `git`         | Git repo + ref            | Optional (`require_signed`) | No (commit-pinned) | Fail | Pre-publish review; informal team distribution. |
| `local-oci`   | OCI image layout dir      | Yes    | No      | **Pass**       | Air-gapped CI; pre-staged build cache.    |

A registry is an *ordered list* of these sources. First match wins, with
optional `scope:` filters per source for per-component overrides. Sindri's
default `init` template writes a single `oci` source — opting into anything
else is deliberate.

---

## The four sources

### `oci` — production default

A real OCI registry (`ghcr.io/sindri-dev/registry-core:<tag>`, your internal
Harbor, ECR, Artifactory, …) hosting a cosign-signed `registry-core` artifact.
Layout, signing flow, and verification rules are in [REGISTRY.md](REGISTRY.md)
and [ADR-014](ADRs/014-signed-registries-cosign.md).

**Reach for it when:**
- You're a normal user running `sindri lock` / `sindri apply`.
- You're running CI against a published, signed registry.
- You don't have a specific reason to pick anything else.

This is the only source `--strict-oci` admits without further configuration.

### `local-path` — the inner-loop authoring source

A directory of loose `component.yaml` files (and an `index.yaml`), walked
directly. No packaging, no signing, no network round-trip.

```yaml
registry:
  sources:
    - type: local-path
      path: ./my-components
```

**Reach for it when:**
- You're *writing* a component and need `cargo run -- lock` to pick up your
  edits the moment you save.
- You're testing layout changes ("does this component group right under
  `collections/`?") that don't depend on signing.
- You're running an internal dev workshop where instructors want to ship
  iterations without standing up a registry.

**Don't reach for it when:** anything downstream depends on
content-addressed digests or signature verification. `local-path` is
unsigned and mutable by design — `--strict-oci` rejects it for exactly that
reason.

### `git` — restores v3's "resolve from GitHub" pattern

A git repository + ref (+ optional `subdir`), resolved to a commit sha that
is recorded in the lockfile. v4 uses `git2` (libgit2 bindings) so resolution
is deterministic across operator installs and supports sparse checkout.

```yaml
registry:
  sources:
    - type: git
      url: https://github.com/acme/sindri-components.git
      ref: feat/new-tool
      subdir: components
      require_signed: true   # optional — reject unverified commits
```

**Reach for it when:**
- **Pre-publish review.** A contributor opened a PR adding a component;
  point a `git` source at the fork's branch to test it before the author
  cuts a release and pushes the OCI bundle.
- **Long-tail / informal distribution.** A small internal team keeps
  components in a git repo and doesn't want the operational overhead of
  running an OCI registry just to share three tools.
- **Migrating from v3.** v3 resolved components directly from GitHub URLs;
  the v4 `git` source preserves that ergonomic for teams who depended on it.

**Don't reach for it when:** you need byte-identical artifacts across
machines. Even with `require_signed`, git is reference-resolved (commits can
be force-pushed onto a different sha until you pin one) — pin the sha
yourself, or push to OCI for true content-addressing.

`--strict-oci` rejects `git` sources by design; `git` is a development and
distribution tool, not a production transport.

### `local-oci` — the air-gap / pre-staged source

An **OCI image layout on disk** — the same `index.json` + sha256-addressed
blob directory an OCI registry would serve, just sitting in a directory.
Read with `oci-spec`, signatures verified offline with `sigstore-rs`.

```yaml
registry:
  sources:
    - type: local-oci
      layout: ./vendor/registry-core
```

**Reach for it when:**
- **Air-gapped environments.** Bank, defense, regulated industry. The build
  server has no internet. You run
  `sindri registry prefetch oci://ghcr.io/… --layout ./vendor/registry-core`
  on a connected machine, copy the directory across the air gap, and the
  air-gapped CI runs `sindri lock` with `--strict-oci` against the layout.
  It gets the *same signed artifacts* it would have gotten from the live
  registry — same digests, same cosign signatures, same trust.
- **Reproducible CI without registry round-trips.** Pre-fetch once into a
  build image; every CI job reads from disk. No flaky network, no rate
  limits, faster builds. Still strict-OCI compliant.
- **Developer image bundles.** Ship a `sindri-bundle.tar` to a contractor's
  laptop; they run `sindri lock` offline and still get verified components.

`--strict-oci` admits `local-oci` because the trust properties are identical
to a live OCI source: signed, content-addressed, immutable.

---

## `local-path` vs `local-oci` — why they aren't the same source

This is the most common point of confusion. Both are "stuff on disk," but
they answer opposite questions:

| Dimension         | `local-path`                  | `local-oci`                       |
| ----------------- | ----------------------------- | --------------------------------- |
| Format            | Loose `component.yaml` files  | OCI image layout (`index.json` + sha256-addressed blobs) |
| Signed            | No                            | Yes (embedded cosign signatures)  |
| Byte-identical to OCI? | No                       | **Yes** — same digests as the registry it was prefetched from |
| Passes `--strict-oci`? | **No**                   | **Yes**                           |
| Mutable?          | Yes — you edit the files      | No — content-addressed; tampering fails verification |
| Intended editor   | Component author              | Nobody — it's a build artifact    |

- `local-path` = *"I'm **producing** a component and want zero ceremony."*
  Unsigned, mutable, dev-only.
- `local-oci` = *"I'm **consuming** components and have no network or want
  determinism."* Signed, immutable, strict-OCI compliant.

Collapsing them into a single "local" source would force one of two losses:

- Make it OCI-shaped → kill the inner-loop ergonomics; authors must package
  and (effectively) re-sign on every save.
- Make it loose-files-shaped → strict-OCI now passes for unsigned
  directories, defeating the whole gate.

Keeping them distinct lets each be the *best* tool for its job.

---

## `--strict-oci` — what it actually checks

After resolution, `--strict-oci` walks the lockfile and rejects any
component whose `SourceDescriptor` came from a source that returns `false`
from `Source::supports_strict_oci()`. Today:

- `oci` — passes when its trust config is satisfied (cosign verifies for
  `sindri/core` or for an explicitly trusted third party).
- `local-oci` — passes under the same trust rule, applied to the embedded
  signatures.
- `local-path`, `git` — never pass.

Failure surfaces as `AdmissionCode::SourceNotProductionGrade` with a list of
the offending components and which source they came from. In non-strict
mode, the resolver prints a loud warning at the top of every report listing
the source mix — strict-OCI is opt-in, but you should never be surprised by
it.

CI templates that enable `--strict-oci` will ship under `v4/docs/ci/` as part
of Phase 4 of the implementation plan; this guide will link to them once they
land.

---

## Composition rules

A `sindri.yaml` may declare multiple sources:

```yaml
registry:
  sources:
    - type: local-path
      path: ./my-components
      scope: [acme-internal-tool]    # only this component routes here
    - type: oci
      ref: ghcr.io/sindri-dev/registry-core:2026.04
```

Resolution rules:

- **First match wins.** Sources are consulted in declared order; the first
  one that yields the requested component is used.
- **`scope:` filters.** A source with `scope: [a, b]` only contributes to
  resolutions for `a` and `b`; everything else falls through.
- **Project sources prepend to global sources by default.** Set
  `registry.replace_global: true` to override the user-level config entirely.
- **The lockfile records which source actually resolved each component**
  (the `SourceDescriptor`). `sindri apply` re-reads only the lockfile —
  if a descriptor cannot be re-resolved (e.g. the `local-path` directory
  was deleted), apply fails loudly.

---

## Phase status

| Source        | Status                                         | Phase |
| ------------- | ---------------------------------------------- | ----- |
| `local-path`  | **Implemented** — `LocalPathSource`            | 1     |
| `oci`         | **Implemented** — `OciSource` wraps the existing `RegistryClient` (oci-client + cosign + cache) | 2 |
| `local-oci`   | **Implemented** — `LocalOciSource` reads OCI image-layout v1.1 directories | 2 |
| `git`         | Stub; `git2`-backed, sparse checkout, commit-pinned | 3 |
| `--strict-oci` admission gate | **Implemented** — CLI flag + `registry.policy.strict_oci` config | 2 |
| `sindri registry serve` / `prefetch` CLI verbs | Pending      | 3     |

See [`plan/source-modes-implementation.md`](plan/source-modes-implementation.md)
for the full sprint breakdown.

---

## See also

- [ADR-028](ADRs/028-component-source-modes.md) — the decision record
- [DDD-08](DDDs/08-registry-source-domain.md) — domain model and trait surface
- [REGISTRY.md](REGISTRY.md) — OCI bytes-on-the-wire, cosign flow
- [AUTHORING.md](AUTHORING.md) — writing components (the natural `local-path` audience)
- [`plan/source-modes-implementation.md`](plan/source-modes-implementation.md) — implementation sprints
