# Sindri Console Agent Extension

The `console-agent` extension installs and runs the `sindri-agent` binary on each Sindri
instance, connecting it to the [Sindri Console](../../../console/) for centralized orchestration,
real-time monitoring, and web terminal access.

## Overview

The agent provides:

- **Heartbeat**: Periodic ping to the Console to track instance liveness
- **Metrics**: System metrics collection (CPU, memory, disk, network) at configurable intervals
- **Web Terminal**: PTY-based terminal sessions streamed to the Console via WebSocket
- **Auto-registration**: Registers the instance with the Console on startup

## Installation

Install the extension via the Sindri CLI:

```bash
sindri extension install console-agent
```

Or declare it in your `sindri.yaml`:

```yaml
extensions:
  - name: console-agent
    env:
      SINDRI_CONSOLE_URL: "https://console.example.com"
      SINDRI_CONSOLE_API_KEY: "your-api-key-here"
```

## Configuration

The agent is configured via environment variables. Set them in your `sindri.yaml`:

| Variable                 | Required | Default | Description                                 |
| ------------------------ | -------- | ------- | ------------------------------------------- |
| `SINDRI_CONSOLE_URL`     | Yes      | `""`    | URL of the Sindri Console API server        |
| `SINDRI_CONSOLE_API_KEY` | Yes      | `""`    | API key for authenticating with the Console |
| `SINDRI_AGENT_HEARTBEAT` | No       | `30`    | Heartbeat interval in seconds               |
| `SINDRI_AGENT_METRICS`   | No       | `60`    | Metrics collection interval in seconds      |

The configuration is written to `~/.config/sindri-agent/config.yaml` by `configure-agent.sh`.

### Manual Configuration

Re-run the configuration script after changing environment variables:

```bash
SINDRI_CONSOLE_URL="https://console.example.com" \
SINDRI_CONSOLE_API_KEY="your-key" \
bash configure-agent.sh
```

## Scripts

| Script               | Purpose                                                                     |
| -------------------- | --------------------------------------------------------------------------- |
| `install.sh`         | Downloads the `sindri-agent` binary from GitHub Releases                    |
| `configure-agent.sh` | Reads env vars / `sindri.yaml`, writes `~/.config/sindri-agent/config.yaml` |
| `start-agent.sh`     | Starts the agent as a systemd user service or background process            |
| `healthcheck.sh`     | Verifies the agent is running and healthy                                   |

## Running

The agent starts automatically after installation. To start, stop, or restart manually:

```bash
# Start
bash start-agent.sh

# Stop (systemd)
systemctl --user stop sindri-agent

# Stop (background process)
kill $(cat /tmp/sindri-agent.pid)

# Restart
bash start-agent.sh
```

## Auto-Start on Boot

The extension enables auto-start automatically:

- **With systemd**: Enables a systemd user service (`sindri-agent.service`) with `WantedBy=default.target`
- **Without systemd**: Adds a startup check to `~/.bashrc`

## Health Check

```bash
bash healthcheck.sh
```

This verifies:

- Binary is installed at `~/.local/bin/sindri-agent`
- Process is running (via systemd status or PID file)
- Config file exists at `~/.config/sindri-agent/config.yaml`
- Console URL is configured

## Logs

```bash
# Background process logs
cat /tmp/sindri-agent.log
tail -f /tmp/sindri-agent.log

# systemd logs
journalctl --user -u sindri-agent -f
```

## Troubleshooting

**Agent won't start**

- Verify binary: `ls -la ~/.local/bin/sindri-agent`
- Check logs: `cat /tmp/sindri-agent.log`
- Re-install: `sindri extension install console-agent`

**Agent can't connect to Console**

- Verify `SINDRI_CONSOLE_URL` is correct and reachable
- Check `SINDRI_CONSOLE_API_KEY` is valid
- Re-run: `bash configure-agent.sh && bash start-agent.sh`

**Agent crashes repeatedly**

- Review logs for error messages
- Verify network connectivity to the Console URL
- Check system resources (disk, memory)

## BOM Tracking

This extension is tracked in the Software Bill of Materials (SBOM):

```yaml
bom:
  tools:
    - name: sindri-agent
      source: github-release
      type: cli-tool
      license: MIT
      homepage: https://github.com/pacphi/sindri
```

## Architecture

```
Sindri Instance
└── sindri-agent
    ├── Heartbeat (every 30s) ──────────────────────► Sindri Console API
    ├── Metrics (every 60s) ────────────────────────► Sindri Console API
    └── WebSocket ──────────────────────────────────► Sindri Console WS
        └── PTY sessions (web terminal)
```

The agent binary is built from `v3/console/agent/` and released as part of the Sindri
project's GitHub Releases under the `console-agent-v*` tag prefix.
