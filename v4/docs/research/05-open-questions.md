# Open Questions

Decisions the team should close out before v4 implementation starts. Grouped by urgency.

## Before any prototype

1. **Registry transport: OCI only, or OCI + git?**
   Primary design assumes OCI. Git-hosted registries are easier to author (just push a
   tag) but lack content-addressability and require bespoke caching. Suggest: OCI as
   the only production transport, with a `registry:local` loader for development.

   **Decision:** OCI only. `registry:local` loader for development.

2. **Does the CLI ship with an embedded "core" registry, or is every registry a download?**
   v3 bundles `/opt/sindri/registry.yaml`. v4 could still ship a core registry as an
   embedded OCI layout for zero-network bootstrap, or require the first `sindri resolve`
   to pull. Suggest: embed a minimal bootstrap registry (mise-config, nodejs, python,
   script) so `sindri init` works offline; everything else on demand.

   **Decision:** No embedded registry. First `sindri resolve` pulls everything.

3. **Component definitions: how do we protect the capability contracts (project-init,
   collision-handling, hooks) when components now live in OCI artifacts outside our
   tree?** These subsystems were hardened through ADR-047 and heavy testing against
   in-tree extensions. Third-party OCI registries can publish components with arbitrary
   collision-handling configs. Options: (a) capability schema is versioned and validated
   at resolve-time, (b) a signed-registry model where only signed publishers can declare
   collision-handling, (c) keep capabilities restricted to components from sindri-core
   registry only. No strong recommendation yet — needs a prototype.

   **Decision:** Deferred — leave open pending prototype feedback.

## Manifest syntax details

4. **Map vs. list for `components:`.**
   Draft uses a map (`mise:nodejs: "22.11"`). Lists (`- backend: mise, name: nodejs,
version: "22.11"`) are uglier but survive order-sensitive scenarios better.
   Recommend: map, with `order:` as an optional component-level override for the DAG.

   **Decision:** Map, with `order:` as an optional component-level override.

5. **Version syntax: exact-only, or allow ranges?**
   aqua forces exact, mise allows ranges, devbox allows ranges. If we allow ranges in
   `sindri.yaml`, `sindri.lock` resolves them to exact. Recommend: allow ranges in
   `sindri.yaml`, require exact in the lockfile, fail `sindri install` if lockfile is
   stale or missing.

   **Decision:** Allow ranges in `sindri.yaml`; exact-only in `sindri.lock`; fail install if lockfile is stale or missing.

6. **How does a user pick between two backends offering the same tool?**
   `mise:python` and `apt:python3` are different components in different registries —
   no ambiguity. But `mise:nodejs` and `npm:@a/nodejs-shim` could both be valid choices.
   Suggest: the backend prefix is authoritative; no "auto-pick" mode. Users make the
   choice explicit.

   **Decision:** Backend prefix is authoritative; no auto-pick. User choice is explicit.

## Ecosystem & distribution

7. **Third-party registries — signed by default?**
   cosign-signed registries with verification at resolve time is the modern pattern.
   Probably required for an enterprise story. Cost: tooling and doc overhead on day one.

   **Decision:** Signed registries required from day 1. cosign verification at resolve time.

8. **Renovate integration — do we commit to shipping a Renovate manager plugin?**
   aqua and mise both have first-party Renovate support. Without it, users can't keep
   `sindri.yaml` current automatically. Recommend: yes, ship a manager plugin in the
   same release.

   **Decision:** Ship a Renovate manager plugin in the same release.

9. **Offline / air-gapped workflow.**
   OCI mirrors are standard but need explicit docs and a `sindri registry mirror <url>`
   helper. Concretely: can a user do `sindri resolve --offline` with a pre-seeded cache
   dir? What's the UX? Needs spec.

   **Decision:** Deferred — needs a dedicated spec before committing to v4.0 scope.

## Capability-specific

