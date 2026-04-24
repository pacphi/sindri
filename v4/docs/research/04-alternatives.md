# Alternative Architectures Considered

Two credible alternatives to the primary recommendation, with the reasons the primary
won. Both remain viable if a prototype reveals serious problems with OCI-native
distribution or with the atomic-component boundary.

---

## Alternative A — "Nix-lite": single backend, content-addressed store

**Shape.** Collapse all install methods into one model: each component is a
reproducible derivation that writes to a content-addressed store
(`~/.sindri/store/<hash>/...`). Sindri becomes a thin Nix-inspired package manager
rather than a multi-backend orchestrator.

**Manifest.**

```yaml
components:
  - nodejs@22.11.0
  - python@3.14.0
  - aws-cli@2.17.21
```

**Strengths.**

- Reproducibility is bulletproof (hash of inputs determines output path).
- No backend-dispatch complexity. One handler.
- Rollback is free (keep old store paths).
- SBOM emerges trivially.

**Why not primary.**

1. Re-implements Nix for a narrower problem. The `mise / apt / npm / binary / script`
   surface already exists upstream; throwing that away to chase Nix-grade reproducibility
   is scope creep for a CLI whose users want "install node 22 and gcloud."
2. Forces Sindri to author a derivation for every tool — enormous maintenance burden
   vs. riding on mise/aqua/apt ecosystems.
3. Users explicitly asked to _pick_ the package manager per tool. This alternative
   removes that choice entirely.
4. The "apt:" verb with system-package semantics (services, users, systemd units) is
   awkward to express in a pure-store model.

**When to revisit.** If, after v4 ships, the biggest user complaint is non-reproducibility
across machines, a Nix-style store could be added as a _new backend_ (`store:`) under
the primary design rather than a replacement for it.

---

## Alternative B — "Keep extensions, add a BOM layer"

**Shape.** Keep v3's `extension.yaml` + `InstallMethod` enum largely as-is. Add a
user-authored `sindri.yaml` whose entries reference extensions by name and a pinned
extension version. The "atomic decomposition" is achieved purely by splitting today's
bundle extensions into smaller extensions; everything else — registry format, install
dispatcher, capabilities — is untouched.

**Manifest.**

```yaml
components:
  - extension: nodejs
    version: "1.2.3"
  - extension: codex
    version: "0.4.1"
  - extension: aws-cli
    version: "2.1.0"
```

**Strengths.**

- Smallest delta to v3. Lowest implementation risk.
- Extension authors' mental model is preserved.
- Registries stay as git repos, not OCI artifacts — no OCI tooling required.

**Why not primary.**

1. **The compatibility matrix doesn't die.** If extensions keep their own `metadata.version`
   and the CLI still has to know which extension versions are valid, you still need a
   mapping — just renamed. The user explicitly wants this gone.
2. **Backend choice stays implicit.** `extension: nodejs` doesn't tell the user which
   backend installs it — that's buried inside the extension's `install.method`. The user
   asked for the backend to be a user-visible choice.
3. **Duplicate pinning stays.** Extensions still declare their tool versions internally;
   users declare extension versions externally. Two layers of pins, same problem as v3.
4. **Collections stay a separate type** (profiles.yaml) rather than unifying as
   meta-components, missing a simplification opportunity.

**When to revisit.** If prototyping the OCI registry story reveals show-stopper friction
(e.g., no good offline story, auth complications with private registries), falling back
to this design keeps v4 shippable without matrix maintenance if — and only if — we
commit to one-version-per-extension via git tags and no longer support multiple
extension versions concurrently.

---

## Alternative C (mentioned, rejected early) — "devcontainer features verbatim"

Use the existing `devcontainer-feature.json` spec directly, publish to GHCR, consume
via a devcontainer-compatible manifest. Free tooling ecosystem (VS Code support,
Dependabot support).

**Why not.** Sindri's scope is broader than devcontainer (system services, multi-distro
runtime provisioning, MCP integration, project scaffolding). Forcing every v3 capability
into the devcontainer-feature schema either breaks the spec or requires a non-standard
superset, at which point the ecosystem benefit evaporates. Better to borrow the _shape_
(primary design) than to adopt the standard wholesale.
