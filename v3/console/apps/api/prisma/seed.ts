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
  TeamMemberRole,
  AuditAction,
  ExtensionScope,
  VulnerabilitySeverity,
  VulnerabilityStatus,
  SshKeyStatus,
  DriftStatus,
  DriftSeverity,
  RemediationStatus,
  SecretType,
  BudgetPeriod,
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

async function seedTeams(
  users: Array<{ id: string; email: string }>,
  instances: Array<{ id: string; status: InstanceStatus }>,
) {
  console.log("Seeding teams...");

  const adminUser = users[0]; // admin@sindri.dev
  const operatorUser = users[1]; // operator@sindri.dev
  const devUser = users[2]; // developer@sindri.dev
  const viewerUser = users[3]; // viewer@sindri.dev

  const teams = [
    {
      id: "team_platform_01",
      name: "Platform Engineering",
      description: "Core infrastructure and platform reliability team.",
      created_by: adminUser.id,
    },
    {
      id: "team_ml_01",
      name: "Machine Learning",
      description: "ML research and experimentation workspaces.",
      created_by: adminUser.id,
    },
    {
      id: "team_frontend_01",
      name: "Frontend",
      description: "Web application development team.",
      created_by: operatorUser.id,
    },
  ];

  for (const team of teams) {
    await prisma.team.upsert({
      where: { name: team.name },
      update: {},
      create: { ...team, updated_at: new Date() },
    });
  }

  // Assign members to teams
  const memberships = [
    // Platform Engineering: admin (ADMIN), operator (OPERATOR), developer (DEVELOPER)
    { team_id: "team_platform_01", user_id: adminUser.id, role: TeamMemberRole.ADMIN },
    { team_id: "team_platform_01", user_id: operatorUser.id, role: TeamMemberRole.OPERATOR },
    { team_id: "team_platform_01", user_id: devUser.id, role: TeamMemberRole.DEVELOPER },
    // Machine Learning: developer (ADMIN of team), viewer (DEVELOPER)
    { team_id: "team_ml_01", user_id: devUser.id, role: TeamMemberRole.ADMIN },
    { team_id: "team_ml_01", user_id: viewerUser.id, role: TeamMemberRole.DEVELOPER },
    // Frontend: operator (ADMIN), developer (DEVELOPER), viewer (VIEWER)
    { team_id: "team_frontend_01", user_id: operatorUser.id, role: TeamMemberRole.ADMIN },
    { team_id: "team_frontend_01", user_id: devUser.id, role: TeamMemberRole.DEVELOPER },
    { team_id: "team_frontend_01", user_id: viewerUser.id, role: TeamMemberRole.VIEWER },
  ];

  for (const m of memberships) {
    await prisma.teamMember.upsert({
      where: { team_id_user_id: { team_id: m.team_id, user_id: m.user_id } },
      update: { role: m.role },
      create: { team_id: m.team_id, user_id: m.user_id, role: m.role },
    });
  }

  // Assign instances to teams
  const runningInstances = instances.filter((i) => i.status === InstanceStatus.RUNNING);
  const stoppedInstances = instances.filter((i) => i.status === InstanceStatus.STOPPED);

  const instanceAssignments: Array<{ id: string; team_id: string }> = [];

  // Assign first two running instances to Platform Engineering
  runningInstances
    .slice(0, 2)
    .forEach((inst) => instanceAssignments.push({ id: inst.id, team_id: "team_platform_01" }));
  // Assign remaining running instances to Frontend
  runningInstances
    .slice(2)
    .forEach((inst) => instanceAssignments.push({ id: inst.id, team_id: "team_frontend_01" }));
  // Assign stopped instances to ML team
  stoppedInstances.forEach((inst) =>
    instanceAssignments.push({ id: inst.id, team_id: "team_ml_01" }),
  );

  for (const assignment of instanceAssignments) {
    await prisma.instance.update({
      where: { id: assignment.id },
      data: { team_id: assignment.team_id },
    });
  }

  console.log(
    `  Created ${teams.length} teams, ${memberships.length} memberships, assigned ${instanceAssignments.length} instances`,
  );
  return teams;
}

