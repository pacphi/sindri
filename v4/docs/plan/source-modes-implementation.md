## Sindri v4 — Source Modes Implementation Plan

**Status:** Draft
**Date:** 2026-04-28
**Implements:** ADR-028, DDD-08
**Amends:** `implementation-plan.md` Sprint 2 (registry) and Sprint 7 (CLI verbs)

This is a focused addendum to the v4 implementation plan covering the work to ship
ADR-028 (component source modes) and DDD-08 (registry source domain). It assumes the
existing Sprint 2 deliverables (OCI client, cache, lint, signing) are in place; the
existing `LocalRegistry` becomes the seed of the `LocalPath` source.

The work is sized as roughly **two sprints (≈4 weeks)** for one engineer, plus one
additional sprint of polish and CI integration that runs concurrently with other v4 work.

---

## Principles

- **No production behavior change without `--strict-oci` opt-out.** Default sindri.yaml
  scaffolding stays single-OCI; users opt in to other sources deliberately.
- **Source trait first, source variants second.** The trait contract is the design
  surface; concrete sources are conformance tests.
- **Lockfile is the contract.** Apply-time refetch reads `SourceDescriptor` only; if a
  descriptor cannot be re-resolved, apply fails loudly.
- **Refactor existing `LocalRegistry` rather than re-implement.** The work is mostly
  shaping current code into the new trait, not green-field writing.

---

## Phase 1 — Trait surface and refactor (Sprint A, week 1)

**Goal:** `Source` trait defined, `LocalPathSource` implemented as a refactor of
existing `LocalRegistry`, resolver consults a `Vec<RegistrySource>`. No behavior change
for existing users.

### Tasks

#### 1.1 Define the `Source` trait and `RegistrySource` enum

- [ ] Create `sindri-registry/src/source/mod.rs` with the `Source` trait, `RegistrySource`
  enum, `SourceContext`, `SourceError`, and `SourceDescriptor` types from DDD-08.
- [ ] Add `scope: Option<Vec<ComponentName>>` to every variant; centralize the scope-check
  helper.
- [ ] Implement `RegistrySource::dispatch_*` (one per trait method) so the resolver can
  call through the enum without importing every variant.
- [ ] Add `schemars` derives so the new YAML shape lands in `v4/schemas/registry-source.json`.

#### 1.2 Refactor `LocalRegistry` into `LocalPathSource`

- [ ] Move `sindri-registry/src/local.rs` to `sindri-registry/src/source/local_path.rs`.
- [ ] Implement `Source` for `LocalPathSource`. `fetch_index` walks the path, reads
  `component.yaml` files, builds an in-memory `RegistryIndex`.
- [ ] Remove the old `LocalRegistry` symbol outright (v4 has no external consumers,
  so no transitional alias is kept).
- [ ] Tests: re-target the existing tests at `LocalPathSource`; add three new tests
  covering the scope filter.

#### 1.3 Resolver wiring

- [ ] `sindri-resolver/src/lib.rs` accepts `&[RegistrySource]` instead of a single
  registry handle. Use the existing first-match-wins helper from DDD-03 with the
  scope filter from §1.1.
- [ ] Lockfile gains `source: SourceDescriptor` per resolved component (DDD-08
  §"Lockfile descriptor"). Backfill via `From<old-shape>` for one release: read an
  absent `source:` as `SourceDescriptor::Oci { ... }` reconstructed from the
  pre-existing `registry:` field.
- [ ] Update `sindri-resolver/tests/` fixtures to assert the new field is populated.

### Acceptance criteria

- `cargo build --workspace` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Existing v4 resolve fixtures pass unchanged.
- A new test resolves a single component from `LocalPathSource` and asserts the
  lockfile descriptor is `LocalPath { path }`.

---

## Phase 2 — `OciSource` and `LocalOciSource` (Sprint A, week 2)

**Goal:** Production OCI source is reachable via the new trait; `local-oci` reads OCI
image layouts on disk.

### Tasks

#### 2.1 Implement `OciSource`

