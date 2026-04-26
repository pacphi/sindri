# Sindri Repository Reorganization Plan

> **Target location once approved:** `docs/REPO_REORG_PLAN.md` on branch `chore/repo-reorg`.
> This file in `~/.claude/plans/` is the working draft per plan-mode rules.

## 1. Context

The `main` branch carries source for **three product generations simultaneously** (v1, v2, v3), and a fourth generation lives on `research/v4` as an isolated experiment. Both layouts cause friction:

| Version | Where it lives today                                                                                                         | Stack            | Status              |
| ------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------- |
| v1      | `main:v1/` (only `CHANGELOG.md`)                                                                                             | legacy bash      | EOL — security-only |
| v2      | `main:v2/` + `examples/v2/` + `docker/lib/` + `RELEASE_NOTES.v2.md` + half the workflows                                     | Bash + Docker    | Maintenance         |
| v3      | `main:v3/` + `examples/v3/` + `packages/@pacphi/sindri-cli*` + `RELEASE_NOTES.v3*.md` + the rest of workflows + `.gitnexus/` | Rust workspace   | Active              |
| v4      | `research/v4:v4/` (Rust workspace; `registry-core`, `renovate-plugin`, `tools`, own `v4/.github/workflows/ci.yml`)           | Rust, redesigned | Pre-release         |

Pain caused by intermingling on `main`:

- Every PR runs both `ci-v2.yml` and `ci-v3.yml` regardless of scope.
- `docs/migration/`, `docs/FAQ.md`, `docs/ides/*` mix v2 and v3 guidance.
- Releases for v2 and v3 are entangled in the same tag namespace and CHANGELOG.
- macOS-duplicate folders (`examples/v2 2`, `.github/actions/packer 2`, `v2/cli/extension-manager-modules 2`, `.claude/commands/git 2`, etc.) have proliferated because everything sits in one tree.
- v4 lives on a long-lived feature branch with its own private workflows, drifting from repo conventions.

**Goal:** four sibling maintenance branches — `v1`, `v2`, `v3`, `v4` — each holding _only_ its own source, docs, examples, release notes, and per-version AI tooling. `main` holds **no product source**: only the umbrella README/CHANGELOG/LICENSE/SECURITY/CONTRIBUTING, repo-wide hygiene, and a centralized `.github/workflows/` that routes to the appropriate v\* branch on push/PR.

## 2. Dedicated branch for the reorg effort

**Yes — `chore/repo-reorg`.** All preparatory work happens there. The cutover (creating the four sibling branches and reshaping `main`) is one atomic operation merged from this branch. Until cutover, `main` stays untouched and any in-flight v3 work continues unaffected.

The reorg branch is deleted after `main` is verified.

## 3. Workflow trigger model (per user requirement: ALL workflows on `main`)

Per-branch workflow files do **not** exist on `v*` branches. Every CI/release/test workflow lives on `main` and triggers on push/PR to a `v*` branch. `actions/checkout@v4` defaults to the SHA that triggered the workflow, so a workflow on `main` checking out `${{ github.ref }}` operates on the v\* branch's source tree. Secrets, permissions, and required-checks settings stay centralized.

### 3.1 Architecture: reusable callables + thin per-version routers

User preference: reusable workflows, _but_ respecting that v2 (Bash/Docker), v3 (Rust + npm wrapper), and v4 (Rust, different crate layout) have **different stacks and idiosyncrasies**. So the recipe is:

```
.github/workflows/
  # Per-version routers — one file per (version, purpose).
  # Each is a small shim that decides which callable to invoke
  # and what inputs to pass.
  ci-v1.yml             # on push/PR to v1 → noop (EOL) or markdown-lint only
  ci-v2.yml             # on push/PR to v2 → calls _ci-bash.yml + _ci-docker.yml
  ci-v3.yml             # on push/PR to v3 → calls _ci-rust.yml(workspace=v3) + _ci-npm.yml
  ci-v4.yml             # on push/PR to v4 → calls _ci-rust.yml(workspace=v4)
  release-v2.yml
  release-v3.yml
  release-v4.yml
  # v2-specific (bash/docker test matrices)
  v2-test-extensions.yml
  v2-test-profiles.yml
  v2-test-provider.yml
  v2-deploy-sindri.yml
  v2-manual-deploy.yml
  v2-teardown-sindri.yml
  # v3-specific (provider matrix, packer)
  v3-discover-extensions.yml
  v3-extension-test.yml
  v3-matrix-generator.yml
  v3-packer-build.yml
  v3-packer-test.yml
  v3-pre-release-test.yml
  v3-provider-{devpod,docker,e2b,fly,k3d,northflank,packer,runpod}.yml
  v3-test-profiles.yml
  integration-test-providers.yml
  # v4-specific (TBD; promoted from research/v4:v4/.github/workflows/ci.yml)
  v4-*.yml as required
  # Reusable callables (workflow_call only, no triggers)
  _ci-rust.yml          # inputs: workspace_dir, toolchain, features
  _ci-bash.yml          # inputs: shellcheck_paths, test_dir
  _ci-docker.yml        # inputs: dockerfile, context, image_tag
  _ci-npm.yml           # inputs: package_dir, node_version
  _release-cargo-dist.yml
  _release-docker.yml
  # Repo-wide hygiene (always trigger on main and v*)
  check-links.yml
  validate-markdown.yml
  validate-shell.yml
  validate-yaml.yml
  cleanup-container-images.yml
  cleanup-workflow-runs.yml
  build-base-image.yml  # audit ownership; likely v2-specific
```

Each `ci-vN.yml` looks roughly like:

```yaml
name: CI v3
on:
  push: { branches: [v3] }
  pull_request: { branches: [v3] }
jobs:
  rust:
    uses: ./.github/workflows/_ci-rust.yml
    with:
      workspace_dir: v3
      toolchain: stable
  npm:
    uses: ./.github/workflows/_ci-npm.yml
    with:
      package_dir: packages/@pacphi/sindri-cli
```

`workflow_dispatch` is added to every router to allow manual re-runs against the v\* HEAD.

### 3.2 `.github/actions/` and `.github/scripts/`

These also live exclusively on `main`. The composite actions (`packer/`, `providers/`, `shared/`, `v3/`) and helper scripts (`generate-changelog.sh`, `validate-versions.sh`, `providers/`, `v3/`) are referenced by the routers. Their existing folder layout is kept; the `* 2` duplicates get purged.

### 3.3 Dependabot

`.github/dependabot.yml` on `main` declares one update group per ecosystem per branch using `target-branch:` so PRs land on the right v\*:

```yaml
updates:
  - { package-ecosystem: cargo, directory: /v3, target-branch: v3, schedule: { interval: weekly } }
  - { package-ecosystem: cargo, directory: /v4, target-branch: v4, schedule: { interval: weekly } }
  - { package-ecosystem: docker, directory: /v2, target-branch: v2, schedule: { interval: weekly } }
  - { package-ecosystem: github-actions, directory: /, target-branch: main, schedule: { interval: weekly } }
  - {
      package-ecosystem: npm,
      directory: /packages/@pacphi/sindri-cli,
      target-branch: v3,
      schedule: { interval: weekly },
    }
```

## 4. Source-tree audit

### 4.1 v1-only

- `v1/CHANGELOG.md` (the entire content of v1)

### 4.2 v2-only

- `v2/**` (cli, config, deploy, docker, scripts, test, docs, Dockerfile, Makefile, README, CHANGELOG)
- `examples/v2/**` → moves into `v2/examples/`
- `docker/lib/**` (root) → moves into `v2/docker/lib/` (after `rg "docker/lib" v3/ v4/` confirms no cross-use; v3 has its own `v3/docker/`)
- `RELEASE_NOTES.v2.md` → `v2/RELEASE_NOTES.md`

### 4.3 v3-only

