# Sindri Console - Development Setup

## Prerequisites

| Tool | Version | Install |
|---|---|---|
| Node.js | 22 LTS | https://nodejs.org or `nvm install 22` |
| pnpm | 9+ | `npm install -g pnpm` |
| Go | 1.22+ | https://go.dev/dl/ |
| Docker | 24+ | https://docs.docker.com/get-docker/ |
| Docker Compose | 2.20+ | Bundled with Docker Desktop |
| PostgreSQL client | Any | `brew install libpq` (macOS) or `apt install postgresql-client` |

---

## Repository Layout

The Console lives under `v3/console/` in the Sindri monorepo:

```
v3/console/
+-- apps/
|   +-- api/          # Node.js Console API (Hono + Prisma)
|   +-- web/          # React 19 SPA (Vite)
+-- agent/            # Go instance agent (cmd/agent, internal/, pkg/)
+-- docs/             # Architecture and API documentation
```

---

## Quick Start

### 1. Install Node.js dependencies

```bash
cd v3/console
pnpm install
```

### 2. Start infrastructure services

```bash
docker compose up -d postgres redis
```

This starts:
- PostgreSQL 16 on `localhost:5432` (database: `sindri_console`, user: `sindri`, password: `sindri`)
- Redis 7 on `localhost:6379`

### 3. Configure environment

```bash
cp apps/api/.env.example apps/api/.env
```

Edit `apps/api/.env`:

```env
DATABASE_URL="postgresql://sindri:sindri@localhost:5432/sindri_console"
REDIS_URL="redis://localhost:6379"

# JWT signing secret (generate with: openssl rand -hex 32)
JWT_SECRET="change-me-in-production"

# Bootstrap API key for agent registration
SINDRI_CONSOLE_API_KEY="dev-api-key-change-me"

# Session cookie secret (generate with: openssl rand -hex 32)
SESSION_SECRET="change-me-in-production"

NODE_ENV="development"
PORT=3000
```

### 4. Run database migrations

```bash
cd apps/api
npx prisma migrate dev
npx prisma generate
```

This creates all tables and generates the Prisma TypeScript client.

### 5. Seed the database (optional)

```bash
cd apps/api
npx prisma db seed
```

Seeds one admin user (`admin@example.com` / `admin`) and a sample team.

### 6. Start the development servers

In separate terminals (or use tmux):

**Console API:**

```bash
# Recommended: run in tmux for log access
tmux new-session -d -s api "cd v3/console && pnpm --filter=api run start:dev"
tmux attach -t api
```

**React frontend:**

```bash
# Recommended: run in tmux for log access
tmux new-session -d -s web "cd v3/console && pnpm --filter=web run start:dev"
tmux attach -t web
```

API server starts at `http://localhost:3000`.
Frontend starts at `http://localhost:5173` with Vite HMR.

### 7. Build the Go agent (optional for local development)

```bash
cd v3/console/agent
go build -o bin/sindri-agent ./cmd/agent
```

Run the agent against your local Console:

```bash
SINDRI_CONSOLE_URL=http://localhost:3000 \
SINDRI_CONSOLE_API_KEY=dev-api-key-change-me \
SINDRI_PROVIDER=docker \
./bin/sindri-agent
```

---

## Docker Compose Reference

### `docker-compose.yml`

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: sindri_console
      POSTGRES_USER: sindri
      POSTGRES_PASSWORD: sindri
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U sindri"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
  redis_data:
```

---

## Environment Variable Reference

### Console API (`apps/api/.env`)

| Variable | Required | Description | Default |
|---|---|---|---|
| `DATABASE_URL` | Yes | PostgreSQL connection string | — |
| `REDIS_URL` | Yes | Redis connection URL | — |
| `JWT_SECRET` | Yes | HMAC secret for JWT signing | — |
| `SESSION_SECRET` | Yes | Secret for session cookies | — |
| `SINDRI_CONSOLE_API_KEY` | Yes | Bootstrap key for agent registration | — |
| `PORT` | No | HTTP server port | `3000` |
| `NODE_ENV` | No | `development` or `production` | `development` |
| `LOG_LEVEL` | No | `debug`, `info`, `warn`, `error` | `info` |
| `METRICS_RETENTION_DAYS` | No | Days to keep metric records | `30` |
| `LOGS_RETENTION_DAYS` | No | Days to keep log records | `14` |
| `CORS_ORIGIN` | No | Allowed CORS origin for browser requests | `http://localhost:5173` |

