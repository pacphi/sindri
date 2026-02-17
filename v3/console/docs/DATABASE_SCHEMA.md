# Sindri Console — Database Schema

## Overview

> **Implementation note:** The Prisma schema at `apps/api/prisma/schema.prisma` is the authoritative source. The schema has grown across three phases and includes all models listed below.

The Console uses **PostgreSQL 16** as its primary database, accessed via **Prisma ORM**. The schema covers:

- Instance registry
- Metric time series (Phase 3: full-fidelity `Metric` hypertable)
- Heartbeat liveness table
- User management and RBAC
- API key management
- Event log
- Terminal sessions
- Deployment templates and deployments
- Scheduled tasks and task executions
- Command executions
- Log entries (Phase 3)
- Alert rules and alert events (Phase 3)

The `Metric` table is designed as a **TimescaleDB hypertable** partitioned by `timestamp`. On standard PostgreSQL it functions as a regular indexed table; the TimescaleDB migration is non-breaking.

---

## Phase 3 Models

### Metric

Full-fidelity per-collection metric snapshot.

| Field | Type | Description |
|-------|------|-------------|
| `id` | String (CUID) | Primary key |
| `instance_id` | String | FK → Instance (CASCADE delete) |
| `timestamp` | DateTime | Collection time (TimescaleDB partition key) |
| `cpu_percent` | Float | Overall CPU 0–100 |
| `load_avg_1/5/15` | Float? | 1, 5, 15-min load averages |
| `cpu_steal` | Float? | Hypervisor steal percent |
| `core_count` | Int? | Number of CPU cores |
| `mem_used` | BigInt | Bytes used |
| `mem_total` | BigInt | Total bytes |
| `mem_cached` | BigInt? | Page cache bytes |
| `swap_used/total` | BigInt? | Swap utilization |
| `disk_used/total` | BigInt | Primary volume bytes |
| `disk_read/write_bps` | BigInt? | I/O throughput bytes/s |
| `net_bytes_sent/recv` | BigInt? | Cumulative bytes since agent start |
| `net_packets_sent/recv` | BigInt? | Cumulative packet counts |

Indexes: `(instance_id, timestamp)`, `(timestamp)`

### LogEntry

Structured log line from an instance agent.

| Field | Type | Description |
|-------|------|-------------|
| `id` | String (CUID) | Primary key |
| `instance_id` | String | FK → Instance |
| `timestamp` | DateTime | Log line time |
| `level` | Enum | `DEBUG`, `INFO`, `WARN`, `ERROR` |
| `source` | Enum | `AGENT`, `EXTENSION`, `BUILD`, `APP`, `SYSTEM` |
| `message` | String | Log message text (searchable) |
| `metadata` | Json? | Structured context (requestId, statusCode, etc.) |

Indexes: `(instance_id, timestamp)`, `(level)`, `(source)`, `(timestamp)`

### AlertRule

Configurable metric threshold alert rule.

| Field | Type | Description |
|-------|------|-------------|
| `id` | String (CUID) | Primary key |
| `name` | String | Rule name |
| `description` | String? | Optional description |
| `instance_id` | String? | Target instance (null = fleet-wide) |
| `conditions` | Json | Array of `{ metric, op, threshold }` |
| `condition_operator` | String | `AND` or `OR` |
| `severity` | String | `info`, `warning`, `critical` |
| `evaluation_window_sec` | Int | Seconds of data to evaluate |
| `pending_for_sec` | Int | Must stay firing for N sec before alerting |
| `cooldown_sec` | Int | Min seconds between repeat notifications |
| `notify_channels` | String[] | `email`, `webhook`, `slack` |
| `notify_emails` | String[] | Email recipients |
| `webhook_url` | String? | Webhook endpoint URL |
| `enabled` | Boolean | Toggle rule without deleting |
| `created_at` | DateTime | Creation time |
| `updated_at` | DateTime | Last modified |

### AlertEvent

Individual alert state transition record.

| Field | Type | Description |
|-------|------|-------------|
| `id` | String (CUID) | Primary key |
| `rule_id` | String | FK → AlertRule |
| `instance_id` | String | Instance this fired for |
| `state` | String | `INACTIVE`, `PENDING`, `FIRING`, `RESOLVED` |
| `severity` | String | Copied from rule at fire time |
| `trigger_value` | Float | Metric value that triggered the alert |
| `trigger_metric` | String | Which metric triggered |
| `message` | String | Human-readable alert message |
| `fired_at` | DateTime? | When state entered FIRING |
| `resolved_at` | DateTime? | When state entered RESOLVED |
| `notifications_sent` | Int | Count of dispatched notifications |

Indexes: `(rule_id)`, `(instance_id)`, `(state)`, `(fired_at)`

---

## Prisma Schema

File: `apps/api/prisma/schema.prisma`

