# Sindri Console

The Sindri Console is a web-based management interface for monitoring and controlling Sindri environment instances deployed across multiple cloud providers.

## Overview

The Console provides:

- **Instance Dashboard** - Real-time status, metrics, and health of all Sindri instances
- **Web Terminal** - Browser-based PTY sessions into running instances via xterm.js
- **Agent System** - Lightweight Go agent deployed alongside each instance, reporting heartbeats and metrics
- **Event Log** - Full audit trail of lifecycle events (deploy, backup, destroy, etc.)
- **RBAC** - Role-based access control with Admin, Operator, Developer, and Viewer roles

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
