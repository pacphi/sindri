# Sindri Console — Database Schema

## Overview

> **Implementation note:** The Prisma schema at `apps/api/prisma/schema.prisma` implements the Phase 1 minimal model set (6 models). The fuller schema shown below reflects the full target design for later phases. The Phase 1 schema is the authoritative source for what is currently deployed.

The Console uses **PostgreSQL 16** as its primary database, accessed via **Prisma ORM**. The schema covers:

- Instance registry
- Metric time series
- User management and RBAC
- API key management
- Audit log
- Event log

For Phase 1, all time-series data is stored in a standard PostgreSQL table with a `timestamp` index. A future migration to **TimescaleDB** (PostgreSQL extension) can be applied transparently once data volumes warrant it.

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
