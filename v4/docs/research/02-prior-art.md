# Prior Art

Eight systems evaluated against the v4 design goals. Ranked roughly by direct relevance.

## mise (`mise.toml`) — highest-value reference

- **Manifest:** TOML `[tools]` table, `"backend:name" = "version"`.
- **Backends (pluggable sources for the same tool):** `core`, `aqua`, `asdf`, `cargo`, `npm`, `pipx`, `go`, `ubi`/`github`, `spm`, `vfox`.
- **Pinning:** exact / fuzzy / ranges / `latest`; `mise.lock` for reproducibility.
- **Collections:** none first-class. Composition via shared files or tasks.
- **Takeaway:** The `backend:tool@version` triple is the cleanest "same tool, pick your source" syntax in the field. Steal it. Backend-as-plugin is also the right extensibility axis.

## aqua — the curation discipline

- **Manifest:** YAML with `registries:` (catalogs) and `packages:` (selections).
- **Pinning:** mandatory exact versions. Checksums (`aqua-checksums.json`) + SLSA provenance. Renovate-native comments (`# renovate: depName=`).
- **Backends:** single (GitHub releases / http / go-install), but multiple curated registries.
- **Takeaway:** Forced pins + checksums + Renovate-friendliness is the gold standard for curated registries. Sindri's "per-PM registries we curate" is exactly the aqua registry model, generalized across backends.

## Devcontainer Features — the atomic-component shape

- **Manifest:** `devcontainer.json` `features: {}` map, `oci-ref` → typed options object.
- **Unit shape:** tarball containing `devcontainer-feature.json` + `install.sh`. Distributed via OCI, addressable by digest.
- **Pinning:** by tag or digest (`ghcr.io/.../node:1.3.1`).
- **Ordering:** `dependsOn` / `installsAfter` DAG.
- **Takeaway:** Atomic, versioned, OCI-addressable units with typed options — this _is_ the granularity Sindri wants. OCI pinning eliminates the CLI↔extension matrix because each unit advances independently and is addressed by digest.

## apt (sources.list.d + preferences.d + meta-packages)

- **Multiple sources:** via repositories (pools) with GPG keys and `Pin-Priority`.
- **Collections:** **meta-packages** — a package whose sole purpose is to depend on others (`build-essential`).
- **Takeaway:** Model collections as meta-components — a component whose only content is `dependsOn`. Unifies the data model (one shape, not two) and mirrors how apt, nixpkgs, and homebrew all ultimately express bundles.

## Chainguard apko

- **Manifest:** YAML `contents.repositories`/`keyring`/`packages`; auto-emits SPDX/CycloneDX SBOM.
- **Takeaway:** SBOM as the _byproduct_ of a resolved manifest — not a thing you manually re-declare in every extension. v3's duplicated BOM section (declare once in extension, again in matrix) should collapse to apko's model: the resolved manifest _is_ the BOM.

## Devbox (`devbox.json`)

- **Manifest:** JSON `packages: ["nodejs@20", "go@1.22"]`; single nixpkgs backend but per-package `@version` resolved via Nix History Search; per-package lockfile entries in `devbox.lock`.
- **Takeaway:** Per-package lockfile entries (not a global channel lock) are what lets users mix recent and old versions safely.

## Homebrew Bundle (`Brewfile`)

- **Manifest:** Ruby DSL — `brew`, `cask`, `mas`, `tap`, `vscode`, `whalebrew` verbs.
- **Pinning:** weak; `brew bundle lock` exists but limited.
- **Takeaway:** Verb-per-source reads clearly and is user-friendly. Weak pinning is the cautionary tale — don't repeat.

## Renovate / Dependabot

- **Manifest:** _update policy_, not dependencies (`rangeStrategy: pin|bump`, `pinDigests`, `packageRules` + `groupName`).
- **Takeaway:** Design the v4 manifest so Renovate can update it — inline `# renovate: depName=` comments, pinned versions not ranges. Also: predicate-based grouping (glob/regex) is more flexible than hardcoded collection lists and is worth considering for "dynamic collections."

## Also evaluated, lower relevance

- **pkgx / pantry** — similar goals to mise; less mature multi-backend story.
- **asdf** — mise is strictly a superset; no reason to pick asdf over mise.
- **proto** — Moon's tool manager; tightly coupled to Moon repo-management; not a fit.

## Synthesis — what to borrow from whom

| Concept                                                         | Source                |
| --------------------------------------------------------------- | --------------------- |
| Manifest syntax (`backend:tool@version`)                        | mise                  |
| Atomic component shape + OCI distribution                       | Devcontainer Features |
| Registry discipline (forced pins, checksums, Renovate-friendly) | aqua                  |
| Collections as meta-components                                  | apt (generalized)     |
| SBOM as resolver byproduct, not declaration                     | apko                  |
| Per-component lockfile entries                                  | Devbox                |
| Update-policy file separate from manifest                       | Renovate              |