- `v3/**`
- `examples/v3/**` → `v3/examples/` _(verify whether it duplicates `v3/examples/`; if so, prefer the more complete one)_
- `packages/@pacphi/sindri-cli*` (6 packages) — npm wrapper + 5 platform packages
- `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `.npmrc` _(if they only reference `packages/@pacphi/_`; otherwise stay on main with a v3-narrowed scope)\*
- `RELEASE_NOTES.v3.md`, `RELEASE_NOTES.v3.1.md` → `v3/RELEASE_NOTES.md`, `v3/RELEASE_NOTES.3.1.md`
- ~~`.gitnexus/`~~ — **NEVER commit; gitignored on every branch.** Each developer regenerates locally via `npx gitnexus analyze`.
- `docker-compose.yml` (apps/api integration; v3-era per memory)
- v3 portions of cross-cutting docs (see §4.6)

### 4.4 v4-only (promoted from `research/v4`)

- `research/v4:v4/**` becomes the new `v4` branch's `v4/` directory
- `v4/.github/workflows/ci.yml` (currently nested) is **promoted to `main`'s `.github/workflows/`** as part of the v4 router/callables, and removed from `v4/`
- Any v4 RELEASE_NOTES (none yet) live at `v4/RELEASE_NOTES.md`
- `research/v4` branch is deleted after promotion

### 4.5 Cross-cutting → stays on `main`

- `LICENSE`
- `README.md` (rewritten as a router: "which version, where it lives, how to pick")
- `CHANGELOG.md` (umbrella; pointers only)
- `SECURITY.md` (single project-wide policy; promoted from `docs/SECURITY.md` to repo root, GitHub convention)
- `CONTRIBUTING.md` (promoted from `docs/CONTRIBUTING.md`)
- `sindri.png` (logo)
- `.github/CODEOWNERS`, `.github/dependabot.yml`, `.github/ISSUE_TEMPLATE/`, `.github/templates/`, `.github/docs/`
- `.github/workflows/**` (ALL workflows — see §3)
- `.github/actions/**`, `.github/scripts/**` (purged of `* 2`/`* 3` duplicates)
- `docs/` on main contains **governance only**: `RELEASE.md`, `CHANGELOG_MANAGEMENT.md`, `COMPARISON_GUIDE.md`, `REPO_REORG_PLAN.md` (this document)
- Repo-hygiene configs: `.markdownlint.json`, `.markdownlintignore`, `.prettierrc`, `.prettierignore`, `.shellcheckrc`, `.yamllint.yml`, `.gitignore`, `.dockerignore`, `.husky/`, `.devcontainer/`
- `Makefile` on main is a thin meta-makefile (`make sync-changelogs`, `make audit`); per-version Makefiles live under each `v*/`

### 4.6 `docs/` on main today — disposition

User preference: **no root-level `docs/` on `v*` branches.** Each branch keeps its docs under its own `v*/docs/`.

| Current path on main                                                 | Disposition                                                                                                                                                                |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/README.md`                                                     | Folded into the rewritten root `README.md` on main                                                                                                                         |
| `docs/FAQ.md`                                                        | Split: v2 portion → `v2/docs/FAQ.md`; v3 portion → `v3/docs/FAQ.md`                                                                                                        |
| `docs/faq/` (build.mjs, Dockerfile, fly.toml, src/, validate-faq.sh) | Audit for v2 vs v3 ownership; likely v3-era — moves to `v3/docs/faq/`                                                                                                      |
| `docs/ides/*.md`                                                     | Each file split by version; v2 sections → `v2/docs/ides/`; v3 sections → `v3/docs/ides/`; v4 gets a fresh stub generated from v3                                           |
| `docs/migration/MIGRATION_GUIDE.md`                                  | v1→v2 portion → `v2/docs/migration/`; v2→v3 portion → `v3/docs/migration/`; v3→v4 stub → `v4/docs/migration/`. High-level "which version" content folds into main's README |
| `docs/migration/COMPARISON_GUIDE.md`                                 | Stays on main as `docs/COMPARISON_GUIDE.md` (inherently cross-version)                                                                                                     |
| `docs/migration/README.md`                                           | Stays on main as the index for `docs/COMPARISON_GUIDE.md`                                                                                                                  |
| `docs/research/` (empty)                                             | Delete                                                                                                                                                                     |
| `docs/SECURITY.md`                                                   | Promoted to root `SECURITY.md` on main                                                                                                                                     |
| `docs/CONTRIBUTING.md`                                               | Promoted to root `CONTRIBUTING.md` on main                                                                                                                                 |
| `docs/CHANGELOG_MANAGEMENT.md`, `docs/RELEASE.md`                    | Stay in `docs/` on main (governance)                                                                                                                                       |

### 4.7 Stray duplicate paths to delete (single audit-and-purge commit)

Tracked `* 2` / `* 3` directories detected:

```
examples/v2 2/
examples/v3 2/
.github/actions/packer 2/
.github/actions/providers 2/
.github/actions/shared 2/
.github/actions/v3 2/
.github/scripts/providers 2/
.github/scripts/v3 2/
.claude/commands/git 2/
.claude/commands/github 2/
v2/cli/extension-manager-modules 2/
v2/deploy/adapters 2/
```

