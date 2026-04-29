# Migrating from Sindri v3 to v4

**Status:** Living document
**Audience:** Operators and component authors upgrading an existing v3
deployment to v4.

v4 is a Rust-native rewrite, not a strict superset of v3. Most of the
day-to-day surface (`sindri.yaml`, `sindri.lock`, the verb names) carries
across, but several v3 mechanisms have been redesigned in v4 around the
new domain-driven design. This guide tells you what to expect and how to
mechanically translate v3 shapes into v4 shapes.

If you're greenfield, skip this and read [`SOURCES.md`](SOURCES.md),
[`REGISTRY.md`](REGISTRY.md), and [`AUTH.md`](AUTH.md) directly.

---

## Overview — what changed at the architecture level

| Concern             | v3                                              | v4                                                              |
| ------------------- | ----------------------------------------------- | --------------------------------------------------------------- |
| Implementation      | Python + TypeScript console                     | Rust workspace                                                  |
| Component source    | One implicit registry (GitHub-resolved)         | Pluggable [`Source` trait](DDDs/08-registry-source-domain.md) — `oci`, `local-path`, `git`, `local-oci` |
| Registry transport  | GitHub raw URLs                                 | OCI (signed, content-addressed) by default                      |
| Lockfile            | Single `sindri.lock`                            | Per-target lockfile (`sindri.<target>.lock`) — [ADR-018](ADRs/018-per-target-lockfiles.md) |
| Auth                | Implicit (whatever the user's env had)          | Declarative `AuthRequirement` + `AuthBinding` — [ADR-026](ADRs/026-auth-aware-components.md), [ADR-027](ADRs/027-target-auth-injection.md) |
| Trust               | None at the registry level                      | Cosign signatures on every artifact — [ADR-014](ADRs/014-signed-registries-cosign.md) |
| Scripted lifecycle  | Ad-hoc shell hooks                              | Typed lifecycle contract — [ADR-024](ADRs/024-script-component-lifecycle-contract.md) |

The decision records that drove these changes live in
[`ADRs/`](ADRs/), and the per-domain models live in
[`DDDs/`](DDDs/). When in doubt, the ADR explains *why*; the DDD
explains the *types and invariants*.

---

## Registry sources

v3 had no concept of "registry source" — it pulled component definitions
straight from a hard-coded GitHub URL pattern. v4 makes the source
explicit and supports four modes; see [`SOURCES.md`](SOURCES.md) for the
"when to use which" reference.

For a v3-to-v4 mechanical mapping:

- **v3 "resolve from GitHub"** → **v4 `git` source.** Point a
  `type: git` source at the same repo + ref. The lockfile records the
  resolved commit sha rather than the ref, so subsequent resolves are
  reproducible. Add `require_signed: true` if your team signs commits.
  Note: `git` does NOT pass `--strict-oci`; it is for development and
  informal distribution. Production deployments should publish through
  `oci` or `local-oci` instead.

- **v3 "bundled registry" (`sindri vendor` or air-gap dump)** → **v4
  `local-oci` source.** v4 ships
  [`sindri registry prefetch`](SOURCES.md#local-oci--the-air-gap--pre-staged-source)
  which writes an OCI image layout you point a `type: local-oci` source
  at. The bytes are byte-identical to the live OCI registry and pass
  `--strict-oci`.

- **v3 "edit component locally"** → **v4 `local-path` source.** Loose
  `component.yaml` files walked from a directory; same shape v3 used
  internally for development.

The v4 `oci` source is the new default and the only one a published
production deployment should consult.

See [`SOURCES.md`](SOURCES.md) for full per-source guidance, the
`local-path` vs `local-oci` distinction, and `--strict-oci` semantics.

---

## Configuration shape

v3 had a flat `sindri.yaml`:

```yaml
# v3 — implicit GitHub-resolved registry
components:
  - mise:nodejs@20.10.0
  - mise:rust@1.75.0
```

v4 nests the registry config under `registry:` (per
[#258](https://github.com/pacphi/sindri/pull/258)):

```yaml
# v4 — explicit registry sources
registry:
  sources:
    - type: oci
      ref: ghcr.io/sindri-dev/registry-core:2026.04
  policy:
    strict_oci: true
components:
  - address: mise:nodejs
  - address: mise:rust
```

Mechanical translation:

1. Wrap every component reference into the `address:` field of a
   component entry under `components:`.
2. Add a `registry.sources:` list with at least one `oci` entry pointing
   at your published registry artifact.
3. Optionally add `registry.policy.strict_oci: true` to enforce that
   only `oci` and `local-oci` sources are admitted.

A user-level `~/.sindri/config.yaml` may declare global
`registry.sources:` that the project's list prepends to (or replaces, if
the project sets `registry.replace_global: true`). See
[`SOURCES.md` §"Project + global merge semantics"](SOURCES.md#project--global-merge-semantics-phase-41-adr-028-41).

---

## Auth model

v3 had no formal auth model — the operator's shell environment was the
contract. If `gh` was logged in and `npm whoami` worked, things
generally worked.

v4 declares auth explicitly:

- Components advertise `AuthRequirement`s
  ([ADR-026](ADRs/026-auth-aware-components.md)).
- Targets advertise `AuthCapability` bindings
  ([ADR-027](ADRs/027-target-auth-injection.md)).
- The resolver matches them and writes `AuthBinding` entries into the
  lockfile.
- `sindri apply` redeems bindings at apply time.

For full detail see [`AUTH.md`](AUTH.md). The short form: if your v3
deployment depended on ambient auth (e.g. a GitHub PAT in `$GH_TOKEN`),
v4 requires you to either declare it on the component (`auth:`) or wire
it in via target capabilities.

---

## Lockfile shape

v3 wrote a single `sindri.lock` per repository. v4 writes one per
deployment target ([ADR-018](ADRs/018-per-target-lockfiles.md)):

```
sindri.local.lock          # for `sindri apply --target local`
sindri.docker-runner.lock  # for `sindri apply --target docker-runner`
…
```

A v4 lockfile entry now carries a `SourceDescriptor` — the bytes-on-disk
fingerprint of the source that resolved it (DDD-08
§"Lockfile descriptor"):

```yaml
- address: mise:nodejs
  version: 22.0.0
  backend: mise
  source:
    type: oci
    url: ghcr.io/sindri-dev/registry-core
    tag: "2026.04"
    manifest_digest: sha256:0b1a…
```

For `git` sources the descriptor records the resolved **commit sha** —
never the user-supplied ref — so the lockfile is reproducible across
machines even if the upstream branch is force-pushed.

---

## CLI verb changes

| v3 verb            | v4 verb               | Notes                                                                  |
| ------------------ | --------------------- | ---------------------------------------------------------------------- |
| `sindri lock`      | `sindri resolve`      | Same purpose: walk the BOM, write the lockfile. v4 keeps a `--target`. |
| `sindri install`   | `sindri apply`        | v4 separates "compute the plan" (`resolve`) from "execute it" (`apply`). |
| `sindri update`    | `sindri upgrade`      | v4 verb is closer to package-manager idiom.                            |
| `sindri sync`      | (gone — combine `resolve && apply`) | The fused verb hid plan vs apply errors; v4 forces the split.          |
| `sindri search`    | `sindri search`       | Same; reads the discovery index ([DDD-06](DDDs/06-discovery-domain.md)). |
| `sindri ls`        | `sindri ls`           | Same.                                                                  |
| `sindri vendor`    | `sindri registry prefetch` | New name reflects that the output is now an OCI image layout, not a vendor dir. |

Run `sindri --help` for the canonical verb list — this table is a
mechanical mapping, not exhaustive.

---

## What's deferred or removed

These v3 features did not carry over to v4:

- **`v3` extension scripts** (Python). v4 components are typed YAML;
  imperative behaviour lives in
  [script-lifecycle](ADRs/024-script-component-lifecycle-contract.md)
  hooks with a documented contract.
- **The console UI's direct GitHub access.** The console (re-architected
  in [`apps/api`](../../apps/api)) now shells out to `sindri` for every
  registry interaction; there is no "console knows about GitHub" path.
- **Renovate-as-extension.** v3 baked Renovate updates into a regular
  extension. v4 ships a [first-class plugin](../renovate-plugin/) that
  the resolver and registry both call into.
- **HTTPS-tarball and S3 sources.** Listed as Phase 5 polish in
  [`plan/source-modes-implementation.md`](plan/source-modes-implementation.md);
  defer until a real user asks.

---

## See also

- [SOURCES.md](SOURCES.md) — the four registry source modes
- [REGISTRY.md](REGISTRY.md) — OCI bytes-on-the-wire
- [AUTH.md](AUTH.md) — declarative auth
- [ADR-003](ADRs/003-oci-only-registry-distribution.md) — why OCI
- [ADR-028](ADRs/028-component-source-modes.md) — pluggable sources
- [ADR-018](ADRs/018-per-target-lockfiles.md) — per-target lockfiles
- [`plan/source-modes-implementation.md`](plan/source-modes-implementation.md) — the implementation plan
