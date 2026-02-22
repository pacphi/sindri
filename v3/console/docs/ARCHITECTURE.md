# Sindri Console — System Architecture

## Overview

Sindri Console is the orchestration, administration, and observability layer for Sindri environments. It acts as a unified control plane for all deployed Sindri instances across every supported provider (Docker, Fly.io, DevPod, E2B, Kubernetes).

The system consists of three primary components:

1. **Console API** — Node.js backend (Hono + tRPC) with PostgreSQL, Redis, and WebSocket gateway
2. **Web Frontend** — React 19 SPA (Vite) with xterm.js terminal, Monaco editor, and real-time dashboards
3. **Instance Agent** — Lightweight Go binary deployed as a Sindri extension on each managed instance

---

## System Boundaries

```
┌─────────────────────────────────────────────────────────────────────┐
│                         USER BROWSER                                │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    React SPA (Vite)                          │   │
│  │  xterm.js  │  Monaco Editor  │  Recharts  │  TanStack Query  │   │
│  └──────────────────────┬───────────────────────────────────────┘   │
└─────────────────────────┼───────────────────────────────────────────┘
                          │  HTTPS + WSS
┌─────────────────────────▼───────────────────────────────────────────┐
│                      CONSOLE API (Node.js / Hono)                   │
│                                                                     │
│   ┌──────────────┐  ┌──────────────┐  ┌───────────────────────┐     │
│   │  tRPC Routes │  │  WebSocket   │  │   BullMQ Workers      │     │
│   │  (REST API)  │  │  Gateway     │  │   (metrics, alerts)   │     │
│   └──────┬───────┘  └──────┬───────┘  └───────────┬───────────┘     │
│          │                 │                      │                 │
│   ┌──────▼─────────────────▼───────────────────────▼───────────┐    │
│   │              Services Layer                                │    │
│   │  InstanceService │ TerminalService │ MetricsService │ Auth │    │
│   └──────┬─────────────────────────────────────────────────────┘    │
│          │                                                          │
│   ┌──────▼──────────┐  ┌─────────────┐                              │
│   │   PostgreSQL    │  │    Redis    │                              │
│   │   (Prisma ORM)  │  │  (Pub/Sub)  │                              │
│   └─────────────────┘  └─────────────┘                              │
└──────────────────────────────┬──────────────────────────────────────┘
                               │  mTLS WebSocket (WSS)
          ┌────────────────────┼─────────────────────┐
          │                    │                     │
┌─────────▼───────┐  ┌─────────▼───────┐  ┌──────────▼──────┐
│ Sindri Instance │  │ Sindri Instance │  │ Sindri Instance │
│  (Fly.io/sea)   │  │   (k8s/us-e1)   │  │  (E2B sandbox)  │
│                 │  │                 │  │                 │
│  [Go Agent]     │  │  [Go Agent]     │  │  [Go Agent]     │
│  - heartbeat    │  │  - heartbeat    │  │  - heartbeat    │
│  - metrics      │  │  - metrics      │  │  - metrics      │
│  - PTY/terminal │  │  - PTY/term     │  │  - PTY/term     │
│  - log stream   │  │  - log stream   │  │  - log stream   │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

---

## Component Details

### Console API

**Technology:** Node.js 24 LTS, Hono (HTTP framework), tRPC (type-safe API), Prisma (ORM)

**Responsibilities:**

- Instance registry: store and query registered Sindri instances
- Authentication and RBAC enforcement
- WebSocket gateway: multiplex connections from browser clients and instance agents
- Metric ingestion and time-series persistence
- Background job orchestration (BullMQ)

**Port allocations:**

- `3000` — HTTP API and WebSocket upgrade endpoint
- `5432` — PostgreSQL (internal)
- `6379` — Redis (internal)

**Key modules:**

| Module        | Path                     | Responsibility                         |
| ------------- | ------------------------ | -------------------------------------- |
| HTTP router   | `apps/api/src/routes/`   | REST endpoints and tRPC adapter        |
| WS gateway    | `apps/api/src/ws/`       | Agent and browser WebSocket management |
| Services      | `apps/api/src/services/` | Business logic layer                   |
| Workers       | `apps/api/src/workers/`  | BullMQ metric aggregation, alerting    |
| Prisma client | `apps/api/prisma/`       | Schema and migration management        |

---

### Web Frontend

**Technology:** React 19, Vite, TanStack Router, TanStack Query, Zustand, shadcn/ui, Tailwind CSS 4

**Responsibilities:**

- Instance list view with real-time status
- Web terminal (xterm.js) connected via WebSocket to agent PTYs
- Fleet health dashboard with live metrics (Recharts)
- Monaco-based sindri.yaml editor
- Authentication UI (login, API key management)

**Component structure:**

```
apps/web/src/
├── components/
│   ├── dashboard/      # Fleet overview, health cards, charts
│   ├── instances/      # Instance list, detail panel, status badges
│   ├── terminal/       # xterm.js wrapper, session manager
│   ├── config/         # Monaco YAML editor, diff viewer
│   ├── extensions/     # Extension browser
│   ├── observability/  # Metrics, logs, alerts panels
│   └── admin/          # Users, teams, API keys
├── hooks/              # useWebSocket, useTerminal, useMetrics
├── stores/             # Zustand: terminal sessions, active instance
├── api/                # tRPC client configuration
└── lib/                # Types, constants, utilities
```

---

### Instance Agent (Go Binary)

**Technology:** Go 1.22+, gopsutil, gorilla/websocket, creack/pty, prometheus/client_golang, fsnotify

**Responsibilities:**

- Maintain persistent WebSocket connection to the Console API
- Collect and ship system metrics (CPU, memory, disk, network) every 30s
- Send heartbeat pings every 10s
- Spawn PTY sessions on request for web terminal access
- Stream logs from init hooks, extension installs, and running processes
- Report lifecycle events (deploy, connect, disconnect)
- Watch sindri.yaml for local config changes

**Distribution:** Compiled as a static binary, delivered as a Sindri extension (`console-agent`). Zero runtime dependencies on the instance.

**Startup sequence:**

1. Read `SINDRI_CONSOLE_URL` and `SINDRI_CONSOLE_API_KEY` from environment
2. POST `/api/v1/instances/register` with instance metadata
3. Establish WebSocket connection to Console
4. Begin metric collection goroutine (30s interval)
5. Begin heartbeat goroutine (10s interval)
6. Listen for inbound commands on the WebSocket

---

## Real-Time Communication

### WebSocket Channel Protocol

All WebSocket messages are JSON-encoded with a `channel` discriminator:

```
Console <---- WSS ----> Instance Agent
   |
   +-- channel: "heartbeat"   agent->console, every 10s
   +-- channel: "metrics"     agent->console, every 30s
   +-- channel: "logs"        agent->console, streaming
   +-- channel: "events"      agent->console, on occurrence
   +-- channel: "terminal"    bidirectional, per-session UUID
   +-- channel: "commands"    console->agent, on demand
