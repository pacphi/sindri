# DDD-02: Registry Domain

## Bounded Context

The Registry domain owns the publication, distribution, and local caching of component
catalogs. It is responsible for making the answer to "which components exist, at what
versions, from which registries?" available to the Resolver and Discovery domains.

## Core Aggregate: `Registry`

```
Registry
├── RegistryRef     (name + OCI URL + tag)
├── TrustConfig     (cosign public key or keyref)
├── CacheEntry      (local index.yaml path + ETag + fetched_at + TTL)
└── RegistryIndex   (the parsed index.yaml from the OCI artifact)
```

A `Registry` is configured globally (`~/.sindri/config.yaml`) or per-project
(`sindri.yaml` `registries:` list). The two lists are merged at runtime.

> **Update (ADR-028, DDD-08).** The `Registry` aggregate is being generalized to compose
> an ordered list of `RegistrySource` instances (local-path, git, oci, local-oci) rather
> than a single OCI ref. The OCI fields documented here become the contents of an
> `OciSource` variant. See [DDD-08](08-registry-source-domain.md) for the source-domain
> model and `Source` trait. This DDD continues to own catalog identity, signing policy,
> cache eviction, and publication semantics.

## RegistryIndex (the catalog)

```
RegistryIndex {
    apiVersion: String,   // "sindri.dev/v4"
    kind:        String,  // "RegistryIndex"
    name:        String,  // "sindri/core"
    updated:     DateTime,
    components:  HashMap<String, ComponentEntry>,
}

ComponentEntry {
    kind:            ComponentKind,    // component | collection
    backend:         Option<Backend>,  // None for collections
    category:        String,
    description:     String,
    backends:        Vec<Backend>,     // can install via any of these
    platforms:       Vec<Platform>,
    license:         String,           // SPDX
    tags:            Vec<String>,
    versions:        HashMap<String, OciDigest>,  // version → component.yaml blob digest
    latest:          String,
    depends_on_preview: Option<Vec<ComponentId>>,  // flattened for search; not authoritative
}
```

`RegistryIndex` is a **denormalized read model** — it is regenerated at registry-publish
time from `components/*/component.yaml` files. Never directly edited.

## OCI Artifact Structure

```
oci://ghcr.io/{owner}/{registry}:{tag}
├── manifest.json               (OCI artifact manifest)
├── signatures/                 (cosign detached signatures)
└── layers/ (tarball):
    ├── index.yaml              (RegistryIndex — the catalog)
    ├── components/
    │   ├── {name}/component.yaml  (ComponentManifest per component)
    │   └── {name}/install.sh / install.ps1 (optional, for script:)
    └── checksums/
        └── sha256sums
```

Each `component.yaml` blob is content-addressed by its `sha256:…` — this is the OCI
digest recorded in `index.yaml` and `sindri.lock`.

## Cache Model

```
~/.sindri/cache/registries/{registry-name}/
├── index.yaml          (cached RegistryIndex)
├── manifest.digest     (last-seen OCI manifest digest for TTL comparison)
└── components/         (lazily fetched component.yaml blobs, keyed by sha256)
    └── sha256:{hash}   (content-addressable; shared across registries)
```

TTL: 24h default (overridable per-registry). Cache invalidation is digest-based: compare
the OCI manifest digest cheaply before pulling the full index. `--refresh` bypasses TTL.

## Domain Services

### `RegistryFetcher`

```
RegistryFetcher::fetch(ref: OciRef) -> Result<RegistryIndex>
  1. Resolve OCI manifest for the tag.
  2. Check cached digest — if matches, return cached index.
  3. Pull index.yaml blob.
  4. Verify cosign signature if registry is trusted.
  5. Write to cache; update digest.
```

### `ComponentBlobFetcher`

```
ComponentBlobFetcher::fetch(digest: OciDigest) -> Result<ComponentManifest>
  1. Check content-addressed cache at sha256:{hash}.
  2. If miss, pull blob from OCI registry by digest.
  3. Verify sha256 matches.
  4. Deserialize and return.
```

### `RegistryPublisher` (maintainer-side)

```
RegistryPublisher::publish(registry_dir: &Path, tag: &str) -> Result<()>
  1. Lint all components (schema, platforms, checksums, license).
  2. Regenerate index.yaml from components/*/component.yaml.
  3. Compute sha256sums.
  4. oras push to OCI registry.
  5. cosign sign the pushed manifest digest.
  6. Emit SLSA provenance attestation.
```

## Domain Events

| Event                 | Trigger                              | Consumer                                          |
| --------------------- | ------------------------------------ | ------------------------------------------------- |
| `RegistryRefreshed`   | Index fetched and written to cache   | Discovery (cache invalidation)                    |
| `RegistryStale`       | TTL expired or `--refresh` requested | Discovery (triggers fetch)                        |
| `ComponentBlobCached` | component.yaml blob written to cache | Resolver                                          |
| `RegistryPublished`   | Publish workflow completes           | Consumer notification (docs site, GitHub release) |

## Invariants

1. Registry tag immutability: republishing the same `YYYY.MM` tag is a CI failure.
2. Every version in `index.yaml` must have a `sha256:…` digest pointing to the blob.
3. `index.yaml` is always a derivative — it is regenerated at publish time, never manually edited.
4. Signed registries: the OCI manifest must have a valid cosign signature before the
   index is written to the user's cache.
5. Component-blob digests in `index.yaml` must match the actual blob sha256 — verified
   by `RegistryPublisher` and again by `ComponentBlobFetcher`.

## Crate location

`sindri-registry/src/` (new crate)  
Submodules: `fetcher.rs`, `publisher.rs`, `cache.rs`, `index.rs`, `signing.rs`
