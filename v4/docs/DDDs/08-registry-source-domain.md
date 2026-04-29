## DDD-08: Registry Source Domain

**Status:** Proposed
**Date:** 2026-04-28
**Related ADRs:** ADR-003, ADR-014, ADR-028
**Supersedes:** Nothing. Extends DDD-02 §"Cache Model" and DDD-03 §"Resolution Algorithm".

## Bounded Context

The Registry Source domain owns the abstraction by which bytes for a `RegistryIndex` and
its `component.yaml` blobs are obtained, regardless of physical transport. It sits one
layer below the Registry domain (DDD-02): a `Registry` aggregate is now defined as an
ordered sequence of `RegistrySource` instances plus shared cache and trust state.

The Registry domain continues to own catalog identity, signing policy, cache eviction,
and publication. The Source domain owns: "given an OCI ref / git ref / filesystem path,
produce a `RegistryIndex` and the `ComponentManifest` blobs it references."

## Why this is a separate context

Before ADR-028, the Registry domain conflated *what* a registry is (a named, signed
catalog) with *how* its bytes are fetched (always OCI). That conflation made it
impossible to model:

- Per-component overrides (one source for one component, a different source for the rest).
- Reproducibility metadata in the lockfile (which source produced this resolved entry).
- Per-source trust policy without overloading the global signing rule.

Splitting Source out as its own bounded context is the smallest change that lets the
Registry aggregate compose multiple transport strategies under a single name and trust
policy.

## Core Aggregate: `RegistrySource`

```rust
pub enum RegistrySource {
    LocalPath(LocalPathSource),
    Git(GitSource),
    Oci(OciSource),
    LocalOci(LocalOciSource),
}

pub struct LocalPathSource {
    pub path: PathBuf,
    pub scope: Option<Vec<ComponentName>>,
}

pub struct GitSource {
    pub url: Url,
    pub r#ref: GitRef,            // branch | tag | sha
    pub subdir: Option<PathBuf>,
    pub scope: Option<Vec<ComponentName>>,
    pub require_signed: bool,     // off by default; verifies signed commits if on
}

pub struct OciSource {
    pub url: OciRef,              // oci://host/path
    pub tag: String,
    pub scope: Option<Vec<ComponentName>>,
    pub trust: TrustConfig,       // cosign key/keyref; required for sindri/core
}

pub struct LocalOciSource {
    pub layout_path: PathBuf,     // OCI image layout v1.1
    pub scope: Option<Vec<ComponentName>>,
    pub trust: TrustConfig,       // inherits original artifact's signing if present
}
```

A `RegistrySource` is a value object on its own — equality is structural. The aggregate
boundary is drawn at `Registry` (DDD-02): a `Registry` *has many* sources in declared
order; sources are not addressable independently of the registry that owns them.

## Source trait (the domain service contract)

```rust
pub trait Source {
    /// Produce the catalog this source contributes. May be a partial catalog if `scope`
    /// is set; the Resolver merges partial catalogs in source order.
    fn fetch_index(&self, ctx: &SourceContext) -> Result<RegistryIndex, SourceError>;

    /// Produce a single `component.yaml` blob by id and version.
    fn fetch_component_blob(
        &self,
        id: &ComponentId,
        version: &Version,
        ctx: &SourceContext,
    ) -> Result<ComponentBlob, SourceError>;

    /// Identity recorded in the lockfile so apply-time fetch is reproducible.
    fn lockfile_descriptor(&self) -> SourceDescriptor;

    /// Whether this source's bytes have a verified signature chain that satisfies
    /// `--strict-oci`. Only `OciSource` and (transitively) `LocalOciSource` can return true.
    fn supports_strict_oci(&self) -> bool;
}
```

Every source implementation lives in `sindri-registry/src/source/` and is registered in
the `RegistrySource::dispatch` match. New source types (e.g., `s3`, `http-tarball`) are
purely additive — they implement `Source` and gain a new enum variant.

## Resolution Algorithm (extends DDD-03 §"Resolution Algorithm")

```
resolve_with_sources(manifest, target, policy, sources: &[RegistrySource])
  → Result<Lockfile>

For each component id in the BFS over dependsOn:
  for source in sources (in declared order):
      if source.scope is Some and id.name not in scope: continue
      match source.fetch_component_blob(id, version):
          Ok(blob)  -> use this blob; record source.lockfile_descriptor() in lockfile entry; break
          Err(NotFound) -> continue
          Err(other) -> propagate
  if no source matched: error ComponentNotFound
```

The first-match-wins semantics is intentional and visible in `sindri lock --explain`,
which prints which source satisfied each component.

## Lockfile descriptor

```rust
pub enum SourceDescriptor {
    LocalPath { path: PathBuf },
    Git { url: Url, commit_sha: String, subdir: Option<PathBuf> },
    Oci { url: OciRef, tag: String, manifest_digest: OciDigest },
    LocalOci { layout_path: PathBuf, manifest_digest: OciDigest },
}
```

`SourceDescriptor` is the **lockfile-stable** projection of a `RegistrySource`. It
captures only what is needed for `sindri apply` to refetch the same bytes. Notably:

- `Git.commit_sha` is the resolved sha at lock time, not the user-supplied ref.
- `Oci.manifest_digest` is recorded alongside the tag so a republished tag is detected
  as drift at apply time.
- `LocalPath` carries no integrity field — it is non-reproducible across machines by
  definition. `--strict-oci` rejects any lockfile that contains a `LocalPath` descriptor.

`Lockfile.ResolvedComponent` (DDD-03) gains a `source: SourceDescriptor` field. The
existing `registry: String` field is deprecated in favor of `source.descriptor_kind()`
for human display; existing lockfiles read with `registry: <name>` are upgraded in
memory to `Oci { ... }` using the registry's configured tag.

## Cache Model (extends DDD-02 §"Cache Model")

```
~/.sindri/cache/
├── registries/
│   └── {registry-name}/
│       ├── manifest.digest         (last-seen OCI manifest digest; oci sources only)
│       └── index.yaml              (merged across non-oci sources at resolve time)
├── git/
│   └── {sha256(url)}/
│       └── {commit-sha}/           (sparse checkout; reused across resolves)
├── oci-layouts/                    (consumed by local-oci sources; not written here)
└── components/
    └── sha256:{hash}               (content-addressable; shared across all sources)
```

Component blobs continue to be content-addressable in `~/.sindri/cache/components/`.
A blob fetched from a `git` source and the same component published later via OCI share
the same `sha256:` path — identical bytes, one cache entry.

## Source-level trust policy

Trust is no longer global. The matrix is:

| Source        | `supports_strict_oci()` | Default verification           | Failure mode                  |
| ------------- | ----------------------- | ------------------------------ | ----------------------------- |
| `LocalPath`   | false                   | None                           | n/a                           |
| `Git`         | false (true if signed-commits + key) | Commit sha recorded; ref stability not enforced | `--require-signed` rejects unsigned commits |
| `Oci`         | true                    | cosign required for sindri/core; configurable for others | Fail-closed; stale cache retained |
| `LocalOci`    | true (iff inner artifact was signed) | Verifies embedded cosign signature if present | Fail-closed if `--require-signed` |

`sindri lock --strict-oci` succeeds iff every `ResolvedComponent.source.supports_strict_oci()`
is true. CI templates flip this on; inner-loop development leaves it off.

## Domain Events

| Event                | Trigger                                                | Consumer                                  |
| -------------------- | ------------------------------------------------------ | ----------------------------------------- |
| `SourceMatched`      | A source produced the blob for a component             | Lockfile writer; `--explain` reporter     |
| `SourceFallthrough`  | A scoped source declined a component (out of scope)    | `--explain` reporter                      |
| `SourceVerificationFailed` | cosign verification failed on an OCI/local-oci source | Resolver (fails resolve), Status Ledger |
| `LocalPathBypass`    | A `LocalPath` source satisfied a component             | Status Ledger; warns in non-strict mode   |

## Invariants

1. Source order in `sindri.yaml` is preserved end-to-end: registry config → resolve →
   `--explain` output.
2. A component blob digest recorded in the lockfile MUST match the bytes produced by
   any source that re-resolves it. This is what makes per-component overrides safe:
   substituting one source for another either yields the same bytes (cache hit) or is
   detectable at apply time.
3. `Git.commit_sha` in the lockfile is the resolved sha, never a branch or tag name.
4. A `LocalPath` source MUST NOT be admitted under `--strict-oci`. The resolver fails
   loudly rather than silently downgrading.
5. A `LocalOci` source's `manifest_digest` MUST match the digest computed by reading
   the layout at apply time; a corrupted layout fails fast.
6. The order in which sources are consulted is independent of cache state — the cache
   is a shared accelerator, never a source of truth.

## Crate location

`sindri-registry/src/source/` (new module within existing `sindri-registry` crate)
- `mod.rs`           — `Source` trait, `RegistrySource` enum, dispatch
- `local_path.rs`    — `LocalPathSource`
- `git.rs`           — `GitSource`; uses `git2` or shells `git` (decision in plan §3)
- `oci.rs`           — `OciSource`; wraps existing `RegistryClient`
- `local_oci.rs`     — `LocalOciSource`; reads OCI image layout via `oci-spec`/`sigstore-rs`

`sindri-registry/src/local.rs` (the existing `LocalRegistry`) is refactored into
`source/local_path.rs` with no behavior change; the public API gains the new
`Source` trait but keeps the old `LocalRegistry` type alias for one release.

## Ubiquitous Language additions

| Term                  | Definition                                                                                        |
| --------------------- | ------------------------------------------------------------------------------------------------- |
| **Registry source**   | One typed origin of registry bytes. A registry is composed of one or more sources in priority order. |
| **Source descriptor** | The lockfile-stable projection of a source: what `sindri apply` re-reads to refetch a component.   |
| **Strict OCI mode**   | The CI/production gate (`--strict-oci`) that fails resolve unless every source is OCI-with-signature. |
| **Bundle / local-oci**| An on-disk OCI image layout (v1.1) consumed without network access.                                 |
| **Source scope**      | Optional component-name allowlist on a source; non-matching components fall through to the next source. |
