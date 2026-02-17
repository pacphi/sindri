/**
 * Prisma seed script for Sindri Console development data.
 *
 * Run with: npx prisma db seed
 * (configured in package.json under "prisma.seed")
 */

import { PrismaClient, InstanceStatus, UserRole, EventType, TerminalSessionStatus } from '@prisma/client';
import * as crypto from 'crypto';

const prisma = new PrismaClient();

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function sha256(value: string): string {
  return crypto.createHash('sha256').update(value).digest('hex');
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
  console.log('Seeding users...');

  const users = [
    {
      id: 'user_admin_01',
      email: 'admin@sindri.dev',
      password_hash: fakeBcrypt('admin-secret-change-me'),
      role: UserRole.ADMIN,
    },
    {
      id: 'user_operator_01',
      email: 'operator@sindri.dev',
      password_hash: fakeBcrypt('operator-secret-change-me'),
      role: UserRole.OPERATOR,
    },
    {
      id: 'user_dev_01',
      email: 'developer@sindri.dev',
      password_hash: fakeBcrypt('developer-secret-change-me'),
      role: UserRole.DEVELOPER,
    },
    {
      id: 'user_viewer_01',
      email: 'viewer@sindri.dev',
      password_hash: fakeBcrypt('viewer-secret-change-me'),
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
  console.log('Seeding API keys...');

  const rawKeys = [
    { userId: users[0].id, name: 'Admin CI Key', raw: 'sk-admin-dev-seed-key-0001', daysToExpire: null },
    { userId: users[2].id, name: 'Developer Local Key', raw: 'sk-dev-seed-key-0001', daysToExpire: 90 },
    { userId: users[2].id, name: 'Developer GitHub Actions', raw: 'sk-dev-gh-seed-key-0002', daysToExpire: 365 },
  ];

  for (const k of rawKeys) {
    const expiresAt = k.daysToExpire
      ? new Date(Date.now() + k.daysToExpire * 86400 * 1000)
      : null;

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
  console.log('Seeding instances...');

  const instances = [
    {
      id: 'inst_fly_sea_01',
      name: 'dev-primary',
      provider: 'fly',
      region: 'sea',
      extensions: ['node-lts', 'python3', 'docker-in-docker', 'claude-code', 'git', 'zsh', 'tmux'],
      config_hash: sha256('dev-primary-sindri-yaml-v1'),
      ssh_endpoint: 'dev-primary.fly.dev:22',
      status: InstanceStatus.RUNNING,
      created_at: new Date(Date.now() - 4 * 24 * 60 * 60 * 1000), // 4 days ago
    },
    {
      id: 'inst_k8s_use1_01',
      name: 'staging',
      provider: 'kubernetes',
      region: 'us-east-1',
      extensions: ['node-lts', 'python3', 'kubectl', 'helm', 'git'],
      config_hash: sha256('staging-sindri-yaml-v2'),
      ssh_endpoint: 'staging.k8s.internal:22',
      status: InstanceStatus.RUNNING,
      created_at: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000), // 1 day ago
    },
    {
      id: 'inst_e2b_01',
      name: 'ml-sandbox',
      provider: 'e2b',
      region: null,
      extensions: ['python3', 'pytorch', 'jupyter'],
      config_hash: sha256('ml-sandbox-sindri-yaml-v1'),
      ssh_endpoint: null,
      status: InstanceStatus.STOPPED,
      created_at: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000), // 7 days ago
    },
    {
      id: 'inst_docker_01',
      name: 'local-dev',
      provider: 'docker',
      region: null,
      extensions: ['node-lts', 'rust', 'git', 'zsh'],
      config_hash: sha256('local-dev-sindri-yaml-v3'),
      ssh_endpoint: 'localhost:2222',
      status: InstanceStatus.RUNNING,
      created_at: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000), // 2 days ago
    },
    {
      id: 'inst_fly_iad_01',
      name: 'ci-runner-03',
      provider: 'fly',
      region: 'iad',
      extensions: ['node-lts', 'docker-in-docker', 'git'],
      config_hash: sha256('ci-runner-sindri-yaml-v1'),
      ssh_endpoint: 'ci-runner-03.fly.dev:22',
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

async function seedHeartbeats(instances: Array<{ id: string; name: string; status: InstanceStatus }>) {
  console.log('Seeding heartbeats...');

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
  console.log('Seeding events...');

  const eventSets = instances.flatMap((inst) => [
    {
      instance_id: inst.id,
      event_type: EventType.DEPLOY,
      timestamp: new Date(Date.now() - 4 * 24 * 60 * 60 * 1000),
      metadata: { triggered_by: 'cli', sindri_version: '3.0.0', duration_seconds: 47 },
    },
    {
      instance_id: inst.id,
      event_type: EventType.CONNECT,
      timestamp: new Date(Date.now() - 2 * 60 * 60 * 1000),
      metadata: { user: 'developer@sindri.dev', ip: '192.168.1.100' },
    },
    {
      instance_id: inst.id,
      event_type: EventType.DISCONNECT,
      timestamp: new Date(Date.now() - 1 * 60 * 60 * 1000),
      metadata: { user: 'developer@sindri.dev', session_duration_seconds: 3600 },
    },
  ]);

  await prisma.event.createMany({ data: eventSets, skipDuplicates: false });
  console.log(`  Created ${eventSets.length} events`);
}

async function seedTerminalSessions(
  instances: Array<{ id: string; status: InstanceStatus }>,
  users: Array<{ id: string }>
) {
  console.log('Seeding terminal sessions...');

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

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

async function main() {
  console.log('Starting Sindri Console seed...\n');

  const users = await seedUsers();
  await seedApiKeys(users);
  const instances = await seedInstances();
  await seedHeartbeats(instances);
  await seedEvents(instances);
  await seedTerminalSessions(instances, users);

  console.log('\nSeed complete.');
}

main()
  .catch((err) => {
    console.error('Seed failed:', err);
    process.exit(1);
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