async function seedAuditLogs(users: Array<{ id: string; email: string }>) {
  console.log("Seeding audit logs...");

  const adminUser = users[0];
  const operatorUser = users[1];
  const devUser = users[2];

  const now = new Date();
  const minutesAgo = (m: number) => new Date(now.getTime() - m * 60 * 1000);

  const logs = [
    // Admin login and user management
    {
      user_id: adminUser.id,
      action: AuditAction.LOGIN,
      resource: "user",
      resource_id: adminUser.id,
      metadata: { ip: "10.0.0.1", method: "password" },
      ip_address: "10.0.0.1",
      user_agent: "Mozilla/5.0 (Sindri Console)",
      timestamp: minutesAgo(120),
    },
    {
      user_id: adminUser.id,
      team_id: "team_platform_01",
      action: AuditAction.TEAM_ADD,
      resource: "team_member",
      resource_id: "team_platform_01",
      metadata: { added_user_id: devUser.id, role: "DEVELOPER" },
      ip_address: "10.0.0.1",
      user_agent: "Mozilla/5.0 (Sindri Console)",
      timestamp: minutesAgo(115),
    },
    {
      user_id: adminUser.id,
      action: AuditAction.CREATE,
      resource: "team",
      resource_id: "team_ml_01",
      metadata: { name: "Machine Learning" },
      ip_address: "10.0.0.1",
      user_agent: "Mozilla/5.0 (Sindri Console)",
      timestamp: minutesAgo(110),
    },
    // Operator actions
    {
      user_id: operatorUser.id,
      action: AuditAction.LOGIN,
      resource: "user",
      resource_id: operatorUser.id,
      metadata: { ip: "10.0.0.2", method: "api_key" },
      ip_address: "10.0.0.2",
      user_agent: "sindri-cli/3.0.0",
      timestamp: minutesAgo(90),
    },
    {
      user_id: operatorUser.id,
      team_id: "team_platform_01",
      action: AuditAction.DEPLOY,
      resource: "instance",
      resource_id: "inst_fly_sea_01",
      metadata: { provider: "fly", region: "sea", template: "fullstack-typescript" },
      ip_address: "10.0.0.2",
      user_agent: "sindri-cli/3.0.0",
      timestamp: minutesAgo(85),
    },
    {
      user_id: operatorUser.id,
      team_id: "team_platform_01",
      action: AuditAction.PERMISSION_CHANGE,
      resource: "team_member",
      resource_id: "team_platform_01",
      metadata: {
        target_user_id: devUser.id,
        old_role: "VIEWER",
        new_role: "DEVELOPER",
      },
      ip_address: "10.0.0.2",
      user_agent: "Mozilla/5.0 (Sindri Console)",
      timestamp: minutesAgo(60),
    },
    // Developer actions
    {
      user_id: devUser.id,
      action: AuditAction.LOGIN,
      resource: "user",
      resource_id: devUser.id,
      metadata: { ip: "192.168.1.50", method: "api_key" },
      ip_address: "192.168.1.50",
      user_agent: "sindri-cli/3.0.0",
      timestamp: minutesAgo(45),
    },
    {
      user_id: devUser.id,
      team_id: "team_platform_01",
      action: AuditAction.CONNECT,
      resource: "terminal_session",
      resource_id: "inst_fly_sea_01",
      metadata: { instance_name: "dev-primary" },
      ip_address: "192.168.1.50",
      user_agent: "sindri-cli/3.0.0",
      timestamp: minutesAgo(40),
    },
    {
      user_id: devUser.id,
      team_id: "team_platform_01",
      action: AuditAction.EXECUTE,
      resource: "command_execution",
      resource_id: "inst_fly_sea_01",
      metadata: { command: "npm run build", exit_code: 0 },
      ip_address: "192.168.1.50",
      user_agent: "sindri-cli/3.0.0",
      timestamp: minutesAgo(35),
    },
    {
      user_id: devUser.id,
      team_id: "team_platform_01",
      action: AuditAction.DISCONNECT,
      resource: "terminal_session",
      resource_id: "inst_fly_sea_01",
      metadata: { session_duration_seconds: 300 },
      ip_address: "192.168.1.50",
      user_agent: "sindri-cli/3.0.0",
      timestamp: minutesAgo(15),
    },
    // System-level audit (no user)
    {
      user_id: null,
      action: AuditAction.SUSPEND,
      resource: "instance",
      resource_id: "inst_e2b_01",
      metadata: { reason: "idle_timeout", idle_minutes: 60 },
      ip_address: null,
      user_agent: null,
      timestamp: minutesAgo(5),
    },
  ];

  await prisma.auditLog.createMany({ data: logs, skipDuplicates: false });
  console.log(`  Created ${logs.length} audit log entries`);
}

