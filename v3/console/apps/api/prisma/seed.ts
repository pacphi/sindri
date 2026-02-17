/**
 * Prisma seed script for Sindri Console development data.
 *
 * Run with: npx prisma db seed
 * (configured in package.json under "prisma.seed")
 */

import {
  PrismaClient,
  InstanceStatus,
  UserRole,
  EventType,
  TerminalSessionStatus,
  ScheduledTaskStatus,
  TaskExecutionStatus,
} from "@prisma/client";
import * as crypto from "crypto";

const prisma = new PrismaClient();

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function sha256(value: string): string {
  return crypto.createHash("sha256").update(value).digest("hex");
}

/** Produce a fake bcrypt-shaped placeholder for seed passwords (NOT real bcrypt). */
function fakeBcrypt(password: string): string {
  // In a real app use bcrypt.hashSync(password, 12)
  return `$2b$12$seed_placeholder_${sha256(password).slice(0, 22)}`;
}

function mbToBytes(mb: number): bigint {
  return BigInt(mb) * 1024n * 1024n;
}

function gbToBytes(gb: number): bigint {
  return mbToBytes(gb * 1024);
}

function hoursToSeconds(hours: number): bigint {
  return BigInt(hours) * 3600n;
}

// ─────────────────────────────────────────────────────────────────────────────
// Seed data
// ─────────────────────────────────────────────────────────────────────────────

async function seedUsers() {
  console.log("Seeding users...");

  const users = [
    {
      id: "user_admin_01",
      email: "admin@sindri.dev",
      password_hash: fakeBcrypt("admin-secret-change-me"),
      role: UserRole.ADMIN,
    },
    {
      id: "user_operator_01",
      email: "operator@sindri.dev",
      password_hash: fakeBcrypt("operator-secret-change-me"),
      role: UserRole.OPERATOR,
    },
    {
      id: "user_dev_01",
      email: "developer@sindri.dev",
      password_hash: fakeBcrypt("developer-secret-change-me"),
      role: UserRole.DEVELOPER,
    },
    {
      id: "user_viewer_01",
      email: "viewer@sindri.dev",
      password_hash: fakeBcrypt("viewer-secret-change-me"),
      role: UserRole.VIEWER,
    },
  ];

  for (const user of users) {
    await prisma.user.upsert({
      where: { email: user.email },
      update: {},
      create: user,
    });
  }

  console.log(`  Created ${users.length} users`);
  return users;
}

async function seedApiKeys(users: Array<{ id: string; email: string }>) {
  console.log("Seeding API keys...");

  const rawKeys = [
    {
      userId: users[0].id,
      name: "Admin CI Key",
      raw: "sk-admin-dev-seed-key-0001",
      daysToExpire: null,
    },
    {
      userId: users[2].id,
      name: "Developer Local Key",
      raw: "sk-dev-seed-key-0001",
      daysToExpire: 90,
    },
    {
      userId: users[2].id,
      name: "Developer GitHub Actions",
      raw: "sk-dev-gh-seed-key-0002",
      daysToExpire: 365,
    },
  ];

  for (const k of rawKeys) {
    const expiresAt = k.daysToExpire ? new Date(Date.now() + k.daysToExpire * 86400 * 1000) : null;

    await prisma.apiKey.upsert({
      where: { key_hash: sha256(k.raw) },
      update: {},
      create: {
        user_id: k.userId,
        key_hash: sha256(k.raw),
        name: k.name,
        expires_at: expiresAt,
      },
    });
  }

  console.log(`  Created ${rawKeys.length} API keys`);
}