- [ ] Create `sindri-registry/src/source/oci.rs` wrapping the existing
  `RegistryClient`/`CosignVerifier`/`RegistryCache`.
- [ ] `fetch_index` reuses cache-with-TTL semantics (DDD-02 §Cache Model).
- [ ] `supports_strict_oci()` returns `true` iff the trust config is satisfied — i.e.
  cosign signature verifies for `sindri/core` or for explicitly-trusted third parties.
- [ ] `lockfile_descriptor()` records `tag` + `manifest_digest`.
- [ ] Migrate two existing OCI integration tests from `sindri-registry/tests/` to
  exercise the trait surface; keep one direct-client test for the underlying client.

#### 2.2 Implement `LocalOciSource`

- [ ] Create `sindri-registry/src/source/local_oci.rs`. Use `oci-spec` for layout
  parsing; reuse `sigstore-rs` for embedded signature verification.
- [ ] `fetch_index` reads the layout's `index.json`, finds the registry-core artifact,
  unpacks the layers, parses `index.yaml`.
- [ ] `fetch_component_blob` reads layer blobs by digest from the layout.
- [ ] Tests: a fixture OCI layout under `v4/crates/sindri-registry/tests/fixtures/oci-layout/`
  (small but realistic — three components, signed and unsigned variants).

#### 2.3 `--strict-oci` gate

- [ ] Add `strict_oci: bool` to the resolver config; default `false`.
- [ ] After resolution, walk `Lockfile.components`; if any `source.supports_strict_oci()`
  is `false`, return `AdmissionCode::SourceNotProductionGrade`.
- [ ] Surface `--strict-oci` as a flag on `sindri lock` and `sindri resolve`; surface
  `registry.policy.strict_oci: true` in `sindri.yaml` (Q3 from ADR-028 — both).
- [ ] Loud warning at the top of every non-strict resolve report listing which sources
  produced which components.

### Acceptance criteria

- `--strict-oci` rejects a lockfile that contains a `LocalPath` source.
- `--strict-oci` accepts a lockfile that contains only verified `Oci` and `LocalOci`
  sources.
- `LocalOciSource` produces byte-for-byte the same component blob digests as the `OciSource`
  it was prefetched from (regression test).

---

## Phase 3 — `GitSource` and CLI verbs (Sprint B, week 1)

**Goal:** Git source resolves; `sindri registry serve` and `sindri registry prefetch`
exist.

### Tasks

#### 3.1 Implement `GitSource`

- [x] Decision (recorded in this plan, not in the ADR): use `git2` (libgit2 bindings)
  rather than shelling out. Rationale: deterministic across user installs, no PATH
  dependence, supports sparse checkout for `subdir`.
- [x] Cache layout: `~/.sindri/cache/git/<sha256(url)>/<commit-sha>/`.
- [x] `fetch_index`: resolve `ref` to a commit sha, sparse-checkout `subdir` if set,
  walk `component.yaml` files.
- [x] `lockfile_descriptor()` records the resolved commit sha — never the ref.
- [x] `require_signed: true` rejects unverified commits; verification shells out to
  `git verify-commit` (libgit2 lacks GPG/SSH verification primitives — see the
  module doc-comment in `git.rs` for the rationale).
- [x] Tests: fixture local git repo (using `git2::Repository::init` + `tempfile`) with three
  components; resolve, then re-resolve to assert sha is recorded and reused.

#### 3.2 `sindri registry serve`

- [x] New CLI verb in `sindri/src/commands/registry/serve.rs`. **Complete.**
- [x] Spins up an embedded OCI registry over a components directory using
  axum (read-only subset of OCI Distribution Spec v1.1; pure Rust, no `zot`
  fallback was required).
- [x] Honors `--addr` and `--root`; exits cleanly on SIGINT (axum
  `with_graceful_shutdown` + `tokio::signal::ctrl_c`).
- [x] Logs every request to stdout.
- *`--sign-with` flag deferred to Phase 5; serve is read-only and does not
  re-sign manifests. See Phase 5 §"registry serve signing".*

