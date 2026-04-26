# Discoverability in v4

**The question.** In v3 users have two entry points:

- `sindri extension list` — catalog from `registry.yaml` (~60 extensions)
- `sindri profile list` — catalog from `profiles.yaml` (7 bundles)

Both read local files bundled with the CLI. Simple, offline, one authority.

v4 fragments the catalog across N registries (Sindri-core + community + private) and
unifies extensions and profiles into one thing (components, some of which are meta).
Discovery gets harder on three axes: **where** to look, **what shape** is the result,
**how** to filter it down. This doc proposes the v4 discovery surface.

## 1. Guiding principles

1. **One verb, unified catalog.** No more separate `extension` vs `profile` surfaces.
   Everything is a component; filters express intent (`--type meta`, `--backend mise`).
2. **Offline-first.** `sindri resolve` already caches registry index files locally.
   Discovery commands read the cache by default; `--refresh` hits the network.
3. **Progressive disclosure.** `ls` shows a table, `show` shows one component in detail,
   `search` is fuzzy across names/descriptions/tags, `graph` renders dependency DAGs.
4. **Registry-aware.** Every result carries its source registry (fully qualified
   `registry/component`), so users can see where a component came from and trust it.

## 2. Proposed CLI surface

### 2.1 `sindri ls` — the unified list

Replaces `extension list` and `profile list`.

```
$ sindri ls
REGISTRY          COMPONENT                       BACKEND    LATEST      KIND
sindri/core       nodejs                          mise       22.11.0     component
sindri/core       python                          mise       3.14.0      component
sindri/core       aws-cli                         binary     2.17.21     component
sindri/core       anthropic-dev                   -          2026.04     collection
sindri/ai         codex                           npm        2.3.1       component
sindri/ai         claude-code                     npm        2.1.4       component
sindri/ai         ai-essentials                   -          2026.04     collection
acme/internal     acme-platform                   -          v7.3        collection
...
```

Flags:

| Flag                           | Purpose                                                        |
| ------------------------------ | -------------------------------------------------------------- |
| `--registry <name>`            | Scope to one registry: `sindri ls --registry sindri/ai`        |
| `--backend <name>`             | `--backend mise`, `--backend apt`, etc.                        |
| `--type component\|collection` | Components only, or collections only (replaces `profile list`) |
| `--category <cat>`             | `languages`, `ai-dev`, `cloud`, etc.                           |
| `--installed`                  | Only what's in the current project's `sindri.lock`             |
| `--outdated`                   | Installed components with a newer version available            |
| `--json`                       | Machine-readable output                                        |
| `--refresh`                    | Force re-fetch of registry indices before listing              |

### 2.2 `sindri search <query>` — fuzzy search

```
$ sindri search kubectl
sindri/core/kubectl              mise      Kubernetes CLI           v1.31.3
sindri/core/kubectx              binary    kubectl context switcher v0.9.5
sindri/core/k9s                  mise      Kubernetes TUI           v0.32.7
sindri/cloud/eks-kubectl         script    EKS-auth-aware kubectl   v2.4.0
```

Searches across component name, description, tags, and `metadata.aliases`. Respects
`--registry`/`--backend`/`--category` filters.

### 2.3 `sindri show <component>` — detail view

Merges `extension info` and `profile info`.

```
$ sindri show sindri/core/anthropic-dev
COLLECTION sindri/core/anthropic-dev@2026.04
  digest:      sha256:8f3e2a1b…
  description: Opinionated stack for Anthropic-assisted development
  maintainer:  sindri-dev
  license:     MIT

DEPENDS ON (12)
  mise:nodejs            22.x
  mise:python            3.14.x
  npm:claude-code        2.x
  npm:codex              2.x
  binary:fabric          1.x
  collection:base-mcp    2026.04
  ...

USED BY
  sindri/anthropic/anthropic-ml-dev  (adds cuda, torch)
  acme/internal/acme-platform        (includes via base-languages)
```

For non-meta components:

```
$ sindri show sindri/core/nodejs
COMPONENT sindri/core/nodejs@22.11.0
  backend:     mise
  category:    languages
  digest:      sha256:1a2b3c…

VERSIONS AVAILABLE
  22.11.0  22.10.0  22.9.0  20.18.1  20.18.0  ...

OPTIONS
  corepack (bool, default true)

DEPENDS ON
  mise-config

CAPABILITIES
  hooks, project-init

INSTALLED
  22.11.0 (from sindri.lock)
```

### 2.4 `sindri graph <component>` — dependency DAG

Text or Mermaid output. Critical for understanding collection hierarchies:

```
$ sindri graph collection:anthropic-dev
collection:anthropic-dev
├── mise:nodejs 22.x
├── mise:python 3.14.x
├── collection:base-mcp
│   ├── npm:mcp-server-filesystem
│   └── npm:mcp-server-github
└── npm:claude-code 2.x
```

`--format mermaid` for docs, `--reverse` for "what depends on this?"

