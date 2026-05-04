#!/usr/bin/env bash
# sindri-helpers.sh — POSIX shell helper library for Sindri lifecycle
# scripts (ADR-030, v4/docs/script-contract.md).
#
# Usage in a phase script:
#
#   #!/usr/bin/env bash
#   set -Eeuo pipefail
#   . "$(dirname "$0")/../../../support/scripts/sindri-helpers.sh"
#   sindri::init
#
#   if sindri::tool_installed mytool && [ "$(mytool --version)" = "$SINDRI_COMPONENT_VERSION" ]; then
#       sindri::emit phase-complete '{"change":false}'
#       exit 0
#   fi
#
#   # …do the install…
#   sindri::log info "installed mytool $SINDRI_COMPONENT_VERSION"
#   sindri::emit phase-complete '{"change":true}'
#
# All public helpers are exposed under the `sindri::` namespace
# (Bash treats `::` as part of the function name; no shell-shell
# portability concerns since this file requires bash explicitly).

# Idempotency guard so a script that sources us twice doesn't run init twice.
if [ "${__SINDRI_HELPERS_LOADED:-0}" = "1" ]; then
    return 0
fi
__SINDRI_HELPERS_LOADED=1

# --- public ----------------------------------------------------------

# sindri::init — validates the contracted env, sets traps, opens
# the per-phase log file. Call this once at the top of every script.
sindri::init() {
    sindri::require_env \
        SINDRI_PHASE \
        SINDRI_COMPONENT_ADDRESS \
        SINDRI_COMPONENT_VERSION \
        SINDRI_TARGET \
        SINDRI_LOG_DIR \
        SINDRI_EVENTS

    # SINDRI_PRIOR_VERSION may be empty on fresh install; that's contractual.
    : "${SINDRI_PRIOR_VERSION:=}"
    : "${SINDRI_DRY_RUN:=0}"

    mkdir -p "$SINDRI_LOG_DIR" || true
    : > "$SINDRI_EVENTS" || true

    # Print where we are so the per-phase stdout log is self-describing.
    sindri::log info "phase=$SINDRI_PHASE component=$SINDRI_COMPONENT_ADDRESS version=$SINDRI_COMPONENT_VERSION prior=${SINDRI_PRIOR_VERSION:-<none>} dry_run=$SINDRI_DRY_RUN"
}

# sindri::log <level> <msg…> — structured stderr line.
# Levels: debug | info | warn | error.
sindri::log() {
    local level="$1"
    shift
    printf '[sindri %s] %s: %s\n' "${SINDRI_PHASE:-?}" "$level" "$*" >&2
}

# sindri::emit <event-name> [json-detail-object]
#
# Append one JSON-Lines record to $SINDRI_EVENTS. The first arg is
# the event name (the dispatcher recognizes "phase-complete"); the
# optional second arg is a JSON object whose top-level keys are
# merged into the event record.
#
# Examples:
#   sindri::emit info '{"detail":"started"}'
#   sindri::emit phase-complete '{"change":true}'
#   sindri::emit skip '{"reason":"already at version"}'
sindri::emit() {
    local name="$1"
    local detail="${2:-{\}}"
    if [ -z "${SINDRI_EVENTS:-}" ]; then
        return 0
    fi
    # shellcheck disable=SC2059
    printf '{"event":"%s",%s}\n' "$name" "$(__sindri_object_inner "$detail")" >> "$SINDRI_EVENTS"
}

# sindri::require_env VAR [VAR ...] — fail fast if any named env
# var is unset or empty (allow-listed exceptions: SINDRI_PRIOR_VERSION
# and SINDRI_DRY_RUN may be empty / unset).
sindri::require_env() {
    local missing=()
    local v
    for v in "$@"; do
        if [ -z "${!v:-}" ]; then
            missing+=("$v")
        fi
    done
    if [ "${#missing[@]}" -gt 0 ]; then
        printf 'sindri-helpers: missing required env vars: %s\n' "${missing[*]}" >&2
        exit 64  # EX_USAGE
    fi
}

# sindri::tool_installed <bin> — `command -v` shorthand. Returns
# 0 (true) if the binary is on PATH; 1 otherwise.
sindri::tool_installed() {
    command -v -- "$1" > /dev/null 2>&1
}

# --- internal --------------------------------------------------------

# Strip the leading and trailing braces of a JSON object literal so
# we can splice it into a parent object. Empty input -> empty output.
__sindri_object_inner() {
    local raw="$1"
    raw="${raw#"${raw%%[![:space:]]*}"}"  # ltrim
    raw="${raw%"${raw##*[![:space:]]}"}"  # rtrim
    raw="${raw#\{}"
    raw="${raw%\}}"
    raw="${raw#"${raw%%[![:space:]]*}"}"
    raw="${raw%"${raw##*[![:space:]]}"}"
    if [ -z "$raw" ]; then
        printf '"detail":null'
    else
        printf '%s' "$raw"
    fi
}