```prisma
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

enum Provider {
  fly
  docker
  devpod
  e2b
  kubernetes
}

enum InstanceStatus {
  running
  stopped
  error
  unknown
}

enum Role {
  admin
  operator
  developer
  viewer
}

enum EventType {
  deploy
  redeploy
  connect
  disconnect
  backup
  destroy
  extension_install
  extension_error
}

enum LogSource {
  init
  extension
  system
  agent
}

enum LogLevel {
  info
  warn
  error
}

// ---------------------------------------------------------------------------
// Teams
// ---------------------------------------------------------------------------

model Team {
  id        String   @id @default(uuid())
  name      String
  slug      String   @unique
  createdAt DateTime @default(now())
  updatedAt DateTime @updatedAt

  users     User[]
  instances Instance[]
  apiKeys   ApiKey[]
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

model User {
  id           String   @id @default(uuid())
  email        String   @unique
  passwordHash String
  role         Role     @default(developer)
  teamId       String
  createdAt    DateTime @default(now())
  updatedAt    DateTime @updatedAt

  team         Team       @relation(fields: [teamId], references: [id])
  sessions     Session[]
  auditEntries AuditEntry[]

  @@index([teamId])
  @@index([email])
}

// ---------------------------------------------------------------------------
// Sessions (browser auth)
// ---------------------------------------------------------------------------

model Session {
  id        String   @id @default(uuid())
  userId    String
  token     String   @unique
  ip        String
  userAgent String
  expiresAt DateTime
  createdAt DateTime @default(now())

  user      User @relation(fields: [userId], references: [id], onDelete: Cascade)

  @@index([userId])
  @@index([token])
  @@index([expiresAt])
}

// ---------------------------------------------------------------------------
// API Keys
// ---------------------------------------------------------------------------

model ApiKey {
  id        String    @id @default(uuid())
  name      String
  keyHash   String    @unique        // bcrypt hash of the full key
  prefix    String                   // first 8 chars for display, e.g. "sk-abcd"
  teamId    String
  createdBy String
  lastUsed  DateTime?
  createdAt DateTime  @default(now())
  revokedAt DateTime?

  team      Team @relation(fields: [teamId], references: [id])

  @@index([teamId])
  @@index([keyHash])
}

// ---------------------------------------------------------------------------
// Instances
// ---------------------------------------------------------------------------

model Instance {
  id           String         @id @default(uuid())
  name         String
  provider     Provider
  region       String
  status       InstanceStatus @default(unknown)
  yamlHash     String?
  sshEndpoint  String?
  agentVersion String?
  teamId       String
  createdAt    DateTime       @default(now())
  updatedAt    DateTime       @updatedAt
  lastHeartbeat DateTime?

  team         Team              @relation(fields: [teamId], references: [id])
  extensions   InstanceExtension[]
  metrics      Metric[]
  events       Event[]
  logs         Log[]
  bom          BomEntry[]
  auditEntries AuditEntry[]
  termSessions TerminalSession[]

  @@index([teamId])
  @@index([provider])
  @@index([status])
  @@index([lastHeartbeat])
}

// ---------------------------------------------------------------------------
// Instance Extensions (many-to-many via junction)
// ---------------------------------------------------------------------------

model InstanceExtension {
  id           String   @id @default(uuid())
  instanceId   String
  extensionName String
  version      String?
  installedAt  DateTime @default(now())

  instance     Instance @relation(fields: [instanceId], references: [id], onDelete: Cascade)

  @@unique([instanceId, extensionName])
  @@index([instanceId])
  @@index([extensionName])
}

// ---------------------------------------------------------------------------
// Metrics (time series)
// ---------------------------------------------------------------------------

model Metric {
  id            String   @id @default(uuid())
  instanceId    String
  timestamp     DateTime @default(now())

  // CPU
  cpuPercent    Float?
  loadAvg1      Float?
  loadAvg5      Float?
  loadAvg15     Float?
  cpuSteal      Float?

  // Memory
  memUsed       BigInt?
  memTotal      BigInt?
  memCached     BigInt?
  swapUsed      BigInt?
  swapTotal     BigInt?

  // Disk
  diskUsed      BigInt?
  diskTotal     BigInt?
  diskReadBps   BigInt?
  diskWriteBps  BigInt?

  // Network
  netBytesIn    BigInt?
  netBytesOut   BigInt?
  netConnections Int?

  instance      Instance @relation(fields: [instanceId], references: [id], onDelete: Cascade)

  @@index([instanceId, timestamp])
  @@index([timestamp])
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

model Event {
  id         String    @id @default(uuid())
  instanceId String
  type       EventType
  timestamp  DateTime  @default(now())
  metadata   Json?

  instance   Instance @relation(fields: [instanceId], references: [id], onDelete: Cascade)

  @@index([instanceId, timestamp])
  @@index([type])
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

model Log {
  id         String    @id @default(uuid())
  instanceId String
  timestamp  DateTime  @default(now())
  source     LogSource
  level      LogLevel
  message    String

  instance   Instance @relation(fields: [instanceId], references: [id], onDelete: Cascade)

  @@index([instanceId, timestamp])
  @@index([level])
}

// ---------------------------------------------------------------------------
// Bill of Materials entries
// ---------------------------------------------------------------------------

model BomEntry {
  id           String   @id @default(uuid())
  instanceId   String
  packageName  String
  version      String
  source       String   // apt, pip, npm, cargo, binary
  hash         String?
  recordedAt   DateTime @default(now())

  instance     Instance @relation(fields: [instanceId], references: [id], onDelete: Cascade)

  @@unique([instanceId, packageName, version])
  @@index([instanceId])
  @@index([packageName])
}

// ---------------------------------------------------------------------------
// Audit Log
// ---------------------------------------------------------------------------

model AuditEntry {
  id         String   @id @default(uuid())
  userId     String
  instanceId String?
  action     String
  target     String?
  targetId   String?
  ip         String
  timestamp  DateTime @default(now())
  metadata   Json?

  user       User      @relation(fields: [userId], references: [id])
  instance   Instance? @relation(fields: [instanceId], references: [id])

  @@index([userId])
  @@index([instanceId])
  @@index([timestamp])
  @@index([action])
}

// ---------------------------------------------------------------------------
// Terminal Sessions
// ---------------------------------------------------------------------------

model TerminalSession {
  id          String    @id @default(uuid())
  instanceId  String
  userId      String
  cols        Int       @default(220)
  rows        Int       @default(50)
  createdAt   DateTime  @default(now())
  closedAt    DateTime?

  instance    Instance @relation(fields: [instanceId], references: [id], onDelete: Cascade)

  @@index([instanceId])
  @@index([userId])
}
```