#### 3.3 `sindri registry prefetch`

- [x] New CLI verb in `sindri/src/commands/registry/prefetch.rs`. **Complete.**
- [x] Resolves the closure of one OCI ref into either a tarball (`--target air-gap.tar`)
  or an OCI image layout (`--layout ./vendor/registry-core`).
- [x] Reuses `OciSource` for fetch; writes the layout directly with the same
  blob-path convention `LocalOciSource` reads (`oci-spec` types not strictly
  required for the current shape).
- [x] Manifest is streamed verbatim from the upstream `OciSource` using
  `RegistryClient::fetch_registry_manifest_bytes`; scope filtering is
  intentionally consumption-side via `LocalOciSource::scope`, not a prefetch
  concern.
- [x] Q1 from ADR-028 (`--with-binaries`) is **deferred to Phase 5**; no flag
  stub was added.

#### 3.0 Prerequisites carried over from Phase 2

- [x] Implement real `OciSource::fetch_component_blob` (per-component OCI layer
  streaming). New `RegistryClient::fetch_component_layer_bytes` performs the
  manifest pull + layer pull + digest verification.
- [x] Implement real `LocalOciSource::fetch_component_blob` (read layer blobs
  from the on-disk OCI image layout by digest). Walks `index.json` for a
  per-component manifest tagged via `org.sindri.component.{backend,name}`
  annotations, reads the layer, verifies the digest.
- [x] Trust-scope audit: confirmed Phase 2's design — per-component override
  matching stays at the resolver layer (`crate::trust_scope::select_override`),
  not inside the source. Phase 3.0 did not change this.

### Acceptance criteria

- `sindri lock` resolves a component from a `GitSource` pointing at a feature branch
  and records the commit sha in the lockfile.
- `sindri registry serve --root ./components` is reachable from `oras pull`.
- `sindri registry prefetch oci://… --layout ./out` produces a directory that
  `LocalOciSource` reads identically to the original OCI source.

---

## Phase 4 — Config, docs, CI templates (Sprint B, week 2)

**Goal:** Users can express the new shape; CI templates ship; docs are honest.

### Tasks

#### 4.1 sindri.yaml schema and merge semantics

- [x] Update `v4/schemas/bom.json` to allow `registry.sources: [...]` with the four
  variants. (Done — `BomManifest.registries: Vec<RegistryConfig>` replaced by
  `BomManifest.registry: RegistrySection { sources, policy, replace_global }`. Schema
  regenerated. See PR `refactor(v4): migrate registry config to registry.sources: shape`.)
- [x] Update the `init` template to write the new shape (single `oci` source). (Done.)
- [ ] Document merge semantics: project sources prepend to global sources by default;
  `registry.replace_global: true` overrides. (Pending — `replace_global` field is wired
  and serializes correctly; runtime multi-file merge logic is a separate Phase 4 task.)

#### 4.2 `--explain` for sources

- [ ] Extend `sindri lock --explain <component>` to show every source that was
  consulted, whether it matched, why it was skipped (out of scope, not found),
  and what descriptor was recorded.
- [ ] Match output style to existing `BackendChooser --explain` output.

#### 4.3 CI template

- [ ] Add `v4/docs/ci/strict-oci.yml` — a GitHub Actions snippet enabling
  `--strict-oci` on every `sindri lock` invocation in CI.
- [ ] Reference it from the v4 user-facing docs alongside the existing CI guidance.

#### 4.4 Migration note

- [ ] Append a section to `v4/docs/MIGRATION_FROM_V3.md` (or seed the file) explaining
  how v3's "resolve from GitHub" maps to a v4 `git` source, and how v3's bundled
  registry pattern maps to `local-oci`. Cross-link to
  [`v4/docs/SOURCES.md`](../SOURCES.md) (the maintainer guide, authored in
  Phase 1) rather than duplicating its decision matrix.
- [ ] Cross-link from ADR-003 §"Air-gapped / offline" once the migration doc lands.
- [ ] Update SOURCES.md "Phase status" table as Phases 2/3 land so its source-by-source
  status stays accurate.

