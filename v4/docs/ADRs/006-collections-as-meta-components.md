# ADR-006: Collections as Meta-Components

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 has two distinct grouping abstractions:

1. **Profiles** (`profiles.yaml`) — named extension lists authored by Sindri (`minimal`,
   `fullstack`, `anthropic-dev`, `systems`, `enterprise`, `devops`, `mobile`). System-
   authored only. Users cannot create their own.
2. **Bundle extensions** (`ai-toolkit`, `cloud-tools`, `infra-tools`) — install-method
   extensions whose job is to run install scripts for multiple tools at once.

Both solve the same problem (grouping tools) with different mechanisms, creating two
objects users must understand. The v3 docs treat them differently, but the UX is the same:
"give me this set of tools."

apt-style **meta-packages** (e.g., `build-essential`) prove that this is a solved problem:
a package whose only content is a list of dependencies. One abstraction, not two.

## Decision

**Collections are components with `type: meta` and no `install` block.** Their only content
is a `dependsOn` list. The component and collection concepts unify.

```yaml
# collections/anthropic-dev/component.yaml
apiVersion: sindri.dev/v4
kind: Component
metadata:
  name: anthropic-dev
  type: meta # no install block — only dependsOn
  category: collections
  license: MIT
  versions: ["2026.04", "2026.03"]

dependsOn:
  - mise:nodejs
  - mise:python
  - npm:claude-code
  - npm:codex@openai
  - binary:fabric
  - collection:base-mcp # collections can depend on other collections
```

Collections live in `collections/{name}/component.yaml`, a directory **sibling to**
`components/`. Atomic components live in `components/{name}/component.yaml` with no
backend prefix in the directory name — the backend is declared in the `install.*` block
and derived by the local registry loader and the OCI `index.yaml`.

A user adds a collection the same way they add any component:

```yaml
components:
  collection:anthropic-dev: "2026.04"
```

The resolver expands the `dependsOn` transitive closure. Each atomic component in the
closure is resolved independently against the user's backend preference (ADR-008).

### What replaces `profiles.yaml`

`profiles.yaml` is deleted. Sindri's opinionated stacks become meta-components published
in the `registry-core` or `registry-ai` OCI artifacts. Users reference them by their
fully-qualified component name.

### User-authored collections

Because collections are just components, users can publish their own to a private registry
and reference them in `sindri.yaml`. Team-wide "opinionated stacks" are first-class:

```yaml
# In acme's private registry:
# collections/acme-platform/component.yaml
dependsOn:
  - collection:anthropic-dev
  - binary:acme-cli
  - script:setup-acme-vpn
```

### Discovery

`sindri ls --type collection` lists all collections across configured registries.
`sindri show collection:anthropic-dev` renders the full `dependsOn` tree.
`sindri graph collection:anthropic-dev` renders the DAG.

### Version conflict resolution

When `sindri.yaml` depends on `collection:anthropic-dev` (which pins `mise:nodejs: "22.11.0"`)
and also explicitly pins `mise:nodejs: "20.x"`, the explicit manifest entry wins.
`sindri resolve --strict` treats this as a hard error requiring an explicit `override:`
block. Open question Q36 resolved.

## Consequences

**Positive**

- One abstraction instead of two (profiles + bundle-extensions).
- Users can author team-level collections.
- `sindri ls --type meta` and `sindri show` work uniformly for atomic and meta components.
- Dependency resolution is unified — collections participate in the same `dependsOn`
  closure algorithm as atomic components.

**Negative / Risks**

- v3 system-authored profiles must be manually converted into meta-components before
  v4.0 ships. Seven profiles × rewriting as `component.yaml` files — scope is known.

## Alternatives rejected

- **Keep profiles.yaml** with user-authored extension to it. Leaves the two-abstraction
  problem; profiles are not target-agnostic BOM entries. Rejected.
- **Dynamic collections (Renovate-style `packageRules` + `groupName`).** Powerful but
  complex. Deferred to v4.1. Open question Q15 resolved: deferred.

## References

- Research: `02-prior-art.md` §apt, `03-proposal-primary.md` §5
- Open questions resolved: Q14 (profiles become templates/meta-components), Q15 (dynamic
  collections deferred), Q36 (conflict resolution policy)
