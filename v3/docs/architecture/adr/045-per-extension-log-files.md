# ADR-045: Per-Extension Log Files Linked from Ledger Events

## Status

Accepted

## Context

Sindri has two disconnected logging mechanisms:

1. **`~/.sindri/logs/install.log`** (Docker only) — raw stdout/stderr from all extensions, written by `entrypoint.sh` via `tee -a`. Contains full tool output but is unstructured, Docker-only, and not per-extension.

2. **`~/.sindri/status_ledger.jsonl`** (ADR-043) — structured JSONL events read by `sindri extension log` (ADR-044). Queryable and cross-platform, but captures only 1-line error messages on failure and zero tool output on success.

**The gap:** When a user runs `sindri extension log` and sees an install failure, they can't drill into the detailed mise/script output. The executor (`executor.rs`) already captures stdout/stderr in `Arc<Mutex<Vec<String>>>` buffers during installation, but these were dropped after each `install_*` method returned. We needed to preserve this output in per-extension log files and link them from ledger events.

## Decision

### InstallOutput type

The executor now returns an `InstallOutput` struct alongside its `Result<()>` from every `install()` call:

```rust
pub struct InstallOutput {
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
    pub install_method: String,
    pub exit_status: String,
}
```

The `install()` method signature changed from `Result<()>` to `(InstallOutput, Result<()>)`, ensuring output is always available regardless of success or failure.

### Per-extension log files

`ExtensionLogWriter` (`log_files.rs`) writes per-extension log files at:

```
~/.sindri/logs/<extension-name>/<YYYYMMDDTHHMMSSz>.log
```

Each log file has a metadata header followed by captured stdout/stderr:

```
# Extension: python
# Timestamp: 2026-02-13T14:30:22Z
# Method: mise
# Status: success
# --- stdout ---
<stdout lines>
# --- stderr ---
<stderr lines>
```

### Event linking

Six event variants gained an optional `log_file: Option<String>` field:

- `InstallCompleted`, `InstallFailed`
- `UpgradeCompleted`, `UpgradeFailed`
- `RemoveCompleted`, `RemoveFailed`

The field uses `#[serde(skip_serializing_if = "Option::is_none", default)]` for backward compatibility — old ledger entries without `log_file` deserialize as `None`.

### Detail command

`sindri extension log --detail <event_id>` displays the event summary and the linked log file content. If the log file has been cleaned up, a warning is shown.

### Log cleanup

Per-extension log files are cleaned up during ledger compaction (`compact()`) using the same retention period (default 90 days). Cleanup is best-effort — failures are logged but don't fail the compaction.

### Docker install.log

The Docker-only `~/.sindri/logs/install.log` is retained for aggregate container bootstrap logging. The new per-extension logs provide a cross-platform, per-extension alternative.

## Consequences

### Positive

- **Bridged gap**: Structured events now link to detailed tool output
- **Cross-platform**: Per-extension logs work on Docker, native CLI, and CI/CD
- **Debugging workflow**: `sindri extension log -l error --json | jq -r '.event_id'` followed by `sindri extension log --detail <id>` gives full context
- **Backward compatible**: The `log_file` field is optional with serde defaults
- **Bounded growth**: Log cleanup during compaction prevents unbounded disk usage

### Negative

- **Disk usage**: Each installation produces a log file (typically 1-50KB). Mitigated by compaction cleanup
- **IO overhead**: Writing log files adds minor IO per installation
- **Coupling**: The executor return type changed from `Result<()>` to `(InstallOutput, Result<()>)`, requiring updates to all callers
