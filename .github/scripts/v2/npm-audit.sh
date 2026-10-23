#!/usr/bin/env bash
# Run `npm audit` on every package-lock.json under v2/, failing on advisories at or above
# AUDIT_LEVEL (default: high), with exceptions from v2/npm-audit-ignore.
#
# The ignore file lists one GHSA id per line; anything after '#' is a comment. Packages that
# are vulnerable only through an ignored advisory (npm reports them with a string "via") are
# covered by that ignore.
#
# Fails closed: if npm returns no usable report for a lockfile (registry error, timeout,
# unparsable output), that lockfile counts as failed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
IGNORE_FILE="$REPO_ROOT/v2/npm-audit-ignore"
AUDIT_LEVEL="${AUDIT_LEVEL:-high}"

case "$AUDIT_LEVEL" in
  low | moderate | high | critical) ;;
  *)
    echo "::error::AUDIT_LEVEL must be low, moderate, high or critical (got '$AUDIT_LEVEL')"
    exit 2
    ;;
esac

IGNORED_IDS=()
if [[ -f "$IGNORE_FILE" ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%%#*}"
    line="$(echo "$line" | xargs)"
    [[ -z "$line" ]] && continue
    IGNORED_IDS+=("$line")
  done < "$IGNORE_FILE"
fi

if [[ ${#IGNORED_IDS[@]} -gt 0 ]]; then
  IGNORE_JSON="$(printf '%s\n' "${IGNORED_IDS[@]}" | jq -R . | jq -cs .)"
  echo "Ignoring ${#IGNORED_IDS[@]} advisory exception(s) from ${IGNORE_FILE#"$REPO_ROOT"/}:"
  printf '  - %s\n' "${IGNORED_IDS[@]}"
else
  IGNORE_JSON='[]'
fi

cd "$REPO_ROOT"

LOCKFILES=()
lock_list="$(git ls-files -- ':(glob)v2/**/package-lock.json')"
while IFS= read -r lock; do
  [[ -n "$lock" ]] && LOCKFILES+=("$lock")
done <<< "$lock_list"

if [[ ${#LOCKFILES[@]} -eq 0 ]]; then
  echo "::error::No package-lock.json found under v2/"
  exit 1
fi

failed=0
incomplete=0
seen_ignored='[]'

for lock in "${LOCKFILES[@]}"; do
  dir="$(dirname "$lock")"
  echo ""
  echo "=== npm audit --audit-level=$AUDIT_LEVEL: $lock"

  # npm audit exits non-zero whenever it finds anything, so judge by the JSON report.
  report="$(cd "$dir" && npm audit --json --package-lock-only 2>/dev/null || true)"

  if ! jq -e 'type == "object" and (has("error") | not) and has("vulnerabilities") and has("metadata")' \
    > /dev/null 2>&1 <<< "$report"; then
    echo "::error file=$lock::npm audit returned no usable report (registry error or bad output)"
    jq -r '.error // empty | "  \(.code // "") \(.summary // "")"' 2> /dev/null <<< "$report" || true
    failed=1
    incomplete=1
    continue
  fi

  jq -r '.metadata.vulnerabilities | "  totals: critical=\(.critical) high=\(.high) moderate=\(.moderate) low=\(.low)"' <<< "$report"

  # Advisory-level view: one row per (package, advisory) at or above the level.
  rows="$(jq -r --arg level "$AUDIT_LEVEL" --argjson ignore "$IGNORE_JSON" '
    def rank: {"info": 0, "low": 1, "moderate": 2, "high": 3, "critical": 4}[.] // 0;
    [ .vulnerabilities[] | .via[] | objects
      | select((.severity | rank) >= ($level | rank))
      | (.url | split("/") | last) as $id
      | { id: $id, name, severity, range, title,
          ignored: ($ignore | index($id) != null) } ]
    | unique_by([.name, .id]) | .[]
    | [ (if .ignored then "IGNORED" else "FAIL" end), .id, .severity, .name, .range, .title ]
    | @tsv' <<< "$report")"

  if [[ -z "$rows" ]]; then
    echo "  ok: no advisories at or above $AUDIT_LEVEL"
    continue
  fi

  while IFS=$'\t' read -r status id severity name range title; do
    echo "  $status $id ($severity) $name $range: $title"
    if [[ "$status" == "FAIL" ]]; then
      echo "::error file=$lock::$id ($severity) $name $range: $title"
      failed=1
    else
      seen_ignored="$(jq -c --arg id "$id" '. + [$id] | unique' <<< "$seen_ignored")"
    fi
  done <<< "$rows"
done

# Flag exceptions that no longer match anything, so they get removed (only when every
# lockfile produced a report; otherwise a missing match proves nothing).
if [[ ${#IGNORED_IDS[@]} -gt 0 && $incomplete -eq 0 ]]; then
  for id in "${IGNORED_IDS[@]}"; do
    if ! jq -e --arg id "$id" 'index($id) != null' > /dev/null <<< "$seen_ignored"; then
      echo "::warning file=v2/npm-audit-ignore::$id no longer matches any finding; remove it"
    fi
  done
fi

echo ""
if [[ $failed -ne 0 ]]; then
  echo "npm audit gate FAILED"
  exit 1
fi
echo "npm audit gate passed (${#LOCKFILES[@]} lockfile(s))"