async function seedExtensions() {
  console.log("Seeding extensions...");

  const extensions = [
    {
      id: "ext_node_lts",
      name: "node-lts",
      display_name: "Node.js LTS",
      description: "Node.js Long Term Support runtime with npm and npx.",
      category: "language",
      version: "20.11.0",
      author: "Sindri Team",
      license: "MIT",
      tags: ["javascript", "typescript", "node", "npm"],
      dependencies: [],
      scope: ExtensionScope.PUBLIC,
      is_official: true,
      download_count: 8420,
    },
    {
      id: "ext_python3",
      name: "python3",
      display_name: "Python 3",
      description: "Python 3 runtime with pip and virtualenv support.",
      category: "language",
      version: "3.12.1",
      author: "Sindri Team",
      license: "PSF-2.0",
      tags: ["python", "pip", "ml", "scripting"],
      dependencies: [],
      scope: ExtensionScope.PUBLIC,
      is_official: true,
      download_count: 7850,
    },
    {
      id: "ext_claude_code",
      name: "claude-code",
      display_name: "Claude Code",
      description: "Anthropic Claude Code AI coding assistant integration.",
      category: "ai",
      version: "1.0.0",
      author: "Anthropic",
      license: "Proprietary",
      tags: ["ai", "llm", "coding-assistant", "anthropic"],
      dependencies: ["node-lts"],
      scope: ExtensionScope.PUBLIC,
      is_official: true,
      download_count: 6210,
    },
    {
      id: "ext_docker",
      name: "docker-in-docker",
      display_name: "Docker-in-Docker",
      description: "Full Docker daemon for building and running containers inside Sindri.",
      category: "infrastructure",
      version: "24.0.7",
      author: "Sindri Team",
      license: "Apache-2.0",
      tags: ["docker", "containers", "build", "ci"],
      dependencies: [],
      scope: ExtensionScope.PUBLIC,
      is_official: true,
      download_count: 5130,
    },
    {
      id: "ext_rust",
      name: "rust",
      display_name: "Rust",
      description: "Rust stable toolchain with cargo, rustfmt, and clippy.",
      category: "language",
      version: "1.76.0",
      author: "Sindri Team",
      license: "MIT OR Apache-2.0",
      tags: ["rust", "cargo", "systems"],
      dependencies: [],
      scope: ExtensionScope.PUBLIC,
      is_official: true,
      download_count: 2980,
    },
  ];

  for (const ext of extensions) {
    await prisma.extension.upsert({
      where: { name: ext.name },
      update: { download_count: ext.download_count },
      create: { ...ext, updated_at: new Date() },
    });
  }

  // Seed usage records for running instances
  const usages = [
    {
      extension_id: "ext_node_lts",
      instance_id: "inst_fly_sea_01",
      version: "20.11.0",
      installed_at: new Date(Date.now() - 4 * 24 * 60 * 60 * 1000),
      install_duration_ms: 12400,
    },
    {
      extension_id: "ext_python3",
      instance_id: "inst_fly_sea_01",
      version: "3.12.1",
      installed_at: new Date(Date.now() - 4 * 24 * 60 * 60 * 1000),
      install_duration_ms: 8900,
    },
    {
      extension_id: "ext_claude_code",
      instance_id: "inst_fly_sea_01",
      version: "1.0.0",
      installed_at: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000),
      install_duration_ms: 45200,
    },
    {
      extension_id: "ext_docker",
      instance_id: "inst_k8s_use1_01",
      version: "24.0.7",
      installed_at: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000),
      install_duration_ms: 89000,
    },
    {
      extension_id: "ext_rust",
      instance_id: "inst_docker_01",
      version: "1.76.0",
      installed_at: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000),
      install_duration_ms: 132000,
    },
  ];

  await prisma.extensionUsage.createMany({ data: usages, skipDuplicates: true });

  // Seed auto-update policy for claude-code on the dev-primary instance
  await prisma.extensionPolicy.upsert({
    where: {
      extension_id_instance_id: { extension_id: "ext_claude_code", instance_id: "inst_fly_sea_01" },
    },
    update: {},
    create: {
      extension_id: "ext_claude_code",
      instance_id: "inst_fly_sea_01",
      policy: "PIN",
      pinned_version: "1.0.0",
      updated_at: new Date(),
    },
  });

  console.log(
    `  Created ${extensions.length} extensions, ${usages.length} usage records, 1 policy`,
  );
}

