# Registry Lifecycle — Consumer & Maintainer Walkthroughs

End-to-end: what a registry _is_, how `sindri search kubectl` resolves through it,
and how a maintainer adds a new component or a new version. The goal is a shared
picture so UX, storage, and CI discussions stay aligned.

## 1. Anatomy of a registry

A registry is an **OCI artifact** — the same addressable, signable, cacheable object
format Docker images use. Sindri doesn't reinvent distribution; it piggybacks on
battle-tested infrastructure (GHCR, ECR, Docker Hub, Harbor, Artifactory).

A single registry artifact looks like:

```
oci://ghcr.io/sindri-dev/registry-core:2026.04
│
├── manifest.json               (OCI artifact manifest)
├── signatures/                 (cosign signatures, if signed)
│
└── layers/ (tarball contents):
    ├── index.yaml              # one line per component, lightweight catalog
    ├── components/
    │   ├── kubectl/
    │   │   ├── component.yaml          # manifest for component `kubectl`
    │   │   ├── install.sh              # optional — when script backend needed
    │   │   ├── install.ps1
    │   │   └── LICENSE
    │   ├── python/
    │   │   └── component.yaml
    │   ├── anthropic-dev/              # a collection (meta-component)
    │   │   └── component.yaml
    │   └── ...
    └── checksums/
        └── sha256sums                  # one line per file in components/
```

The registry is versioned: `:2026.04`, `:2026.05`, etc. Each tag is **immutable** —
republishing the same tag is forbidden by registry CI. Users pin to a tag in their
`sindri.yaml` `registries:` list; the digest beneath is captured in `sindri.lock` for
reproducibility.

## 2. `index.yaml` — the lightweight catalog

The index is what `sindri ls` / `search` / `show` read. Small, cacheable, denormalized:

```yaml
apiVersion: sindri.dev/v4
kind: RegistryIndex
name: sindri/core
updated: 2026-04-20T12:00:00Z

components:
  kubectl:
    kind: component
    category: devops
    description: "Kubernetes command-line tool"
    backends: [mise, binary, brew, apt, winget]   # which backends it can install via
    platforms: [linux-x86_64, linux-aarch64, macos-aarch64, windows-x86_64, windows-aarch64]
    license: Apache-2.0
    versions:
      "1.31.3":  sha256:a1b2c3...        # digest of components/kubectl/component.yaml@1.31.3
      "1.31.2":  sha256:d4e5f6...
      "1.30.7":  sha256:789abc...
      # ... trimmed in practice to last N minor versions per policy
    latest: "1.31.3"
    tags: [k8s, kubernetes, cli]

  python:
    kind: component
    ...

  anthropic-dev:
    kind: collection            # meta-component — only depends_on, no install
    versions:
      "2026.04": sha256:fff000...
      "2026.03": sha256:eee111...
    latest: "2026.04"
    depends_on_preview:         # flattened for index search; authoritative closure lives in component.yaml
      - mise:nodejs
      - mise:python
      - npm:claude-code
      - ...
```

Note: **the index is a derivative**, regenerated from `components/*/component.yaml`
at publish time. The source of truth for any one component is its `component.yaml`
blob — the index exists only to make listing fast.

## 3. `component.yaml` — the component manifest

One file per (component, version). Structurally identical to what was described in
`03-proposal-primary.md` §3, with the lifecycle detail that matters here:

```yaml
apiVersion: sindri.dev/v4
kind: Component
metadata:
  name: kubectl
  version: "1.31.3"
  category: devops
  license: Apache-2.0
  homepage: https://kubernetes.io/docs/reference/kubectl/
  upstream: https://github.com/kubernetes/kubernetes

platforms:
  - linux-x86_64
  - linux-aarch64
  - macos-aarch64
  - windows-x86_64
  - windows-aarch64

install:
  # Author's preference order per platform.
  preferences:
    macos: [brew, binary]
    linux: [mise, binary, apt]
    windows: [winget, scoop, binary]

  default:
    binary:
      source: github-release
      repo: kubernetes/kubernetes
      assets:
        linux-x86_64: "kubernetes-client-linux-amd64.tar.gz"
        linux-aarch64: "kubernetes-client-linux-arm64.tar.gz"
        macos-aarch64: "kubernetes-client-darwin-arm64.tar.gz"
        windows-x86_64: "kubernetes-client-windows-amd64.tar.gz"
        windows-aarch64: "kubernetes-client-windows-arm64.tar.gz"
      checksums:
        linux-x86_64: sha256:aaaa...
        linux-aarch64: sha256:bbbb...
        # …mandatory for every listed platform

  overrides:
    macos: { brew: { package: kubectl } }
    linux: { mise: { tools: ["kubectl@{{ version }}"] } }
    windows: { winget: { package: Kubernetes.kubectl } }

validate:
  commands:
    - name: kubectl
      versionFlag: "version --client --output=json"
      expectedPattern: '"gitVersion":"v1\.31\.3"'

# …plus optional configure/remove/capabilities blocks
```

