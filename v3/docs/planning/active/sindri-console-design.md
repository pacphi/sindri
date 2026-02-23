# Sindri Console: Orchestration, Administration & Observability UI

## Design Vision Document — February 2026

---

## 1. Understanding Sindri v3 Today

### What Sindri Is

Sindri is a declarative, provider-agnostic cloud development environment system. At its core, it lets you define a development environment in YAML (`sindri.yaml`) and deploy that identical environment across multiple providers — Docker (local), Fly.io (cloud), DevPod (with Kubernetes, AWS, GCP, Azure backends), and E2B (ultra-fast sandboxes).

### The V3 Directory Hierarchy

```
sindri/
├── .claude/                    # Claude Code integration
│   └── skills/                 # Extension authoring guidance
├── .github/                    # CI/CD workflows
│   └── workflows/              # GitHub Actions pipelines
├── cli/                        # The sindri CLI
│   └── sindri                  # Primary entry point (config init, deploy, etc.)
├── deploy/
│   └── adapters/               # Provider adapter implementations
│       ├── docker/             # Local Docker deployment logic
│       ├── fly/                # Fly.io deployment adapter
│       ├── devpod/             # DevPod/DevContainer adapter
│       ├── e2b/                # E2B sandbox adapter
│       └── kubernetes/         # K8s direct adapter
├── docker/                     # Dockerfile layers, base images
│   ├── extensions/             # 70+ extension install scripts
│   │   ├── ai/                 # Claude Code, Aider, Continue, GPT-Engineer...
│   │   ├── languages/          # Python, Node, Rust, Go, Java, Ruby...
│   │   ├── infrastructure/     # Terraform, Ansible, kubectl, helm...
│   │   ├── databases/          # PostgreSQL, Redis, MongoDB clients...
│   │   └── tools/              # Git, Docker-in-Docker, tmux, zsh...
│   └── lib/                    # Immutable system layer
├── docs/                       # Comprehensive documentation
│   ├── ARCHITECTURE.md         # System design and concepts
│   ├── CONFIGURATION.md        # sindri.yaml reference
│   ├── CLI.md                  # Command-line reference
│   ├── EXTENSIONS.md           # Extension catalog
│   ├── EXTENSION_AUTHORING.md  # Creating custom extensions
│   ├── DEPLOYMENT.md           # Provider comparison
│   ├── BOM.md                  # Software bill of materials
│   ├── BACKUP_RESTORE.md       # Workspace recovery
│   ├── PROJECT_MANAGEMENT.md   # new-project / clone-project
│   ├── SECRETS_MANAGEMENT.md   # Cross-provider secrets
│   ├── providers/              # Per-provider guides
│   │   ├── FLY.md
│   │   ├── DOCKER.md
│   │   ├── DEVPOD.md
│   │   ├── E2B.md
│   │   └── KUBERNETES.md
│   └── ides/                   # VS Code, IntelliJ, Zed, Eclipse, Warp
├── examples/                   # sindri.yaml templates
├── scripts/                    # Build, test, utility scripts
├── test/                       # Test suites
├── Dockerfile                  # Primary build file
├── sindri.yaml                 # Configuration schema
├── package.json                # pnpm workspace root
└── pnpm-workspace.yaml         # Monorepo config
```

### Key Architectural Concepts

| Concept                 | Description                                                                                                                                                                                                                    |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Extension System**    | YAML-driven modules with dependency resolution. 70+ extensions across AI, languages, infrastructure, databases, and tools. Each extension declares install methods (apt, script, binary download), dependencies, and metadata. |
| **Provider Adapters**   | Clean abstraction layer. Each provider (Docker, Fly.io, DevPod, E2B, K8s) implements a common interface for deploy, destroy, status, and connect operations.                                                                   |
| **Volume Architecture** | Two-layer system: immutable `/docker/lib` (system tools, extensions) and mutable `$HOME` volume (user workspace, projects, dotfiles). Survives redeployments.                                                                  |
| **Schema Validation**   | All YAML validated against JSON schemas before deployment.                                                                                                                                                                     |
| **BOM Tracking**        | Every installed tool's version, source, and hash tracked for SBOM generation and security auditing.                                                                                                                            |
| **CLI Workflow**        | `sindri config init` → edit `sindri.yaml` → `sindri deploy --provider <target>` → `sindri connect`                                                                                                                             |

### What's Missing

Sindri today is entirely CLI-driven. Each instance is an island — deployed independently, managed independently, with no visibility across a fleet of environments. There is no:

- Central registry of deployed environments
- Visual way to switch between instances
- Real-time health or resource monitoring
- Web-based terminal access to instances
- Audit trail of deployments and changes
- Collaborative environment sharing
- Extension usage analytics
- Cost tracking across providers

