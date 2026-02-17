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

**Phase 3 - Observability**
- **Metrics Pipeline** - Full-fidelity time-series collection (CPU, memory, disk, network, load avg) stored in TimescaleDB hypertable with configurable granularity downsampling
- **Fleet Dashboard** - Fleet-wide health summary, resource utilization rollup, top-N consumers, stale instance detection, and real-time status updates
- **Instance Dashboard** - Per-instance charts with selectable time ranges (1h–30d), auto-refresh, event timeline overlay, and sparklines
- **Log Aggregation** - Structured log ingestion from agents, full-text search, level/source/time-range filtering, cursor pagination, and real-time streaming
- **Alerting Engine** - Rule-based alert evaluation with AND/OR conditions, pending window, cooldown suppression, email/webhook notifications, and FIRING/RESOLVED state machine

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

## Phase 3: Observability

### Metrics Pipeline

The metrics pipeline ingests, stores, and queries full-fidelity time-series data from all running agents.

#### Ingest

Agents send `metrics:update` WebSocket messages every 30 seconds containing:

| Field | Type | Description |
|-------|------|-------------|
| `cpuPercent` | float 0–100 | Overall CPU utilization |
| `loadAvg1/5/15` | float | 1, 5, 15-minute load averages |
| `cpuSteal` | float | Hypervisor steal percentage |
| `coreCount` | int | Number of CPU cores |
| `memUsed / memTotal` | bigint (bytes) | Memory utilization |
| `memCached` | bigint (bytes) | Page cache (optional) |
| `swapUsed / swapTotal` | bigint (bytes) | Swap utilization (optional) |
| `diskUsed / diskTotal` | bigint (bytes) | Primary volume utilization |
| `diskReadBps / diskWriteBps` | bigint (bytes/s) | Disk I/O throughput (optional) |
| `netBytesSent / netBytesRecv` | bigint (bytes) | Cumulative network bytes since agent start |
| `netPacketsSent / netPacketsRecv` | bigint | Cumulative packet counts (optional) |

#### Query Granularity

| Granularity | Use Case | Retention |
|-------------|----------|-----------|
| `raw` | Last 7 days, high-resolution charts | 7 days |
| `1m` | 1h–6h chart ranges | 30 days |
| `5m` | 6h–24h chart ranges | 90 days |
| `1h` | 7-day charts | 1 year |
| `1d` | 30-day+ charts | Indefinite |

#### API

```
GET /api/v1/metrics/:instanceId/timeseries
  ?from=<ISO8601>&to=<ISO8601>&granularity=5m&limit=500

GET /api/v1/metrics/:instanceId/aggregate
  ?from=<ISO8601>&to=<ISO8601>&metrics=cpu_percent,mem_used

GET /api/v1/metrics/latest
  ?instanceIds=id1,id2,id3
```

---

### Log Aggregation

Structured log ingestion supports both individual lines and batched delivery.

#### Log Levels

| Level | Usage |
|-------|-------|
| `DEBUG` | Verbose diagnostic output |
| `INFO` | Normal operational events |
| `WARN` | Potentially problematic conditions |
| `ERROR` | Failures requiring attention |

#### Log Sources

| Source | Description |
|--------|-------------|
| `AGENT` | Agent internal messages |
| `EXTENSION` | Extension install/runtime output |
| `BUILD` | Build step output |
| `APP` | Application stdout/stderr |
| `SYSTEM` | OS-level messages |

#### Search API

```
GET /api/v1/logs
  ?instanceId=<id>
  &level=ERROR,WARN
  &source=AGENT
  &from=<ISO8601>&to=<ISO8601>
  &q=<full-text-search>
  &limit=50&cursor=<logId>
```

Results are paginated with cursor-based navigation. Response includes `hasMore` and `nextCursor` fields.

#### Real-Time Streaming

Subscribe to new log entries via SSE:

```
GET /api/v1/logs/stream?instanceId=<id>&level=ERROR
```

Or via the WebSocket `logs` channel (filters applied server-side before forwarding to subscribers).

---

### Alerting Engine

The alerting engine evaluates rules on a 60-second cycle against the latest metric snapshot for each instance.

#### Alert Rule Structure

