#!/usr/bin/env bash
# sindri-helpers.sh — POSIX shell helper library for Sindri lifecycle
# scripts (ADR-030, v4/docs/script-contract.md).
#
# Sourced by every phase script via the dispatcher-injected env var:
#
#   #!/usr/bin/env bash
#   set -Eeuo pipefail
#   . "$SINDRI_HELPERS_SH"
#   sindri::init
#
# `$SINDRI_HELPERS_SH` is set by the dispatcher to an absolute path —
# no relative `..` traversal needed and the same script body works on
# any target the dispatcher reaches.
#
# All public helpers are exposed under the `sindri::` namespace plus a
# small set of un-prefixed shorthand aliases (`info`, `warn`, `error`,
# `die`, `has`, `require`) documented in `script-contract.md` as the
# "convenience layer." Both spellings are first-class.

# Idempotency guard — sourcing twice in one process is a no-op.
if [ "${__SINDRI_HELPERS_LOADED:-0}" = "1" ]; then
    return 0
fi
__SINDRI_HELPERS_LOADED=1

# --- public, namespaced ----------------------------------------------

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

    sindri::log info "phase=$SINDRI_PHASE component=$SINDRI_COMPONENT_ADDRESS version=$SINDRI_COMPONENT_VERSION prior=${SINDRI_PRIOR_VERSION:-<none>} dry_run=$SINDRI_DRY_RUN"
}

# sindri::log <level> <msg…> — structured stderr.
# Levels: debug | info | warn | error.
sindri::log() {
    local level="$1"
    shift
    printf '[sindri %s] %s: %s\n' "${SINDRI_PHASE:-?}" "$level" "$*" >&2
}

# sindri::emit <event-name> [json-detail-fragment]
# Append one JSON-Lines record to $SINDRI_EVENTS. The optional second
# arg is the *inner* fragment of a JSON object — keys are spliced into
# the parent record. Examples:
#   sindri::emit info '"detail":"started"'
#   sindri::emit phase-complete '"change":true'
#   sindri::emit skip '"reason":"already-installed"'
sindri::emit() {
    local name="$1"
    local frag="${2:-}"
    if [ -z "${SINDRI_EVENTS:-}" ]; then
        return 0
    fi
    if [ -n "$frag" ]; then
        printf '{"event":"%s",%s}\n' "$name" "$frag" >> "$SINDRI_EVENTS"
    else
        printf '{"event":"%s"}\n' "$name" >> "$SINDRI_EVENTS"
    fi
}

# sindri::require_env VAR [VAR ...] — fail fast if any named env
# var is unset or empty.
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

# sindri::tool_installed <bin> — `command -v` shorthand. Returns 0
# (true) if the binary is on PATH.
sindri::tool_installed() {
    command -v -- "$1" > /dev/null 2>&1
}

# sindri::version_of <bin> — best-effort version extraction from a
# binary's --version / version output. Prints the first semver-shaped
# token; empty string on failure.
sindri::version_of() {
    local bin="$1"
    "$bin" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?' | head -1 \
        || "$bin" version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?' | head -1 \
        || echo ""
}

# sindri::at_version <bin> [bin ...] — idempotency check.
#
# Returns 0 (true) if the first binary's version (per
# `sindri::version_of`) starts-with-or-contains $SINDRI_COMPONENT_VERSION
# *and* emits a `skip` + `phase-complete change:false` event so the
# dispatcher records the no-op cleanly. Returns 1 (false) when work
# is needed; the caller proceeds with the real install.
#
# Typical use:
#   sindri::at_version gcloud && exit 0
#   # …actual install…
#   sindri::emit phase-complete '"change":true'
sindri::at_version() {
    local bin="$1" have want="$SINDRI_COMPONENT_VERSION"
    have=$(sindri::version_of "$bin")
    if [ -n "$want" ] && { [ "${have#"$want"}" != "$have" ] || [ "${have%"$want"}" != "$have" ] || [ "${have#*"$want"}" != "$have" ]; }; then
        sindri::log info "$bin already at $want; skipping"
        sindri::emit skip '"reason":"already-installed"'
        sindri::emit phase-complete '"change":false'
        return 0
    fi
    if [ -n "$have" ]; then
        sindri::log info "$bin: $have → $want"
    fi
    return 1
}

# --- shorthand aliases (convenience layer) ----------------------------
#
# Documented as first-class in script-contract.md. Use whichever style
# reads better in your script — both surfaces are stable.

info()    { sindri::log info  "$*"; }
warn()    { sindri::log warn  "$*"; }
error()   { sindri::log error "$*"; }
die()     { sindri::log error "$*"; exit 1; }
has()     { sindri::tool_installed "$1"; }
require() { sindri::tool_installed "$1" || die "Required: $1 — install it before running this script."; }