---

## 2. Sindri Console: The Vision

**Sindri Console** is a TypeScript/React web application that serves as the orchestration, administration, and observability layer for Sindri environments. It acts as a unified control plane for all deployed Sindri instances across every provider.

### Core Principle: Bootstrap from Sindri

The Console itself should be deployable as a Sindri extension or as a standalone Sindri instance. Imagine:

```yaml
# sindri.yaml for the Console itself
name: sindri-console
extensions:
  - node-lts
  - postgresql-client
  - sindri-console # The Console extension
provider:
  fly:
    region: sea
    vm_size: shared-cpu-2x
    memory: 1024
```

Or alternatively, the Console ships as a standalone Docker image / Fly.io app that Sindri instances phone home to upon deployment.

### Auto-Registration Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  sindri deploy   │────▶│  Instance boots   │────▶│  Registration   │
│  --provider fly  │     │  runs init hooks  │     │  POST /api/v1/  │
│                  │     │                   │     │  instances       │
└─────────────────┘     └──────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
                                                 ┌─────────────────┐
                                                 │ Sindri Console   │
                                                 │ ┌─────────────┐ │
                                                 │ │ Instance     │ │
                                                 │ │ Registry     │ │
                                                 │ └─────────────┘ │
                                                 │ ┌─────────────┐ │
                                                 │ │ Heartbeat    │ │
                                                 │ │ Monitor      │ │
                                                 │ └─────────────┘ │
                                                 └─────────────────┘
```

Each Sindri instance, upon deploy, would:

1. **POST registration** to the Console API with: instance ID, provider, region, extensions, sindri.yaml hash, BOM manifest, SSH endpoint, created timestamp
2. **Begin heartbeat** — periodic health pings with CPU, memory, disk, uptime
3. **Stream logs** — stdout/stderr from init, extension installs, and ongoing activity
4. **Report events** — deploy, redeploy, connect, disconnect, backup, destroy

The Console endpoint URL would be configurable via `sindri.yaml`:

```yaml
console:
  endpoint: https://sindri-console.fly.dev
  api_key: ${SINDRI_CONSOLE_API_KEY}
  heartbeat_interval: 30s
  metrics_interval: 60s