async function seedSecurityData(instances: Array<{ id: string; status: InstanceStatus }>) {
  console.log("Seeding security data...");

  const runningInstances = instances.filter((i) => i.status === InstanceStatus.RUNNING);
  if (runningInstances.length === 0) {
    console.log("  No running instances — skipping security seed");
    return;
  }

  const primaryInst = runningInstances[0].id;
  const secondaryInst = runningInstances[1]?.id ?? primaryInst;

  // Vulnerabilities
  const vulnerabilities = [
    {
      instance_id: primaryInst,
      cve_id: "CVE-2024-21626",
      package_name: "runc",
      package_version: "1.1.11",
      ecosystem: "Go",
      severity: VulnerabilitySeverity.HIGH,
      cvss_score: 8.6,
      title: "runc container escape via process.cwd",
      description: "A path traversal vulnerability in runc allows container escape.",
      fix_version: "1.1.12",
      references: ["https://nvd.nist.gov/vuln/detail/CVE-2024-21626"],
      status: VulnerabilityStatus.OPEN,
      detected_at: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000),
    },
    {
      instance_id: primaryInst,
      cve_id: "CVE-2023-46233",
      package_name: "crypto-js",
      package_version: "4.1.1",
      ecosystem: "npm",
      severity: VulnerabilitySeverity.CRITICAL,
      cvss_score: 9.1,
      title: "PBKDF2 1000x weaker than expected in crypto-js",
      description: "crypto-js PBKDF2 is 1000x weaker than specified in the standard.",
      fix_version: "4.2.0",
      references: ["https://nvd.nist.gov/vuln/detail/CVE-2023-46233"],
      status: VulnerabilityStatus.ACKNOWLEDGED,
      detected_at: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000),
      acknowledged_at: new Date(Date.now() - 5 * 24 * 60 * 60 * 1000),
      acknowledged_by: "user_admin_01",
    },
    {
      instance_id: secondaryInst,
      cve_id: "CVE-2024-24790",
      package_name: "stdlib",
      package_version: "go1.21.5",
      ecosystem: "Go",
      severity: VulnerabilitySeverity.MEDIUM,
      cvss_score: 6.1,
      title: "net/netip: unexpected behavior from Is methods for IPv4-mapped IPv6 addresses",
      description: "The IPv4-mapped IPv6 address handling in Go stdlib is inconsistent.",
      fix_version: "go1.22.0",
      references: ["https://nvd.nist.gov/vuln/detail/CVE-2024-24790"],
      status: VulnerabilityStatus.OPEN,
      detected_at: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000),
    },
  ];

  await prisma.vulnerability.createMany({ data: vulnerabilities, skipDuplicates: false });

  // BOM entries
  const bomEntries = [
    {
      instance_id: primaryInst,
      package_name: "node",
      package_version: "20.11.0",
      ecosystem: "system",
      license: "MIT",
    },
    {
      instance_id: primaryInst,
      package_name: "python3",
      package_version: "3.12.1",
      ecosystem: "system",
      license: "PSF-2.0",
    },
    {
      instance_id: primaryInst,
      package_name: "express",
      package_version: "4.18.2",
      ecosystem: "npm",
      license: "MIT",
    },
    {
      instance_id: primaryInst,
      package_name: "crypto-js",
      package_version: "4.1.1",
      ecosystem: "npm",
      license: "MIT",
    },
    {
      instance_id: secondaryInst,
      package_name: "runc",
      package_version: "1.1.11",
      ecosystem: "Go",
      license: "Apache-2.0",
    },
    {
      instance_id: secondaryInst,
      package_name: "kubectl",
      package_version: "1.29.0",
      ecosystem: "system",
      license: "Apache-2.0",
    },
  ];

  await prisma.bomEntry.createMany({ data: bomEntries, skipDuplicates: true });

  // SSH keys
  const sshKeys = [
    {
      instance_id: primaryInst,
      fingerprint: "SHA256:AbCdEfGhIjKlMnOpQrStUvWxYz1234567890abcdefg",
      comment: "developer@sindri.dev",
      key_type: "ed25519",
      key_bits: 256,
      status: SshKeyStatus.ACTIVE,
      last_used_at: new Date(Date.now() - 2 * 60 * 60 * 1000),
    },
    {
      instance_id: primaryInst,
      fingerprint: "SHA256:ZyXwVuTsRqPoNmLkJiHgFeDcBa0987654321zyxwvuts",
      comment: "ci-github-actions",
      key_type: "rsa",
      key_bits: 4096,
      status: SshKeyStatus.ACTIVE,
      expires_at: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000),
    },
    {
      instance_id: secondaryInst,
      fingerprint: "SHA256:MnOpQrStUvWxYzAbCdEfGhIjKl1234567890mnopqrst",
      comment: "old-deploy-key",
      key_type: "rsa",
      key_bits: 2048,
      status: SshKeyStatus.REVOKED,
    },
  ];

  await prisma.sshKey.createMany({ data: sshKeys, skipDuplicates: true });

  // Secret rotation records
  const rotations = [
    {
      instance_id: primaryInst,
      secret_name: "DATABASE_URL",
      secret_type: "ENV_VAR",
      last_rotated: new Date(Date.now() - 45 * 24 * 60 * 60 * 1000),
      next_rotation: new Date(Date.now() + 45 * 24 * 60 * 60 * 1000),
      rotation_days: 90,
      is_overdue: false,
      updated_at: new Date(),
    },
    {
      instance_id: primaryInst,
      secret_name: "OPENAI_API_KEY",
      secret_type: "API_KEY",
      last_rotated: new Date(Date.now() - 100 * 24 * 60 * 60 * 1000),
      next_rotation: new Date(Date.now() - 10 * 24 * 60 * 60 * 1000),
      rotation_days: 90,
      is_overdue: true,
      updated_at: new Date(),
    },
  ];

  await prisma.secretRotation.createMany({ data: rotations, skipDuplicates: false });

  console.log(
    `  Created ${vulnerabilities.length} vulnerabilities, ${bomEntries.length} BOM entries, ` +
      `${sshKeys.length} SSH keys, ${rotations.length} rotation records`,
  );
}

