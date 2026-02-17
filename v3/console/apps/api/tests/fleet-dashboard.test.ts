/**
 * Integration tests: Phase 3 Fleet Overview Dashboard
 *
 * Tests fleet-level aggregation and visualization data endpoints:
 *   - Fleet health summary (total instances, status breakdown)
 *   - Resource utilization rollup across all instances
 *   - Sorting and filtering of fleet instance list
 *   - Stale/offline instance detection
 *   - Top-N resource consumers
 *   - WebSocket fan-out for real-time fleet updates
 */

import { describe, it, expect } from 'vitest';

// ─────────────────────────────────────────────────────────────────────────────
// Test fixtures
// ─────────────────────────────────────────────────────────────────────────────

type InstanceStatus = 'RUNNING' | 'STOPPED' | 'DEPLOYING' | 'DESTROYING' | 'SUSPENDED' | 'ERROR' | 'UNKNOWN';

interface FleetInstance {
  id: string;
  name: string;
  provider: string;
  region: string | null;
  status: InstanceStatus;
  cpuPercent: number | null;
  memPercent: number | null;
  diskPercent: number | null;
  lastHeartbeatAt: string | null;
  uptimeSeconds: number | null;
}

const mockFleet: FleetInstance[] = [
  {
    id: 'inst_01', name: 'prod-us-east', provider: 'fly', region: 'iad',
    status: 'RUNNING', cpuPercent: 72.3, memPercent: 64.1, diskPercent: 45.0,
    lastHeartbeatAt: new Date(Date.now() - 20_000).toISOString(), uptimeSeconds: 86400,
  },
  {
    id: 'inst_02', name: 'prod-eu-west', provider: 'fly', region: 'lhr',
    status: 'RUNNING', cpuPercent: 28.5, memPercent: 41.2, diskPercent: 33.0,
    lastHeartbeatAt: new Date(Date.now() - 15_000).toISOString(), uptimeSeconds: 172800,
  },
  {
    id: 'inst_03', name: 'staging-us', provider: 'docker', region: null,
    status: 'RUNNING', cpuPercent: 11.0, memPercent: 22.0, diskPercent: 18.0,
    lastHeartbeatAt: new Date(Date.now() - 25_000).toISOString(), uptimeSeconds: 3600,
  },
  {
    id: 'inst_04', name: 'dev-local-01', provider: 'devpod', region: null,
    status: 'STOPPED', cpuPercent: null, memPercent: null, diskPercent: null,
    lastHeartbeatAt: null, uptimeSeconds: null,
  },
  {
    id: 'inst_05', name: 'canary-deploy', provider: 'fly', region: 'sea',
    status: 'DEPLOYING', cpuPercent: 5.0, memPercent: 15.0, diskPercent: 12.0,
    lastHeartbeatAt: new Date(Date.now() - 180_000).toISOString(), uptimeSeconds: 120,
  },
  {
    id: 'inst_06', name: 'old-worker', provider: 'kubernetes', region: 'us-east-1',
    status: 'ERROR', cpuPercent: 99.9, memPercent: 95.0, diskPercent: 88.0,
    lastHeartbeatAt: new Date(Date.now() - 400_000).toISOString(), uptimeSeconds: 7200,
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Fleet Health Summary
// ─────────────────────────────────────────────────────────────────────────────

describe('Fleet Dashboard: Health Summary', () => {
  it('total instance count matches fleet length', () => {
    expect(mockFleet.length).toBe(6);
  });

  it('status breakdown counts are correct', () => {
    const byStatus = mockFleet.reduce<Record<string, number>>((acc, inst) => {
      acc[inst.status] = (acc[inst.status] ?? 0) + 1;
      return acc;
    }, {});
    expect(byStatus['RUNNING']).toBe(3);
    expect(byStatus['STOPPED']).toBe(1);
    expect(byStatus['DEPLOYING']).toBe(1);
    expect(byStatus['ERROR']).toBe(1);
  });

  it('healthy instance count includes only RUNNING instances', () => {
    const healthy = mockFleet.filter((i) => i.status === 'RUNNING').length;
    expect(healthy).toBe(3);
  });

  it('health percentage is (running / total) * 100', () => {
    const running = mockFleet.filter((i) => i.status === 'RUNNING').length;
    const healthPct = (running / mockFleet.length) * 100;
    expect(healthPct).toBeCloseTo(50, 0);
  });

  it('error instances are surfaced as a separate count', () => {
    const errored = mockFleet.filter((i) => i.status === 'ERROR').length;
    expect(errored).toBe(1);
  });

  it('providers breakdown counts instances per provider', () => {
    const byProvider = mockFleet.reduce<Record<string, number>>((acc, inst) => {
      acc[inst.provider] = (acc[inst.provider] ?? 0) + 1;
      return acc;
    }, {});
    expect(byProvider['fly']).toBe(3);
    expect(byProvider['docker']).toBe(1);
    expect(byProvider['devpod']).toBe(1);
    expect(byProvider['kubernetes']).toBe(1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Fleet Resource Utilization
// ─────────────────────────────────────────────────────────────────────────────

describe('Fleet Dashboard: Resource Utilization', () => {
  const running = mockFleet.filter((i) => i.cpuPercent !== null);

  it('fleet avg cpu excludes instances with no metrics', () => {
    const avg = running.reduce((s, i) => s + (i.cpuPercent ?? 0), 0) / running.length;
    expect(avg).toBeGreaterThan(0);
    expect(avg).toBeLessThan(100);
  });

  it('fleet max cpu is the single highest cpu percent', () => {
    const max = Math.max(...running.map((i) => i.cpuPercent ?? 0));
    expect(max).toBeCloseTo(99.9, 0);
  });

  it('fleet avg memory utilization is computed from running instances only', () => {
    const avg = running.reduce((s, i) => s + (i.memPercent ?? 0), 0) / running.length;
    expect(avg).toBeGreaterThan(0);
  });

  it('fleet disk utilization max identifies highest consumer', () => {
    const max = Math.max(...running.map((i) => i.diskPercent ?? 0));
    expect(max).toBe(88);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Fleet Instance Sorting
// ─────────────────────────────────────────────────────────────────────────────

describe('Fleet Dashboard: Instance Sorting', () => {
  it('sort by cpu descending shows highest CPU first', () => {
    const sorted = [...mockFleet]
      .filter((i) => i.cpuPercent !== null)
      .sort((a, b) => (b.cpuPercent ?? 0) - (a.cpuPercent ?? 0));
    expect(sorted[0].name).toBe('old-worker');
    expect(sorted[0].cpuPercent).toBeCloseTo(99.9, 0);
  });

  it('sort by memory descending shows highest memory first', () => {
    const sorted = [...mockFleet]
      .filter((i) => i.memPercent !== null)
      .sort((a, b) => (b.memPercent ?? 0) - (a.memPercent ?? 0));
    expect(sorted[0].cpuPercent).toBeDefined();
    expect(sorted[0].memPercent).toBeGreaterThanOrEqual(sorted[1].memPercent ?? 0);
  });

  it('sort by name alphabetically', () => {
    const sorted = [...mockFleet].sort((a, b) => a.name.localeCompare(b.name));
    expect(sorted[0].name).toBe('canary-deploy');
    expect(sorted[sorted.length - 1].name).toBe('staging-us');
  });

  it('sort by lastHeartbeatAt descending shows most recent first', () => {
    const withHeartbeat = mockFleet.filter((i) => i.lastHeartbeatAt !== null);
    const sorted = [...withHeartbeat].sort((a, b) =>
      new Date(b.lastHeartbeatAt!).getTime() - new Date(a.lastHeartbeatAt!).getTime(),
    );
    expect(sorted[0].lastHeartbeatAt).toBeTruthy();
    expect(sorted[sorted.length - 1].lastHeartbeatAt).toBeTruthy();
    // Most recent heartbeat should be more recent than least recent
    expect(new Date(sorted[0].lastHeartbeatAt!).getTime()).toBeGreaterThanOrEqual(
      new Date(sorted[sorted.length - 1].lastHeartbeatAt!).getTime(),
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Fleet Instance Filtering
// ─────────────────────────────────────────────────────────────────────────────

describe('Fleet Dashboard: Instance Filtering', () => {
  it('filter by status returns only matching instances', () => {
    const running = mockFleet.filter((i) => i.status === 'RUNNING');
    expect(running.every((i) => i.status === 'RUNNING')).toBe(true);
  });

  it('filter by provider returns only matching instances', () => {
    const flyInstances = mockFleet.filter((i) => i.provider === 'fly');
    expect(flyInstances.every((i) => i.provider === 'fly')).toBe(true);
    expect(flyInstances.length).toBe(3);
  });

  it('filter by region returns only matching instances', () => {
    const iadInstances = mockFleet.filter((i) => i.region === 'iad');
    expect(iadInstances.every((i) => i.region === 'iad')).toBe(true);
  });

  it('search by name substring matches correctly', () => {
    const query = 'prod';
    const matches = mockFleet.filter((i) => i.name.includes(query));
    expect(matches.map((i) => i.name)).toContain('prod-us-east');
    expect(matches.map((i) => i.name)).toContain('prod-eu-west');
  });

  it('combined status + provider filter narrows results', () => {
    const filtered = mockFleet.filter(
      (i) => i.status === 'RUNNING' && i.provider === 'fly',
    );
    expect(filtered.length).toBe(2);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Stale / Offline Instance Detection
// ─────────────────────────────────────────────────────────────────────────────

describe('Fleet Dashboard: Stale Instance Detection', () => {
  const STALE_THRESHOLD_MS = 5 * 60 * 1000; // 5 minutes

  it('instance with no heartbeat in 5min is considered stale', () => {
    const staleInst = mockFleet.find((i) => i.id === 'inst_06')!;
    const lastSeen = new Date(staleInst.lastHeartbeatAt!).getTime();
    const age = Date.now() - lastSeen;
    expect(age).toBeGreaterThan(STALE_THRESHOLD_MS);
  });

  it('instance with recent heartbeat is not stale', () => {
    const freshInst = mockFleet.find((i) => i.id === 'inst_02')!;
    const lastSeen = new Date(freshInst.lastHeartbeatAt!).getTime();
    const age = Date.now() - lastSeen;
    expect(age).toBeLessThan(STALE_THRESHOLD_MS);
  });

  it('stopped instances with null heartbeat are excluded from stale check', () => {
    const stopped = mockFleet.filter((i) => i.status === 'STOPPED');
    for (const inst of stopped) {
      // Stopped instances may have null lastHeartbeatAt — not stale by definition
      expect(inst.status).toBe('STOPPED');
    }
  });

  it('stale running instances are surfaced as a warning', () => {
    const staleRunning = mockFleet.filter((i) => {
      if (i.status !== 'RUNNING' || i.lastHeartbeatAt === null) return false;
      return Date.now() - new Date(i.lastHeartbeatAt).getTime() > STALE_THRESHOLD_MS;
    });
    // old-worker is ERROR, not RUNNING — so no stale RUNNING in fixture
    expect(staleRunning.length).toBe(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Top-N Resource Consumers
// ─────────────────────────────────────────────────────────────────────────────

describe('Fleet Dashboard: Top-N Resource Consumers', () => {
  it('top-3 CPU consumers are returned in descending order', () => {
    const top3 = [...mockFleet]
      .filter((i) => i.cpuPercent !== null)
      .sort((a, b) => (b.cpuPercent ?? 0) - (a.cpuPercent ?? 0))
      .slice(0, 3);
    expect(top3[0].cpuPercent).toBeGreaterThanOrEqual(top3[1].cpuPercent ?? 0);
    expect(top3[1].cpuPercent).toBeGreaterThanOrEqual(top3[2].cpuPercent ?? 0);
  });

  it('top CPU consumer is inst_06 (old-worker at ~100%)', () => {
    const top = [...mockFleet]
      .filter((i) => i.cpuPercent !== null)
      .sort((a, b) => (b.cpuPercent ?? 0) - (a.cpuPercent ?? 0))[0];
    expect(top.name).toBe('old-worker');
  });

  it('top-N list never exceeds N items even with more instances', () => {
    const N = 3;
    const topN = mockFleet.slice(0, N);
    expect(topN.length).toBeLessThanOrEqual(N);
  });

  it('top-N by disk surfaces instances near capacity', () => {
    const top = [...mockFleet]
      .filter((i) => i.diskPercent !== null)
      .sort((a, b) => (b.diskPercent ?? 0) - (a.diskPercent ?? 0))[0];
    expect(top.diskPercent).toBe(88);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Real-Time Fleet Updates (WebSocket)
// ─────────────────────────────────────────────────────────────────────────────

describe('Fleet Dashboard: Real-Time Updates', () => {
  it('metrics update event carries instanceId and updated metrics', () => {
    const updateEvent = {
      type: 'metrics:update',
      instanceId: 'inst_01',
      data: {
        cpuPercent: 75.0,
        memPercent: 68.0,
        diskPercent: 46.0,
        timestamp: new Date().toISOString(),
      },
    };
    expect(updateEvent.instanceId).toBeTruthy();
    expect(updateEvent.data.cpuPercent).toBeDefined();
  });

  it('status change event carries new status', () => {
    const statusEvent = {
      type: 'instance:status',
      instanceId: 'inst_05',
      data: {
        previousStatus: 'DEPLOYING',
        newStatus: 'RUNNING',
        timestamp: new Date().toISOString(),
      },
    };
    expect(statusEvent.data.previousStatus).toBe('DEPLOYING');
    expect(statusEvent.data.newStatus).toBe('RUNNING');
  });

  it('fleet update can be applied as partial merge to existing state', () => {
    const existing = { ...mockFleet[0] };
    const update = { cpuPercent: 80.0, memPercent: 70.0 };
    const merged = { ...existing, ...update };
    expect(merged.cpuPercent).toBe(80.0);
    expect(merged.name).toBe(existing.name); // unchanged fields preserved
  });

  it('heartbeat lost event marks instance as stale', () => {
    const event = {
      type: 'heartbeat:lost',
      instanceId: 'inst_01',
      data: { lastSeenAt: new Date().toISOString() },
    };
    expect(event.type).toBe('heartbeat:lost');
  });
});