```

---

## 3. Orchestration Capabilities

### Instance Lifecycle Management

From the Console UI, users can:

- **Deploy new environments** — select a template or paste a `sindri.yaml`, choose provider + region, and kick off deployment
- **Clone environments** — duplicate an existing instance's configuration to a new provider/region
- **Redeploy / update** — push config changes, trigger extension updates
- **Suspend / resume** — pause instances to save cost (especially on Fly.io where machines can auto-stop)
- **Destroy** — clean teardown with optional volume backup

### Multi-Instance Switching

The Console provides a persistent sidebar or command palette for switching between instances:

```
┌─────────────────────────────────────────────┐
│ 🔨 Sindri Console                    ☰  ⚙  │
├──────────────┬──────────────────────────────┤
│              │                              │
│ INSTANCES    │  📊 dev-primary (fly/sea)    │
│              │  ──────────────────────────── │
│ ● dev-primary│  Status: Running             │
│   fly / sea  │  Uptime: 4d 12h             │
│              │  CPU: 23%  RAM: 412MB/1GB    │
│ ● staging    │  Extensions: 12 active       │
│   k8s / us-e │  Last deploy: 2h ago         │
│              │                              │
│ ○ ml-sandbox │  [Terminal] [Logs] [Config]  │
│   e2b        │                              │
│              │  ┌────────────────────────┐  │
│ ● local-dev  │  │ $ ls -la ~/projects    │  │
│   docker     │  │ drwxr-xr-x  3 dev ...  │  │
│              │  │ drwxr-xr-x  5 dev ...  │  │
│ + New...     │  │ -rw-r--r--  1 dev ...  │  │
│              │  │ $                    █  │  │
└──────────────┴──┴────────────────────────┴──┘
```

### Web Shell / Terminal Access

Users can spawn interactive shells directly in the Console, connected to any running instance. This is one of the most powerful capabilities — no need to configure SSH, no local tooling required.

### Job Execution

Beyond interactive shells, the Console can dispatch work to instances:

- **Run commands** — execute one-off commands with stdout/stderr streaming
- **Run scripts** — upload and execute scripts across one or multiple instances
- **Scheduled tasks** — cron-like scheduling for maintenance operations
- **Parallel execution** — fan out commands across selected instances (e.g., "update all staging environments")

---

## 4. Administration Capabilities

### User & Access Management

- **RBAC** — roles like Admin, Operator, Developer, Viewer
- **Team workspaces** — group instances by team/project
- **API key management** — generate/revoke keys for CI/CD integration
- **SSO integration** — OIDC/SAML for enterprise environments
- **Audit log** — who did what, when, on which instance

### Configuration Management

- **Template library** — curated `sindri.yaml` templates for common stacks (Python ML, Full-stack JS, Rust systems, Java enterprise, etc.)
- **Configuration diff** — compare configs across instances, see what changed between deploys
- **Drift detection** — flag instances whose running state diverges from their declared config
- **Secrets vault** — centralized secrets management with per-instance injection

### Extension Administration

- **Extension registry** — browse, search, and audit all 70+ extensions
- **Usage matrix** — which instances use which extensions
- **Custom extension hosting** — upload private extensions for your org
- **Update policies** — auto-update, pin, or freeze extension versions
- **Dependency graph** — visualize extension dependency chains

### Cost Management

- **Per-instance cost tracking** — compute, storage, network by provider
- **Budget alerts** — set thresholds per team/project
- **Right-sizing recommendations** — "this instance consistently uses <10% CPU, consider downgrading"
- **Idle instance detection** — flag environments with no activity for N days

---

## 5. Observability Capabilities

### Metrics to Capture

**Instance-Level Metrics (collected via agent on each instance)**

| Category      | Metrics                                                                                           |
| ------------- | ------------------------------------------------------------------------------------------------- |
| **Compute**   | CPU usage (%), load average, process count, CPU steal time                                        |
| **Memory**    | Used/available/cached, swap usage, OOM events                                                     |
| **Disk**      | Volume usage (mutable $HOME), inode usage, I/O throughput, I/O latency                            |
| **Network**   | Bytes in/out, active connections, SSH sessions, DNS resolution time                               |
| **Process**   | Top processes by CPU/memory, language runtime usage (node, python, rustc), build process duration |
| **Container** | Container restarts, image pull time, layer cache hit rate                                         |

**Environment-Level Metrics (collected by Console)**

| Category         | Metrics                                                                  |
| ---------------- | ------------------------------------------------------------------------ |
| **Deployment**   | Deploy frequency, deploy duration, rollback count, success/failure rate  |
| **Extension**    | Install time per extension, install failures, dependency resolution time |
| **Availability** | Uptime percentage, heartbeat gaps, cold start latency, time-to-ready     |
| **Usage**        | Active shell sessions, commands executed, files modified, git operations |
| **Build**        | Docker build time, layer cache efficiency, image size trends             |
| **BOM/Security** | CVE counts by severity, outdated packages, SBOM drift from baseline      |

**Fleet-Level Metrics (aggregated across all instances)**

| Category       | Metrics                                                                   |
| -------------- | ------------------------------------------------------------------------- |
| **Capacity**   | Total instances by provider/region, active vs idle ratio                  |
| **Cost**       | Spend by provider, spend by team, cost per active developer-hour          |
| **Compliance** | Extension version distribution, config drift count, secret rotation age   |
| **Trends**     | Environment creation rate, average environment lifespan, peak concurrency |

### Dashboards

**Fleet Overview Dashboard**

- World map showing instance locations by region
- Health summary: green/yellow/red status counts
- Provider distribution pie chart
- Active sessions count
- 24h deployment activity timeline

**Instance Detail Dashboard**

- Real-time CPU/memory/disk gauges
- Process tree with resource attribution
- Network traffic graph
- Extension health checklist
- Recent events timeline
- Git activity (commits, branches, PRs)

**Deployment Pipeline Dashboard**

- Deploy history with duration trends
- Extension install waterfall (showing parallel vs sequential installs)
- Build cache hit rate over time
- Failed deploy analysis with root cause categorization

**Security Dashboard**

- BOM vulnerability summary across fleet
- Extension version freshness
- Secret age and rotation compliance
- SSH key audit
- Network exposure map

### Alerting

- **Threshold alerts** — CPU > 90% for 5m, disk > 85%, memory pressure
- **Anomaly alerts** — unusual network traffic, unexpected process spawning
- **Lifecycle alerts** — instance unresponsive, heartbeat lost, deploy failed
- **Security alerts** — new CVE in installed package, expired secret, unauthorized access attempt
- **Cost alerts** — budget threshold reached, runaway instance

Alerts delivered via webhook, Slack, email, or PagerDuty integration.

### Log Aggregation

- Structured log collection from all instances
- Full-text search across logs
- Log correlation by deployment ID
- Extension install log analysis
- Build output capture and indexing

---

## 6. Technology Stack: TypeScript/React Frontend

### Recommended Architecture

```
sindri-console/
├── apps/
│   ├── web/                        # React SPA (Vite + React 19)
│   │   ├── src/
│   │   │   ├── components/
│   │   │   │   ├── dashboard/      # Fleet overview, charts
│   │   │   │   ├── instances/      # Instance list, detail, switching
│   │   │   │   ├── terminal/       # Web terminal components
│   │   │   │   ├── config/         # YAML editor, diff viewer
│   │   │   │   ├── extensions/     # Extension browser/manager
│   │   │   │   ├── observability/  # Metrics, logs, alerts
│   │   │   │   └── admin/          # Users, teams, settings
│   │   │   ├── hooks/              # Custom React hooks
│   │   │   ├── stores/             # Zustand state management
│   │   │   ├── api/                # tRPC or REST client layer
│   │   │   └── lib/                # Utilities, types, constants
│   │   └── public/
│   └── api/                        # Backend API (Node.js)
│       ├── src/
│       │   ├── routes/             # API endpoints
│       │   ├── services/           # Business logic
│       │   ├── models/             # Database models
│       │   ├── agents/             # Instance communication
│       │   └── workers/            # Background jobs (metrics, alerts)
│       └── prisma/                 # Database schema
├── packages/
│   ├── shared/                     # Shared types, utils
│   ├── agent/                      # Lightweight agent that runs on instances
│   └── ui/                         # Shared UI components
└── turbo.json                      # Turborepo config
```

### Key Library Recommendations

#### Frontend (React SPA)

| Library                              | Purpose            | Why This One                                                                                                                                        |
| ------------------------------------ | ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| **React 19 + Vite**                  | Core framework     | Fast HMR, RSC-ready, best TypeScript DX                                                                                                             |
| **TanStack Router**                  | Routing            | Type-safe routing, nested layouts, search params                                                                                                    |
| **TanStack Query**                   | Server state       | Caching, real-time refetch, optimistic updates                                                                                                      |
| **Zustand**                          | Client state       | Minimal boilerplate, great for terminal/session state                                                                                               |
| **shadcn/ui + Radix**                | Component library  | Accessible, customizable, not opinionated on styling                                                                                                |
| **Tailwind CSS 4**                   | Styling            | Rapid iteration, design system consistency                                                                                                          |
| **xterm.js**                         | Web terminal       | The gold standard for browser-based terminals. Powers VS Code's terminal. Full VT100/xterm emulation, addon ecosystem (fit, WebGL renderer, search) |
| **Monaco Editor**                    | YAML/config editor | VS Code's editor component. Syntax highlighting, validation, diff view for sindri.yaml editing                                                      |
| **Recharts** or **visx**             | Charts/metrics     | Recharts for quick dashboards; visx (by Airbnb) for custom, D3-based visualizations                                                                 |
| **React Flow**                       | Dependency graphs  | For extension dependency visualization and deployment pipeline views                                                                                |
| **deck.gl** or **react-simple-maps** | Geo visualization  | Instance location map; deck.gl for 3D globe, react-simple-maps for simpler 2D                                                                       |
| **date-fns**                         | Time handling      | Lightweight date manipulation for logs, metrics timestamps                                                                                          |
| **zod**                              | Schema validation  | Runtime type validation for API responses, form data, sindri.yaml parsing                                                                           |

#### Backend (Node.js API)

| Library                      | Purpose          | Why This One                                                                           |
| ---------------------------- | ---------------- | -------------------------------------------------------------------------------------- |
| **Hono** or **Fastify**      | HTTP framework   | Hono for edge-compatible, ultra-fast routing; Fastify for mature plugin ecosystem      |
| **tRPC**                     | API layer        | End-to-end type safety between frontend and backend, great with TanStack Query         |
| **Prisma**                   | ORM / database   | Type-safe database access, migrations, excellent DX                                    |
| **PostgreSQL**               | Primary database | Instance registry, audit logs, config history, user management                         |
| **Redis** or **Valkey**      | Real-time layer  | Pub/sub for live metrics streaming, session state, heartbeat tracking                  |
| **BullMQ**                   | Job queue        | Background jobs for metric aggregation, alerting, scheduled tasks                      |
| **Socket.IO** or **ws**      | WebSocket        | Real-time terminal multiplexing, live metric streaming, log tailing                    |
| **node-ssh** / **ssh2**      | SSH client       | Establishing SSH tunnels to Sindri instances for terminal access and command execution |
| **Passport.js** or **Lucia** | Authentication   | Lucia for modern, session-based auth; Passport for SSO/OIDC flexibility                |
| **Pino**                     | Logging          | Structured, high-performance logging                                                   |
| **cron** (node-cron)         | Scheduling       | Heartbeat monitoring, metric collection intervals, scheduled maintenance               |

#### Instance Agent (Lightweight — runs on each Sindri instance)

| Library               | Purpose            | Why This One                                             |
| --------------------- | ------------------ | -------------------------------------------------------- |
| **systeminformation** | System metrics     | Cross-platform CPU, memory, disk, network, process data  |
| **prom-client**       | Metrics exposition | Prometheus-compatible metrics endpoint on each instance  |
| **ws**                | WebSocket client   | Persistent connection back to Console for real-time data |
| **node-pty**          | PTY spawning       | Pseudo-terminal allocation for web shell sessions        |
| **chokidar**          | File watching      | Watch sindri.yaml for local changes, trigger config sync |

### Alternative: Go-Based Agent

Given your Go experience (claude-code-agent-manager), the instance agent could alternatively be a lightweight Go binary:

| Library                      | Purpose                                 |
| ---------------------------- | --------------------------------------- |
| **gopsutil**                 | System metrics (CPU, memory, disk, net) |
| **gorilla/websocket**        | WebSocket connection to Console         |
| **creack/pty**               | PTY allocation for shell sessions       |
| **prometheus/client_golang** | Metrics exposition                      |
| **fsnotify**                 | File system watching                    |

A Go agent would have advantages: single static binary (no Node.js runtime dependency on the instance), lower memory footprint (~5MB vs ~50MB), and faster startup. It can be bundled as a Sindri extension.

---

## 7. Real-Time Architecture

### WebSocket Channels

The Console maintains persistent WebSocket connections for:

```
Console ◄──── ws:// ────► Instance Agent
   │
   ├── channel: metrics     (instance → console, every 30s)
   ├── channel: heartbeat   (instance → console, every 10s)
   ├── channel: logs        (instance → console, streaming)
   ├── channel: terminal    (bidirectional, per-session)
   ├── channel: events      (instance → console, on occurrence)
   └── channel: commands    (console → instance, on demand)