async function seedInstances() {
  console.log("Seeding instances...");

  const instances = [
    {
      id: "inst_fly_sea_01",
      name: "dev-primary",
      provider: "fly",
      region: "sea",
      extensions: ["node-lts", "python3", "docker-in-docker", "claude-code", "git", "zsh", "tmux"],
      config_hash: sha256("dev-primary-sindri-yaml-v1"),
      ssh_endpoint: "dev-primary.fly.dev:22",
      status: InstanceStatus.RUNNING,
      created_at: new Date(Date.now() - 4 * 24 * 60 * 60 * 1000), // 4 days ago
    },
    {
      id: "inst_k8s_use1_01",
      name: "staging",
      provider: "kubernetes",
      region: "us-east-1",
      extensions: ["node-lts", "python3", "kubectl", "helm", "git"],
      config_hash: sha256("staging-sindri-yaml-v2"),
      ssh_endpoint: "staging.k8s.internal:22",
      status: InstanceStatus.RUNNING,
      created_at: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000), // 1 day ago
    },
    {
      id: "inst_e2b_01",
      name: "ml-sandbox",
      provider: "e2b",
      region: null,
      extensions: ["python3", "pytorch", "jupyter"],
      config_hash: sha256("ml-sandbox-sindri-yaml-v1"),
      ssh_endpoint: null,
      status: InstanceStatus.STOPPED,
      created_at: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000), // 7 days ago
    },
    {
      id: "inst_docker_01",
      name: "local-dev",
      provider: "docker",
      region: null,
      extensions: ["node-lts", "rust", "git", "zsh"],
      config_hash: sha256("local-dev-sindri-yaml-v3"),
      ssh_endpoint: "localhost:2222",
      status: InstanceStatus.RUNNING,
      created_at: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000), // 2 days ago
    },
    {
      id: "inst_fly_iad_01",
      name: "ci-runner-03",
      provider: "fly",
      region: "iad",
      extensions: ["node-lts", "docker-in-docker", "git"],
      config_hash: sha256("ci-runner-sindri-yaml-v1"),
      ssh_endpoint: "ci-runner-03.fly.dev:22",
      status: InstanceStatus.ERROR,
      created_at: new Date(Date.now() - 12 * 60 * 60 * 1000), // 12 hours ago
    },
  ];

  for (const inst of instances) {
    await prisma.instance.upsert({
      where: { id: inst.id },
      update: { status: inst.status },
      create: inst,
    });
  }

  console.log(`  Created ${instances.length} instances`);
  return instances;
}

async function seedHeartbeats(
  instances: Array<{ id: string; name: string; status: InstanceStatus }>,
) {
  console.log("Seeding heartbeats...");

  const runningInstances = instances.filter((i) => i.status === InstanceStatus.RUNNING);
  let count = 0;

  for (const inst of runningInstances) {
    // Seed 10 heartbeats per running instance, spaced 30 seconds apart
    const heartbeats = Array.from({ length: 10 }, (_, idx) => ({
      instance_id: inst.id,
      timestamp: new Date(Date.now() - (10 - idx) * 30 * 1000),
      cpu_percent: 10 + Math.random() * 40,
      memory_used: mbToBytes(300 + Math.floor(Math.random() * 400)),
      memory_total: gbToBytes(1),
      disk_used: gbToBytes(5 + Math.floor(Math.random() * 10)),
      disk_total: gbToBytes(50),
      uptime: hoursToSeconds(4 * 24 + idx),
    }));

    await prisma.heartbeat.createMany({ data: heartbeats, skipDuplicates: true });
    count += heartbeats.length;
  }

  console.log(`  Created ${count} heartbeats`);
}

async function seedEvents(instances: Array<{ id: string }>) {
  console.log("Seeding events...");

  const eventSets = instances.flatMap((inst) => [
    {
      instance_id: inst.id,
      event_type: EventType.DEPLOY,
      timestamp: new Date(Date.now() - 4 * 24 * 60 * 60 * 1000),
      metadata: { triggered_by: "cli", sindri_version: "3.0.0", duration_seconds: 47 },
    },
    {
      instance_id: inst.id,
      event_type: EventType.CONNECT,
      timestamp: new Date(Date.now() - 2 * 60 * 60 * 1000),
      metadata: { user: "developer@sindri.dev", ip: "192.168.1.100" },
    },
    {
      instance_id: inst.id,
      event_type: EventType.DISCONNECT,
      timestamp: new Date(Date.now() - 1 * 60 * 60 * 1000),
      metadata: { user: "developer@sindri.dev", session_duration_seconds: 3600 },
    },
  ]);

  await prisma.event.createMany({ data: eventSets, skipDuplicates: false });
  console.log(`  Created ${eventSets.length} events`);
}