---

## Table Relationships

```
Team (1) ---< User (many)
Team (1) ---< Instance (many)
Team (1) ---< ApiKey (many)

User (1) ---< Session (many)
User (1) ---< AuditEntry (many)

Instance (1) ---< InstanceExtension (many)
Instance (1) ---< Metric (many)
Instance (1) ---< Event (many)
Instance (1) ---< Log (many)
Instance (1) ---< BomEntry (many)
Instance (1) ---< AuditEntry (many)
Instance (1) ---< TerminalSession (many)
```

---

## Index Strategy

### Performance-Critical Indexes

| Table | Index | Rationale |
|---|---|---|
| `Metric` | `(instanceId, timestamp)` | Primary query pattern: metrics for a given instance over a time range |
| `Metric` | `(timestamp)` | Fleet-wide metric queries |
| `Event` | `(instanceId, timestamp)` | Recent events per instance |
| `Log` | `(instanceId, timestamp)` | Log tailing per instance |
| `AuditEntry` | `(timestamp)` | Audit log pagination |
| `Instance` | `(lastHeartbeat)` | Identify stale instances (heartbeat monitor) |
| `Session` | `(expiresAt)` | Cleanup job index for expired sessions |
| `User` | `(email)` | Login lookup |
| `ApiKey` | `(keyHash)` | API key verification |

### Unique Constraints

| Table | Unique | Rationale |
|---|---|---|
| `User` | `email` | One account per email |
| `Team` | `slug` | URL-safe team identifier |
| `Session` | `token` | Each session token is globally unique |
| `ApiKey` | `keyHash` | Each key is unique |
| `InstanceExtension` | `(instanceId, extensionName)` | An extension is installed once per instance |
| `BomEntry` | `(instanceId, packageName, version)` | BOM deduplication |

---

## Data Retention Policy

| Table | Default Retention | Configurable |
|---|---|---|
| `Metric` | 30 days | Yes (env: `METRICS_RETENTION_DAYS`) |
| `Log` | 14 days | Yes (env: `LOGS_RETENTION_DAYS`) |
| `Event` | 90 days | Yes (env: `EVENTS_RETENTION_DAYS`) |
| `AuditEntry` | 1 year | Yes (env: `AUDIT_RETENTION_DAYS`) |
| `Session` | Until expiry | No |
| `TerminalSession` | 90 days | Yes |

A BullMQ worker runs nightly to purge rows older than the configured retention window.

---

## Migration Strategy

Migrations are managed via Prisma Migrate:

```bash
# Create a new migration
npx prisma migrate dev --name <description>

# Apply migrations in production
npx prisma migrate deploy

# Reset development database
npx prisma migrate reset
```

Migrations are tracked in `apps/api/prisma/migrations/` and committed to version control.

Initial migration creates all tables listed above with all indexes.

---

## Future: TimescaleDB

For deployments with >100 instances or >30-day metric retention, the `Metric` table can be converted to a TimescaleDB hypertable with minimal code changes:

```sql
-- One-time migration (applied after TimescaleDB extension install)
SELECT create_hypertable('Metric', 'timestamp');
SELECT add_compression_policy('Metric', INTERVAL '7 days');
SELECT add_retention_policy('Metric', INTERVAL '30 days');
```

The Prisma client continues to work unchanged; only the storage engine changes.

---

## Environment Variables

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | required |
| `METRICS_RETENTION_DAYS` | Days to keep metric rows | `30` |
| `LOGS_RETENTION_DAYS` | Days to keep log rows | `14` |
| `EVENTS_RETENTION_DAYS` | Days to keep event rows | `90` |
| `AUDIT_RETENTION_DAYS` | Days to keep audit entries | `365` |