```

### Terminal Multiplexing

When a user opens a terminal in the Console:

1. Console sends `terminal:create` to the instance agent
2. Agent spawns a PTY via `node-pty` (or `creack/pty` in Go)
3. Agent bridges PTY stdin/stdout to a WebSocket channel
4. Console renders the stream in xterm.js
5. Multiple terminals per instance, multiple instances per user
6. Sessions persist across page refreshes (reconnectable)

### Metric Pipeline

```
Instance Agent                    Console Backend              Frontend
──────────────                    ───────────────              ────────
systeminformation ──▶ collect ──▶ WebSocket ──▶ Redis Pub/Sub ──▶ TanStack Query
                      (30s)       ingest         (real-time)      subscription
                                    │                                │
                                    ▼                                ▼
                                 PostgreSQL                     Recharts
                                 (time-series                   (live gauges
                                  retention)                     & graphs)
```

For larger-scale deployments, you might replace PostgreSQL time-series with **TimescaleDB** (PostgreSQL extension) or **VictoriaMetrics** for efficient metric storage and querying.

---

## 8. The Agent Extension

The agent that runs on each Sindri instance would itself be a Sindri extension:

```yaml
# docker/extensions/console-agent/extension.yaml
name: console-agent
version: 1.0.0
description: Sindri Console agent for orchestration and observability
category: tools
install:
  method: binary
  url: https://github.com/pacphi/sindri/releases/download/console-agent-v${version}/agent-${arch}
  binary_name: sindri-agent