async function seedTerminalSessions(
  instances: Array<{ id: string; status: InstanceStatus }>,
  users: Array<{ id: string }>,
) {
  console.log("Seeding terminal sessions...");

  const runningInstances = instances.filter((i) => i.status === InstanceStatus.RUNNING);
  const devUser = users[2]; // developer user

  const sessions = runningInstances.map((inst) => ({
    instance_id: inst.id,
    user_id: devUser.id,
    started_at: new Date(Date.now() - 90 * 60 * 1000),
    ended_at: new Date(Date.now() - 30 * 60 * 1000),
    status: TerminalSessionStatus.CLOSED,
  }));

  // Add one active session on the first running instance
  if (runningInstances.length > 0) {
    sessions.push({
      instance_id: runningInstances[0].id,
      user_id: devUser.id,
      started_at: new Date(Date.now() - 15 * 60 * 1000),
      ended_at: null as unknown as Date,
      status: TerminalSessionStatus.ACTIVE,
    });
  }

  await prisma.terminalSession.createMany({ data: sessions, skipDuplicates: false });
  console.log(`  Created ${sessions.length} terminal sessions`);
}

async function seedDeploymentTemplates() {
  console.log("Seeding deployment templates...");

  const templates = [
    {
      id: "tmpl_python_ml",
      name: "Python ML Stack",
      slug: "python-ml",
      category: "ai",
      description: "Python 3 with PyTorch, Jupyter, and Claude Code for ML experimentation.",
      yaml_content: [
        "name: ml-workspace",
        "extensions:",
        "  - python3",
        "  - pytorch",
        "  - jupyter",
        "  - claude-code",
        "  - git",
      ].join("\n"),
      extensions: ["python3", "pytorch", "jupyter", "claude-code", "git"],
      provider_recommendations: ["e2b", "fly", "docker"],
      is_official: true,
    },
    {
      id: "tmpl_fullstack_ts",
      name: "Full-Stack TypeScript",
      slug: "fullstack-typescript",
      category: "language",
      description: "Node.js LTS with Docker-in-Docker, PostgreSQL client, and Claude Code.",
      yaml_content: [
        "name: ts-workspace",
        "extensions:",
        "  - node-lts",
        "  - docker-in-docker",
        "  - postgresql-client",
        "  - claude-code",
        "  - git",
        "  - zsh",
      ].join("\n"),
      extensions: [
        "node-lts",
        "docker-in-docker",
        "postgresql-client",
        "claude-code",
        "git",
        "zsh",
      ],
      provider_recommendations: ["fly", "docker", "devpod"],
      is_official: true,
    },
    {
      id: "tmpl_rust_systems",
      name: "Rust Systems",
      slug: "rust-systems",
      category: "language",
      description: "Rust stable toolchain with cargo, clippy, and cross-compilation targets.",
      yaml_content: [
        "name: rust-workspace",
        "extensions:",
        "  - rust",
        "  - git",
        "  - zsh",
        "  - tmux",
      ].join("\n"),
      extensions: ["rust", "git", "zsh", "tmux"],
      provider_recommendations: ["fly", "docker"],
      is_official: true,
    },
    {
      id: "tmpl_k8s_infra",
      name: "Kubernetes / Infrastructure",
      slug: "kubernetes-infra",
      category: "infrastructure",
      description: "kubectl, helm, terraform, and ansible for cloud infrastructure work.",
      yaml_content: [
        "name: infra-workspace",
        "extensions:",
        "  - kubectl",
        "  - helm",
        "  - terraform",
        "  - ansible",
        "  - git",
      ].join("\n"),
      extensions: ["kubectl", "helm", "terraform", "ansible", "git"],
      provider_recommendations: ["fly", "kubernetes"],
      is_official: true,
    },
  ];

  for (const t of templates) {
    await prisma.deploymentTemplate.upsert({
      where: { slug: t.slug },
      update: {},
      create: { ...t, updated_at: new Date() },
    });
  }

  console.log(`  Created ${templates.length} deployment templates`);
  return templates;
}

