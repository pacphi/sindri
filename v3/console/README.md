# Sindri Console

Orchestration, administration, and observability for Sindri environments.

## What is this?

Sindri Console is a unified control plane for all deployed Sindri instances across every supported provider (Docker, Fly.io, DevPod, E2B, Kubernetes). It provides:

- **Fleet overview** — real-time status, health metrics, and resource utilisation across all instances
- **Web terminal** — browser-based PTY access to any running instance via xterm.js
- **Instance registry** — automatic registration when instances deploy via the `console-agent` extension
- **WebSocket streaming** — live CPU/memory/disk metrics and event feeds from agents
- **RBAC & Teams** — role-based access control with team workspaces and audit log (Phase 4)
- **Extension Administration** — extension registry, custom upload, approval workflow (Phase 4)
- **Configuration Drift** — drift detection, severity classification, and remediation (Phase 4)
- **Cost Tracking** — cost analysis, budget management, and optimization recommendations (Phase 4)
- **Security Dashboard** — SBOM generation, CVE monitoring, secrets scanning (Phase 4)

See `docs/README.md` for the full feature documentation and `docs/ARCHITECTURE.md` for system design.

## Directory Structure

```
v3/console/
├── apps/
│   ├── api/          # Node.js backend (Hono + Prisma + WebSocket gateway)
│   └── web/          # React 19 SPA (Vite + TanStack Router + xterm.js)
├── packages/
│   ├── shared/       # TypeScript types shared by API and web
│   └── ui/           # Shared React component library
├── agent/            # Go instance agent (heartbeat, metrics, PTY)
├── docs/             # Architecture and API documentation
├── docker-compose.yml
├── turbo.json
├── pnpm-workspace.yaml
└── package.json
```

## Prerequisites

| Tool           | Version | Notes                                    |
| -------------- | ------- | ---------------------------------------- |
| Node.js        | 20 LTS+ | Use `nvm install 20` or `nvm install 22` |
| pnpm           | 9+      | `npm install -g pnpm`                    |
| Go             | 1.22+   | For building the agent binary            |
| Docker         | 24+     | For PostgreSQL and Redis                 |
| Docker Compose | 2.20+   | Bundled with Docker Desktop              |

## Quick Start

### 1. Install Node.js dependencies

```bash
cd v3/console
pnpm install
```

### 2. Start infrastructure

```bash
pnpm infra:up
```

This starts PostgreSQL 16 on `localhost:5432` and Valkey (Redis-compatible) on `localhost:6379`.

### 3. Configure the API

```bash
cp apps/api/.env.example apps/api/.env
# Edit apps/api/.env — defaults work with docker-compose out of the box
```

### 4. Run database migrations

```bash
pnpm db:migrate
pnpm db:seed     # optional: populate sample instances, users, and API keys
```

### 5. Start the development servers

```bash
pnpm dev
```

Turborepo starts both the API and web frontend in parallel:

- API: `http://localhost:3000`
- Web: `http://localhost:5173` (Vite HMR)

### 6. Build the Go agent (optional)

```bash
cd agent
make build
```

Run against your local Console:

```bash
SINDRI_CONSOLE_URL=http://localhost:3000 \
SINDRI_CONSOLE_API_KEY=dev-api-key-change-me \
SINDRI_PROVIDER=docker \
./dist/sindri-agent
```

## Common Commands

| Command            | Description                             |
| ------------------ | --------------------------------------- |
| `pnpm dev`         | Start API + web in watch mode           |
| `pnpm build`       | Production build for all packages       |
| `pnpm test`        | Run all test suites                     |
| `pnpm typecheck`   | TypeScript check across all packages    |
| `pnpm lint`        | ESLint across all packages              |
| `pnpm infra:up`    | Start PostgreSQL + Valkey               |
| `pnpm infra:down`  | Stop infrastructure containers          |
| `pnpm infra:reset` | Wipe volumes and restart infrastructure |
| `pnpm db:migrate`  | Apply pending Prisma migrations         |
| `pnpm db:seed`     | Populate development seed data          |
| `pnpm db:studio`   | Open Prisma Studio (database GUI)       |
| `pnpm db:reset`    | Reset DB and re-apply all migrations    |

## Package Names

| Package           | Name                     | Description             |
| ----------------- | ------------------------ | ----------------------- |
| `apps/api`        | `@sindri-console/api`    | Backend API             |
| `apps/web`        | `@sindri/console-web`    | React frontend          |
| `packages/shared` | `@sindri-console/shared` | Shared TypeScript types |
| `packages/ui`     | `@sindri-console/ui`     | Shared React components |

To run a command in a specific package:

```bash
pnpm --filter=@sindri-console/api run test
pnpm --filter=@sindri/console-web run build
```

## Documentation

| Document                  | Description                                      |
| ------------------------- | ------------------------------------------------ |
| `docs/ARCHITECTURE.md`    | System design, component diagram, security model |
| `docs/API_SPEC.md`        | REST + WebSocket API reference                   |
| `docs/DATABASE_SCHEMA.md` | Prisma schema, indexes, retention policy         |
| `docs/SETUP.md`           | Detailed development environment setup           |

## Agent Environment Variables

The Go agent binary reads all config from environment variables:

| Variable                 | Required | Default     | Description                                                   |
| ------------------------ | -------- | ----------- | ------------------------------------------------------------- |
| `SINDRI_CONSOLE_URL`     | Yes      | —           | Console base URL (e.g. `https://console.sindri.dev`)          |
| `SINDRI_CONSOLE_API_KEY` | Yes      | —           | Bootstrap API key for registration                            |
| `SINDRI_INSTANCE_ID`     | No       | hostname    | Unique instance identifier                                    |
| `SINDRI_PROVIDER`        | No       | —           | Deployment provider (`fly`, `docker`, `k8s`, `e2b`, `devpod`) |
| `SINDRI_REGION`          | No       | —           | Geographic region (e.g. `sea`, `us-east-1`)                   |
| `SINDRI_AGENT_HEARTBEAT` | No       | `30`        | Heartbeat interval in seconds                                 |
| `SINDRI_AGENT_METRICS`   | No       | `60`        | Metrics collection interval in seconds                        |
| `SINDRI_AGENT_SHELL`     | No       | `/bin/bash` | Default shell for PTY sessions                                |
| `SINDRI_LOG_LEVEL`       | No       | `info`      | Log verbosity (`debug`, `info`, `warn`, `error`)              |
| `SINDRI_AGENT_TAGS`      | No       | —           | Comma-separated `key=value` metadata labels                   |
