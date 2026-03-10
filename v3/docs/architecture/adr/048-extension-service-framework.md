# ADR-048: Extension Service Framework

## Status

Accepted

## Context

Extensions like draupnir need to run as background daemons. Currently, the `post-install` hook can start a daemon on first install, but on container restart the entrypoint skips installation (the `bootstrap-complete` marker exists), so daemons are never restarted. The existing `daemon_autostart` flag in `CoreFeatures` was unused — there was no runtime machinery to act on it.

We need a generic service registry that any extension can use to declare, start, stop, and monitor background processes — not a draupnir-specific hack.

## Decision

### New `service:` top-level block in extension.yaml

Extensions that run background daemons declare a `service:` block in their `extension.yaml`:

```yaml
service:
  enabled: true
  start:
    command: "draupnir"
    pidfile: ~/.sindri/draupnir.pid
    logfile: ~/.sindri/logs/draupnir.log
  stop:
    timeout: 10
  readiness:
    check: "kill -0 $(cat ~/.sindri/draupnir.pid 2>/dev/null) 2>/dev/null"
    timeout: 5
  requires-env:
    - SINDRI_CONSOLE_URL
```

### Service registry directory

On `sindri extension install`, if the extension has a `service:` block, the executor generates an idempotent start script at `~/.sindri/services/<name>.sh`. On `sindri extension remove`, the executor stops the service and deletes the script.

### Entrypoint integration

A new `start_extension_services()` function in `entrypoint.sh` runs on every container boot (including restarts). It scans `~/.sindri/services/*.sh` and executes each script as the developer user. This runs after extension installation completes (or immediately on restart when `bootstrap-complete` already exists).

### CLI subcommand

`sindri extension services` provides manual lifecycle control:

- `sindri extension services` — list registered services and status
- `sindri extension services start <name>` — start a specific service
- `sindri extension services stop <name>` — stop a specific service
- `sindri extension services restart <name>` — stop + start

## Consequences

- **Restart resilience**: Daemons auto-restart on container reboot without reinstallation.
- **Generic framework**: Any extension can declare service support — not just draupnir.
- **Idempotent**: Start scripts check PID files and skip if already running.
- **Graceful shutdown**: Stop commands or SIGTERM with configurable timeout before SIGKILL.
- **Conditional startup**: `requires-env` allows skipping services when required configuration is absent.
- **Backwards compatible**: Existing `post-install` hooks continue to work. The `service:` block is optional.
- **Observable**: `sindri extension services` gives operators visibility into running daemons.