10. **Collision-handling scope.**
    Today collision rules are declared by the extension and enforced at project-init.
    In v4, if components are pulled from third-party registries, a malicious component
    could declare `collision-handling: on-conflict: overwrite` on paths it shouldn't
    touch. Recommend: restrict `collision-handling` declarations to a list of paths
    matching a prefix derived from the component name (`{component-name}/...`), with
    a `:shared` escape hatch for components in the core registry only.

    **Decision:** Restrict `collision-handling` declarations to `{component-name}/...` path prefixes, with `:shared` escape hatch limited to core-registry components.

11. **MCP capability ergonomics.**
    The MCP capability binds component → MCP server config. In v4, with atomic
    components, users may compose N MCP-producing components into a single project;
    merging N MCP server configs reliably needs a spec. Likely already solved by v3's
    merge-json collision action, but worth confirming.

    **Decision:** Validate v3's merge-json collision action against N-component MCP merging first, then write a dedicated MCP-merge spec drawing on lessons learned and best-practice research.

## Product / naming

12. **Rename "extension" to "component"?**
    The design reads more naturally with "component" (matches SBOM, avoids overloaded
    "extension"). But v3 docs, CLI subcommands, and user mindshare all use "extension".
    Recommend: rename in v4. Breaking change is already the premise.

    **Decision:** Rename "extension" to "component" in v4.

13. **`sindri.yaml` vs `sindri.bom.yaml` vs `sindriproject.yaml`.**
    Shorter wins unless there's a collision. Recommend: `sindri.yaml` at repo root,
    `sindri.lock` alongside.

    **Decision:** `sindri.yaml` at repo root with `sindri.lock` alongside.

14. **Do profiles / project templates go away entirely, or live on as convenience
    commands that write a starter `sindri.yaml`?**
    Recommend: `sindri init --template anthropic-dev` writes a seeded `sindri.yaml`;
    templates are just starter manifests. No separate runtime object.

    **Decision:** Collapse. Templates are just starter `sindri.yaml` manifests written by `sindri init --template <name>`; no separate runtime profile object.

## Stretch / later

15. **Dynamic collections (Renovate-style `packageRules` + `groupName`).**
    Powerful but complex. Defer to v4.1 unless a clear user demand emerges in the
    prototype feedback.

    **Decision:** Deferred to v4.1.0 roadmap.

16. **Forced-override audit trail format.**
    Should `--allow-*` overrides require a justification string? Leaning: optional,
    mandatory only when `policy.audit.require_justification: true`. See
    `08-install-policy.md`.

    **Decision:** Optional by default; mandatory only when `policy.audit.require_justification: true`.

17. **License data source.**
    Trust the SPDX identifier declared in `component.yaml`, or have registry CI
    cross-check with an upstream scanner (scancode, etc.)? Speed vs. safety.
    See `08-install-policy.md`.

    **Decision:** Trust the declared SPDX identifier but cross-check via registry CI (safety-first). Ship maintainer tooling so cross-check is convenient:
    - `sindri component lint` runs a local scancode/licensee pass and flags SPDX mismatches before publish.
    - A reusable GitHub Action (`sindri/license-check-action`) that component repos drop into CI; publish-to-registry is gated on a passing license report.
    - Registry CI re-runs the scan on ingest and attaches a signed license attestation to the OCI artifact; resolver surfaces mismatch warnings at `sindri resolve` time.

18. **Component preference vs user preference tie-break.**
    When both declare a backend order, who wins? Leaning: user project-level
    `backendOrder` beats component hint. Component author's list is a floor, not a
    ceiling. See `08-install-policy.md`.

    **Decision:** User project-level `backendOrder` beats component hint. Component author's list is a floor, not a ceiling.

19. **Default policy strictness.**
    Ship the `default` policy preset as permissive (current lean) or as `strict`?
    Strict = pinned-only, signed registries, license allowlist, deny privileged.
    See `08-install-policy.md`.

    **Decision:** Default policy is permissive. `sindri init` prompts the user to confirm the policy preset (permissive vs. strict) interactively, with `--policy strict` / `--policy permissive` flags for non-interactive flows.

20. **Container-execution backend.**
    Docker only, or abstract over docker/podman/nerdctl/finch? Affects the
    "execution target" story in `07-cross-platform.md` §2.6.

    **Decision:** Docker only in v4.0.