async function seedDriftData(instances: Array<{ id: string; status: InstanceStatus }>) {
  console.log("Seeding configuration drift data...");

  const runningInstances = instances.filter((i) => i.status === InstanceStatus.RUNNING);
  if (runningInstances.length === 0) {
    console.log("  No running instances — skipping drift seed");
    return;
  }

  const primaryInst = runningInstances[0].id;

  // Config snapshot — clean
  const cleanSnap = await prisma.configSnapshot.create({
    data: {
      instance_id: primaryInst,
      taken_at: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000),
      declared: {
        node_version: "20.11.0",
        extensions: ["node-lts", "git"],
        env: { NODE_ENV: "production" },
      },
      actual: {
        node_version: "20.11.0",
        extensions: ["node-lts", "git"],
        env: { NODE_ENV: "production" },
      },
      config_hash: sha256("clean-snapshot-v1"),
      drift_status: DriftStatus.CLEAN,
    },
  });

  // Config snapshot — drifted
  const driftedSnap = await prisma.configSnapshot.create({
    data: {
      instance_id: primaryInst,
      taken_at: new Date(Date.now() - 30 * 60 * 1000),
      declared: {
        node_version: "20.11.0",
        extensions: ["node-lts", "git"],
        env: { NODE_ENV: "production" },
      },
      actual: {
        node_version: "20.11.0",
        extensions: ["node-lts", "git", "curl"],
        env: { NODE_ENV: "development" },
      },
      config_hash: sha256("drifted-snapshot-v1"),
      drift_status: DriftStatus.DRIFTED,
    },
  });

  // Drift events for the drifted snapshot
  const envDrift = await prisma.driftEvent.create({
    data: {
      snapshot_id: driftedSnap.id,
      instance_id: primaryInst,
      detected_at: new Date(Date.now() - 25 * 60 * 1000),
      field_path: "env.NODE_ENV",
      declared_val: "production",
      actual_val: "development",
      severity: DriftSeverity.HIGH,
      description:
        "NODE_ENV has been changed from 'production' to 'development' on the running instance.",
    },
  });

  await prisma.driftEvent.create({
    data: {
      snapshot_id: driftedSnap.id,
      instance_id: primaryInst,
      detected_at: new Date(Date.now() - 25 * 60 * 1000),
      field_path: "extensions[2]",
      declared_val: null,
      actual_val: "curl",
      severity: DriftSeverity.LOW,
      description: "Undeclared extension 'curl' found installed on instance.",
    },
  });

  // Remediation for the env drift
  await prisma.driftRemediation.create({
    data: {
      drift_event_id: envDrift.id,
      instance_id: primaryInst,
      action: "Reset NODE_ENV to production",
      command: "export NODE_ENV=production",
      status: RemediationStatus.PENDING,
    },
  });

  // Vault secrets
  await prisma.secret.createMany({
    data: [
      {
        name: "DATABASE_URL",
        description: "Primary PostgreSQL connection string",
        type: SecretType.ENV_VAR,
        instance_id: primaryInst,
        encrypted_val: "enc:v1:aes256gcm:dGVzdC1lbmNyeXB0ZWQtdmFsdWU=",
        scope: ["api", "migration"],
        created_by: "user_admin_01",
        updated_at: new Date(),
      },
      {
        name: "SINDRI_DEPLOY_KEY",
        description: "SSH private key for deployment automation",
        type: SecretType.FILE,
        instance_id: null,
        encrypted_val: "enc:v1:aes256gcm:c3NoLXByaXZhdGUta2V5LWVuY3J5cHRlZA==",
        scope: ["ci", "deploy"],
        created_by: "user_operator_01",
        updated_at: new Date(),
      },
    ],
    skipDuplicates: true,
  });

  console.log(
    `  Created 2 config snapshots (1 clean, 1 drifted), 2 drift events, 1 remediation, 2 secrets`,
  );
  void cleanSnap; // used for reference
}