### 2.5 `sindri registry` — registry management

```
sindri registry add <name> <oci-url>   # add a registry to ~/.sindri/config.yaml
sindri registry ls                     # list registries with cache freshness
sindri registry refresh [<name>]       # re-pull index for one or all
sindri registry trust <name>           # pin cosign signer for this registry
sindri registry remove <name>
```

Registries are opt-in. Sindri ships with `sindri/core` enabled by default; everything
else the user adds explicitly. `sindri ls --all-registries` scans configured registries;
the default scope can be per-registry or per-project.

### 2.6 `sindri explain <component> [--in <other-component>]`

Rare but powerful: "why is this in my install?"

```
$ sindri explain mise:python --in collection:anthropic-dev
collection:anthropic-dev
  └── collection:base-languages
        └── mise:python  (pinned 3.14.x)
```

Same mental model as `npm why` or `cargo tree --invert`.

## 3. Where the data comes from

Each registry's OCI artifact contains an `index.yaml` at the root:

```yaml
apiVersion: sindri.dev/v4
kind: RegistryIndex
name: sindri/core
updated: 2026-04-20T12:00:00Z
components:
  nodejs:
    kind: component
    backend: mise
    category: languages
    description: "Node.js JavaScript runtime"
    versions: ["22.11.0", "22.10.0", "20.18.1", ...]
    latest: "22.11.0"
    digest: sha256:...
  anthropic-dev:
    kind: collection
    description: "Opinionated Anthropic dev stack"
    versions: ["2026.04", "2026.03"]
    latest: "2026.04"
    digest: sha256:...
    depends_on: [mise:nodejs, mise:python, npm:claude-code, ...]
```

The index is the authority for `ls`/`search`/`show`. Full component manifests are
fetched lazily (for `graph`, `explain`, and install).

Cache lives at `~/.sindri/cache/registries/<registry>/index.yaml` with an ETag and a
TTL (suggest: 24h default, overridable per-registry). `--refresh` bypasses TTL.

## 4. UX surface beyond the CLI

1. **Web catalog.** Static site generated from the core registry index at
   `sindri.dev/catalog`, searchable, indexable by Google. Low-lift: it's just a Hugo/
   Astro build from `index.yaml`. Private registries ship with a `sindri catalog serve`
   that renders the same UI from a local registry clone — good for internal orgs.
2. **Console integration.** The existing Sindri console (`apps/api` + frontend) already
   calls `sindri registry extensions` via the CLI shell-out layer. v4 updates those
   routes to hit the unified `sindri ls --json` and a new `GET /api/v1/components`,
   `GET /api/v1/components/:registry/:name`. Same Step1Configuration screen, richer
   filtering (backend, collection membership).
3. **Shell completion.** Dynamic completion reads the cached registry index — tab-
   completing `sindri install sindri/core/<TAB>` lists component names. Small but
   material for daily ergonomics.
4. **`sindri init` template-driven scaffolding.** `sindri init --template
anthropic-dev` writes a seeded `sindri.yaml` depending on that collection. Templates
   live in their registries; discoverable via `sindri ls --type collection
--tag template`.

## 5. What gets retired

| v3                                       | v4 replacement                                        |
| ---------------------------------------- | ----------------------------------------------------- |
| `sindri extension list`                  | `sindri ls --type component` (or just `sindri ls`)    |
| `sindri extension info <name>`           | `sindri show <registry>/<name>`                       |
| `sindri profile list`                    | `sindri ls --type collection`                         |
| `sindri profile info <name>`             | `sindri show <registry>/<collection>`                 |
| `sindri profile status <name>`           | `sindri ls --installed` (collection membership shown) |
| Static `registry.yaml` + `profiles.yaml` | Cached registry `index.yaml` files per registry       |

## 6. Risks and open points

1. **Stale caches give stale answers.** Default TTL too long → users miss updates;
   too short → constant network chatter. Suggest 24h with a prominent "last refreshed
   Xh ago" footer on `ls` output.
2. **Ambiguous short names.** If two registries ship `aws-cli`, `sindri show aws-cli`
   needs a rule. Suggest: ambiguous short names fail with a disambiguation list; users
   type the fully-qualified name; only the user's primary registry can be aliased for
   unqualified use.
3. **Search relevance.** Fuzzy search over 60 components is trivial; over 600 across
   10 registries needs scoring (name > alias > description > tags). Punt to a simple
   bleve/tantivy index built at cache-refresh time. Not blocking v4.0.
4. **Private-registry auth.** `sindri registry add` must accept OCI auth (docker
   config.json reuse is the clean path). Needs a concrete spec before v4.0; orgs
   won't adopt without it.
5. **Offline discoverability after a fresh install.** First run with no network → only
   the embedded core registry is visible. Acceptable if `sindri/core` ships meaningful
   breadth out of the box; needs a decision on what's "core" (open question §2 in
   `05-open-questions.md`).

Added as open questions §18 and §19 in `05-open-questions.md`.
