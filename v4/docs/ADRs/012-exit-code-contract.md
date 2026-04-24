# ADR-012: Standardized Exit-Code Contract

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 has no documented exit-code contract. CI pipelines must resort to string-matching
stderr to understand failure categories. This is fragile and prevents clean toolchain
integration.

## Decision

All v4 CLI verbs produce the following exit codes, never changing within a major version:

| Code | Name                  | Meaning                                                                   |
| ---- | --------------------- | ------------------------------------------------------------------------- |
| 0    | `SUCCESS`             | Operation completed successfully                                          |
| 1    | `ERROR`               | Generic error (I/O, network, unexpected panic)                            |
| 2    | `POLICY_DENIED`       | One or more components denied by install policy                           |
| 3    | `RESOLUTION_CONFLICT` | Dependency closure has an unresolvable conflict                           |
| 4    | `SCHEMA_ERROR`        | `sindri.yaml` or `sindri.policy.yaml` failed schema/constraint validation |
| 5    | `STALE_LOCKFILE`      | `sindri.lock` is absent or does not match `sindri.yaml`                   |

### CI usage

```bash
sindri validate; case $? in
  0) echo "OK" ;;
  4) echo "schema error — fix sindri.yaml" ;;
  *) echo "unexpected error" ;;
esac

sindri resolve --strict
if [ $? -eq 2 ]; then
  echo "❌ Policy denial — SBOM blocked" && exit 1
fi
```

Consumer CI should fail-fast on codes 2/3/4/5.
Maintainer registry-publish CI maps the same codes.

### Structured machine output

Every verb that produces output supports `--json` for machine-readable structured
results (objects, not line-delimited raw text). The exit code is independent of
`--json` — both are always set.

## References

- Research: `09-imperative-ux.md` §7, `11-command-comparison.md` §7