```json
{
  "name": "High CPU",
  "instanceId": null,
  "conditions": [
    { "metric": "cpu_percent", "op": "gt", "threshold": 80 }
  ],
  "conditionOperator": "AND",
  "severity": "warning",
  "evaluationWindowSec": 60,
  "pendingForSec": 120,
  "cooldownSec": 300,
  "notifyChannels": ["email", "webhook"],
  "notifyEmails": ["ops@example.com"],
  "webhookUrl": "https://hooks.example.com/alerts"
}
```

#### Alert State Machine

```
INACTIVE ──(condition fires)──→ PENDING
PENDING  ──(pendingForSec elapsed)──→ FIRING ──(notification sent)
FIRING   ──(condition clears)──→ RESOLVED
RESOLVED ──(next eval cycle)──→ INACTIVE
```

#### Severity Levels

| Severity | Use Case |
|----------|----------|
| `info` | Informational threshold crossed |
| `warning` | Performance degradation |
| `critical` | Imminent failure / capacity exceeded |

#### Alert API

```
GET  /api/v1/alerts/rules
POST /api/v1/alerts/rules
GET  /api/v1/alerts/rules/:id
PUT  /api/v1/alerts/rules/:id
DELETE /api/v1/alerts/rules/:id

GET  /api/v1/alerts/events
  ?ruleId=<id>&instanceId=<id>&state=FIRING&from=<ISO8601>
```

---

### Fleet Dashboard

The fleet dashboard provides a single-pane-of-glass view across all registered instances.

#### Summary Metrics

- Total instances, breakdown by status (RUNNING / STOPPED / DEPLOYING / ERROR)
- Fleet-wide average and maximum CPU, memory, and disk utilization
- Top-5 resource consumers (sortable by CPU, memory, disk)
- Stale instance list (no heartbeat in 5+ minutes, status = RUNNING)

#### Real-Time Updates

Status changes and metric updates are pushed via the `/ws/console` WebSocket channel and applied as partial state merges to the React fleet store.

---

### Instance Dashboard

Per-instance observability with configurable time ranges.

#### Time Ranges

| Range | Granularity | Auto-Refresh |
|-------|-------------|--------------|
| 1h | 1m | Yes (30s) |
| 6h | 5m | Yes (30s) |
| 24h | 5m | Yes (30s) |
| 7d | 1h | No |
| 30d | 1d | No |

#### Chart Panels

1. **CPU** — Utilization % with load average overlay (1/5/15 min)
2. **Memory** — Used/Total with percentage gauge
3. **Disk** — Used/Total with percentage gauge; optional read/write throughput
4. **Network** — Bytes sent/received throughput
5. **Events** — Lifecycle events overlaid on the time axis

The latest-values panel (top of page) always reflects the most recent metric regardless of selected time range.

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
npm test                        # All tests (Phase 1 + 2 + 3)
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

---

## Phase 3 Testing Guide

### Integration Tests (Vitest)

Phase 3 tests are included in the default `npm test` run alongside Phase 1 and Phase 2 tests.

```bash
cd apps/api

# All Phase 3 observability tests
npm test -- tests/metrics-pipeline.test.ts
npm test -- tests/fleet-dashboard.test.ts
npm test -- tests/instance-dashboard.test.ts
npm test -- tests/log-aggregation.test.ts
npm test -- tests/alerting.test.ts
```

#### Test Coverage by File

| File | Tests | Coverage Area |
|------|-------|---------------|
| `metrics-pipeline.test.ts` | Ingest validation, time-series shapes, aggregation, fleet rollup, granularity, retention | Metrics service types and data shapes |
| `fleet-dashboard.test.ts` | Health summary, resource utilization, sorting, filtering, stale detection, top-N, real-time updates | Fleet aggregation logic |
| `instance-dashboard.test.ts` | Chart data, time range selection, real-time stream, threshold alerts, event timeline, auto-refresh | Per-instance dashboard data layer |
| `log-aggregation.test.ts` | WebSocket ingestion, storage schema, full-text search, filtering, streaming, pagination | Log pipeline from ingest to query |
| `alerting.test.ts` | Rule validation, threshold evaluation, state machine, notifications, suppression, history | Alerting engine rules and events |

### E2E Tests (Playwright)

Phase 3 E2E tests cover critical observability user journeys:

```bash
cd apps/web

# Fleet dashboard visualizations
npx playwright test tests/e2e/fleet-dashboard.spec.ts

# Instance dashboard real-time metrics
npx playwright test tests/e2e/instance-dashboard.spec.ts

# Log search functionality
npx playwright test tests/e2e/log-search.spec.ts

# Alert triggering and notifications
npx playwright test tests/e2e/alerting.spec.ts

# Run all Phase 3 E2E tests
npx playwright test tests/e2e/fleet-dashboard.spec.ts tests/e2e/instance-dashboard.spec.ts tests/e2e/log-search.spec.ts tests/e2e/alerting.spec.ts
```

#### E2E Test Scenarios

**fleet-dashboard.spec.ts:**
- Fleet health summary renders correct instance counts by status
- Sorting by CPU descending shows highest utilization first
- Filtering by provider narrows the instance list
- Stale instance badge appears after heartbeat timeout
- Real-time status update applies without page reload

**instance-dashboard.spec.ts:**
- CPU/memory/disk charts render data for selected time range
- Switching time range from 1h to 7d updates chart granularity
- Real-time metric update appends to chart without full reload
- Threshold breach triggers alert banner on the dashboard
- Event timeline shows lifecycle events overlaid on chart

**log-search.spec.ts:**
- Full-text search returns matching log lines
- Level filter (ERROR) hides INFO/DEBUG/WARN entries
- Source filter (AGENT) scopes results correctly
- Cursor pagination loads next page of results
- Real-time stream appends new log lines as they arrive

**alerting.spec.ts:**
- Creating an alert rule persists and appears in the rules list
- Alert transitions to FIRING when metric exceeds threshold
- Notification is sent (mock email/webhook) on FIRING transition
- Alert shows RESOLVED after metric drops below threshold
- Alert history shows firedAt and resolvedAt timestamps

### Phase 3 Coverage Targets

| Layer | Target |
|-------|--------|
| Metrics service unit tests | ≥ 85% statements |
| Log service unit tests | ≥ 85% statements |
| Alerting engine unit tests | ≥ 85% statements |
| API integration (Vitest) | ≥ 80% statements |
| Frontend unit (Vitest) | ≥ 75% statements |
| E2E (Playwright) | All critical observability paths covered |

---

## Phase 4: Administration & Security

Phase 4 adds enterprise-grade administration, security monitoring, and cost governance.

### Phase 4 Feature Overview

| Feature | Description |
|---------|-------------|
| RBAC & Team Workspaces | User and team management with permission enforcement, audit log |
| Extension Administration | Extension registry, custom extension upload, approval workflow, usage tracking |
| Configuration Drift Detection | Drift detection, severity classification, remediation, suppression rules |
| Cost Tracking & Optimization | Cost calculation, budget management, anomaly detection, optimization recommendations |
| Security Dashboard & BOM/CVE | SBOM generation, CVE detection, secrets scanning, security scoring |

---

## RBAC & Team Workspaces

### User Roles

| Role | Capabilities |
|------|-------------|
| `ADMIN` | Full system access — users, teams, all instances, audit log, settings |
| `OPERATOR` | Deploy, suspend, resume instances; manage extensions; view audit log |
| `DEVELOPER` | Read instances, execute commands, connect via terminal, install extensions |
| `VIEWER` | Read-only access to instances, metrics, and logs |

### Teams

Teams group users and control access to sets of instances collectively.

- Team slugs are URL-safe: `[a-z0-9-]+`
- Users can belong to multiple teams with different roles per team
- Team members inherit the instance access granted to the team
- Adding/removing members generates `TEAM_ADD` / `TEAM_REMOVE` audit entries

### Audit Log

All permission-sensitive operations are recorded:

| Action | Trigger |
|--------|---------|
| `CREATE` / `UPDATE` / `DELETE` | Resource lifecycle |
| `LOGIN` / `LOGOUT` | User sessions |
| `DEPLOY` / `DESTROY` / `SUSPEND` / `RESUME` | Instance lifecycle |
| `EXECUTE` / `CONNECT` / `DISCONNECT` | Command and terminal events |
| `PERMISSION_CHANGE` | Role changes |
| `TEAM_ADD` / `TEAM_REMOVE` | Team membership |

### API Key Management

- Keys are stored as SHA-256 hashes; the raw value is shown once at creation
- Keys can have an expiration date or be permanent (`expires_at: null`)
- Keys inherit the permissions of the owning user

