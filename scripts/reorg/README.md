# Sindri Repo-Reorg Tooling

Scripts in this directory are used **once** to materialize the four sibling
maintenance branches (`v1`, `v2`, `v3`, `v4`) and the slimmed-down `main`
described in `docs/REPO_REORG_PLAN.md`.

After the cutover is complete and verified, this entire directory is removed
in a follow-up commit on `main`.

## Files

| File                | Purpose                                                                                                                                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `audit.sh`          | Inventory: walks the working tree and classifies every tracked top-level path as `main`, `v1`, `v2`, `v3`, `v4`, or `purge`. Output is committed to `scripts/reorg/AUDIT.txt` for the cutover PR description. |
| `manifest-main.txt` | Allow-list of paths that survive on `main` after cutover.                                                                                                                                                     |
| `manifest-v1.txt`   | Allow-list of paths that survive on the `v1` branch.                                                                                                                                                          |
| `manifest-v2.txt`   | Allow-list of paths that survive on the `v2` branch.                                                                                                                                                          |
| `manifest-v3.txt`   | Allow-list of paths that survive on the `v3` branch.                                                                                                                                                          |
| `manifest-v4.txt`   | Allow-list of paths that survive on the `v4` branch (sourced from `research/v4`).                                                                                                                             |
| `build-branches.sh` | Idempotent branch materializer. Reads the manifests, creates `v1`/`v2`/`v3`/`v4` from `chore/repo-reorg`, and reshapes `main`. **Does not push.**                                                             |
| `verify.sh`         | Post-cutover verification (§10 of the plan). Run on a fresh clone after the four branches are pushed.                                                                                                         |

## Manifest format

Plain text, one path per line, blank lines and `#` comments allowed. Globs are
expanded via `git ls-files`. A path listed in `manifest-vN.txt` is _kept_ on
that branch; everything else is `git rm -r`'d in the isolating commit.

## Order of operations

```
1. ./scripts/reorg/audit.sh                  # produces AUDIT.txt
2. git add scripts/reorg/AUDIT.txt && git commit
3. ./scripts/reorg/build-branches.sh         # creates v1, v2, v3, v4 + reshapes main locally
4. (manual review) git log --oneline v1 v2 v3 v4 main
5. (with explicit user approval) git push origin v1 v2 v3 v4 main
6. ./scripts/reorg/verify.sh                 # against the pushed remote
```

Step 5 is destructive to the shared remote and **must not be automated**.

## Follow-ups deferred to post-cutover

These items were intentionally not done during the initial reorg PR; they
require human judgment and are tracked as separate work items:

1. **Split cross-version docs.** `docs/FAQ.md`, `docs/ides/*.md`, and
   `docs/migration/MIGRATION_GUIDE.md` currently mix v2 and v3 content. They
   stay on `main` initially. After cutover, split them per branch:
   - v2 portions → `v2/docs/FAQ.md`, `v2/docs/ides/`, `v2/docs/migration/`.
   - v3 portions → `v3/docs/FAQ.md`, `v3/docs/ides/`, `v3/docs/migration/`.
   - Generate v4 stubs from v3 content.
   - Replace the root `docs/FAQ.md` with a stub that points to per-branch copies.
2. **Move `docs/faq/` site builder** (currently at `main:docs/faq/`) into
   `v3/docs/faq/` once verified v3-only.
3. **Split `Makefile`** (70KB) into a thin meta-makefile on `main` plus
   per-version Makefiles. First-pass copy already lives at `vN/Makefile`.
4. **Merge or refactor `ci-v3.yml`** (1330 lines) into the reusable callables
   `_ci-rust.yml` + `_ci-npm.yml`. Current state retains the original logic
   verbatim with only triggers retargeted.
5. **Trim `_release-cargo-dist.yml`** to align with whatever cargo-dist
   configuration v3 settles on. Currently a generic skeleton.
6. **Drop `scripts/reorg/`** from `main` entirely once cutover is verified
   (this whole directory is single-use tooling).