### Acceptance criteria

- `cargo run -p sindri -- lock --explain mise:nodejs` prints source consultation order.
- `cargo run -p sindri -- init` writes a sindri.yaml that uses the new `sources:` shape.
- A user pasting the strict-oci CI snippet into a fresh repo gets a passing run that
  fails the moment a `LocalPath` source is introduced.

#### 4.5 GitSource cache eviction

**Goal:** Bound `~/.sindri/cache/git/` so v4.0 RC doesn't ship with an unbounded
on-disk cache.

- [ ] Add `cache.git.max_size` (default `"10GB"`) and `cache.git.max_age`
      (default `"90d"`) to `~/.sindri/config.yaml`. Eviction fires when
      *either* threshold is exceeded.
- [ ] LRU by directory mtime: on each `GitSource::fetch_index`, walk the
      cache root, compute totals, evict oldest entries until under both
      thresholds.
- [ ] Info-level log per eviction (`tracing::info!`) so operators see what
      was reclaimed.
- [ ] Tests: cache populated past `max_size` triggers eviction; cache
      entry older than `max_age` evicted; concurrent eviction is safe
      (file-locking or temp-rename pattern, your call); descriptor-driven
      cache path derivation still hits after eviction with a re-clone.

##### Acceptance criteria

- A 12 GB cache shrinks to under 10 GB on the next `sindri lock` against
  any git source.
- Cache entries older than 90 days are evicted regardless of size.
- `cargo test --workspace` covers both eviction triggers.

---

## Phase 5 — Optional polish (concurrent with later sprints)

These are improvements that should not block the v4.0 RC but should be tracked.

- [ ] **`--with-binaries`** for `sindri registry prefetch` (ADR-028 Q1).
  Requires the prefetch step to know target platforms; coordinate with Sprint 9
  (Target subsystem).
- [ ] **HTTPS tarball source** (`type: http-tarball`) — natural extension of the
  trait; useful for legacy distribution channels. Defer until a real user asks.
- [ ] **`s3://` source** — for organizations that already have S3-backed mirrors but
  not OCI. Same trait, new variant.
- [ ] **`registry serve --sign-with <key>`** — re-sign manifests served by
  `sindri registry serve` so end-to-end strict-OCI verification can be
  tested against a local dev registry. The flag was stripped in Phase 3
  follow-up because shipping a no-op flag was misleading; track here for
  v4.x post-RC.

---

## Risk register

| Risk | Mitigation |
| ---- | ---------- |
| `git2` build complexity on Windows runners | Pin libgit2 system feature off; ship vendored libgit2 by default. |
| `oci-distribution-spec` server library is immature | If `axum` + `oci-distribution` is too thin, fall back to shelling out to `zot` for `registry serve`; the verb is a developer convenience, not a production path. |
| Lockfile schema churn alarms users | Backfill is implemented in Phase 1.3; add a one-line warning on first read of a legacy lockfile. |
| `--strict-oci` not noticed in CI templates | Loud warning in non-strict mode listing source mix; ship the CI template; mention in release notes. |
| Source-trait surface grows uncontrolled as more variants land | Conformance test crate that every new source must pass; review at ADR level for any new variant beyond the four in ADR-028. |

---

## Out of scope

- Discovery (`sindri ls`, `sindri search`) source attribution. The discovery domain
  (DDD-06) reads a merged `RegistryIndex` and does not need to know which source it
  came from.
- Per-source rate limiting, retry, or circuit-breaking. Each source's transport
  concerns are local; cross-source policy is a v4.1 problem.
- Dynamic source registration via plugin. The four source types are baked in; new
  types require an ADR.

---

## References

- ADR-028 — Component source modes for development and air-gap
- ADR-003, ADR-014, ADR-016 — context this plan respects
- DDD-02, DDD-03, DDD-08 — domain model
- `implementation-plan.md` Sprint 2 (registry) — extended by Phase 1–2 here
- `implementation-plan.md` Sprint 7 (CLI verbs) — extended by Phase 3 here