### Phase 4 API Endpoints (RBAC)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/users` | List all users (ADMIN only) |
| `POST` | `/api/v1/users` | Create a user (ADMIN only) |
| `PATCH` | `/api/v1/users/:id` | Update user role (ADMIN only) |
| `DELETE` | `/api/v1/users/:id` | Delete a user (ADMIN only) |
| `GET` | `/api/v1/teams` | List all teams |
| `POST` | `/api/v1/teams` | Create a team (ADMIN only) |
| `GET` | `/api/v1/teams/:id` | Get team details |
| `PATCH` | `/api/v1/teams/:id` | Update team (ADMIN only) |
| `DELETE` | `/api/v1/teams/:id` | Delete team (ADMIN only) |
| `GET` | `/api/v1/teams/:id/members` | List team members |
| `POST` | `/api/v1/teams/:id/members` | Add a member (Team ADMIN+) |
| `DELETE` | `/api/v1/teams/:id/members/:userId` | Remove a member (Team ADMIN+) |
| `GET` | `/api/v1/audit` | Query audit log (ADMIN only) |

---

## Extension Administration & Registry

### Extension Lifecycle

```
PENDING ──(admin approves)──→ APPROVED ──(admin deprecates)──→ DEPRECATED
PENDING ──(admin rejects)──→ REJECTED
```

### Extension Visibility

| Visibility | Access |
|------------|--------|
| `PUBLIC` | All authenticated users |
| `TEAM` | Members of the owning team |
| `PRIVATE` | Creator and ADMINs only |

### Extension Versioning

- Versions follow semantic versioning: `MAJOR.MINOR.PATCH`
- Artifacts are stored with a `sha256:` checksum
- Only one version can be marked as `is_latest: true` per extension

### Usage Tracking

The `install_count` field increments on each new installation. Usage reports show:
- Which instances have each extension installed
- Active vs. removed installation counts per extension

### Phase 4 API Endpoints (Extensions)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/extensions` | List extensions (default: APPROVED only) |
| `POST` | `/api/v1/extensions` | Upload a custom extension (DEVELOPER+) |
| `GET` | `/api/v1/extensions/:id` | Get extension details |
| `PATCH` | `/api/v1/extensions/:id` | Update extension metadata |
| `GET` | `/api/v1/extensions/:id/versions` | List all versions |
| `POST` | `/api/v1/extensions/:id/approve` | Approve extension (ADMIN only) |
| `POST` | `/api/v1/extensions/:id/reject` | Reject extension (ADMIN only) |
| `POST` | `/api/v1/extensions/:id/deprecate` | Deprecate extension (ADMIN only) |
| `GET` | `/api/v1/extensions/:id/installations` | List instance installations |
| `POST` | `/api/v1/instances/:id/extensions` | Install extension on an instance |
| `DELETE` | `/api/v1/instances/:id/extensions/:extId` | Remove extension from an instance |

---

## Configuration Drift Detection

### Drift Types

| Type | Description | Default Severity |
|------|-------------|-----------------|
| `MISSING_EXTENSION` | Required extension not installed | CRITICAL |
| `CONFIG_HASH_CHANGE` | `sindri.yaml` hash differs from deployed | HIGH |
| `EXTENSION_MISMATCH` | Installed version differs from desired | HIGH |
| `VERSION_MISMATCH` | Minor version mismatch | MEDIUM |
| `RESOURCE_DRIFT` | Resource limits differ from spec | MEDIUM |
| `EXTRA_EXTENSION` | Extension installed but not in desired config | LOW |

### Drift State Machine

```
DETECTED ──(user acknowledges)──→ ACKNOWLEDGED
ACKNOWLEDGED ──(remediation starts)──→ REMEDIATING
REMEDIATING ──(success)──→ RESOLVED
any ──(user suppresses)──→ SUPPRESSED
```

### Suppression Rules

- Scoped per instance, per drift type, or fleet-wide
- Can be permanent or time-limited (`expires_at`)
- Used to acknowledge known deviations that are intentional

### Remediation Modes

| Mode | Description |
|------|-------------|
| `MANUAL` | User-triggered — requires explicit action |
| `AUTOMATIC` | System-triggered — applies fix without user intervention |

### Phase 4 API Endpoints (Drift)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/drift` | Fleet drift summary |
| `GET` | `/api/v1/drift/:instanceId` | Instance drift report |
| `POST` | `/api/v1/drift/:id/acknowledge` | Acknowledge a drift report |
| `POST` | `/api/v1/drift/:id/remediate` | Trigger remediation |
| `POST` | `/api/v1/drift/:id/suppress` | Create a suppression rule |
| `GET` | `/api/v1/drift/suppress` | List suppression rules |
| `DELETE` | `/api/v1/drift/suppress/:id` | Delete a suppression rule |