21. **WSL detection on Windows.**
    When Sindri runs on Windows and WSL is installed, do we (a) ignore WSL and use
    native Windows backends, (b) offer WSL as an execution target, or (c) auto-detect
    and warn the user about the trade-offs? See `07-cross-platform.md`.

    **Decision:** Combine (b) + (c). Auto-detect WSL when running on Windows, surface the trade-offs to the user, and let them choose between WSL as execution target vs. native Windows backends. Persist the choice in `sindri.yaml` / local config so the prompt is one-time.

22. **Multi-backend preference on a single platform.**
    If a macOS component ships both `brew:gh` and `binary:gh` install blocks, who
    picks? User-explicit, Sindri auto-pick by heuristic, or component-declared
    preference? Leaning: component declares, user can override in `sindri.yaml`.
    See `07-cross-platform.md` §4.

    **Decision:** Component declares preference; user can override in `sindri.yaml`.

23. **Sindri's own distribution on macOS / Windows.**
    Do we publish to a Sindri Homebrew tap and a winget/scoop manifest in v4.0, or
    point users at direct downloads / `curl | sh`? Affects onboarding UX.

    **Decision:** Ship multiple distribution channels for flexibility:
    - Docker container image (OCI) for container-first users and CI.
    - npm package (wrapper + platform-specific binaries, mirroring the existing `packages/@sindri/cli*` scaffolding).
    - Direct download via `curl | sh` (POSIX) and a PowerShell installer script (Windows).
      Homebrew tap / winget / scoop manifests can follow post-v4.0 once distribution volume justifies maintenance.

24. **Windows shell target — PowerShell 7+ only, or pwsh + Windows PowerShell 5.1?**
    5.1 ships with Windows; 7+ requires install. Being 7+-only simplifies scripting
    dramatically but adds a bootstrap step. See `07-cross-platform.md`.

    **Decision:** PowerShell 7+ only.

25. **Discovery cache TTL.**
    How long `sindri ls` / `search` / `show` trust the cached registry index before
    re-fetching. Default 24h proposed; may need per-registry overrides (private
    registries change more often than `sindri/core`). See `06-discoverability.md`.

    **Decision:** Default 24h cache TTL with per-registry overrides. Expose `--refresh` / `--force-refresh` flag on `sindri ls / search / show` for on-demand re-fetch.

26. **Target-infra field updatability.**
    Some fields update in place (replica count); others require destroy+recreate
    (region change, volume shrink). `target update` must classify per-kind the
    way `terraform plan` does, and either do the right thing or fail loudly.
    Needs an implementation-time classification table. See `12-provider-targets.md` §16.

    **Decision:** Terraform-plan-style classification table per kind. `target update` either does the right thing or fails loudly. In interactive mode, prompt for confirmation before applying changes (especially destroy+recreate).

27. **Target-infra boundary — where does Sindri stop and Terraform start?**
    Sindri owns the dev-environment surface (one Fly app, one RunPod pod, one K8s
    namespace scope). It does not own the surrounding graph (VPCs, IAM, DNS beyond
    what the target itself creates). The line needs to be spelled out so users
    know when to reach for Terraform. Leaning: own only what the target's provider
    API can atomically create/destroy. See `12-provider-targets.md` §16.

    **Decision:** Sindri owns only what the target's provider API can atomically create/destroy. Surrounding graph (VPCs, IAM, DNS beyond target-managed records) is out of scope and belongs to Terraform.

28. **Virtual / aggregate targets (`devpod` today).**
    v3 treats `devpod` as one provider with a nested `type:` selector for its
    sub-backends. Collapse to `type: devpod-aws` (etc.) in v4, or keep nested?
    See `12-provider-targets.md` §14.

    **Decision:** Collapse to `type: devpod-aws` / `devpod-gcp` / etc. in v4. No nested `type:` selector.