The entire `component.yaml` for one version is content-addressed — the digest in the
index is the hash of this file.

## 4. Consumer flow: `sindri search kubectl`

Step by step, given a user with `sindri/core:2026.04` and `acme/internal:v7`
configured:

### Step 1 — load configured registries

`~/.sindri/config.yaml` and project-level `sindri.yaml` both list registries. CLI
merges them:

```
registries:
  - name: sindri/core     oci://ghcr.io/sindri-dev/registry-core:2026.04
  - name: acme/internal   oci://ghcr.io/acme/registry-internal:v7
```

### Step 2 — ensure indices are cached

For each registry, Sindri looks in `~/.sindri/cache/registries/<name>/index.yaml`:

```
~/.sindri/cache/registries/
├── sindri-core/
│   ├── index.yaml              (last fetched 2h ago, TTL 24h → fresh)
│   └── manifest.digest         (sha256:...)
└── acme-internal/
    ├── index.yaml              (last fetched 31h ago, TTL 24h → STALE)
    └── manifest.digest
```

Stale indices are refreshed in the background:

1. Resolve the OCI manifest for the tag (`:v7`) — records the digest.
2. If digest == cached digest, mark fresh, done.
3. Else pull the `index.yaml` blob, replace cache, update digest.

With `--offline` or a network error on a cached registry, use the stale copy and
print a warning.

### Step 3 — search each index

In-memory fuzzy search across each registry's `components:` map. Match fields in
priority order:

1. Exact name match
2. Alias match
3. Tag match (`k8s`, `kubernetes`)
4. Description substring
5. Fuzzy on name (Levenshtein / subsequence)

Score each hit; annotate with source registry.

### Step 4 — render results

```
$ sindri search kubectl
REGISTRY         COMPONENT        BACKENDS          LATEST      DESCRIPTION
sindri/core      kubectl          mise,binary,brew,apt,winget  1.31.3  Kubernetes command-line tool
sindri/core      kubectx          binary,brew       0.9.5       kubectl context switcher
sindri/core      k9s              mise,binary,brew  0.32.7      Kubernetes TUI
acme/internal    eks-kubectl      script,binary     2.4.0       EKS-auth-aware kubectl (acme fork)
```

No network touched beyond the freshness check in step 2. Entirely local
search over cached indices.

### Step 5 — drill deeper (optional)

```bash
sindri show sindri/core/kubectl
```

Now Sindri needs the `component.yaml` for a specific version (the latest by default).
Pulls only that blob from the OCI artifact — a few KB — using its digest from the
index, and verifies the digest matches before displaying.

## 5. Consumer flow end-to-end — search → install

```
sindri search kubectl          # which one to add?
sindri show sindri/core/kubectl
sindri add mise:kubectl        # writes sindri.yaml; validates admissibility
sindri resolve                 # writes sindri.lock with the exact digest
sindri apply                   # installs; emits SBOM
```

What each step touches:

| Step      | Registry operation                                                                                          | Local writes                          |
| --------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| `search`  | Read cached indices                                                                                         | —                                     |
| `show`    | Pull one `component.yaml` blob if not cached                                                                | `~/.sindri/cache/components/<digest>` |
| `add`     | Validate name against cached indices                                                                        | `sindri.yaml`                         |
| `resolve` | Pull `component.yaml` for every referenced component + transitive deps; verify digests; run admission gates | `sindri.lock`                         |
| `apply`   | Pull actual install artifacts (binaries, etc.) via backends; verify checksums from component.yaml           | Installed state + StatusLedger + SBOM |

The OCI cache is content-addressed: `~/.sindri/cache/components/sha256:aaaa...` is
the blob regardless of which registry served it. Two registries mirroring the same
component share cache.

## 6. Maintainer flow — publishing a new component

A maintainer wants to add `tilt` (a Kubernetes dev tool) to `sindri/core`.