---

## Cost Tracking & Optimization

### Cost Categories

| Category | Description |
|----------|-------------|
| `COMPUTE` | CPU/compute hours |
| `STORAGE` | Persistent volume costs |
| `NETWORK` | Ingress/egress bandwidth |
| `EGRESS` | Outbound data transfer |
| `OTHER` | Miscellaneous provider charges |

### Budget Scoping

Budgets can be scoped to:
- Fleet-wide (no scope)
- A specific team
- A specific instance

### Alert Thresholds

Budgets trigger alerts at configurable thresholds: 50%, 75%, 80%, 90%, 100% of limit.

### Anomaly Detection

A cost anomaly is detected when actual spend exceeds expected spend by more than 50% within a measurement window. Anomaly sensitivity is configurable.

### Optimization Recommendations

| Action | Trigger Condition |
|--------|------------------|
| `SUSPEND_IDLE` | Instance idle for 48h+ |
| `DOWNSIZE` | Consistently low resource utilization |
| `RIGHTSIZE` | Resource spec doesn't match actual usage pattern |
| `SWITCH_PROVIDER` | Significantly cheaper alternative provider available |
| `REMOVE_UNUSED` | Instance has had no activity for 30d+ |

### Phase 4 API Endpoints (Costs)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/costs` | Fleet cost summary |
| `GET` | `/api/v1/costs/instances/:id` | Instance cost breakdown |
| `GET` | `/api/v1/costs/instances/:id/history` | Historical cost data |
| `GET` | `/api/v1/budgets` | List all budgets |
| `POST` | `/api/v1/budgets` | Create a budget |
| `GET` | `/api/v1/budgets/:id` | Get budget details |
| `PUT` | `/api/v1/budgets/:id` | Update a budget |
| `DELETE` | `/api/v1/budgets/:id` | Delete a budget |
| `GET` | `/api/v1/costs/anomalies` | List cost anomalies |
| `GET` | `/api/v1/costs/recommendations` | List optimization recommendations |

---

## Security Dashboard & BOM/CVE Monitoring

### SBOM (Software Bill of Materials)

The Console generates SBOMs in **CycloneDX** and **SPDX** formats. An SBOM is regenerated after each extension installation and captures:

- All direct and transitive dependencies per extension
- Package URLs (PURL format: `pkg:type/namespace/name@version`)
- License information for compliance

### CVE Detection

CVEs are matched against SBOM components using the NVD (National Vulnerability Database). Severity is classified by CVSS score:

| CVSS Range | Severity |
|-----------|---------|
| 9.0–10.0 | CRITICAL |
| 7.0–8.9 | HIGH |
| 4.0–6.9 | MEDIUM |
| 0.1–3.9 | LOW |
| 0.0 | NONE / INFORMATIONAL |

### Vulnerability Status Lifecycle

```
OPEN ──(user reviews)──→ ACKNOWLEDGED
ACKNOWLEDGED ──(patch applied)──→ PATCHING ──→ FIXED
OPEN ──(risk accepted)──→ ACCEPTED_RISK
OPEN ──(not applicable)──→ FALSE_POSITIVE
```

### Secrets Scanning

Scans sindri.yaml, .env files, and config directories for high-entropy strings indicative of:
- API keys (`API_KEY`)
- Tokens (`TOKEN`)
- Passwords (`PASSWORD`)
- Certificates (`CERTIFICATE`)
- SSH private keys (`SSH_KEY`)
- Generic high-entropy secrets (`GENERIC`)

Shannon entropy > 4.0 combined with minimum length thresholds is used as the detection heuristic.

### Security Scoring

Each instance receives a security score (0–100) and letter grade:

| Score | Grade |
|-------|-------|
| 90–100 | A |
| 80–89 | B |
| 70–79 | C |
| 60–69 | D |
| 0–59 | F |

Scoring factors:
- Open CVEs (weighted by severity — CRITICAL has the highest weight)
- Unrotated secret findings
- Unpatched high-severity vulnerabilities