async function seedScheduledTasks(instances: Array<{ id: string; status: InstanceStatus }>) {
  console.log("Seeding scheduled tasks...");

  const runningInst = instances.find((i) => i.status === InstanceStatus.RUNNING);
  if (!runningInst) {
    console.log("  No running instances — skipping scheduled tasks");
    return [];
  }

  const now = new Date();
  const nextMidnight = new Date(now);
  nextMidnight.setUTCHours(24, 0, 0, 0);

  const tasks = [
    {
      id: "task_daily_cleanup",
      name: "Daily workspace cleanup",
      description: "Remove temporary files and clear build caches",
      cron: "0 2 * * *",
      timezone: "UTC",
      command: "find /tmp -mtime +1 -delete && rm -rf ~/.cache/pip /root/.npm/_npx",
      instance_id: runningInst.id,
      status: ScheduledTaskStatus.ACTIVE,
      timeout_sec: 120,
      max_retries: 1,
      notify_on_failure: true,
      notify_on_success: false,
      notify_emails: ["admin@sindri.dev"],
      next_run_at: nextMidnight,
      updated_at: now,
    },
    {
      id: "task_weekly_update",
      name: "Weekly package updates",
      description: "Update system packages on all instances",
      cron: "0 3 * * 0",
      timezone: "UTC",
      command: "apt-get update && apt-get upgrade -y --no-install-recommends",
      instance_id: null,
      status: ScheduledTaskStatus.PAUSED,
      timeout_sec: 600,
      max_retries: 0,
      notify_on_failure: true,
      notify_on_success: true,
      notify_emails: ["admin@sindri.dev", "operator@sindri.dev"],
      next_run_at: null,
      updated_at: now,
    },
  ];

  for (const task of tasks) {
    await prisma.scheduledTask.upsert({
      where: { id: task.id },
      update: {},
      create: task,
    });
  }

  // Seed one completed and one failed execution for the daily cleanup task
  const executions = [
    {
      task_id: "task_daily_cleanup",
      instance_id: runningInst.id,
      status: TaskExecutionStatus.SUCCESS,
      exit_code: 0,
      stdout: "Removed 142 files. Cache cleared.",
      stderr: null,
      started_at: new Date(Date.now() - 24 * 60 * 60 * 1000),
      finished_at: new Date(Date.now() - 24 * 60 * 60 * 1000 + 8200),
      duration_ms: 8200,
      triggered_by: "scheduler",
    },
    {
      task_id: "task_daily_cleanup",
      instance_id: runningInst.id,
      status: TaskExecutionStatus.FAILED,
      exit_code: 1,
      stdout: null,
      stderr: "Permission denied: /tmp/protected",
      started_at: new Date(Date.now() - 48 * 60 * 60 * 1000),
      finished_at: new Date(Date.now() - 48 * 60 * 60 * 1000 + 1200),
      duration_ms: 1200,
      triggered_by: "scheduler",
    },
  ];

  await prisma.taskExecution.createMany({ data: executions, skipDuplicates: false });

  console.log(`  Created ${tasks.length} scheduled tasks, ${executions.length} executions`);
  return tasks;
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

async function main() {
  console.log("Starting Sindri Console seed...\n");

  const users = await seedUsers();
  await seedApiKeys(users);
  const instances = await seedInstances();
  await seedHeartbeats(instances);
  await seedEvents(instances);
  await seedTerminalSessions(instances, users);
  await seedDeploymentTemplates();
  await seedScheduledTasks(instances);

  console.log("\nSeed complete.");
}

main()
  .catch((err) => {
    console.error("Seed failed:", err);
    process.exit(1);
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