### React Frontend (`apps/web/.env`)

| Variable | Required | Description | Default |
|---|---|---|---|
| `VITE_API_URL` | No | Console API URL | `http://localhost:3000` |
| `VITE_WS_URL` | No | WebSocket URL | `ws://localhost:3000` |

---

## Development Workflow

### Running tests

```bash
# API unit tests
pnpm --filter=api run test

# Frontend component tests
pnpm --filter=web run test

# All tests
pnpm run test
```

### Type checking

```bash
pnpm run typecheck
```

### Linting

```bash
pnpm run lint
```

### Formatting

```bash
pnpm run format
```

### Prisma operations

```bash
# Open Prisma Studio (database GUI)
cd apps/api && npx prisma studio

# Create a new migration after schema changes
cd apps/api && npx prisma migrate dev --name <description>

# Reset and re-seed development database
cd apps/api && npx prisma migrate reset
```

### Go agent development

```bash
cd v3/console/agent

# Run tests
go test ./...

# Build for current platform
go build -o bin/sindri-agent ./cmd/agent
```

---

## Turborepo

The monorepo uses Turborepo for task orchestration:

```bash
# Build all packages
pnpm run build

# Run all tests
pnpm run test
```

`turbo.json` configures caching and dependency ordering between packages.

---

## Connecting a Test Instance

To test the full registration and heartbeat flow without a real Sindri deployment, run the agent in mock mode:

```bash
cd packages/agent
go run ./cmd/agent --mock --console-url http://localhost:3000 --api-key dev-api-key-change-me
```

In mock mode the agent generates simulated CPU/memory/disk metrics and sends heartbeats without requiring a real Sindri environment.

---

## Troubleshooting

### PostgreSQL connection refused

Ensure the container is running:

```bash
docker compose ps postgres
docker compose logs postgres
```

If healthy, verify the `DATABASE_URL` in `apps/api/.env` matches the Docker Compose config.

### Prisma client out of date

After any schema change, regenerate the client:

```bash
cd apps/api && npx prisma generate
```

### Redis connection errors

```bash
docker compose ps redis
docker compose logs redis
```

Test connectivity: `redis-cli -u redis://localhost:6379 ping` (should return `PONG`).

### Port conflicts

If ports 3000, 5432, or 6379 are in use, edit `docker-compose.yml` host port mappings and update `.env` accordingly.

### Go build errors

Ensure Go 1.22+ is installed:

```bash
go version
```

If modules are stale:

```bash
cd v3/console/agent && go mod tidy
```

---

## IDE Setup

### VS Code

Recommended extensions (add to `.vscode/extensions.json`):
- `dbaeumer.vscode-eslint` - ESLint
- `esbenp.prettier-vscode` - Prettier
- `prisma.prisma` - Prisma schema support
- `golang.go` - Go language support
- `bradlc.vscode-tailwindcss` - Tailwind CSS IntelliSense

Workspace settings (`.vscode/settings.json`):

```json
{
  "editor.defaultFormatter": "esbenp.prettier-vscode",
  "editor.formatOnSave": true,
  "[go]": {
    "editor.defaultFormatter": "golang.go"
  }
}
```

---

## Production Deployment

For production, the Console can be deployed as a Sindri instance itself:

```yaml
# sindri.yaml (example for Fly.io)
name: sindri-console
extensions:
  - node-lts
  - postgresql-client
  - redis-cli
provider:
  fly:
    region: sea
    vm_size: shared-cpu-2x
    memory: 1024
console:
  enabled: false
```

Or via Docker:

```bash
docker build -t sindri-console .
docker run -p 3000:3000 --env-file .env.production sindri-console
```

See the Fly.io deployment guide in `docs/providers/FLY.md` for full production setup instructions.