### Phase 4 API Endpoints (Security)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/security` | Fleet security summary |
| `GET` | `/api/v1/security/instances/:id` | Instance security score |
| `GET` | `/api/v1/security/instances/:id/sbom` | SBOM for an instance |
| `GET` | `/api/v1/security/instances/:id/cves` | CVEs affecting an instance |
| `GET` | `/api/v1/security/cves` | All CVEs (fleet-wide) |
| `PATCH` | `/api/v1/security/cves/:id` | Update CVE status |
| `GET` | `/api/v1/security/secrets` | All secret findings |
| `PATCH` | `/api/v1/security/secrets/:id` | Update secret finding status |

---

## Phase 4 Testing Guide

### Integration Tests (Vitest)

Phase 4 tests are included in the default `npm test` run alongside Phase 1–3 tests.

```bash
cd apps/api

# RBAC and team workspaces
npm test -- tests/rbac-teams.test.ts

# Extension administration
npm test -- tests/extension-admin.test.ts

# Configuration drift detection
npm test -- tests/config-drift.test.ts

# Cost tracking and budgets
npm test -- tests/cost-tracking.test.ts

# Security dashboard and CVE monitoring
npm test -- tests/security-dashboard.test.ts
```

#### Test Coverage by File

| File | Tests | Coverage Area |
|------|-------|---------------|
| `rbac-teams.test.ts` | User CRUD, team management, membership, permission matrix, team-scoped access, audit log, API keys | RBAC permission enforcement and team workspace data model |
| `extension-admin.test.ts` | Registry listing/search, versioning, installation, custom upload, usage tracking, governance | Extension registry and installation lifecycle |
| `config-drift.test.ts` | Drift detection algorithms, severity classification, state machine, remediation jobs, suppression rules, fleet summary | Drift detection engine |
| `cost-tracking.test.ts` | Cost calculation, budget thresholds, anomaly detection, cost attribution, optimization recommendations | Cost tracking data model and budget alert logic |
| `security-dashboard.test.ts` | SBOM format, CVE detection/scoring, secrets scanning, security scoring, fleet summary | Security monitoring data model |

### E2E Tests (Playwright)

```bash
cd apps/web

# User and team management flows
npx playwright test tests/e2e/rbac-teams.spec.ts

# Extension browsing, upload, and admin governance
npx playwright test tests/e2e/extension-admin.spec.ts

# Drift detection dashboard and remediation UI
npx playwright test tests/e2e/drift-detection.spec.ts

# Budget management and alert notifications
npx playwright test tests/e2e/budget-alerts.spec.ts

# Security dashboard, CVE list, SBOM, and secrets
npx playwright test tests/e2e/security-dashboard.spec.ts

# Run all Phase 4 E2E tests
npx playwright test tests/e2e/rbac-teams.spec.ts tests/e2e/extension-admin.spec.ts tests/e2e/drift-detection.spec.ts tests/e2e/budget-alerts.spec.ts tests/e2e/security-dashboard.spec.ts
```

#### E2E Test Scenarios

**rbac-teams.spec.ts:**
- Team list renders with member counts
- Creating a team persists and appears in list
- Team detail page shows member list with roles
- Users page shows email and role for each user
- Audit log entries are ordered newest first

**extension-admin.spec.ts:**
- Extension registry displays name, version, and description
- Search bar filters extensions by name
- Extension detail shows install button and version history
- Upload dialog validates required fields
- Admin governance page shows pending/approve/reject controls

**drift-detection.spec.ts:**
- Drift dashboard shows fleet health percentage
- Instance drift rows show severity badges
- Drift detail shows individual drift items with expected/actual values
- Acknowledge, remediate, and suppress buttons are available

**budget-alerts.spec.ts:**
- Budget rows show progress bars vs. limit
- Budget threshold badges appear when spending exceeds 50%/80%/100%
- Creating a valid budget adds it to the list
- Optimization recommendations show action type and potential savings

**security-dashboard.spec.ts:**
- Fleet security score is displayed
- CVE list shows severity badges and CVSS scores
- CVE severity filter narrows the list
- SBOM tab shows component names and versions
- Secret findings show type, location, and rotation status

### Phase 4 Coverage Targets

| Layer | Target |
|-------|--------|
| RBAC unit tests | ≥ 85% statements |
| Extension admin unit tests | ≥ 85% statements |
| Drift detection unit tests | ≥ 85% statements |
| Cost tracking unit tests | ≥ 85% statements |
| Security dashboard unit tests | ≥ 85% statements |
| API integration (Vitest) | ≥ 80% statements |
| Frontend unit (Vitest) | ≥ 75% statements |
| E2E (Playwright) | All critical Phase 4 user paths covered |