```

### Browser WebSocket Channels

```
Browser Client <---- WSS ----> Console API
   |
   +-- channel: "terminal"    bidirectional, proxied from agent
   +-- channel: "metrics"     console->browser, live metric updates
   +-- channel: "events"      console->browser, instance lifecycle events
   +-- channel: "alerts"      console->browser, alert notifications
```

### Metric Pipeline

```
Instance Agent           Console API                 Browser
--------------           -----------                 -------
gopsutil        -->  WebSocket ingest  -->  Redis Pub/Sub  -->  TanStack Query
(every 30s)          (MetricsService)       (real-time)          subscription
                             |                                       |
                             v                                       v
                        PostgreSQL                              Recharts
                        metrics table                           live gauges
                        (retention: 30d)
```

### Terminal Multiplexing Flow

```
1. Browser sends:   terminal:create { instanceId, cols, rows }
2. Console routes to agent WS, forwards terminal:create
3. Agent spawns PTY via creack/pty, assigns session UUID
4. Agent responds:  terminal:ready { sessionId }
5. Browser opens xterm.js, sends input as terminal:input { sessionId, data }
6. Agent writes to PTY stdin, reads stdout, sends terminal:output { sessionId, data }
7. Resize:          terminal:resize { sessionId, cols, rows }
8. Close:           terminal:close { sessionId }
```

---

## Auto-Registration Flow

```
sindri deploy        Instance boots      Console API
-------------        --------------      -----------
  |                        |                  |
  |------ deploys -------> |                  |
  |                        |                  |
  |               init hooks run              |
  |                        |                  |
  |               console-agent starts        |
  |                        |                  |
  |                        |--POST /register-->|
  |                        |  { instanceId,    |
  |                        |    provider,      |
  |                        |    region,        |
  |                        |    extensions,    |
  |                        |    yamlHash,      |
  |                        |    bom,           |
  |                        |    sshEndpoint }  |
  |                        |                  |
  |                        |<-- 200 OK --------|
  |                        |    { agentToken } |
  |                        |                  |
  |                        |==WS connect======>|
  |                        |  (persistent)     |
  |                        |                  |
  |                        |--heartbeat------> | (every 10s)
  |                        |--metrics--------> | (every 30s)
