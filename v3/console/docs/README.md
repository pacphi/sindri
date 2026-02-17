# Sindri Console

The Sindri Console is a web-based management interface for monitoring and controlling Sindri environment instances deployed across multiple cloud providers.

## Overview

The Console provides:

**Phase 1 - Core Platform**
- **Instance Dashboard** - Real-time status, metrics, and health of all Sindri instances
- **Web Terminal** - Browser-based PTY sessions into running instances via xterm.js
- **Agent System** - Lightweight Go agent deployed alongside each instance, reporting heartbeats and metrics
- **Event Log** - Full audit trail of lifecycle events (deploy, backup, destroy, etc.)
- **RBAC** - Role-based access control with Admin, Operator, Developer, and Viewer roles

**Phase 2 - Orchestration**
- **Deployment Wizard** - Multi-step deployment flow with YAML editor and template gallery
- **Instance Lifecycle** - Suspend, resume, destroy, backup, clone, and bulk operations
- **Command Execution** - Dispatch commands to single or multiple instances in parallel
- **Scheduled Tasks** - Cron-based task scheduling with execution history and notifications
- **Command Palette** - Quick keyboard-driven navigation and instance management
- **Multi-Terminal** - Multiple simultaneous PTY sessions with broadcast mode

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Browser Client                        │
│   React 19 + TanStack Router + TanStack Query + xterm.js    │
│                     (port 5173 in dev)                       │
└─────────────────────┬───────────────────────────────────────┘
                      │  HTTP /api/v1/*
                      │  WebSocket /ws/console
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                      Console API Server                      │
│              Hono + Node.js + Prisma + Redis                 │
│                       (port 3000)                            │
└──────────┬──────────────────────────────────┬───────────────┘
           │  PostgreSQL                       │  WebSocket /ws/agent
           ▼                                  ▼
┌──────────────────┐               ┌──────────────────────────┐
│   PostgreSQL DB  │               │  Sindri Instance + Agent  │
│   (port 5432)    │               │     Go binary, PTY        │
└──────────────────┘               └──────────────────────────┘
```

### Components

| Component | Language | Location |
|-----------|----------|----------|
| Web frontend | TypeScript / React 19 | `apps/web/` |
| API server | TypeScript / Hono | `apps/api/` |
| Instance agent | Go 1.25 | `agent/` |
| Database schema | Prisma / PostgreSQL | `apps/api/prisma/` |

### WebSocket Channels

The agent-to-Console communication uses an Envelope-based protocol:

| Channel | Direction | Purpose |
|---------|-----------|---------|
| `heartbeat` | Agent → Console | Liveness ping every 30s |
| `metrics` | Agent → Console | System metrics every 60s |
| `logs` | Agent → Console | Streaming log lines |
| `terminal` | Bidirectional | PTY session multiplexing |
| `events` | Agent → Console | Instance lifecycle events |
| `commands` | Console → Agent | On-demand command execution |

## Quick Start

### Prerequisites

- Node.js 20+
- Go 1.25+
- Docker (for local PostgreSQL and Redis)
- `pnpm` or `npm`

### 1. Start Infrastructure

```bash
docker run -d --name sindri-postgres \
  -e POSTGRES_DB=sindri_console \
  -e POSTGRES_USER=sindri \
  -e POSTGRES_PASSWORD=sindri \
  -p 5432:5432 postgres:16

docker run -d --name sindri-redis \
  -p 6379:6379 redis:7
```

### 2. Configure the API

```bash
cd apps/api
cp .env.example .env
# Edit .env with your database URL and API key
```

Required environment variables:

```env
DATABASE_URL=postgresql://sindri:sindri@localhost:5432/sindri_console
REDIS_URL=redis://localhost:6379
CONSOLE_API_KEY=your-secret-key-here
JWT_SECRET=your-jwt-secret-here
PORT=3000
```

### 3. Run Database Migrations

```bash
cd apps/api
npm install
npm run db:migrate
```

### 4. Start the API Server

```bash
cd apps/api
npm run dev
```

### 5. Start the Web Frontend

```bash
cd apps/web
npm install
npm run dev
```

Open http://localhost:5173 in your browser.

### 6. Install and Run an Agent

On the Sindri instance machine:

```bash
curl -sSL https://console.example.com/install-agent.sh | sh
# Or build from source:
cd agent && go build -o sindri-agent ./cmd/agent
```

Configure and run:

```bash
export SINDRI_CONSOLE_URL=http://localhost:3000
export SINDRI_CONSOLE_API_KEY=your-secret-key-here
export SINDRI_PROVIDER=docker
./sindri-agent
```

## Repository Structure

```
v3/console/
├── agent/                    # Go instance agent
│   ├── cmd/agent/            # Main entrypoint
│   ├── internal/
│   │   ├── config/           # Environment-based configuration
│   │   ├── heartbeat/        # Heartbeat loop
│   │   ├── metrics/          # System metrics collection
│   │   ├── registration/     # Instance registration with Console
│   │   ├── terminal/         # PTY session management
│   │   └── websocket/        # WebSocket connection to Console
│   └── pkg/protocol/         # Shared protocol types
├── apps/
│   ├── api/                  # Hono API server
│   │   ├── prisma/           # Schema and migrations
│   │   ├── src/
│   │   │   ├── agents/       # Agent WebSocket handler
│   │   │   ├── middleware/   # Auth, logging, error handling
│   │   │   ├── models/       # Prisma model helpers
│   │   │   ├── routes/       # REST API routes
│   │   │   ├── services/     # Business logic
│   │   │   ├── websocket/    # Channel definitions and gateway
│   │   │   └── workers/      # Background jobs
│   │   └── tests/            # Integration tests
│   └── web/                  # React frontend
│       ├── src/
│       │   ├── api/          # API client
│       │   ├── components/   # UI components
│       │   ├── hooks/        # Custom React hooks
│       │   ├── lib/          # Utilities
│       │   ├── routes/       # TanStack Router routes
│       │   ├── stores/       # Zustand state stores
│       │   └── types/        # TypeScript types
│       └── tests/            # Frontend tests
└── docs/                     # Documentation
```

---

## Agent Installation and Configuration

The Sindri Agent is a lightweight Go binary deployed alongside each Sindri environment instance.

### Required Environment Variables

| Variable | Description |
|----------|-------------|
| `SINDRI_CONSOLE_URL` | Base URL of the Console API (e.g. `https://console.example.com`) |
| `SINDRI_CONSOLE_API_KEY` | Shared secret for authenticating with the Console |

### Optional Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SINDRI_INSTANCE_ID` | System hostname | Unique identifier for this instance |
| `SINDRI_PROVIDER` | _(empty)_ | Deployment provider (`fly`, `docker`, `devpod`, `e2b`, `kubernetes`) |
| `SINDRI_REGION` | _(empty)_ | Geographic region (e.g. `sea`, `iad`, `us-east-1`) |
| `SINDRI_AGENT_HEARTBEAT` | `30` | Heartbeat interval in seconds |
| `SINDRI_AGENT_METRICS` | `60` | Metrics collection interval in seconds |
| `SINDRI_AGENT_SHELL` | `/bin/bash` | Default shell for PTY sessions |
| `SINDRI_AGENT_TAGS` | _(empty)_ | Comma-separated `key=value` labels attached to registration |
| `SINDRI_LOG_LEVEL` | `info` | Log verbosity: `debug`, `info`, `warn`, `error` |

### Building the Agent from Source

```bash
cd v3/console/agent
go build -o sindri-agent ./cmd/agent
```

### Running the Agent

```bash
export SINDRI_CONSOLE_URL=https://console.example.com
export SINDRI_CONSOLE_API_KEY=your-secret-key
export SINDRI_PROVIDER=fly
export SINDRI_REGION=sea
./sindri-agent
```

The agent will:
1. Register itself via `POST {CONSOLE_URL}/api/v1/instances`
2. Open a persistent WebSocket to `{CONSOLE_URL}/ws/agent` (using `wss://` for `https://` URLs)
3. Send heartbeat pings every 30 seconds
4. Send system metrics every 60 seconds
5. Spawn PTY processes on demand for terminal sessions

### systemd Service

```ini
[Unit]
Description=Sindri Console Agent
After=network.target

[Service]
EnvironmentFile=/etc/sindri/agent.env
ExecStart=/usr/local/bin/sindri-agent
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

---

## REST API Reference

Base URL: `http://localhost:3000/api/v1` (development)

All endpoints require authentication via `X-API-Key` header or `Authorization: Bearer <token>`.

### Instances

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/instances` | List all instances (supports `?status=`, `?provider=`, `?page=`, `?per_page=`) |
| `POST` | `/instances` | Register a new instance (called by agent on startup) |
| `GET` | `/instances/:id` | Get instance details |
| `PATCH` | `/instances/:id` | Update instance status or metadata |
| `DELETE` | `/instances/:id` | Delete an instance |
| `GET` | `/instances/:id/heartbeats` | List heartbeat history |
| `GET` | `/instances/:id/events` | List instance events |

### Instance Lifecycle (Phase 2)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/instances/:id/suspend` | Suspend a RUNNING instance (OPERATOR+) |
| `POST` | `/instances/:id/resume` | Resume a SUSPENDED instance (OPERATOR+) |
| `DELETE` | `/instances/:id` | Destroy an instance with optional volume backup (OPERATOR+) |
| `POST` | `/instances/:id/backup` | Initiate a volume backup (OPERATOR+) |
| `POST` | `/instances/bulk-action` | Bulk suspend/resume/destroy (up to 50 instances, OPERATOR+) |
| `GET` | `/instances/:id/lifecycle` | Get available lifecycle actions for current status |

### Deployments (Phase 2)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/deployments` | Create a new deployment (all authenticated users) |
| `GET` | `/deployments/:id` | Get deployment status and logs |

Supported providers: `fly`, `docker`, `devpod`, `e2b`, `kubernetes`, `runpod`, `northflank`

### Commands (Phase 2)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/commands` | Dispatch a command to one instance (DEVELOPER+) |
| `POST` | `/commands/bulk` | Dispatch a command to multiple instances in parallel (OPERATOR+) |
| `POST` | `/commands/script` | Execute a script on one or more instances (DEVELOPER+) |
| `GET` | `/commands/history` | List recent command executions |
| `GET` | `/commands/:id` | Get a specific execution record |

### Scheduled Tasks (Phase 2)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/tasks` | List tasks (supports `?status=`, `?instanceId=`, pagination) |
| `POST` | `/tasks` | Create a scheduled task |
| `GET` | `/tasks/templates` | List built-in task templates |
| `GET` | `/tasks/:id` | Get task details |
| `PUT` | `/tasks/:id` | Update a task |
| `DELETE` | `/tasks/:id` | Delete a task |
| `POST` | `/tasks/:id/pause` | Pause a task |
| `POST` | `/tasks/:id/resume` | Resume a paused task |
| `POST` | `/tasks/:id/trigger` | Trigger a task manually |
| `GET` | `/tasks/:id/history` | List execution history for a task |

### Registration Payload (POST /instances)

```json
{
  "name": "my-instance",
  "provider": "fly",
  "region": "sea",
  "extensions": ["python3", "nodejs"],
  "config_hash": "sha256:abc123...",
  "ssh_endpoint": "ssh.example.com:22",
  "agent_version": "0.1.0"
}
```

Response `201 Created`:

```json
{
  "id": "clxyz123...",
  "name": "my-instance",
  "provider": "fly",
  "region": "sea",
  "status": "RUNNING",
  "created_at": "2024-01-01T00:00:00.000Z",
  "updated_at": "2024-01-01T00:00:00.000Z"
}
```

### Authentication Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/auth/login` | Login with email/password, returns JWT |
| `POST` | `/auth/register` | Create a new user (Admin only) |
| `GET` | `/api-keys` | List API keys for current user |
| `POST` | `/api-keys` | Create a new API key |
| `DELETE` | `/api-keys/:id` | Revoke an API key |

### Terminal Sessions

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/terminal-sessions` | List active sessions |
| `GET` | `/terminal-sessions/:id` | Get session details |

### WebSocket Endpoints

| Endpoint | Users | Description |
|----------|-------|-------------|
| `/ws/agent` | Agents | Agent connection (auth via `X-API-Key` header) |
| `/ws/console` | Browser clients | Real-time dashboard updates |
| `/ws/terminal/:instanceId` | Browser clients | PTY session relay |

---

## Database Schema

The Console uses PostgreSQL via Prisma. Key models:

### Instance

Represents a deployed Sindri environment instance.

| Field | Type | Description |
|-------|------|-------------|
| `id` | String (CUID) | Primary key |
| `name` | String (unique) | Human-readable instance name |
| `provider` | String | Deployment provider |
| `region` | String? | Geographic region |
| `extensions` | String[] | Installed extension names |
| `config_hash` | String? | SHA256 of `sindri.yaml` at deploy time |
| `ssh_endpoint` | String? | SSH connection string |
| `status` | Enum | `RUNNING`, `STOPPED`, `DEPLOYING`, `DESTROYING`, `ERROR`, `UNKNOWN` |

### Heartbeat

Periodic health/metrics ping from a running agent.

| Field | Type | Description |
|-------|------|-------------|
| `id` | String (CUID) | Primary key |
| `instance_id` | String | FK → Instance |
| `cpu_percent` | Float | 0–100 |
| `memory_used` | BigInt | Bytes |
| `memory_total` | BigInt | Bytes |
| `disk_used` | BigInt | Bytes |
| `disk_total` | BigInt | Bytes |
| `uptime` | BigInt | Seconds |

### User and ApiKey

| Model | Key Fields |
|-------|-----------|
| `User` | `email` (unique), `role` (ADMIN/OPERATOR/DEVELOPER/VIEWER) |
| `ApiKey` | `key_hash` (SHA256 of raw key), `expires_at` (null = non-expiring) |

---

## Development Guide

### Project Setup

```bash
# Clone and navigate to the console directory
cd v3/console

# Install API dependencies
cd apps/api && npm install

# Install web dependencies
cd ../web && npm install

# Build the Go agent
cd ../../agent && go build ./...
```

### Development Workflow

```bash
# Terminal 1: Start API server with hot reload
cd apps/api && npm run dev

# Terminal 2: Start web frontend with hot reload
cd apps/web && npm run dev

# Terminal 3: Run agent against local API
SINDRI_CONSOLE_URL=http://localhost:3000 \
SINDRI_CONSOLE_API_KEY=dev-secret \
./agent/sindri-agent
```

### Code Style

- **TypeScript**: Strict mode enabled. No `any` types.
- **Go**: Standard `gofmt` formatting. Errors wrapped with context.
- **Imports**: Absolute imports from `@/` alias in the web frontend.
- **Tests**: Unit tests co-located with source; integration tests in `tests/` directories.

### Adding a New API Route (Hono)

```typescript
// apps/api/src/routes/examples.ts
import { Hono } from 'hono'
import { z } from 'zod'
import { zValidator } from '@hono/zod-validator'

const app = new Hono()

const bodySchema = z.object({ name: z.string().min(1) })

app.post('/', zValidator('json', bodySchema), async (c) => {
  const { name } = c.req.valid('json')
  return c.json({ name }, 201)
})

export { app as examplesRoutes }
```

### Adding a New WebSocket Channel

Channel constants are defined in `apps/api/src/websocket/channels.ts`. Add to the `CHANNEL` and `MESSAGE_TYPE` objects, then add corresponding payload interfaces.

---

## Testing Guide

### Test Stack

| Layer | Tool |
|-------|------|
| API unit/integration tests | Vitest |
| Frontend utility tests | Vitest |
| Frontend E2E tests | Playwright (planned) |
| Go agent tests | `go test` |

### Running Tests

```bash
# API tests (unit + integration)
cd apps/api
npm test                    # Run all tests
npm run test:watch          # Watch mode
npm run test:coverage       # With coverage report

# Specific test file
npm test -- tests/websocket-channels.test.ts

# Frontend tests
cd apps/web
npm test

# Go agent tests
cd agent
go test ./...
go test -v ./internal/config/...
```

### Test Categories

**Unit tests** (no server required):
- `apps/api/tests/websocket-channels.test.ts` — Channel protocol helpers
- `apps/web/tests/utils.test.ts` — Frontend utility functions

**Integration tests** (require running server + database):
- `apps/api/tests/agent-registration.test.ts` — Registration flow
- `apps/api/tests/heartbeat-metrics.test.ts` — Heartbeat and metrics pipeline
- `apps/api/tests/terminal-session.test.ts` — PTY session lifecycle
- `apps/api/tests/auth-middleware.test.ts` — Authentication and RBAC
- `apps/api/tests/database-operations.test.ts` — Prisma model operations

**Real-time tests** (require running server + agent):
- `apps/web/tests/instance-realtime.test.ts` — WebSocket broadcast verification

### Integration Test Setup

```bash
# Start test database
docker run -d --name sindri-test-db \
  -e POSTGRES_DB=sindri_test \
  -e POSTGRES_USER=sindri \
  -e POSTGRES_PASSWORD=sindri \
  -p 5433:5432 postgres:16

# Set test environment
export TEST_DATABASE_URL=postgresql://sindri:sindri@localhost:5433/sindri_test
export TEST_ADMIN_API_KEY=test-admin-api-key
export TEST_OPERATOR_API_KEY=test-operator-api-key
export TEST_VIEWER_API_KEY=test-viewer-api-key

# Run migrations on test database
DATABASE_URL=$TEST_DATABASE_URL npm run db:migrate

# Start test server
NODE_ENV=test npm run dev &

# Run integration tests
npm test
```

### Coverage Requirements

| Metric | Target |
|--------|--------|
| Statements | ≥ 80% |
| Branches | ≥ 75% |
| Functions | ≥ 80% |
| Lines | ≥ 80% |

---

## Production Deployment

### Environment Variables (Production)

```env
# API Server
DATABASE_URL=postgresql://user:pass@db.example.com:5432/sindri_console
REDIS_URL=redis://redis.example.com:6379
CONSOLE_API_KEY=sk_prod_...
JWT_SECRET=...
PORT=3000
NODE_ENV=production
LOG_LEVEL=info

# For the agent on each instance
SINDRI_CONSOLE_URL=https://console.example.com
SINDRI_CONSOLE_API_KEY=sk_prod_...
```

### Docker Compose (Development/Staging)

```yaml
version: "3.9"
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_DB: sindri_console
      POSTGRES_USER: sindri
      POSTGRES_PASSWORD: sindri
    ports: ["5432:5432"]

  redis:
    image: redis:7
    ports: ["6379:6379"]

  api:
    build: ./apps/api
    environment:
      DATABASE_URL: postgresql://sindri:sindri@postgres:5432/sindri_console
      REDIS_URL: redis://redis:6379
      CONSOLE_API_KEY: dev-secret
      JWT_SECRET: dev-jwt-secret
    ports: ["3000:3000"]
    depends_on: [postgres, redis]

  web:
    build: ./apps/web
    ports: ["5173:80"]
    depends_on: [api]
```

### Health Check Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | API server health |
| `GET /health/db` | Database connectivity |
| `GET /health/redis` | Redis connectivity |

### Migrations in Production

Always run migrations before deploying new code:

```bash
DATABASE_URL=... npx prisma migrate deploy
```

---

## Phase 2: Advanced Instance Management

Phase 2 extends the console with deployment automation, lifecycle management, and multi-instance operations.

### Phase 2 Feature Overview

| Feature | Description |
|---------|-------------|
| Deployment Wizard | Template-based multi-step guided instance deployment |
| YAML Editor | In-browser `sindri.yaml` editing with live validation |
| Instance Lifecycle | Clone, suspend, resume, and destroy operations |
| Multi-Instance Commands | Parallel command dispatch and output streaming |
| Scheduled Tasks | Cron-based task scheduling with execution history |
| Command Palette | `Cmd+K` quick navigation, instance switching, action search |
| Multi-Terminal | Multiple PTY sessions per instance with tab management |

---

## Deployment Wizard

The Deployment Wizard provides a guided 5-step flow for launching new instances from templates.

### Steps

1. **Template Selection** — Browse the template library and select a base environment. Templates are organized by category (data science, web development, systems, etc.) and filtered by provider compatibility.

2. **Configure** — Edit the generated `sindri.yaml` in an in-browser editor. The editor validates YAML syntax and warns about missing required fields.

3. **Provider** — Select the deployment provider (`fly`, `docker`, `devpod`, `e2b`, `kubernetes`) and configure provider-specific options (region, namespace, resource limits).

4. **Review** — Set the instance name and review the full configuration before deployment.

5. **Deploy** — Submit the configuration. The console registers the instance (status: `DEPLOYING`) and waits for the agent to connect and report `RUNNING`.

### Template Format

Templates are `sindri.yaml` documents with metadata:

```yaml
name: python-data-science
extensions:
  - python3
  - jupyter
  - pandas
  - scikit-learn
resources:
  cpu: 2
  memory: 4096
```

Template metadata (stored in the database):

```json
{
  "id": "tmpl_python_01",
  "name": "Python Data Science",
  "description": "Jupyter + pandas + scikit-learn environment",
  "category": "data-science",
  "extensions": ["python3", "jupyter", "pandas", "scikit-learn"],
  "providers": ["fly", "docker", "e2b"],
  "version": "1.0.0"
}
```

### Naming Rules

Instance names must match `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$` (lowercase alphanumeric and hyphens, no leading/trailing hyphens).

---

## Instance Lifecycle Management

All lifecycle operations are available from the instance detail page and the instance list actions menu.

### Clone

Creates a copy of an existing instance with the same configuration.

- The clone name defaults to `{source-name}-clone` (adjustable before submit).
- Clone starts in `DEPLOYING` state; SSH endpoint is `null` until provisioned.
- Cannot clone instances in `DESTROYING` or `UNKNOWN` state.

**API:** `POST /api/v1/instances` with source config fields copied.

### Suspend

Gracefully stops a running instance without destroying it.

- Publishes a stop signal to `sindri:instance:{id}:lifecycle` Redis channel.
- Agent receives signal, drains active connections, and stops processes.
- Status transitions: `RUNNING` → `STOPPED`.

**Requires:** `OPERATOR` role.

### Resume

Restarts a previously suspended instance.

- Publishes a start signal to `sindri:instance:{id}:lifecycle` Redis channel.
- Status transitions: `STOPPED` → `RUNNING`.

**Requires:** `OPERATOR` role.

### Destroy

Permanently deregisters an instance. Does not destroy cloud infrastructure — use provider-specific tooling for that.

- Requires confirmation dialog where the user types the instance name.
- Removes instance and all associated heartbeats and events from the database.
- Removes Redis keys for the instance.
- Status transitions: any → `DESTROYING` → deleted.

**Requires:** `ADMIN` or `OPERATOR` role.

### State Transition Table

| From | Allowed Transitions |
|------|---------------------|
| `RUNNING` | `STOPPED`, `DESTROYING`, `ERROR` |
| `STOPPED` | `RUNNING`, `DESTROYING` |
| `DEPLOYING` | `RUNNING`, `ERROR` |
| `DESTROYING` | `UNKNOWN` |
| `ERROR` | `RUNNING`, `STOPPED`, `DESTROYING` |

---

## Multi-Instance Command Execution

Dispatch shell commands to one or more running instances simultaneously and stream the output back to the browser.

### Usage

1. Click the **Run Command** button (or use `Cmd+Shift+R`).
2. Select target instances from the list (filtered to `RUNNING` only).
3. Enter the command string.
4. Optionally set environment variables and timeout (default: 30s, max: 3600s).
5. Click **Execute** or press `Enter`.

The output panel shows a separate output stream per instance, labeled with the instance name. Partial failures (some instances fail, others succeed) are displayed individually.

### WebSocket Protocol

Commands flow through the `commands` WebSocket channel:

**Dispatch (Console → Agent):**
```json
{
  "channel": "commands",
  "type": "exec",
  "payload": {
    "commandId": "cmd_abc123",
    "command": "node --version",
    "timeout": 30,
    "env": {}
  }
}
```

**Output (Agent → Console, streaming):**
```json
{
  "channel": "commands",
  "type": "output",
  "payload": {
    "commandId": "cmd_abc123",
    "data": "v20.10.0\n",
    "stream": "stdout"
  }
}
```

**Completion (Agent → Console):**
```json
{
  "channel": "commands",
  "type": "complete",
  "payload": {
    "commandId": "cmd_abc123",
    "exitCode": 0
  }
}
```

### Cancellation

An in-flight command can be cancelled by commandId. The agent receives a `cancel` message and terminates the subprocess. Cancelled commands report `exitCode: -1` with `status: "cancelled"`.

---

## Scheduled Tasks

Schedule recurring commands to run across one or more instances using cron expressions.

### Cron Expression Format

Standard 5-field cron: `minute hour day-of-month month day-of-week`

| Expression | Meaning |
|------------|---------|
| `* * * * *` | Every minute |
| `0 * * * *` | Every hour on the hour |
| `0 2 * * *` | 2:00 AM every day |
| `0 0 * * 0` | Midnight every Sunday |
| `*/15 * * * *` | Every 15 minutes |
| `0 8-18 * * 1-5` | Every hour, 8am–6pm, weekdays |

### Task Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Human-readable task name |
| `command` | string | Shell command to run |
| `cronExpression` | string | Standard 5-field cron |
| `instanceIds` | string[] | Target instance IDs |
| `enabled` | boolean | Toggle without deleting |
| `timezone` | string | IANA timezone (default: UTC) |

### Execution History

Each task run creates one execution record per target instance with:

- `status`: `pending` | `running` | `success` | `failure` | `timeout`
- `startedAt` / `completedAt`: ISO 8601 timestamps
- `exitCode`: shell exit code (124 = timeout, -1 = cancelled)
- `output`: captured stdout/stderr (truncated to 64KB)

The task list shows success rate and last run time. Click the history icon to view per-execution logs.

---

## Command Palette

Access the command palette with **`Cmd+K`** (Mac) or **`Ctrl+K`** (Windows/Linux).

### Search Syntax

| Prefix | Scope |
|--------|-------|
| _(none)_ | All items (instances + navigation + actions) |
| `@` | Instances only |
| `>` | Actions and navigation only |

### Keyboard Navigation

| Key | Action |
|-----|--------|
| `Cmd+K` / `Ctrl+K` | Open palette |
| `Escape` | Close palette |
| `↑` / `↓` | Navigate items |
| `Enter` | Execute selected item |
| `Backspace` (on empty query) | Clear selection |

### Item Types

- **Instance** — Navigate to instance detail or terminal
- **Navigation** — Jump to a page (Dashboard, Instances, Settings)
- **Action** — Trigger an operation (Deploy new instance, Import YAML)

Recent items are shown when the query is empty (up to 5, newest first).

---

## Multi-Instance Terminal Multiplexing

Open multiple terminal sessions for the same instance in parallel, each as a separate tab.

### Tab Management

- **New tab:** Click `+` button in the terminal tab bar, or press `Cmd+T`.
- **Close tab:** Click the `×` on the tab, or press `Cmd+W`.
- **Switch tab:** Click the tab, or press `Cmd+[1-9]` for numbered tabs.

### Broadcast Mode

Toggle **Broadcast** to send the same keystrokes to all open sessions simultaneously. Useful for running the same command across multiple shells.

- Broadcast is opt-out per session via the session settings menu.
- Disconnected sessions are skipped during broadcast.

### Session Persistence

The last active tab ID per instance is persisted to `localStorage` so the correct tab is restored on page reload. Live session state (WebSocket connections) is not persisted and re-connects on navigation.

### Terminal Protocol

Sessions use the `terminal` WebSocket channel. All input/output data is base64-encoded:

```json
{ "channel": "terminal", "type": "create", "payload": { "sessionId": "...", "cols": 220, "rows": 50, "shell": "/bin/bash" } }
{ "channel": "terminal", "type": "input",  "payload": { "sessionId": "...", "data": "<base64>" } }
{ "channel": "terminal", "type": "output", "payload": { "sessionId": "...", "data": "<base64>" } }
{ "channel": "terminal", "type": "resize", "payload": { "sessionId": "...", "cols": 180, "rows": 40 } }
{ "channel": "terminal", "type": "close",  "payload": { "sessionId": "..." } }
```

---

## Keyboard Shortcuts Reference

### Global

| Shortcut | Action |
|----------|--------|
| `Cmd+K` / `Ctrl+K` | Open command palette |
| `Cmd+/` / `Ctrl+/` | Focus instance search |
| `Cmd+Shift+R` | Open run command panel |

### Deployment Wizard

| Shortcut | Action |
|----------|--------|
| `Escape` | Close wizard |
| `Cmd+Enter` | Advance to next step |
| `Cmd+[` | Go to previous step |

### Terminal

| Shortcut | Action |
|----------|--------|
| `Cmd+T` | New terminal tab |
| `Cmd+W` | Close current tab |
| `Cmd+1`–`Cmd+9` | Switch to tab N |
| `Cmd+Shift+B` | Toggle broadcast mode |
| `Cmd+K` | Clear terminal (in terminal focus) |

### Command Palette

| Shortcut | Action |
|----------|--------|
| `↑` / `↓` | Navigate results |
| `Enter` | Execute selected item |
| `Escape` | Close palette |

### Instance List

| Shortcut | Action |
|----------|--------|
| `j` / `k` | Navigate instances (vim-style) |
| `Enter` | Open selected instance |
| `n` | Deploy new instance |

---

## Phase 2 Testing Guide

### Integration Tests (Vitest)

Tests in `apps/api/tests/` require no running server — the Hono app is exercised directly via `app.request()` with mocked database and Redis.

```bash
cd apps/api
npm test                        # All tests
npm test -- tests/deployment-wizard.test.ts
npm test -- tests/instance-lifecycle.test.ts
npm test -- tests/command-execution.test.ts
npm test -- tests/scheduled-tasks.test.ts
npm test -- tests/command-palette.test.ts
npm test -- tests/multi-terminal.test.ts
```

### E2E Tests (Playwright)

E2E tests in `apps/web/tests/e2e/` require a running stack:

```bash
# Start the full stack
docker compose up -d
cd apps/api && npm run dev &
cd apps/web && npm run dev &

# Run E2E tests
cd apps/web
npx playwright test tests/e2e/deployment-wizard.spec.ts
npx playwright test tests/e2e/instance-lifecycle.spec.ts
npx playwright test tests/e2e/parallel-commands.spec.ts
npx playwright test tests/e2e/scheduled-tasks.spec.ts

# Run all E2E tests
npx playwright test tests/e2e/
```

E2E tests use `TEST_BASE_URL` environment variable (default: `http://localhost:5173`).

### Phase 2 Coverage Targets

| Layer | Target |
|-------|--------|
| API integration (Vitest) | ≥ 80% statements |
| Frontend unit (Vitest) | ≥ 75% statements |
| E2E (Playwright) | Critical user paths covered |