async function seedCostData(instances: Array<{ id: string; provider: string }>) {
  console.log("Seeding cost data...");

  const now = new Date();

  // Helper — start/end of a past day
  const dayRange = (daysAgo: number) => {
    const start = new Date(now);
    start.setUTCDate(start.getUTCDate() - daysAgo);
    start.setUTCHours(0, 0, 0, 0);
    const end = new Date(start);
    end.setUTCHours(23, 59, 59, 999);
    return { start, end };
  };

  const costEntries = instances.flatMap((inst, idx) => {
    // 7 days of cost entries per instance, varying slightly
    const base = 1.5 + idx * 0.8;
    return Array.from({ length: 7 }, (_, d) => {
      const { start, end } = dayRange(6 - d);
      const jitter = Math.random() * 0.3 - 0.15;
      const compute = parseFloat((base + jitter).toFixed(4));
      const storage = parseFloat((0.12 + Math.random() * 0.05).toFixed(4));
      const network = parseFloat((0.04 + Math.random() * 0.02).toFixed(4));
      return {
        instance_id: inst.id,
        provider: inst.provider,
        period_start: start,
        period_end: end,
        compute_usd: compute,
        storage_usd: storage,
        network_usd: network,
        total_usd: parseFloat((compute + storage + network).toFixed(4)),
        currency: "USD",
      };
    });
  });

  await prisma.costEntry.createMany({ data: costEntries, skipDuplicates: true });

  // Budgets
  await prisma.budget.createMany({
    data: [
      {
        name: "Fleet Monthly Budget",
        amount_usd: 500,
        period: BudgetPeriod.MONTHLY,
        instance_id: null,
        provider: null,
        alert_threshold: 0.8,
        created_by: "user_admin_01",
        updated_at: now,
      },
      {
        name: "dev-primary Daily Cap",
        amount_usd: 10,
        period: BudgetPeriod.DAILY,
        instance_id: "inst_fly_sea_01",
        provider: "fly",
        alert_threshold: 0.9,
        created_by: "user_operator_01",
        updated_at: now,
      },
    ],
    skipDuplicates: false,
  });

  // Right-sizing recommendations for running instances
  const recommendations = instances.slice(0, 2).map((inst, idx) => ({
    instance_id: inst.id,
    current_tier: "fly-performance-2x",
    suggested_tier: idx === 0 ? "fly-performance-1x" : "fly-shared-cpu-2x",
    current_usd_mo: 60 + idx * 20,
    suggested_usd_mo: 30 + idx * 10,
    savings_usd_mo: 30 + idx * 10,
    avg_cpu_percent: 18.5 + idx * 5,
    avg_mem_percent: 34.2 - idx * 3,
    confidence: 0.87 - idx * 0.05,
    dismissed: false,
  }));

  for (const rec of recommendations) {
    await prisma.rightSizingRecommendation.upsert({
      where: { instance_id: rec.instance_id },
      update: {},
      create: rec,
    });
  }

  console.log(
    `  Created ${costEntries.length} cost entries, 2 budgets, ${recommendations.length} right-sizing recommendations`,
  );
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
  await seedTeams(users, instances);
  await seedAuditLogs(users);
  await seedExtensions();
  await seedSecurityData(instances);
  await seedDriftData(instances);
  await seedCostData(instances);

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