depends_on: []
post_install:
  - script: configure-agent.sh # reads console endpoint from sindri.yaml
  - script: start-agent.sh # runs agent as background service
env:
  SINDRI_CONSOLE_URL: ""
  SINDRI_CONSOLE_API_KEY: ""
  SINDRI_AGENT_HEARTBEAT: "30"
  SINDRI_AGENT_METRICS: "60"
bom:
  track: true
```

This means the Console agent is opt-in, installed like any other extension, and follows Sindri's declarative philosophy.

---

## 9. Administrative UI Wireframes

### Instance List View

```
┌─────────────────────────────────────────────────────────────────┐
│ Instances                                    [+ Deploy New]     │
├─────────────────────────────────────────────────────────────────┤
│ Filter: [All Providers ▾] [All Regions ▾] [All Status ▾]  🔍   │
├─────────────────────────────────────────────────────────────────┤
│ ● dev-primary        fly/sea     12 ext    4d 12h   23% CPU    │
│ ● staging-api        k8s/us-e1   8 ext    1d  3h   45% CPU    │
│ ● ml-training        e2b          6 ext    0d  2h   89% CPU    │
│ ○ weekend-project    docker       4 ext    stopped   —         │
│ ● prod-debug         fly/iad     14 ext    0d  8h   12% CPU    │
│ ⚠ ci-runner-03       fly/ord      5 ext    0d  0h   HIGH MEM  │
└─────────────────────────────────────────────────────────────────┘
```

### Deployment Wizard

```
Step 1: Configuration
┌─────────────────────────────────────────┐
│ Deployment Name: [my-instance          ]│
│                                         │
│ ┌─ Provider ──────────────────────────┐ │
│ │ ● Fly.io  ○ Docker  ○ DevPod        │ │
│ │ ○ E2B     ○ Kubernetes  ○ RunPod    │ │
│ └─────────────────────────────────────┘ │
│                                         │
│ ┌─ Profile (optional shortcut) ───────┐ │
│ │ minimal  fullstack  anthropic-dev   │ │
│ │ systems  enterprise  devops  mobile │ │
│ └─────────────────────────────────────┘ │
│                                         │
│ ┌─ Extensions ────────────── 🔍 search ┐│
│ │ [All] languages devops ai-dev mcp   ││
│ │ ☑ nodejs   ☑ python   ☐ golang      ││
│ │ ☐ docker   ☐ claude-cli  ☐ rust     ││
│ └─────────────────────────────────────┘│
│                                         │
│ ┌─ YAML Editor (Monaco) ─────────────┐ │
│ │ name: my-instance                   │ │
│ │ provider: fly                       │ │
│ │ extensions:                         │ │
│ │   - nodejs                          │ │
│ │   - python                          │ │
│ └─────────────────────────────────────┘ │
│                            [Next →]     │
└─────────────────────────────────────────┘

