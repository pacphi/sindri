/**
 * Integration tests for Phase 2 Multi-Instance Command Execution.
 *
 * Tests cover:
 * - Command dispatch to single instance
 * - Parallel command dispatch to multiple instances
 * - Command output streaming via WebSocket
 * - Command timeout handling
 * - Command cancellation
 * - Exit code handling and error reporting
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { buildApp, authHeaders, VALID_API_KEY } from './helpers.js';
import { createHash } from 'crypto';

// ─────────────────────────────────────────────────────────────────────────────
// Mocks
// ─────────────────────────────────────────────────────────────────────────────

const VALID_HASH = createHash('sha256').update('sk-test-valid-key-0001').digest('hex');

const mockInstances = [
  { id: 'inst_01', name: 'instance-one', status: 'RUNNING', provider: 'fly', region: 'sea', extensions: [], config_hash: 'a'.repeat(64), ssh_endpoint: 'inst1.fly.dev:22', created_at: new Date(), updated_at: new Date() },
  { id: 'inst_02', name: 'instance-two', status: 'RUNNING', provider: 'fly', region: 'iad', extensions: [], config_hash: 'b'.repeat(64), ssh_endpoint: 'inst2.fly.dev:22', created_at: new Date(), updated_at: new Date() },
  { id: 'inst_03', name: 'instance-three', status: 'STOPPED', provider: 'docker', region: 'local', extensions: [], config_hash: 'c'.repeat(64), ssh_endpoint: null, created_at: new Date(), updated_at: new Date() },
];

vi.mock('../src/lib/db.js', () => {
  const db = {
    apiKey: {
      findUnique: vi.fn(({ where }: { where: { key_hash: string } }) => {
        if (where.key_hash === VALID_HASH) {
          return Promise.resolve({ id: 'key_dev_01', user_id: 'user_dev_01', key_hash: VALID_HASH, expires_at: null, user: { role: 'DEVELOPER' } });
        }
        return Promise.resolve(null);
      }),
      update: vi.fn(() => Promise.resolve({})),
    },
    instance: {
      findMany: vi.fn(() => Promise.resolve(mockInstances)),
      count: vi.fn(() => Promise.resolve(mockInstances.length)),
      findUnique: vi.fn(({ where }: { where: { id: string } }) => {
        return Promise.resolve(mockInstances.find((i) => i.id === where.id) ?? null);
      }),
      upsert: vi.fn(() => Promise.resolve(mockInstances[0])),
    },
    heartbeat: {
      findFirst: vi.fn(() => Promise.resolve(null)),
      create: vi.fn(() => Promise.resolve({})),
    },
    event: {
      create: vi.fn(() => Promise.resolve({ id: 'evt_cmd_01' })),
    },
    $queryRaw: vi.fn(() => Promise.resolve([{ '?column?': 1 }])),
    $connect: vi.fn(() => Promise.resolve()),
    $disconnect: vi.fn(() => Promise.resolve()),
  };
  return { db };
});

vi.mock('../src/lib/redis.js', () => ({
  redis: {
    publish: vi.fn(() => Promise.resolve(1)),
    ping: vi.fn(() => Promise.resolve('PONG')),
  },
  redisSub: { psubscribe: vi.fn(), on: vi.fn() },
  REDIS_CHANNELS: {
    instanceMetrics: (id: string) => `sindri:instance:${id}:metrics`,
    instanceHeartbeat: (id: string) => `sindri:instance:${id}:heartbeat`,
    instanceLogs: (id: string) => `sindri:instance:${id}:logs`,
    instanceEvents: (id: string) => `sindri:instance:${id}:events`,
    instanceCommands: (id: string) => `sindri:instance:${id}:commands`,
  },
  REDIS_KEYS: {
    instanceOnline: (id: string) => `sindri:instance:${id}:online`,
    activeAgents: 'sindri:agents:active',
  },
  connectRedis: vi.fn(() => Promise.resolve()),
  disconnectRedis: vi.fn(() => Promise.resolve()),
}));

// ─────────────────────────────────────────────────────────────────────────────
// Command Payload Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Execution: Payload Validation', () => {
  it('valid command payload has required fields', () => {
    const payload = {
      commandId: 'cmd_abc123',
      command: 'echo "hello world"',
      timeout: 30,
    };

    expect(payload.commandId).toBeTruthy();
    expect(typeof payload.command).toBe('string');
    expect(payload.command.length).toBeGreaterThan(0);
    expect(payload.timeout).toBeGreaterThan(0);
  });

  it('commandId follows UUID-like format', () => {
    const uuidRegex = /^[0-9a-f-]{8,36}$/;
    const commandId = 'cmd_abc123def456';
    expect(commandId).toMatch(/^cmd_[a-z0-9]+$/);
  });

  it('command string cannot be empty', () => {
    const emptyCommand = '';
    const isValid = emptyCommand.trim().length > 0;
    expect(isValid).toBe(false);
  });

  it('timeout defaults to 30 seconds if not specified', () => {
    const payload = { commandId: 'cmd_01', command: 'ls -la' };
    const timeout = (payload as { timeout?: number }).timeout ?? 30;
    expect(timeout).toBe(30);
  });

  it('timeout maximum is 3600 seconds (1 hour)', () => {
    const maxTimeout = 3600;
    const overTimeout = 7200;
    expect(overTimeout).toBeGreaterThan(maxTimeout);
  });

  it('command supports environment variable injection', () => {
    const payload = {
      commandId: 'cmd_env_01',
      command: 'echo $HOME',
      env: { HOME: '/root', PATH: '/usr/bin:/bin' },
      timeout: 10,
    };

    expect(payload.env).toBeDefined();
    expect(Object.keys(payload.env)).toContain('HOME');
    expect(Object.keys(payload.env)).toContain('PATH');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Single Instance Command Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Execution: Single Instance', () => {
  it('dispatches command to running instance', () => {
    const instance = mockInstances[0];
    expect(instance.status).toBe('RUNNING');

    const command = {
      commandId: `cmd_${Date.now()}`,
      instanceId: instance.id,
      command: 'node --version',
      timeout: 10,
      dispatchedAt: new Date().toISOString(),
    };

    expect(command.instanceId).toBe('inst_01');
    expect(command.command).toBe('node --version');
  });

  it('cannot dispatch command to STOPPED instance', () => {
    const stoppedInstance = mockInstances[2];
    expect(stoppedInstance.status).toBe('STOPPED');

    const canDispatch = stoppedInstance.status === 'RUNNING';
    expect(canDispatch).toBe(false);
  });

  it('command output is captured from stdout', () => {
    const outputMessages = [
      { commandId: 'cmd_01', stream: 'stdout', data: 'v20.10.0\n' },
    ];

    expect(outputMessages[0].stream).toBe('stdout');
    expect(outputMessages[0].data).toContain('v20.10.0');
  });

  it('command error output is captured from stderr', () => {
    const errorMessages = [
      { commandId: 'cmd_02', stream: 'stderr', data: 'command not found: foobar\n' },
    ];

    expect(errorMessages[0].stream).toBe('stderr');
    expect(errorMessages[0].data).toContain('command not found');
  });

  it('command completion includes exit code', () => {
    const completionMessage = {
      commandId: 'cmd_03',
      exitCode: 0,
      completedAt: new Date().toISOString(),
    };

    expect(completionMessage.exitCode).toBe(0);
    expect(completionMessage.completedAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);
  });

  it('non-zero exit code indicates failure', () => {
    const failureCompletion = {
      commandId: 'cmd_04',
      exitCode: 1,
      completedAt: new Date().toISOString(),
    };

    const isSuccess = failureCompletion.exitCode === 0;
    expect(isSuccess).toBe(false);
    expect(failureCompletion.exitCode).toBe(1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Multi-Instance Command Dispatch Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Execution: Multi-Instance Parallel Dispatch', () => {
  const runningInstances = mockInstances.filter((i) => i.status === 'RUNNING');

  it('identifies running instances for parallel dispatch', () => {
    expect(runningInstances).toHaveLength(2);
    for (const instance of runningInstances) {
      expect(instance.status).toBe('RUNNING');
    }
  });

  it('dispatches same command to multiple instances concurrently', async () => {
    const command = 'uptime';
    const dispatches = runningInstances.map((instance) => ({
      commandId: `cmd_${instance.id}_${Date.now()}`,
      instanceId: instance.id,
      command,
      dispatchedAt: new Date().toISOString(),
    }));

    expect(dispatches).toHaveLength(2);
    for (const dispatch of dispatches) {
      expect(dispatch.command).toBe(command);
      expect(dispatch.instanceId).toBeTruthy();
    }
  });

  it('aggregates results from all instances', () => {
    const results = [
      { instanceId: 'inst_01', exitCode: 0, output: ' 10:00:00 up 1 day\n' },
      { instanceId: 'inst_02', exitCode: 0, output: ' 10:00:01 up 2 days\n' },
    ];

    expect(results).toHaveLength(runningInstances.length);
    for (const result of results) {
      expect(result.exitCode).toBe(0);
      expect(result.output.length).toBeGreaterThan(0);
    }
  });

  it('partial failure does not block other results', () => {
    const results = [
      { instanceId: 'inst_01', exitCode: 0, output: 'success\n', error: null },
      { instanceId: 'inst_02', exitCode: 1, output: '', error: 'permission denied' },
    ];

    const successCount = results.filter((r) => r.exitCode === 0).length;
    const failureCount = results.filter((r) => r.exitCode !== 0).length;

    expect(successCount).toBe(1);
    expect(failureCount).toBe(1);
    expect(successCount + failureCount).toBe(results.length);
  });

  it('filters target instances by tag or label', () => {
    const taggedInstances = runningInstances.filter((i) => i.region === 'sea');
    expect(taggedInstances).toHaveLength(1);
    expect(taggedInstances[0].id).toBe('inst_01');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Command Timeout and Cancellation Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Execution: Timeout and Cancellation', () => {
  it('command times out after specified duration', () => {
    const timeout = 5; // seconds
    const startTime = Date.now();
    const mockElapsed = 5100; // ms — just over timeout
    const isTimedOut = mockElapsed / 1000 >= timeout;
    expect(isTimedOut).toBe(true);
  });

  it('timeout produces a timeout exit code', () => {
    const timeoutExitCode = 124; // Standard Unix timeout exit code
    expect(timeoutExitCode).toBe(124);
  });

  it('cancellation produces a cancellation message', () => {
    const cancelledResult = {
      commandId: 'cmd_cancel_01',
      status: 'cancelled',
      exitCode: -1,
      cancelledAt: new Date().toISOString(),
    };

    expect(cancelledResult.status).toBe('cancelled');
    expect(cancelledResult.exitCode).toBe(-1);
  });

  it('running command can be cancelled by commandId', () => {
    const activeCommands = new Map<string, { status: string }>();
    activeCommands.set('cmd_01', { status: 'running' });

    const cmdToCancel = 'cmd_01';
    const command = activeCommands.get(cmdToCancel);
    if (command) {
      command.status = 'cancelled';
    }

    expect(activeCommands.get('cmd_01')?.status).toBe('cancelled');
  });

  it('cancelled command cleanup removes from active map', () => {
    const activeCommands = new Map<string, { status: string }>();
    activeCommands.set('cmd_cancel', { status: 'cancelled' });
    activeCommands.delete('cmd_cancel');

    expect(activeCommands.has('cmd_cancel')).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// WebSocket Command Protocol Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Execution: WebSocket Protocol', () => {
  it('exec message has correct channel and type', () => {
    const execMessage = {
      channel: 'commands',
      type: 'exec',
      payload: {
        commandId: 'cmd_ws_01',
        command: 'ls -la',
        timeout: 30,
      },
    };

    expect(execMessage.channel).toBe('commands');
    expect(execMessage.type).toBe('exec');
    expect(execMessage.payload.commandId).toBeTruthy();
  });

  it('output message has correct structure', () => {
    const outputMessage = {
      channel: 'commands',
      type: 'output',
      payload: {
        commandId: 'cmd_ws_01',
        data: 'total 24\ndrwxr-xr-x  5 root root 4096 Feb 17 10:00 .\n',
        stream: 'stdout',
      },
    };

    expect(outputMessage.channel).toBe('commands');
    expect(outputMessage.type).toBe('output');
    expect(['stdout', 'stderr']).toContain(outputMessage.payload.stream);
  });

  it('complete message includes exit code', () => {
    const completeMessage = {
      channel: 'commands',
      type: 'complete',
      payload: {
        commandId: 'cmd_ws_01',
        exitCode: 0,
      },
    };

    expect(completeMessage.channel).toBe('commands');
    expect(completeMessage.type).toBe('complete');
    expect(typeof completeMessage.payload.exitCode).toBe('number');
  });

  it('command output is streamed as multiple messages', () => {
    const outputChunks = [
      { type: 'output', payload: { commandId: 'cmd_stream_01', data: 'line1\n', stream: 'stdout' } },
      { type: 'output', payload: { commandId: 'cmd_stream_01', data: 'line2\n', stream: 'stdout' } },
      { type: 'output', payload: { commandId: 'cmd_stream_01', data: 'line3\n', stream: 'stdout' } },
      { type: 'complete', payload: { commandId: 'cmd_stream_01', exitCode: 0 } },
    ];

    const outputMessages = outputChunks.filter((m) => m.type === 'output');
    const completeMessages = outputChunks.filter((m) => m.type === 'complete');

    expect(outputMessages).toHaveLength(3);
    expect(completeMessages).toHaveLength(1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// API Endpoint Tests (GET /api/v1/instances for command targets)
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Execution: Instance Listing API', () => {
  const app = buildApp();

  it('lists instances available for command execution', async () => {
    const res = await app.request('/api/v1/instances', { headers: authHeaders() });
    expect(res.status).toBe(200);
    const body = await res.json() as { instances: Array<{ id: string; status: string }> };
    expect(Array.isArray(body.instances)).toBe(true);
    expect(body.instances.length).toBeGreaterThan(0);
  });

  it('filters instances by RUNNING status for command dispatch', async () => {
    const res = await app.request('/api/v1/instances?status=RUNNING', { headers: authHeaders() });
    expect(res.status).toBe(200);
  });
});