Also build-artifact dups under `v3/target/` (not tracked — `.gitignore`'d) — left to the user's local working tree; nothing to do in git.

**Action:** `git rm -r` all tracked `* 2`/`* 3` paths in a single commit on `chore/repo-reorg` named `chore: purge tracked filesystem duplicates`. Audit log included in the cutover PR description.

## 5. Branch-by-branch target layout

### 5.1 `main`

```
README.md             # router: pick your version
CHANGELOG.md          # umbrella; links to v*/CHANGELOG.md
LICENSE
SECURITY.md
CONTRIBUTING.md
sindri.png
docs/
  RELEASE.md
  CHANGELOG_MANAGEMENT.md
  COMPARISON_GUIDE.md
  REPO_REORG_PLAN.md
  migration/README.md
.github/
  CODEOWNERS  dependabot.yml  ISSUE_TEMPLATE/  templates/  docs/
  workflows/                  # ALL workflows live here (see §3)
    ci-v1.yml ci-v2.yml ci-v3.yml ci-v4.yml
    release-v2.yml release-v3.yml release-v4.yml
    v2-*.yml v3-*.yml v4-*.yml integration-test-providers.yml
    _ci-rust.yml _ci-bash.yml _ci-docker.yml _ci-npm.yml
    _release-cargo-dist.yml _release-docker.yml
    check-links.yml validate-markdown.yml validate-shell.yml validate-yaml.yml
    cleanup-container-images.yml cleanup-workflow-runs.yml build-base-image.yml
  actions/                    # composite actions used by workflows
    packer/  providers/  shared/  v3/  v4/
  scripts/                    # workflow helper scripts
    generate-changelog.sh  generate-migration-guide.sh
    generate-slack-notification.sh  validate-versions.sh
    providers/  v3/  v4/
.markdownlint.json  .markdownlintignore  .prettierrc  .prettierignore
.shellcheckrc  .yamllint.yml  .gitignore  .dockerignore  .npmrc
.husky/  .devcontainer/
Makefile              # thin meta-makefile only
CLAUDE.md             # repo-wide AI tooling pointer (delegates to per-branch)
AGENTS.md             # repo-wide AI tooling pointer
```

**No `v1/`, `v2/`, `v3/`, `v4/` directories on main.**

### 5.2 `v1` (live, read-only)

```
README.md             # promoted/wrapped: "v1 is EOL; security-only patches"
CHANGELOG.md          # = current v1/CHANGELOG.md
LICENSE
v1/
  CHANGELOG.md
CLAUDE.md             # minimal v1-flavored AI instructions (or omit)
AGENTS.md
```

Branch protection: blocks pushes except by maintainers for security backports. No CI workflows live on this branch (they live on main and trigger on push/PR to `v1`). main's `ci-v1.yml` is intentionally minimal — likely just markdown-lint and link-check, since there's no source to build.

### 5.3 `v2`

```
README.md             # = current v2/README.md, lightly edited
CHANGELOG.md          # = v2/CHANGELOG.md
LICENSE
RELEASE_NOTES.md      # = current RELEASE_NOTES.v2.md (relocated)
v2/
  cli/  config/  deploy/  docker/lib/  scripts/  test/
  docs/             # incl. moved migration, ides, FAQ portions
  examples/         # moved from root examples/v2/
  Dockerfile  Makefile  README.md  CHANGELOG.md
CLAUDE.md             # v2-flavored AI instructions (Bash/Docker)
AGENTS.md
.markdownlint.json  .prettierrc  .shellcheckrc  .yamllint.yml
.gitignore  .dockerignore  .husky/  .devcontainer/
```

**No `.github/workflows/`** on this branch. Workflows trigger from main on push/PR to `v2`.

### 5.4 `v3`

```
README.md             # = current v3/README.md
CHANGELOG.md          # = v3/CHANGELOG.md
LICENSE
RELEASE_NOTES.md      # = RELEASE_NOTES.v3.md
RELEASE_NOTES.3.1.md  # = RELEASE_NOTES.v3.1.md
docker-compose.yml
package.json  pnpm-workspace.yaml  pnpm-lock.yaml  .npmrc
v3/
  crates/  bin/  config/  docker/  docs/  embedded/  examples/
  extensions/  inspec/  schemas/  scripts/  tests/
  Cargo.toml  Cargo.lock  rust-toolchain.toml
  Dockerfile  Dockerfile.base  Dockerfile.dev
  Makefile  README.md  CHANGELOG.md
  registry.yaml  profiles.yaml  compatibility-matrix.yaml
  audit-ignore  common.sh  extension-source.yaml
packages/@pacphi/
  sindri-cli/  sindri-cli-darwin-arm64/  sindri-cli-darwin-x64/
  sindri-cli-linux-arm64/  sindri-cli-linux-x64/  sindri-cli-win32-x64/
.gitnexus/
CLAUDE.md             # current Rust + GitNexus instructions
AGENTS.md
.markdownlint.json  .prettierrc  .shellcheckrc  .yamllint.yml
.gitignore  .dockerignore  .husky/  .devcontainer/
```

**No `.github/workflows/`** on this branch.

### 5.5 `v4` (promoted from `research/v4`)

```
README.md             # promoted from v4/README.md (or generated stub if absent)
CHANGELOG.md          # new, with first entry "promoted from research/v4"
LICENSE
v4/
  crates/  docs/  registry-core/  renovate-plugin/  schemas/  tools/
  Cargo.toml  Cargo.lock
  # NB: v4/.github/workflows/ci.yml is moved to main during promotion
CLAUDE.md             # v4-flavored AI instructions
AGENTS.md
.markdownlint.json  .prettierrc  .shellcheckrc  .yamllint.yml
.gitignore  .dockerignore  .husky/  .devcontainer/
```

`research/v4` branch is deleted after promotion.

## 6. Per-branch AI tooling (`CLAUDE.md`, `AGENTS.md`, `.gitnexus/`)

User choice: **on every branch, customized per version.**

| Branch | CLAUDE.md content                                                  | AGENTS.md | .gitnexus/                                                          |
| ------ | ------------------------------------------------------------------ | --------- | ------------------------------------------------------------------- |
| main   | Repo-wide overview, "see v\*/CLAUDE.md for stack-specific" pointer | Same      | absent                                                              |
| v1     | Minimal — "EOL, security backports only"                           | Same      | absent                                                              |
| v2     | Bash/Docker conventions, v2-specific patterns                      | Same      | absent                                                              |
| v3     | Current Rust + GitNexus instructions verbatim                      | Same      | **gitignored — never committed**; regenerated locally per developer |
| v4     | Rust + new architecture (registry-core, renovate-plugin)           | Same      | **gitignored — never committed**; regenerated locally per developer |

## 7. History strategy

User choice: **branch-from-head + `git rm`.** Each `v*` branch is created from `main`'s reorg-finalized HEAD, then non-owned files are deleted in a single commit per branch. Full shared history is preserved. SHAs are not rewritten. All existing PR/issue/blame links remain valid.

Safety net: `git tag pre-reorg-2026-04-25 main && git push --tags` _before_ anything else.

## 8. Open `feature/*` branches

User choice: **freeze all, require fresh branches post-cutover.** All current `feature/*` branches are marked `do-not-merge` (label or PR comment). The cutover PR explicitly lists them and instructs authors to recreate against the appropriate `v*` branch. In-progress code remains accessible via the existing branch refs; only the PRs are closed-without-merge with a pointer to the new branch.

Inventory of branches to communicate about (29 total per `git branch`):

- `claude/nervous-bouman`, `feature/add-backup-and-restore-commands`, `feature/add-better-devpod-k8s-support`, `feature/add-bom-support`, `feature/add-e2b-provider-adapter`, `feature/add-faq`, `feature/add-gpu-provisioning-support`, `feature/add-northflank-and-runpod-provider-support`, `feature/additional-extensions-3`, `feature/additional-extensions-for-3.1.0`, `feature/ci-workflow-enhancements`, `feature/documentation-improvements`, `feature/extension-installation-fixes`, `feature/extensions-galore`, `feature/extensions-responsible-for-adding-project-management-capabilities`, `feature/fix-fly-deploy-and-test-workflows`, `feature/improve-devpod-support`, `feature/improve-docker-extension`, `feature/new-providers-for-3.1.0`, `feature/refactor-github-actions-and-workflows`, `feature/repair-v2-release-automation`, `feature/security-improvements`, `feature/sindri-administrator`, `feature/v3`, `feature/v3-additions`, `feature/v3-dist-prep`, `feature/v3-stabilization`, `feature/v3-stabilization-part-2`, `feature/v3-vm-command`

## 9. Execution plan (on `chore/repo-reorg`)

> Steps 1–13 happen entirely on `chore/repo-reorg`. The cutover (step 14) flips main and creates the four sibling branches in one sitting.

1. **Safety tag.** `git tag pre-reorg-2026-04-25 main && git push --tags`.
2. **Create reorg branch.** `git checkout -b chore/repo-reorg main`.
3. **Inventory script.** `scripts/reorg/audit.sh` walks the tree and classifies every top-level path as `main`, `v1`, `v2`, `v3`, `v4`, or `purge`. Output committed for the PR description.
4. **Purge stray duplicates.** `git rm -r` every tracked path in §4.7. Single commit: `chore: purge tracked filesystem duplicates`.
5. **Manifests.** Author `scripts/reorg/manifest-{main,v1,v2,v3,v4}.txt` — explicit allow-lists per branch.
6. **Branch builder.** `scripts/reorg/build-branches.sh` is idempotent; given the manifests, produces the four branches by:
   - For each `v*`: `git checkout -B v* chore/repo-reorg && git rm -r <not-in-manifest> && git commit -m "chore(v*): isolate v* tree"`.
   - For `main`: `git checkout chore/repo-reorg && git rm -r v1/ v2/ v3/ examples/ packages/ docker/lib/ docker-compose.yml RELEASE_NOTES.* package.json pnpm-* .gitnexus/` and re-stages the rewritten umbrella files.
7. **Doc-splitting commits** (still on `chore/repo-reorg`, before branch creation):
   - Pre-stage `v2/docs/migration/`, `v3/docs/migration/`, `v4/docs/migration/` from splitting `docs/migration/MIGRATION_GUIDE.md`.
   - Split `docs/ides/*.md` into `v2/docs/ides/` and `v3/docs/ides/`. Generate v4 stubs.
   - Split `docs/FAQ.md` into `v2/docs/FAQ.md` and `v3/docs/FAQ.md`.
   - Move `docs/faq/` into `v3/docs/faq/` (after audit confirms v3-only).
   - Promote `v2/README.md` → root `README.md` material for v2 manifest; same for v3, v4.
   - Compose new main `README.md` (router + COMPARISON_GUIDE link).
   - Compose new main `CHANGELOG.md` (umbrella).
   - Promote `docs/SECURITY.md` → root `SECURITY.md` and `docs/CONTRIBUTING.md` → root `CONTRIBUTING.md`.
   - Author per-branch `CLAUDE.md` and `AGENTS.md` (§6).
8. **Workflow consolidation** (all on main):
   - Move `v4/.github/workflows/ci.yml` from `research/v4` into main as `v4-ci.yml` (or fold into `ci-v4.yml`).
   - Author the four router workflows `ci-v1.yml`…`ci-v4.yml`.
   - Author the reusable callables `_ci-rust.yml`, `_ci-bash.yml`, `_ci-docker.yml`, `_ci-npm.yml`, `_release-cargo-dist.yml`, `_release-docker.yml`.
   - Refactor existing `ci-v2.yml`, `ci-v3.yml`, `release-v2.yml`, `release-v3.yml`, the `v2-*.yml` and `v3-*.yml` set to _trigger only on their respective `v_` branch\* and to call the appropriate reusable callable for shared steps.
   - Audit `build-base-image.yml` ownership; move into v2 or v3 router as appropriate.
9. **Dependabot rewrite** with `target-branch:` per ecosystem (§3.3).
10. **Promote `research/v4`.** `git fetch origin research/v4 && git cherry-pick`/`git merge -s ours` strategy: snapshot `research/v4:v4/` into `chore/repo-reorg`'s tree under `v4/` (after manifest is built, this gets isolated to the v4 branch). Strip `v4/.github/workflows/ci.yml` from the v4 manifest (already moved to main in step 8).
11. **Update `.github/CODEOWNERS`** to map `v1/**`, `v2/**`, `v3/**`, `v4/**` to the right maintainers.
12. **Dry run** on a throwaway clone: run `build-branches.sh`, push to a fork, observe CI behavior on synthetic pushes to fake `v1`/`v2`/`v3`/`v4` branches.
13. **Open PR** `chore/repo-reorg → main`. PR body contains: this plan, the audit output, the manifests, the freeze-list of `feature/*` branches with author @-mentions.
14. **Cutover (one sitting):**
    a. Branch-protect main with admin-only push during cutover.
    b. Label all open `feature/*` PRs as `do-not-merge: pre-reorg` and post a comment with rebase guidance.
    c. `git checkout main && git merge --ff-only chore/repo-reorg`.
    d. Run `build-branches.sh` to materialize `v1`, `v2`, `v3`, `v4` locally.
    e. `git push origin main v1 v2 v3 v4` (main is fast-forward; `v*` are clean creations — no force needed since they don't exist remotely).
    f. `git push origin :research/v4` (delete the research branch).
    g. Configure GitHub branch protections: `v1` read-only, `v2`/`v3`/`v4` require PR + status checks from main's routers.
15. **Post-cutover verification** (§10).
16. **Delete `chore/repo-reorg`** once main and the four siblings verify green.

## 10. Verification (end-to-end)

On a fresh clone, post-cutover:

1. **Tree shape per branch:**
   ```bash
   for b in main v1 v2 v3 v4; do
     echo "=== $b ==="; git ls-tree --name-only origin/$b | sort
   done
   ```
   Expect: main has no `v*/` dirs; each `v*` has only its own `vN/` plus root README/CHANGELOG/LICENSE.
2. **No `.github/workflows/` on v\* branches:**
   ```bash
   for b in v1 v2 v3 v4; do
     git ls-tree -r origin/$b -- .github/workflows | grep . && echo "FAIL: $b has workflows"
   done
   ```
3. **No cross-pollution:**
   ```bash
   git checkout v2 && rg -l 'v3/|v4/' && echo "FAIL"
   git checkout v3 && rg -l 'v2/|v4/' && echo "FAIL"
   git checkout v4 && rg -l 'v2/|v3/' && echo "FAIL"
   ```
4. **No stray `* 2` / `* 3` paths anywhere:**
   ```bash
   for b in main v1 v2 v3 v4; do
     git ls-tree -r --name-only origin/$b | grep -E ' [0-9]+(/|$)' && echo "FAIL: $b"
   done
   ```
5. **Workflow trigger smoke test:**
   - Push a no-op commit to `v2` → only `ci-v2.yml`, `v2-*.yml`, and hygiene workflows run.
   - Push to `v3` → only v3-flavored workflows run.
   - Push to `v4` → only v4 workflows run.
   - Open PR `feature/foo → v3` → only v3 workflows run.
   - Push to `main` → only hygiene workflows run; no v\* workflows trigger.
6. **Build per branch:**
   - `v3`: `cd v3 && cargo build --workspace` succeeds.
   - `v4`: `cd v4 && cargo build --workspace` succeeds.
   - `v2`: `make -C v2 build` (or canonical equivalent) succeeds.
   - `v1`: no build; `markdownlint v1/CHANGELOG.md` succeeds.
7. **History preserved:** `git log --follow v3/Cargo.toml` on `v3` shows full history pre-reorg. Random older PR refs (`git show <older-sha>`) still resolve.
8. **Dependabot:** trigger via GitHub UI; PRs target the correct `v*`.
9. **Link checker** passes on main; router README's branch-qualified links to `v2/README.md@v2`, `v3/README.md@v3`, `v4/README.md@v4` resolve.
10. **GitNexus reindex:**
    - `v3`: `npx gitnexus analyze --embeddings` per CLAUDE.md.
    - `v4`: fresh `npx gitnexus analyze --embeddings`.
11. **Open issues / external bookmarks:** spot-check 5 historical PRs and 3 release-notes outbound links.

## 11. Critical files (created or modified)

| Path                                                 | Branch           | Action                                                     |
| ---------------------------------------------------- | ---------------- | ---------------------------------------------------------- |
| `scripts/reorg/audit.sh`                             | chore/repo-reorg | new                                                        |
| `scripts/reorg/manifest-{main,v1,v2,v3,v4}.txt`      | chore/repo-reorg | new                                                        |
| `scripts/reorg/build-branches.sh`                    | chore/repo-reorg | new                                                        |
| `docs/REPO_REORG_PLAN.md`                            | main             | new (this doc)                                             |
| `README.md`                                          | main             | rewrite (router)                                           |
| `CHANGELOG.md`                                       | main             | rewrite (umbrella)                                         |
| `SECURITY.md`                                        | main             | promote from `docs/SECURITY.md`                            |
| `CONTRIBUTING.md`                                    | main             | promote from `docs/CONTRIBUTING.md`                        |
| `.github/workflows/ci-v{1,2,3,4}.yml`                | main             | new (routers)                                              |
| `.github/workflows/release-v{2,3,4}.yml`             | main             | refactor / new for v4                                      |
| `.github/workflows/_ci-{rust,bash,docker,npm}.yml`   | main             | new (callables)                                            |
| `.github/workflows/_release-{cargo-dist,docker}.yml` | main             | new (callables)                                            |
| `.github/workflows/v{2,3,4}-*.yml`                   | main             | refactor branch filters to single-version                  |
| `.github/dependabot.yml`                             | main             | rewrite with `target-branch:`                              |
| `.github/CODEOWNERS`                                 | main             | update path globs                                          |
| `Makefile`                                           | main             | shrink to meta-makefile                                    |
| `Makefile`                                           | each v\*         | new per-branch from current root Makefile (split)          |
| `CLAUDE.md`, `AGENTS.md`                             | each branch      | per-version customization                                  |
| `docs/migration/MIGRATION_GUIDE.md`                  | (split)          | content moves to `v{2,3,4}/docs/migration/`                |
| `docs/ides/*.md`, `docs/FAQ.md`, `docs/faq/`         | (split/move)     | per §4.6                                                   |
| `examples/v{2,3}/**`                                 | (move)           | → `v{2,3}/examples/`                                       |
| `RELEASE_NOTES.v*.md`                                | (move)           | → `v*/RELEASE_NOTES.md`                                    |
| `packages/@pacphi/sindri-cli*`                       | v3 only          | stays at branch root on v3                                 |
| `docker/lib/`                                        | v2 only          | → `v2/docker/lib/`                                         |
| `docker-compose.yml`                                 | v3 only          | stays at v3 branch root                                    |
| `.gitnexus/`                                         | (none)           | **never committed**; added to `.gitignore` on every branch |
| All tracked `* 2` / `* 3` paths                      | (delete)         | `git rm -r` per §4.7                                       |
| `research/v4` branch                                 | (delete)         | post-promotion                                             |

## 12. Risks & mitigations

| Risk                                                                                     | Mitigation                                                                                                                                                                                      |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `actions/checkout@v4` defaults differ between event types                                | Pin `ref: ${{ github.event.pull_request.head.sha \|\| github.sha }}` in callables.                                                                                                              |
| Reusable callable receives wrong `workspace_dir` and silently builds nothing             | Fail-fast guard step: `test -f ${{ inputs.workspace_dir }}/Cargo.toml`.                                                                                                                         |
| v2 and v3 stacks diverge enough that `_ci-bash.yml`/`_ci-rust.yml` become awkward        | Per user note: don't over-DRY. If a callable needs >2 conditional branches per input, fork it (`_ci-rust-v3.yml`, `_ci-rust-v4.yml`). Reusable workflows are an optimization, not a constraint. |
| Promoting `research/v4` discards v4 work-in-progress commits                             | Confirm with user that `research/v4` HEAD is the desired snapshot; tag it before deletion (`git tag research-v4-final research/v4`).                                                            |
| External links to `docs/FAQ.md`, `docs/ides/*`                                           | Leave one-release stub `docs/FAQ.md` on main with a "moved" pointer; remove next release.                                                                                                       |
| FAQ site (`docs/faq/build.mjs`, `fly.toml`) bound to root paths                          | Audit and update relative paths before relocating to `v3/docs/faq/`.                                                                                                                            |
| `packages/@pacphi/sindri-cli*` published from a workspace root that now lives on v3 only | Confirm `package.json:workspaces` glob resolves on v3; bump only on a v3 patch release post-reorg.                                                                                              |
| Frozen `feature/*` branches confuse contributors                                         | Cutover PR explicitly lists each branch with rebase guidance; comment posted on each open PR before flipping main.                                                                              |
| `Makefile` (70KB) hard to split cleanly                                                  | First pass: copy full file to each `v*/Makefile` with `# FIXME: trim to vN-only targets` header. Track surgical split as post-cutover ticket.                                                   |
| Hidden v3 dependency on root `docker/lib/`                                               | `rg "docker/lib" v3/ v4/` before deleting; if found, copy into `v3/docker/lib/` first.                                                                                                          |
| `examples/v3 2/` may contain unique content                                              | Diff `examples/v3` vs `examples/v3 2` before deletion; if non-trivial deltas exist, merge into the canonical copy first.                                                                        |
| Branch protection misconfigured at cutover                                               | Document required status checks per branch in cutover runbook; verify in step 14g before unlocking main.                                                                                        |

## 13. Rollback

If the cutover causes problems within 24h:

1. `git checkout main && git reset --hard pre-reorg-2026-04-25 && git push --force-with-lease origin main`
2. `git push --delete origin v1 v2 v3 v4`
3. `git push origin <local research/v4 backup tag>:refs/heads/research/v4`
4. Restore branch protections.
5. Reopen the frozen `feature/*` PRs.
6. Post-mortem; iterate on `chore/repo-reorg`.

The safety tag and `research-v4-final` tag make this a 1-minute operation.
