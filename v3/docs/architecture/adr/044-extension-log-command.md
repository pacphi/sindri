# ADR-044: Extension Log Command

## Status

Accepted

## Context

Sindri v3 has no CLI-native log viewing capability. The only monitoring guidance in Docker environments is `tail -f ~/.sindri/logs/install.log`, which is Docker-only and doesn't work with the native Rust CLI. The structured event ledger at `~/.sindri/status_ledger.jsonl` (see ADR-043) already captures all extension lifecycle events (install, upgrade, remove, validation, outdated) with timestamps, but `sindri extension status` only shows current state snapshots -- not a chronological event log.

Users need a unified way to:

- View recent extension activity (tail mode)
- Monitor live installations (follow mode)
- Debug failures by filtering on severity or event type
- Audit extension history with time-range queries
- Integrate with scripts via machine-readable JSON output

## Decision

Add `sindri extension log` as a new CLI subcommand that provides user-friendly log viewing with filtering and real-time following, built on top of the existing `StatusLedger` infrastructure.

### Key design choices:

1. **Tail-by-default**: Shows last 25 events by default (configurable with `-n`), matching `journalctl` and `docker logs` behavior. Use `--no-tail` for full history.

2. **Follow mode** (`-f`): Polling-based at 1-second intervals using the existing file-based ledger. No new dependency on `notify` -- the polling approach is adequate for the expected event frequency.

3. **Multi-dimensional filtering**: Filter by extension name (`-e`), event type group (`-t install`), severity level (`-l error`), and time range (`--since`/`--until`). Filters compose naturally (intersection).

4. **Severity mapping** (derived, not stored): Events are classified as info/warn/error based on their type suffix:
   - Info: `*_started`, `*_completed`, `validation_succeeded`
   - Warn: `outdated_detected`
   - Error: `*_failed`, `validation_failed`

5. **Color-coded output**: Uses the existing `console` crate for colored terminal output with icons indicating event status.

6. **Query API**: `StatusLedger::query_events(EventFilter)` provides a reusable filtering API that can be consumed by both the CLI and future programmatic use cases.

### Code locations:

- `sindri-extensions/src/ledger.rs`: `EventFilter` struct, `query_events()` method, `DEFAULT_LOG_TAIL_LINES`, `DEFAULT_FOLLOW_POLL_SECS` constants
- `sindri/src/cli.rs`: `ExtensionLogArgs` struct, `Log` variant
- `sindri/src/commands/extension.rs`: `log()`, `show_logs()`, `follow_logs()`, `print_log_line()` and helper functions

## Consequences

### Positive

- **Unified monitoring**: Single command works across Docker, native CLI, and CI/CD environments
- **Debugging workflow**: `sindri extension log -l error` instantly shows failures with context
- **Audit trail**: Time-range queries support compliance and incident investigation
- **Script-friendly**: `--json` output enables integration with `jq`, monitoring tools, etc.
- **No new dependencies**: Built entirely on existing `StatusLedger`, `console`, `chrono`, and `tokio`

### Negative

- **Sequential scan**: `query_events()` performs a full sequential scan of the JSONL ledger. This is acceptable given the expected ledger size (hundreds to low thousands of events) and the existing compaction mechanism
- **Polling overhead**: Follow mode polls every second, adding minimal I/O. The poll interval is configurable via the `DEFAULT_FOLLOW_POLL_SECS` constant
- **No backward seek**: Follow mode only tracks events newer than the initial tail, not events concurrent with the initial read. This is consistent with `tail -f` behavior