29. **Auth backward-compatibility shorthand.**
    v3 accepts provider-specific env vars (`E2B_API_KEY`, `FLY_API_TOKEN`) directly.
    Does v4 preserve these as implicit `env:` prefixes, or force every auth through
    explicit `auth:` blocks? Leaning: support both. See `12-provider-targets.md` §14.

    **Decision:** Support both. Provider-specific env-var shorthand remains as an implicit `env:` prefix alongside explicit `auth:` blocks — the two forms serve different use cases (quick local env vs. explicit policy/multi-account setups).

30. **Default target when none declared in `sindri.yaml`.**
    Implicit `local`? Error prompting user to pick? Affects the happy-path user who
    never opens YAML. See `12-provider-targets.md` §14.

    **Decision:** Implicit `local` default, with a visible indication in CLI output (e.g., `→ target: local (default)`) so users know which target is active.

31. **Target plugin extensibility — subprocess or WASM?**
    Subprocess-JSON (v3 pattern for `terraform-provider-*`) for v4.0, WASM later?
    Or compile-time only? Affects how fast Modal, Replit, Lambda, Azure Container
    Apps can appear as community targets. See `12-provider-targets.md` §14.

    **Decision:** Subprocess-JSON for v4.0 (same pattern as v3 `terraform-provider-*`). WASM deferred to a later release.

32. **Per-target lockfiles vs one lockfile with target sections.**
    Proposed: `sindri.<target>.lock` per target. Alternative: one `sindri.lock` with
    per-target sections. Affects git hygiene, merge conflicts, mental model.
    See `12-provider-targets.md` §14.

    **Decision:** Per-target lockfiles (`sindri.<target>.lock`).

33. **Scope of the v4 CLI — does `k8s` / `vm` / `image` stay?**
    Real features today. Keeping them expands v4 scope and dilutes the
    "one-page cheat sheet" goal. Product decision. See `11-command-comparison.md` §2.10.

    **Decision:** Drop `k8s` / `vm` / `image` from the core v4 CLI surface. Preserve functionality via supplemental docs/guides and convenience scripts (distributed as components or companion repo) so users with those workflows have a stopgap without diluting the cheat sheet.

34. **Registry-tag cadence vs rolling additions.**
    When a new component version lands between monthly `:YYYY.MM` registry tags,
    does it go into the existing tag, a `:YYYY.MM.N` patch tag, or wait for the
    next major? Leaning: patch tags, so the majors stay immutable while rolling
    pointers (`:latest`, `:stable`) carry rolling additions.
    See `11-command-comparison.md` §5.2.

    **Decision:** Patch tags (`:YYYY.MM.N`) for additions between monthlies. Monthly majors (`:YYYY.MM`) stay immutable. Rolling pointers `:latest` and `:stable` track rolling additions.

35. **Ambiguous short names across registries.**
    If `sindri/core` and `acme/internal` both publish `aws-cli`, what does
    `sindri show aws-cli` do? Proposed: error with a disambiguation list, user types
    the fully-qualified `registry/name`, with a configurable "primary registry" that
    can be referenced unqualified. Needs a spec. See `06-discoverability.md`.

    **Decision:** Error with a disambiguation list; user must type the fully-qualified `registry/name`. A configurable "primary registry" may be referenced unqualified.

36. **Collection-vs-explicit version conflicts.**
    When `sindri.yaml` depends on `collection:anthropic-dev` (which pins `mise:nodejs: "22.11.0"`)
    _and_ explicitly pins `mise:nodejs: "20.x"`, which wins? Two defensible policies: (a)
    explicit manifest entry overrides collection transitive pin, (b) conflict is a hard
    error that the user must resolve with an explicit `override:` block. Tentative lean:
    (a) for ergonomics, with a `sindri resolve --strict` mode that enforces (b) for CI.

    **Decision:** (a) explicit manifest entry overrides collection transitive pin for ergonomics. `sindri resolve --strict` enforces (b) — hard error requiring an explicit `override:` block — for CI.

37. **Per-machine manifest overlays** ("my laptop wants `gui-tools`, CI doesn't").
    Devbox solves this with includes. Suggest: `sindri.yaml` supports `include:` + `override:`
    (standard YAML merge semantics). Not blocking for v4.0.

    **Decision:** Deferred to v4.1.0 as a feature design + implementation roadmap item.
