# Sindri — `v2` branch (Bash + Docker)

v2 is the Bash/Docker implementation. **Maintenance lane** — bug fixes, security
updates, and documented enhancements. New features generally land in v3 or v4.

## Stack

- Bash CLI under `v2/cli/`.
- Docker images built from `v2/Dockerfile` and `v2/docker/lib/`.
- Deployment adapters under `v2/deploy/`.
- Bats tests under `v2/test/`.

## Standards

- ShellCheck must pass on all `*.sh` files (config in `.shellcheckrc`).
- Use `set -euo pipefail` at the top of every script.
- Quote all variable expansions: `"$var"` not `$var`.
- Reference shared helpers in `v2/cli/lib/` rather than duplicating logic.

## CI

Workflows live on `main` (`.github/workflows/ci-v2.yml` + `v2-*.yml`) and trigger
on push/PR to `v2`. Locally:

```bash
cd v2 && make test           # bats + shellcheck
cd v2 && make build-image    # docker image
```

## Where docs live

`v2/docs/` only. Cross-version docs (FAQ, IDE guides, migration) have been split
per branch — this branch's copies live under `v2/docs/`.