Step 2: Region      (provider pre-selected from Step 1)
Step 3: Resources & Secrets
Step 4: Review & Deploy
```

### Extension Browser

```
┌─────────────────────────────────────────────────────────────────┐
│ Extensions                              [Upload Custom]         │
├──────────────┬──────────────────────────────────────────────────┤
│ Categories   │  claude-code              v1.5.2                 │
│              │  AI coding assistant with MCP support             │
│ ▸ AI (8)     │  Used by: 14 instances  Avg install: 12s        │
│ ▸ Languages  │  Depends on: node-lts                            │
│   (16)       │  [View BOM] [Version History] [Install Graph]   │
│ ▸ Infra (12) │  ────────────────────────────────────────────    │
│ ▸ Databases  │  python3                  v3.12.1                │
│   (6)        │  Python runtime with pip, venv, poetry           │
│ ▸ Tools (28) │  Used by: 22 instances  Avg install: 8s         │
│              │  Depends on: build-essential                     │
│              │  [View BOM] [Version History] [Install Graph]   │
└──────────────┴──────────────────────────────────────────────────┘
```

---

## 10. Security Considerations

- **mTLS** between Console and agents for all communication
- **API key rotation** — automatic rotation with zero-downtime handoff
- **Network isolation** — agents connect outbound to Console; Console never pushes unsolicited to instances
- **Audit logging** — every administrative action logged with user, timestamp, IP, action, target
- **RBAC enforcement** — terminal access requires explicit permission per instance
- **Secret zero problem** — Console API key bootstrapped via environment variable, never stored in sindri.yaml in git
- **Rate limiting** — protect registration and API endpoints
- **CSP headers** — strict Content Security Policy on the web app

---

## 11. Implementation Roadmap

### Phase 1: Foundation (4-6 weeks)

- Instance agent (Go binary) with heartbeat, system metrics, and PTY support
- Console API with instance registry, auth, and WebSocket gateway
- React app with instance list, basic status dashboard, and web terminal (xterm.js)
- Auto-registration flow via `console-agent` extension

### Phase 2: Orchestration (4-6 weeks)

- Deployment wizard with template library and Monaco YAML editor
- Instance lifecycle operations (deploy, clone, suspend, resume, destroy)
- Multi-instance terminal multiplexing
- Command dispatch and parallel execution

### Phase 3: Observability (4-6 weeks)

- Full metrics pipeline with time-series storage
- Fleet overview dashboard with geo visualization
- Instance detail dashboards with real-time charts
- Log aggregation and full-text search
- Alerting engine with webhook/Slack/email delivery

### Phase 4: Administration (4-6 weeks)

- RBAC with team workspaces
- Extension administration and custom extension hosting
- Configuration drift detection
- Cost tracking and right-sizing recommendations
- Security dashboard with BOM/CVE monitoring
- SSO/OIDC integration

---

## 12. Implementation Status (as of February 2026)

All four phases of the roadmap are implemented. The console runs as a Docker Compose stack (`make console-stack-build && make console-stack-up`) or in pnpm dev mode (`make console-dev-full`).

### Phase Completion Summary

| Phase              | Scope                                                                                                                 | Status      |
| ------------------ | --------------------------------------------------------------------------------------------------------------------- | ----------- |
| 1 — Foundation     | Instance registry, API auth, WebSocket gateway, web terminal, React app                                               | ✅ Complete |
| 2 — Orchestration  | Deployment wizard, lifecycle ops (suspend/resume/destroy/clone/redeploy), command dispatch, scheduled tasks           | ✅ Complete |
| 3 — Observability  | Metrics pipeline (TimescaleDB hypertables), fleet dashboard, instance detail charts, log aggregation, alerting engine | ✅ Complete |
| 4 — Administration | RBAC, team workspaces, extension registry, drift detection, cost tracking, security/BOM dashboard                     | ✅ Complete |

### Docker Compose Operational Notes

The following issues were discovered and fixed during initial Docker Compose bring-up. Fixes are committed in the repo.

#### 1. Stale `pnpm-lock.yaml` after dependency version changes

After upgrading `@prisma/client` (pinned to `^6.3.1`) and `@types/recharts` (pinned to `^1.8.29`), the lockfile still referenced older/newer resolved versions. Docker builds use `--frozen-lockfile` and fail when the lockfile diverges from `package.json`.

**Fix:** Run `pnpm install` from `v3/console/` to regenerate the lockfile after any `package.json` version change.

#### 2. Duplicate migration — `20260217000002_scheduled_tasks`

`ScheduledTaskStatus`, `TaskExecutionStatus`, `ScheduledTask`, and `TaskExecution` were already created in `20260217000001_phase2_orchestration`. Migration 002 tried to create them again, causing `prisma migrate deploy` to fail on every container start (including fresh volumes).

**Fix:** Replaced migration 002's SQL with a no-op `SELECT 1;` to preserve migration history without re-executing the duplicate DDL.

#### 3. Wrong postgres image — TimescaleDB required

Migration `20260217000004_timescaledb_metrics` uses `CREATE EXTENSION IF NOT EXISTS timescaledb`, `create_hypertable`, continuous aggregates, retention policies, and compression. The standard `postgres:16-alpine` image does not ship with TimescaleDB.

**Fix:** Changed `docker-compose.yml` postgres image from `postgres:${POSTGRES_VERSION:-16}-alpine` to `timescale/timescaledb:latest-pg16`.

#### 4. `Heartbeat` primary key incompatible with TimescaleDB hypertable conversion

Migration 004 converts the `Heartbeat` table (created in migration 000) to a TimescaleDB hypertable partitioned by `timestamp`. TimescaleDB requires that any unique constraint or primary key on a hypertable includes the partition column. The original `Heartbeat` table had `PRIMARY KEY ("id")` which excludes `timestamp`.

**Fix:** Added DDL to migration 004 to drop `Heartbeat_pkey` and replace it with `PRIMARY KEY ("id", "timestamp")` before calling `create_hypertable`.

#### 5. Frontend authentication not wired up

The web app's `apiFetch` utility sends no auth headers. The API requires `Authorization: Bearer <key>` or `X-Api-Key: <key>` on every request. In Docker Compose mode, the nginx reverse proxy sits between the browser and the API — it is the right place to inject the key without modifying the React app.

**Fix:** Added `proxy_set_header X-Api-Key "sk-admin-dev-seed-key-0001"` to the `/api/` proxy block in `apps/web/nginx.conf`. This authenticates all browser requests as the seeded admin user in the dev stack.

> **Note:** This is a dev/demo-only approach. When a real login flow is implemented (JWT, session, OIDC), this proxy header should be removed and auth should come from the user's session instead.

### Running the Full Stack

```bash
# First time or after dependency changes:
make console-stack-build   # builds api + web Docker images