### Step 1 — author locally

In the `sindri-dev/registry-core` repo:

```
registry-core/
├── components/
│   ├── kubectl/
│   ├── python/
│   └── tilt/                    ← new
│       ├── component.yaml
│       └── install.ps1          ← only if a backend needs it
└── .github/workflows/publish.yml
```

`components/tilt/component.yaml`:

```yaml
apiVersion: sindri.dev/v4
kind: Component
metadata:
  name: tilt
  version: "0.33.21"
  category: devops
  license: Apache-2.0
  homepage: https://tilt.dev

platforms: [linux-x86_64, linux-aarch64, macos-aarch64, windows-x86_64]

install:
  default:
    binary:
      source: github-release
      repo: tilt-dev/tilt
      assets:
        linux-x86_64: "tilt.{{version}}.linux.x86_64.tar.gz"
        linux-aarch64: "tilt.{{version}}.linux.arm64_64.tar.gz"
        macos-aarch64: "tilt.{{version}}.mac.arm64_64.tar.gz"
        windows-x86_64: "tilt.{{version}}.windows.x86_64.zip"
  overrides:
    macos: { brew: { package: tilt, tap: tilt-dev/tap } }
    linux: { mise: { tools: ["tilt@{{version}}"] } }
    windows: { scoop: { bucket: extras, package: tilt } }

validate:
  commands:
    - name: tilt
      versionFlag: version
      expectedPattern: "v0\\.33\\.21"
```

### Step 2 — local validation

Maintainer runs the same tools registry CI will run:

```
$ sindri registry lint ./components/tilt
✔ schema OK
✔ platforms complete
✔ all platforms have a default or override install path
✔ license Apache-2.0 in SPDX allowlist
✘ checksums missing for 4 platforms
  → run: sindri registry fetch-checksums ./components/tilt
```

```
$ sindri registry fetch-checksums ./components/tilt
Downloading 4 assets from github.com/tilt-dev/tilt releases...
✔ linux-x86_64   sha256:aaaa...
✔ linux-aarch64  sha256:bbbb...
✔ macos-aarch64  sha256:cccc...
✔ windows-x86_64 sha256:dddd...
Updated components/tilt/component.yaml with checksums.
```

```
$ sindri registry lint ./components/tilt
✔ all checks passed
```

### Step 3 — open PR

Maintainer opens a PR in the registry repo. Registry CI:

1. Re-runs `sindri registry lint` for every changed component.
2. Runs **install smoke-tests** on the platforms the component declares: spin up a
   GHA runner per platform, run `sindri install tilt@0.33.21` in isolation, assert
   `validate` passes. This is the contract that the component actually works.
3. Cross-checks: any component with `capabilities.collision-handling` touching paths
   outside `tilt/**` fails unless the PR also modifies a signed-off `core-capability-allowlist.yaml`.
4. Builds a preview registry artifact and links it in the PR:
   `oci://ghcr.io/sindri-dev/registry-core:pr-1472` — reviewers can install components
   from the preview registry to eyeball behavior.
5. License scanner (scancode) runs on any `install.sh` / `install.ps1` to catch
   contradictions with `metadata.license`.

Reviewers merge → publish workflow triggers.

### Step 4 — publish

The publish workflow runs on merge to `main`:

```
1. Regenerate `index.yaml` from `components/*/component.yaml`.
2. Compute sha256 for every file in components/.
3. Tar up components/ + index.yaml + checksums/.
4. Push as an OCI artifact:
   $ oras push ghcr.io/sindri-dev/registry-core:2026.05 \
       --artifact-type application/vnd.sindri.registry.v4+yaml \
       <layers...>
5. Sign:
   $ cosign sign ghcr.io/sindri-dev/registry-core:2026.05
6. Emit SLSA provenance attestation (GitHub-generated).
7. Update registry documentation site (static Hugo/Astro build from index.yaml).
```

Tag format recommendation: `YYYY.MM` for core registries on a monthly cadence, plus
`:latest` and `:stable` moving pointers. Private registries pick their own tag
conventions.

### Step 5 — users pick it up

Next time any user runs `sindri registry refresh`, the new index is pulled and
`tilt` appears in `sindri search`. Users who had `oci://.../registry-core:2026.04`
pinned don't get `tilt` until they bump to `:2026.05` — tag-immutability guarantees
they can't be surprised by registry changes.

## 7. Maintainer flow — publishing a new _version_ of an existing component

Smaller-scope change. Two paths:

### Path A — fully automated (recommended for leaf components tracking upstream)

Sindri ships a `sindri registry bump` command that maintainers wire into Renovate:

```
# .github/renovate.json (in registry-core repo)
{
  "customManagers": [{
    "customType": "regex",
    "fileMatch": ["^components/.+/component\\.yaml$"],
    "matchStrings": [
      "# renovate: depName=(?<depName>.+?) datasource=(?<datasource>.+?)\\s*?version:\\s*?\"?(?<currentValue>[^\"\\s]+)"
    ]
  }]
}
```

Renovate watches upstream releases; when kubernetes/kubernetes tags v1.31.4, it
opens a PR that:

1. Updates `components/kubectl/component.yaml` `version: "1.31.4"`.
2. Runs `sindri registry fetch-checksums` to refresh checksums.
3. Runs the same CI pipeline as in §6 step 3.

Humans review and merge; publish runs as in §6 step 4.

### Path B — manual

Maintainer edits `version:` + `checksums:` in `component.yaml`, opens PR, CI runs,
merges. Exactly the same pipeline, less automation.

### Multi-version retention

`index.yaml` keeps the last N versions per policy (suggestion: last 12 months _or_
last 10 minor versions, whichever is longer; LTS versions pinned indefinitely).
Older versions stay addressable by digest but drop out of `ls`/`search` results.
Users pinning to a pruned version get a clear message at `sindri resolve`:
`kubectl@1.29.0 not listed in sindri/core — still available by digest if you add it
explicitly`.

## 8. Signing and trust — end to end

- Registry publishes cosign-sign the top-level OCI manifest digest.
- `sindri registry trust acme --signer cosign:key=...` records the public key.
- `sindri registry refresh` verifies the signature before writing to cache. Failure
  aborts the refresh with the stale index left in place.
- `sindri resolve` verifies the component-blob digests against the index's declared
  digests. Any mismatch is a hard error.
- `sindri apply` verifies downloaded install artifacts against the checksums in
  `component.yaml` before execution.

Three independent integrity checks, each at a layer boundary.

## 9. One diagram to hold the whole thing

```
MAINTAINER SIDE                         CONSUMER SIDE
───────────────                         ─────────────

registry-repo/                          user runs:
├── components/*/component.yaml           sindri search kubectl
├── CI: lint + smoke-test                  │
└── publish on merge                       ▼
    │                                  ~/.sindri/cache/registries/<name>/index.yaml
    │ oras push                            │ (fresh within TTL, else pull manifest digest,
    ▼                                      │  compare, pull index.yaml blob, update cache)
OCI REGISTRY (ghcr.io, etc.)               ▼
    tag :2026.05 (immutable)           fuzzy search → matches displayed
    ├── manifest.json                      │
    ├── cosign signature                   ▼  sindri add / resolve
    └── layers:                        pull component.yaml blob (content-addressed cache)
        ├── index.yaml                     │
        ├── components/*/component.yaml    ▼
        └── checksums/sha256sums       admission gates (§08) → sindri.lock
                                           │
                                           ▼  sindri apply
                                       per-backend install; verify binary checksums
                                           │
                                           ▼
                                       installed state + SBOM
```

## 10. Summary answers

> Are registries where the installable software lives?

Yes — registries hold **components**, which are manifests describing how to install
a piece of software. The software itself (binaries, packages) lives upstream; the
registry is the curated description that points at it, pins versions, and asserts
checksums.

> Do registries hold versions?

Yes. Each component has multiple versions, each represented by a distinct
`component.yaml` blob addressed by content digest. The `index.yaml` enumerates
available versions; `sindri.lock` captures the exact digest the user resolved
against.

> What happens on `sindri search kubectl`?

Load configured registries → ensure indices fresh (cache + TTL + manifest-digest
comparison) → fuzzy-search across indices → render annotated results. No component
manifests are pulled until the user drills in with `show`/`add`.

> What happens when a maintainer publishes?

Author `component.yaml` + assets → `sindri registry lint` locally → PR → CI
(lint + smoke-test installs + license scan + preview registry) → merge →
regenerate `index.yaml` → `oras push` a new immutable tag → cosign-sign → users pick
it up on next `sindri registry refresh`.

The entire lifecycle runs on OCI primitives — no Sindri-specific distribution
infrastructure, no custom protocols.
