#!/usr/bin/env bash
# Run `npm audit --audit-level=high` against every committed package-lock.json in the v2 line,
# with per-lockfile exceptions from v2/audit-ignore.
#
# Fails (exit 1) when:
#   - any high or critical advisory remains that v2/audit-ignore doesn't list for that lockfile;
#   - npm audit returns an error body (e.g. registry timeout) instead of a report: fail closed;
#   - no lockfile is found, or an exception no longer matches anything (stale ignore).
#
# v2/audit-ignore format, one exception per line:
#   <GHSA-id> <lockfile directory, relative to the repo root>  # advisory, path, reason, exit condition
#
# Usage: .github/scripts/v2/npm-audit.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
IGNORE_FILE="${REPO_ROOT}/v2/audit-ignore"

command -v npm >/dev/null || { echo "npm not found" >&2; exit 2; }
command -v jq >/dev/null || { echo "jq not found" >&2; exit 2; }

# "<GHSA> <dir>" pairs from the ignore file
ignores=""
if [[ -f "${IGNORE_FILE}" ]]; then
  while IFS= read -r line || [[ -n "${line}" ]]; do
    line="${line%%#*}"
    read -r ghsa dir _ <<<"${line}" || true
    [[ -z "${ghsa:-}" ]] && continue
    if [[ -z "${dir:-}" ]]; then
      echo "::error::${IGNORE_FILE}: '${ghsa}' has no lockfile directory" >&2
      exit 2
    fi
    ignores+="${ghsa} ${dir}"$'\n'
  done <"${IGNORE_FILE}"
fi

cd "${REPO_ROOT}"
lockfiles=$(git ls-files -- '*package-lock.json' | grep -v '/node_modules/' || true)
if [[ -z "${lockfiles}" ]]; then
  echo "::error::no package-lock.json found; nothing was audited" >&2
  exit 1
fi

status=0
errored=0
used=""
while IFS= read -r lock; do
  dir=$(dirname "${lock}")
  echo "== npm audit --audit-level=high: ${dir}"
  report=$(cd "${dir}" && npm audit --json --package-lock-only 2>/dev/null || true)

  if ! jq -e 'type == "object" and (.error | not) and (.metadata.vulnerabilities | type == "object")' \
    >/dev/null 2>&1 <<<"${report}"; then
    echo "::error::npm audit returned no report for ${dir} (registry error?); failing closed" >&2
    jq -c '.error // .' <<<"${report}" 2>/dev/null | head -c 2000 >&2 || printf '%s\n' "${report}" | head -c 2000 >&2
    echo >&2
    status=1
    errored=1
    continue
  fi

  jq -r '.metadata.vulnerabilities | "   counts: critical=\(.critical) high=\(.high) moderate=\(.moderate) low=\(.low) total=\(.total)"' <<<"${report}"

  # Every advisory object (not "via another package" strings) at high or critical.
  findings=$(jq -r '
    [.vulnerabilities[] | .via[] | objects | select(.severity == "high" or .severity == "critical")
      | {ghsa: ((.url // "" | (capture("(?<id>GHSA-[0-9a-z-]+)") | .id)?) // "npm-advisory-\(.source)"),
         name, severity, title}]
    | unique_by(.ghsa) | .[] | "\(.ghsa)\t\(.name)\t\(.severity)\t\(.title)"' <<<"${report}")

  while IFS=$'\t' read -r ghsa name severity title; do
    [[ -z "${ghsa}" ]] && continue
    if grep -qxF "${ghsa} ${dir}" <<<"${ignores}"; then
      echo "   ignored  ${ghsa} ${name} (${severity}): ${title}"
      used+="${ghsa} ${dir}"$'\n'
    else
      echo "::error::${dir}: ${ghsa} ${name} (${severity}): ${title}"
      status=1
    fi
  done <<<"${findings}"
done <<<"${lockfiles}"

# An exception that matched nothing is stale: remove it from v2/audit-ignore.
# (Skipped when a report was missing, since nothing could match then.)
while IFS= read -r entry; do
  [[ -z "${entry}" || "${errored}" -eq 1 ]] && continue
  if ! grep -qxF "${entry}" <<<"${used}"; then
    echo "::error::stale exception in v2/audit-ignore (no longer reported): ${entry}" >&2
    status=1
  fi
done <<<"${ignores}"

if [[ "${status}" -eq 0 ]]; then
  echo "npm audit: no unignored high or critical advisories"
fi
exit "${status}"
