# ADR-043: Event-Driven Extension Status Architecture

## Status

Accepted

## Context

The `sindri extension status` command had critical performance and accuracy issues:

**Performance:** Status queries took 17+ seconds for 27 extensions. Each extension triggered synchronous subprocess calls (mise, npm, dpkg, which, validation commands), resulting in ~135+ sequential shell commands.

**Accuracy:** Inconsistent states were shown (e.g., "installing" + "failed" simultaneously). The manifest state could differ from actual installation state. Verification only happened during status queries, not in real-time.

**Architecture:** The manifest file (`~/.sindri/manifest.yaml`) tracked extension state. The status command loaded the manifest, then called `verify_extension_installed()` for each installed extension. Lifecycle operations (install/remove/upgrade) updated the manifest synchronously.

## Decision

We replaced the manifest-based status system with an **event-driven architecture** using an append-only event ledger.

### Core Components

1. **Event Types** (`events.rs`): 12 lifecycle event types covering install, upgrade, remove, validation, and outdated detection. Each event is wrapped in an `EventEnvelope` with metadata (UUID, timestamp, CLI version, state transitions).

2. **Status Ledger** (`ledger.rs`): JSON Lines (`.jsonl`) append-only storage at `~/.sindri/status_ledger.jsonl`. Uses `fs4` crate for file-level advisory locking to ensure concurrent write safety. Supports aggregate queries (latest status per extension), history queries (events for specific extension), and time-range queries.

3. **Event Publishing**: Extension lifecycle operations (install, upgrade, remove) publish events to the ledger at operation start and completion/failure. Events capture duration, error messages, and state transitions.

4. **Status Queries**: The `status` command reads the latest event per extension from the ledger instead of running subprocess verification. The `--verify` flag enables optional filesystem verification for users who want deeper checks.

### Breaking Change

This is a breaking change for v3.0.0:

- `manifest.yaml` has been completely removed
- Users upgrading from v2.x must reinstall their extensions
- The `ManifestManager` module has been deleted from the codebase
- All commands now use `StatusLedger` exclusively

### New Commands

- `sindri extension verify [name]` - Explicit verification of installed extensions with event publishing
- `sindri ledger compact [--retention-days 90]` - Prune old events from the ledger
- `sindri ledger export <path>` - Export ledger to JSON file for auditing
- `sindri ledger stats` - Show event counts, file size, and timestamp ranges

### CLI Changes

- `sindri extension status` - Now reads from ledger (<1s vs 17+ seconds)
- `sindri extension status --verify` - Optional filesystem verification
- `sindri extension status --limit N` - Limit history entries
- `sindri extension status --since <ISO8601>` - Filter events by date

## Consequences

### Positive

- **17x performance improvement**: Status queries complete in <1s vs 17+ seconds
- **Accurate state tracking**: Ledger is single source of truth, updated in real-time
- **Complete audit trail**: Every lifecycle operation logged with timestamps, durations, and error context
- **Better debugging**: Full error messages and event history for all extensions
- **Extensibility**: Event foundation enables future monitoring, alerting, and analytics
- **Simpler codebase**: No dual-write, no backward compatibility layers

### Negative

- **Breaking change**: Users must reinstall extensions when upgrading to v3.0.0
- **No migration path**: Clean slate approach means losing previous installation state
- **Ledger growth**: File grows over time (mitigated by compaction with 90-day retention)
- **Sequential scan**: Status queries scan the full ledger file (acceptable for <10K events)

### Risks Mitigated

- **File corruption**: Advisory file locking via `fs4` crate, `fsync()` for durability
- **Performance at scale**: Compaction keeps ledger bounded; SQLite migration path available if >10K events
- **User data loss**: Documented in CHANGELOG and release notes; clear reinstallation workflow provided
