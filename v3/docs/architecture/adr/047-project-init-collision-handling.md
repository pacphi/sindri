# ADR-047: Project-Init Collision Handling Runtime

## Status

Accepted

## Context

Extensions like ruflo and agentic-qe declare comprehensive `collision-handling` configs in their `extension.yaml` files — priority ordering, version markers, collision scenarios, and per-file conflict rules. However, `initialize_project_tools()` in `enhance.rs` iterated a HashMap of installed extensions with no priority sorting, no version detection, no scenario evaluation, and no conflict-rule application. Extensions ran in arbitrary order and silently overwrote each other's files during `sindri project new/clone`.

The type system (ADR-008), configure processing (ADR-032), and dependency resolution (ADR-009) were all complete. The missing piece was the runtime orchestration layer that reads collision-handling declarations and enforces them during project-init.

## Decision

### New `collision` Module

We added a `collision` module to `sindri-extensions` (mirroring the `configure/` module pattern) with four sub-modules:

1. **`ordering.rs`** — Priority-based sorting (ascending, alphabetical tiebreak)
2. **`detection.rs`** — Version marker detection (FileExists, DirectoryExists, ContentMatch with match_any/exclude_if)
3. **`scenarios.rs`** — Scenario evaluation (Stop/Skip/Proceed/Backup/Prompt, first-match-wins)
4. **`conflict.rs`** — Conflict rule application (Skip, Overwrite, Append, MergeJson, MergeYaml, Merge, Backup, Prompt→Skip fallback)

### Orchestration Flow

The `CollisionResolver` orchestrates project-init in this order:

1. Sort extensions by priority (ascending), alphabetical tiebreak
2. For each extension:
   a. Detect version markers in workspace
   b. Evaluate scenarios against detected versions → Proceed/Skip/Stop
   c. Execute project-init commands (with auth checking)
   d. Apply conflict rules to workspace files
3. Write per-extension collision logs via `ExtensionLogWriter`

### `enhance.rs` Refactoring

Replaced the HashMap iteration in `initialize_project_tools()` with `CollisionResolver::new(workspace, NonInteractive).resolve_and_execute(entries, auth_checker)`. The function now:

- Collects `ProjectInitEntry` structs with priority from each extension
- Delegates to `CollisionResolver` for ordering and execution
- Logs results (executed/skipped/stopped/failed) per extension

### Merge Function Reuse

Changed `merge_json_values`, `merge_yaml_values` in `configure/templates.rs` from `fn` to `pub(crate) fn` and re-exported via `configure/mod.rs` with `pub(crate) use`. This enables the `conflict.rs` module to reuse proven merge logic without duplication.

### Per-Extension Collision Logging

Added `write_collision_log()` to `ExtensionLogWriter` (ADR-045). Collision logs use the same directory structure (`~/.sindri/logs/<name>/<timestamp>.log`) with `# Phase: project-init-collision` header.

### `SINDRI_LOG_DIR` Environment Variable

Injected `SINDRI_LOG_DIR=~/.sindri/logs/<extension-name>` into all script and hook command executions in `executor.rs`. Updated 5 extension scripts to use `${SINDRI_LOG_DIR:-/tmp}` instead of hardcoded `/tmp/` paths. This is backward-compatible — scripts still work standalone.

## Key Design Decisions

| Decision                    | Choice                             | Rationale                                                                  |
| --------------------------- | ---------------------------------- | -------------------------------------------------------------------------- |
| Module location             | `sindri-extensions/src/collision/` | Same crate as configure module it reuses                                   |
| Conflict rules applied WHEN | AFTER project-init commands        | Commands produce files first, then rules resolve conflicts                 |
| Scenarios evaluated WHEN    | BEFORE commands run                | Decide whether to execute at all                                           |
| Prompt fallback             | Skip (NonInteractive)              | Safe default; no data loss                                                 |
| Ordering                    | Priority ASC, then alphabetical    | Deterministic across runs                                                  |
| Merge function reuse        | `pub(crate)` from templates.rs     | DRY; proven implementations                                                |
| Sync vs async               | Sync (std::fs)                     | Called from sync `initialize_project_tools()`; merge logic is already sync |
| Script logging              | `SINDRI_LOG_DIR` env var           | Standard location; backward-compatible `/tmp` fallback                     |

## Consequences

### Positive

- Extensions run in deterministic priority order (ruflo at 20 before agentic-qe at 50)
- Version detection prevents silent overwrites and dangerous upgrades
- Scenario evaluation allows extensions to stop/skip/proceed based on workspace state
- Conflict rules resolve file collisions declaratively
- Per-extension collision logs provide full auditability
- Scripts log to `~/.sindri/logs/` instead of ephemeral `/tmp/` locations

### Negative

- Slightly more complex project-init path (additional module + log writes)
- Extensions without collision-handling configs still work but don't benefit from ordering guarantees relative to collision-aware extensions

## References

- ADR-008: Extension type system
- ADR-009: Dependency resolution
- ADR-023: Project management
- ADR-032: Configure processing
- ADR-045: Per-extension log files