```

---

## Directory Structure

```
v3/console/
+-- apps/
|   +-- web/                        # React SPA (Vite + React 19)
|   |   +-- src/
|   |   |   +-- components/
|   |   |   +-- hooks/
|   |   |   +-- stores/
|   |   |   +-- api/
|   |   |   +-- lib/
|   |   +-- index.html
|   |   +-- vite.config.ts
|   |   +-- package.json
|   +-- api/                        # Console API (Node.js + Hono)
|       +-- src/
|       |   +-- routes/
|       |   +-- ws/
|       |   +-- services/
|       |   +-- workers/
|       |   +-- index.ts
|       +-- prisma/
|       |   +-- schema.prisma
|       +-- package.json
+-- packages/
|   +-- shared/                     # Shared TypeScript types
|   +-- agent/                      # Go agent source
|   |   +-- cmd/agent/main.go
|   |   +-- internal/
|   |   |   +-- metrics/
|   |   |   +-- terminal/
|   |   |   +-- heartbeat/
|   |   |   +-- ws/
|   |   +-- go.mod
|   +-- ui/                         # Shared React component library
+-- docs/
|   +-- ARCHITECTURE.md             # This file
|   +-- API_SPEC.md
|   +-- DATABASE_SCHEMA.md
|   +-- SETUP.md
+-- turbo.json
+-- pnpm-workspace.yaml
+-- package.json
```

---

## Technology Stack Justifications

| Layer              | Technology        | Rationale                                                                                            |
| ------------------ | ----------------- | ---------------------------------------------------------------------------------------------------- |
| API framework      | Hono              | Edge-compatible, ultra-low overhead, excellent TypeScript support                                    |
| API type safety    | tRPC              | End-to-end type safety from DB to browser; eliminates API contract drift                             |
| ORM                | Prisma            | Type-safe queries, migration management, first-class PostgreSQL support                              |
| Database           | PostgreSQL        | Relational integrity for instance registry, audit logs, RBAC; TimescaleDB extension path for metrics |
| Cache/Pub-Sub      | Redis             | Real-time metric streaming via Pub/Sub; session state; BullMQ job backend                            |
| Job queue          | BullMQ            | Built on Redis; reliable background jobs for metric aggregation and alerting                         |
| WebSocket          | ws (Node.js)      | Low-level control for both agent connections and browser proxying                                    |
| Frontend framework | React 19          | Concurrent features, Suspense-based data fetching; large ecosystem                                   |
| Frontend build     | Vite              | Fast HMR, native ESM, excellent TypeScript DX                                                        |
| Router             | TanStack Router   | Fully type-safe routes and search params                                                             |
| Server state       | TanStack Query    | Caching, real-time subscriptions, optimistic updates                                                 |
| Client state       | Zustand           | Minimal boilerplate for terminal sessions and UI state                                               |
| UI components      | shadcn/ui + Radix | Accessible primitives; copy-owned, not locked to a package version                                   |
| Styling            | Tailwind CSS 4    | Rapid iteration; design system via CSS variables                                                     |
| Web terminal       | xterm.js          | Gold standard; powers VS Code terminal; full VT100 emulation                                         |
| YAML editor        | Monaco Editor     | VS Code editor component; YAML syntax, validation, diff                                              |
| Charts             | Recharts          | React-native charts with composable API                                                              |
| Agent language     | Go 1.22+          | Single static binary; low memory (~5MB); no runtime on instances                                     |
| Agent metrics      | gopsutil          | Cross-platform CPU, memory, disk, network collection                                                 |
| Agent PTY          | creack/pty        | PTY allocation for web shell sessions                                                                |

---

## Security Architecture

### mTLS Between Agent and Console

Every agent-to-Console WebSocket connection uses mutual TLS:

- Console presents a server certificate
- Each agent is issued a client certificate upon registration
- Certificates are rotated on a configurable schedule

### API Key Management

- Instance registration requires a shared `SINDRI_CONSOLE_API_KEY` (bootstrap key)
- After registration the agent receives a per-instance JWT with a 24h expiry
- The bootstrap key is injected via environment variable, never written to `sindri.yaml`

### RBAC Model

Roles (highest to lowest privilege): `admin`, `operator`, `developer`, `viewer`

| Action                | viewer | developer | operator | admin |
| --------------------- | ------ | --------- | -------- | ----- |
| View instance list    | Y      | Y         | Y        | Y     |
| View instance metrics | Y      | Y         | Y        | Y     |
| Open terminal         | N      | Y         | Y        | Y     |
| Deploy new instance   | N      | Y         | Y        | Y     |
| Destroy instance      | N      | N         | Y        | Y     |
| Manage users          | N      | N         | N        | Y     |
| Manage API keys       | N      | N         | Y        | Y     |

### Additional Security Controls

- **Rate limiting** — registration endpoint: 10 req/min per IP; API endpoints: 1000 req/min per token
- **Audit logging** — every mutating action logged with `userId`, `action`, `target`, `ip`, `timestamp`
- **CSP headers** — strict Content Security Policy on the web application
- **Agents connect outbound** — Console never initiates unsolicited connections to instances
- **Network isolation** — agents only open an outbound WebSocket; no inbound port required on instances

---

## Phase 1 Scope

Phase 1 delivers the foundational infrastructure:

- [x] Go agent: heartbeat, system metrics collection, PTY terminal (`v3/console/agent/`)
- [ ] Console API: instance registry, authentication, WebSocket gateway (`v3/console/apps/api/`)
- [x] React frontend: instance list, status dashboard, web terminal (`v3/console/apps/web/`)
- [ ] Auto-registration flow via `console-agent` Sindri extension
- [x] PostgreSQL schema with Prisma migrations (`v3/console/apps/api/prisma/`)
- [x] WebSocket gateway for agent and browser connections (`v3/console/apps/api/src/websocket/`)
- [ ] Basic RBAC (admin + developer roles) — auth middleware scaffolded
- [ ] Development environment setup (Docker Compose)