# Start everything:
make console-stack-up      # postgres (TimescaleDB) + redis + api + web

# Seed development data (only needed once per fresh volume):
make console-db-seed       # seeds users, instances, teams, extensions, costs, security data, etc.

# Access:
#   Web UI: http://localhost:5173
#   API:    http://localhost:3001
#   Health: http://localhost:3001/health

# Tear down (preserves volumes):
make console-stack-down

# Full reset (destroys all data):
docker compose -f v3/console/docker-compose.yml down -v
```

### Seeded Development Credentials

The seed populates these API keys for direct API access (e.g. via curl or API clients):

| User      | Email                  | Raw API Key                  | Role      |
| --------- | ---------------------- | ---------------------------- | --------- |
| Admin     | `admin@sindri.dev`     | `sk-admin-dev-seed-key-0001` | ADMIN     |
| Developer | `developer@sindri.dev` | `sk-dev-seed-key-0001`       | DEVELOPER |
| Developer | `developer@sindri.dev` | `sk-dev-gh-seed-key-0002`    | DEVELOPER |

In the Docker Compose stack, the web UI automatically authenticates as admin via the nginx proxy injection. No manual key entry is required.

### Deployment Wizard: Step 1 Redesign (February 2026)

The Step 1 "Configuration" page was redesigned to align with the Sindri V3 CLI workflow:

| Before | After |
|--------|-------|
| Hardcoded persona templates (Node.js Dev, Python ML, Go Backend) | Provider selector driving `config init`-style YAML generation |
| Template-driven YAML population | Extensions multi-select from live registry (`/api/v1/extensions`) |
| No extension browsing in wizard | Profile shortcuts from new `/api/v1/profiles` endpoint |
| Provider selection deferred to Step 2 | Provider selection on Step 1; Step 2 shows region only |

New backend endpoint added: `GET /api/v1/profiles` (static, sourced from `v3/profiles.yaml`).

### CLI Integration Architecture (February 2026)

The Console API now treats the Sindri CLI as a first-class runtime dependency.
Instead of maintaining stale hardcoded TypeScript constants for extensions,
profiles, and providers, the API shells out to the `sindri` binary using
`child_process.execFile` and returns the live JSON output.

**New endpoint group: `GET /api/v1/registry/*`**

| Endpoint | CLI command |
|----------|-------------|
| `GET /api/v1/registry/extensions` | `sindri extension list --all --json` |
| `GET /api/v1/registry/extensions/categories` | derived from extension list |
| `GET /api/v1/registry/profiles` | `sindri profile list --json` + `profile info` |
| `GET /api/v1/registry/version` | `sindri version --json` |

**`GET /api/v1/profiles`** now calls the CLI and falls back to a static snapshot when
the binary is unavailable — ensuring the wizard remains functional at all times.

Binary delivery supports two modes:
- **Dev**: volume-mount `v3/target/release/sindri` via docker-compose (`../../target/release/sindri:/usr/local/bin/sindri:ro`)
- **Production**: download pinned release binary at Docker build time (`ARG SINDRI_VERSION`)

Fallback chain: `SINDRI_BIN_PATH` env → `./node_modules/.bin/sindri` → `sindri` on PATH → graceful `{ error: "CLI_UNAVAILABLE", fallback: true }`.

---

## 13. CLI npm Package Distribution (Phase 2 Roadmap)

The Sindri CLI (Rust binary) will be distributed as an npm package `@sindri/cli`
using the **optionalDependencies + platform packages** pattern (same as esbuild, Biome, SWC).

### Package layout (`v3/console/packages/`)

| Package | OS/CPU | Contents |
|---------|--------|----------|
| `@sindri/cli` | wrapper | `optionalDependencies` + resolver script |
| `@sindri/cli-darwin-arm64` | macOS/ARM64 | `sindri` binary |
| `@sindri/cli-darwin-x64` | macOS/x64 | `sindri` binary |
| `@sindri/cli-linux-x64` | Linux/x64 | `sindri` binary |
| `@sindri/cli-linux-arm64` | Linux/ARM64 | `sindri` binary |
| `@sindri/cli-win32-x64` | Windows/x64 | `sindri.exe` binary |

### Version alignment

The npm packages are versioned identically to the Rust binary (`Cargo.toml` is
the source of truth). **cargo-dist** automates multi-platform builds and npm
publishing triggered by `git tag v3.x.y`.

### API integration

Once `@sindri/cli` is published, `apps/api/package.json` adds it as a regular
dependency. `lib/cli.ts` resolves the binary via `require.resolve('@sindri/cli/bin/sindri')`.
No system-level binary installation is needed in the console Docker image.

### Instance version awareness (Phase 3)

Sindri instances will report `sindri_version` in heartbeat payloads. The console
will store this on the `Instance` model and surface compatibility badges in the UI:
- Green: same minor version as console CLI
- Yellow: older patch/minor (minor feature gaps possible)
- Red: major version mismatch (API calls may fail)

A `GET /api/v1/version` endpoint will expose:
```json
{
  "console_api": "0.1.0",
  "sindri_cli": "3.0.1",
  "cli_target": "x86_64-apple-darwin",
  "min_instance_version": "3.0.0"
}
```

---

## 15. Why This Fits Sindri's Philosophy

Sindri's ethos is "define once, deploy anywhere." The Console extends this to "define once, deploy anywhere, **observe everywhere**." It preserves the declarative nature — the Console doesn't replace the CLI or the YAML-driven workflow. Instead it:

- **Complements** the CLI for users who prefer visual interfaces
- **Aggregates** what was previously invisible across isolated instances
- **Enables** workflows impossible from a single terminal (fleet management, cross-instance comparison, collaborative debugging)
- **Dogfoods** Sindri itself by running as a Sindri-deployed application

The Console turns Sindri from a powerful individual tool into a platform.
