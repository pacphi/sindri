# Extension Log Command

**Status:** Implemented

## Overview

`sindri extension log` provides CLI-native log viewing for extension lifecycle events, replacing Docker-specific `tail -f ~/.sindri/logs/install.log` guidance with a cross-platform, filterable, real-time log viewer.

## Use Cases

1. **Quick check**: See recent extension activity after login or deploy
2. **Live monitoring**: Watch installation progress in real-time during `sindri extension install`
3. **Failure diagnosis**: Filter to error-level events to debug failed installations
4. **Audit trail**: Query events within a date range for specific extensions
5. **Automation**: Pipe JSON output to monitoring tools or scripts

## User Journeys

### New user checking what happened

```bash
sindri extension log           # Shows last 25 events
```

### Monitoring a running installation

```bash
sindri extension log -f        # Follow mode, Ctrl+C to stop
```

### Debugging a failed installation

```bash
sindri extension log -e python -l error
sindri extension log -t install -l error
```

### Querying historical events

```bash
sindri extension log --since 2026-02-01 --until 2026-02-10
sindri extension log -e kubectl --no-tail
```

### Machine-readable output

```bash
sindri extension log --json | jq '.event.type'
sindri extension log --json -l error | jq -r '.extension_name'
```

## Architecture

- **Query layer**: `StatusLedger::query_events(EventFilter)` in `sindri-extensions/src/ledger.rs`
- **CLI layer**: `ExtensionLogArgs` in `sindri/src/cli.rs`, handler in `sindri/src/commands/extension.rs`
- **Constants**: `DEFAULT_LOG_TAIL_LINES` (25), `DEFAULT_FOLLOW_POLL_SECS` (1) in ledger.rs
- **ADR**: ADR-044

## Verification

1. `cargo build` -- zero errors
2. `cargo clippy --all-targets` -- zero warnings
3. `cargo test -p sindri-extensions` -- unit tests pass
4. `cargo test -p sindri --test cli_extension_log_tests` -- integration tests pass
5. Manual smoke test with generated events
