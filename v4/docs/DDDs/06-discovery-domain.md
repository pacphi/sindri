# DDD-06: Discovery Domain

## Bounded Context

The Discovery domain answers questions about what components exist and how they relate.
It is read-only from the user's perspective. It reads cached registry indices (populated
by the Registry domain) and presents them via CLI commands (`ls`, `search`, `show`,
`graph`, `explain`).

## Core Aggregate: `RegistryCache`

```
RegistryCache
├── Entries: HashMap<RegistryName, CachedIndex>
└── methods:
    ├── search(query: &str, filters: SearchFilters) -> Vec<SearchResult>
    ├── show(id: ComponentId) -> Option<ComponentDetail>
    ├── list(filters: ListFilters) -> Vec<ListEntry>
    └── graph(id: ComponentId, direction: Direction) -> DependencyGraph
```

`RegistryCache` is built at CLI startup from `~/.sindri/cache/registries/*/index.yaml`.
Refresh happens when the TTL expires or `--refresh` is passed.

## Value Objects

### SearchResult

```
SearchResult {
    registry:    String,
    component:   ComponentId,
    kind:        ComponentKind,   // component | collection
    backend:     Option<Backend>,
    latest:      Version,
    description: String,
    score:       f32,             // relevance score for ranking
}
```

Search scoring priority: exact name > alias > tag > description substring > fuzzy name.

### ListEntry

```
ListEntry {
    registry:  String,
    component: ComponentId,
    backend:   Option<Backend>,
    latest:    Version,
    kind:      ComponentKind,
    installed: Option<InstalledVersion>,  // from sindri.lock if --installed
    outdated:  bool,                       // latest > installed
}
```

### ComponentDetail

```
ComponentDetail {
    registry:    String,
    manifest:    ComponentManifest,       // the full component.yaml
    versions:    Vec<Version>,
    used_by:     Vec<ComponentId>,        // reverse deps (from registry index)
    installed:   Option<InstalledEntry>,  // from sindri.lock
}
```

`used_by` is populated from the `depends_on_preview` field in the registry index — a
denormalized "what depends on me?" lookup.

### DependencyGraph

```
DependencyGraph {
    root:  ComponentId,
    nodes: HashMap<ComponentId, GraphNode>,
    edges: Vec<(ComponentId, ComponentId)>,  // (parent, child) = dependsOn
}

impl DependencyGraph {
    pub fn to_tree_string(&self) -> String   // text tree output
    pub fn to_mermaid(&self)   -> String     // Mermaid diagram
    pub fn invert(&self)       -> Self       // --reverse
}
```

## CLI Surface → Domain Service Mapping

| CLI command                     | Domain service call                                       |
| ------------------------------- | --------------------------------------------------------- |
| `sindri ls`                     | `RegistryCache::list(filters)`                            |
| `sindri ls --installed`         | `RegistryCache::list` + join with `Lockfile`              |
| `sindri ls --outdated`          | `RegistryCache::list` + compare installed vs latest       |
| `sindri search <q>`             | `RegistryCache::search(q, filters)`                       |
| `sindri show <name>`            | `RegistryCache::show(id)` + `ComponentBlobFetcher::fetch` |
| `sindri graph <name>`           | `RegistryCache::graph(id, Forward)`                       |
| `sindri graph <name> --reverse` | `RegistryCache::graph(id, Reverse)`                       |
| `sindri explain <a> --in <b>`   | `RegistryCache::graph(b, Forward)` → path to `a`          |

## Cache Freshness

`sindri ls` (and all discovery commands) reads the cache by default. If the cache
for a configured registry is stale (> TTL) or absent, the CLI prints a banner:

```
⚠ Registry sindri/core last refreshed 26h ago. Run `sindri registry refresh` to update.
```

`--refresh` forces a network fetch before displaying results.

The default TTL is 24h (ADR per open question Q25). Per-registry TTL overrides are
supported in `~/.sindri/config.yaml`.

## Disambiguation for short names

If a user runs `sindri show aws-cli` and both `sindri/core` and `acme/internal` publish
`aws-cli`, the response is a disambiguation list:

```
Multiple matches for "aws-cli":
  1. sindri/core/aws-cli     (binary)   v2.17.21
  2. acme/internal/aws-cli   (binary)   v2.17.21-acme3
Specify the fully-qualified name, e.g.:  sindri show sindri/core/aws-cli
```

A configurable "primary registry" may be referenced unqualified. Open question Q35 resolved.

## Shell Completion Integration

`sindri add <TAB>` reads the cached registry index and provides component-name
completions. Same for `sindri show`, `sindri graph`, `sindri explain`. Completion is
fast (in-memory search over cached index).

## Invariants

1. Discovery commands are ALWAYS read-only — they never write `sindri.yaml`, config,
   or the cache.
2. If the cache is empty (first run, no network), only `sindri/core` with its offline
   bundle is available. Users see a clear message.
3. `sindri explain` always traces through the actual `dependsOn` closure from the
   fetched component manifests — not from the denormalized `depends_on_preview` index
   field, which is only a hint for search.

## Crate location

`sindri-discovery/src/` (new crate)  
Submodules: `cache.rs`, `search.rs`, `show.rs`, `graph.rs`, `explain.rs`, `completion.rs`
