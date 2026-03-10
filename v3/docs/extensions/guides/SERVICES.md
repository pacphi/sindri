# Extension Service Framework

This guide explains how to add background daemon/service support to a Sindri extension.

## When to use `service:`

Use the `service:` block when your extension runs a **long-lived background process** (daemon) that should survive container restarts. Examples:

- Fleet management agents (draupnir)
- MCP servers that serve tools over stdio/HTTP
- Monitoring collectors or log forwarders

Do **not** use `service:` for one-shot tools (compilers, linters, CLI utilities). Those are handled by `install:` and `validate:` alone.

## Schema

Add a top-level `service:` block to your `extension.yaml`:

```yaml
service:
  enabled: true
  start:
    command: "my-daemon"
    args: ["--config", "~/.sindri/my-daemon.conf"]
    pidfile: ~/.sindri/my-daemon.pid
    logfile: ~/.sindri/logs/my-daemon.log
  stop:
    command: "my-daemon --shutdown" # optional, defaults to SIGTERM
    timeout: 10 # seconds before SIGKILL
  readiness:
    check: "curl -sf http://localhost:8080/health"
    timeout: 5
  requires-env:
    - MY_DAEMON_API_KEY
```

### Required fields

| Field           | Description                                           |
| --------------- | ----------------------------------------------------- |
| `start.command` | Command to launch the daemon (executed via `bash -c`) |

### Optional fields

| Field               | Default        | Description                                                   |
| ------------------- | -------------- | ------------------------------------------------------------- |
| `enabled`           | `true`         | Whether the service auto-starts on boot                       |
| `start.args`        | `[]`           | Arguments appended to the command                             |
| `start.pidfile`     | none           | PID file path (convention: `~/.sindri/<name>.pid`)            |
| `start.logfile`     | none           | Log file path (convention: `~/.sindri/logs/<name>.log`)       |
| `stop.command`      | SIGTERM to PID | Custom stop command                                           |
| `stop.timeout`      | `10`           | Seconds to wait before SIGKILL after SIGTERM                  |
| `readiness.check`   | none           | Command that exits 0 when service is ready                    |
| `readiness.timeout` | `5`            | Seconds to wait for readiness                                 |
| `requires-env`      | `[]`           | Environment variables required to start (skip if any missing) |

## Conventions

### PID files

Store PID files at `~/.sindri/<name>.pid`. The generated service script uses this to detect if the daemon is already running and to send signals on stop.

### Log files

Store logs at `~/.sindri/logs/<name>.log`. Use a single log file per service — log rotation is not managed by the framework.

### Idempotent start scripts

The framework generates `~/.sindri/services/<name>.sh` from your `service:` config. These scripts are idempotent: they check the PID file and skip startup if the process is already running.

## Lifecycle

### On install

When `sindri extension install` processes an extension with a `service:` block:

1. Normal installation completes (install method, hooks, configure)
2. The executor generates `~/.sindri/services/<name>.sh`
3. The generated script runs to start the daemon

### On container boot

The entrypoint calls `start_extension_services()` after extension installation:

1. Scans `~/.sindri/services/*.sh`
2. Runs each executable script as the developer user
3. Each script checks its PID file and starts only if needed

### On remove

When `sindri extension remove` processes the extension:

1. Sends stop command or SIGTERM to the PID
2. Waits for timeout, then SIGKILL if still running
3. Deletes `~/.sindri/services/<name>.sh`
4. Normal removal continues (paths, scripts, etc.)

## CLI commands

```bash
# List all registered services and their status
sindri extension services

# Manually start/stop/restart a service
sindri extension services start draupnir
sindri extension services stop draupnir
sindri extension services restart draupnir
```

## Example: MCP server extension

Here's a hypothetical MCP server extension that runs as a background service:

```yaml
metadata:
  name: my-mcp-server
  version: 1.0.0
  description: Custom MCP server for project-specific tools
  category: mcp

install:
  method: npm
  npm:
    package: "@myorg/mcp-server@1.0.0"

service:
  enabled: true
  start:
    command: "npx @myorg/mcp-server"
    args: ["--port", "3100"]
    pidfile: ~/.sindri/my-mcp-server.pid
    logfile: ~/.sindri/logs/my-mcp-server.log
  stop:
    timeout: 5
  readiness:
    check: "curl -sf http://localhost:3100/health"
    timeout: 10

validate:
  commands:
    - name: npx
      versionFlag: --version
```

## Testing locally

1. Install your extension: `sindri extension install my-extension`
2. Verify the service script was generated: `ls ~/.sindri/services/`
3. Check service status: `sindri extension services`
4. Test stop/start: `sindri extension services restart my-extension`
5. Simulate container restart: remove PID file, then run `bash ~/.sindri/services/my-extension.sh`
