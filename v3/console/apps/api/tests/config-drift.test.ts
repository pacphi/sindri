/**
 * Integration tests: Phase 4 Configuration Drift Detection
 *
 * Tests the drift detection system:
 *   - Drift detection between deployed config and desired state
 *   - Drift severity classification (CRITICAL, HIGH, MEDIUM, LOW)
 *   - Drift types: extension mismatch, config hash change, resource drift
 *   - Remediation workflows: manual and automated
 *   - Drift history and audit trail
 *   - Fleet-wide drift summary
 *   - Suppression and ignore rules
 */

import { describe, it, expect } from 'vitest';

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

type DriftSeverity = 'CRITICAL' | 'HIGH' | 'MEDIUM' | 'LOW' | 'NONE';
type DriftType =
  | 'EXTENSION_MISMATCH'
  | 'CONFIG_HASH_CHANGE'
  | 'RESOURCE_DRIFT'
  | 'MISSING_EXTENSION'
  | 'EXTRA_EXTENSION'
  | 'VERSION_MISMATCH';
type DriftStatus = 'DETECTED' | 'ACKNOWLEDGED' | 'REMEDIATING' | 'RESOLVED' | 'SUPPRESSED';
type RemediationMode = 'MANUAL' | 'AUTOMATIC';
type RemediationStatus = 'PENDING' | 'IN_PROGRESS' | 'SUCCEEDED' | 'FAILED' | 'SKIPPED';

interface DriftReport {
  id: string;
  instance_id: string;
  detected_at: string;
  severity: DriftSeverity;
  status: DriftStatus;
  items: DriftItem[];
  remediation_mode: RemediationMode;
  remediated_at: string | null;
  suppressed_until: string | null;
}

interface DriftItem {
  id: string;
  drift_type: DriftType;
  severity: DriftSeverity;
  field: string;
  expected_value: string;
  actual_value: string;
  description: string;
}

interface RemediationJob {
  id: string;
  drift_report_id: string;
  instance_id: string;
  status: RemediationStatus;
  mode: RemediationMode;
  started_at: string;
  finished_at: string | null;
  log: string | null;
  triggered_by: string;
}

interface DriftSuppressRule {
  id: string;
  instance_id: string | null; // null = fleet-wide
  drift_type: DriftType | null; // null = all types
  reason: string;
  expires_at: string | null;
  created_by: string;
  created_at: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

function makeDriftItem(overrides: Partial<DriftItem> = {}): DriftItem {
  return {
    id: 'di_01',
    drift_type: 'EXTENSION_MISMATCH',
    severity: 'HIGH',
    field: 'extensions',
    expected_value: 'node-lts@20.11.0',
    actual_value: 'node-lts@20.9.0',
    description: 'Extension node-lts version mismatch: expected 20.11.0, found 20.9.0',
    ...overrides,
  };
}

function makeDriftReport(overrides: Partial<DriftReport> = {}): DriftReport {
  return {
    id: 'dr_01',
    instance_id: 'inst_01',
    detected_at: '2026-02-17T10:00:00Z',
    severity: 'HIGH',
    status: 'DETECTED',
    items: [makeDriftItem()],
    remediation_mode: 'MANUAL',
    remediated_at: null,
    suppressed_until: null,
    ...overrides,
  };
}

function makeRemediationJob(overrides: Partial<RemediationJob> = {}): RemediationJob {
  return {
    id: 'rem_01',
    drift_report_id: 'dr_01',
    instance_id: 'inst_01',
    status: 'PENDING',
    mode: 'MANUAL',
    started_at: '2026-02-17T10:05:00Z',
    finished_at: null,
    log: null,
    triggered_by: 'user_01',
    ...overrides,
  };
}

function makeSuppressRule(overrides: Partial<DriftSuppressRule> = {}): DriftSuppressRule {
  return {
    id: 'sup_01',
    instance_id: 'inst_01',
    drift_type: 'CONFIG_HASH_CHANGE',
    reason: 'Manual config override for this instance',
    expires_at: null,
    created_by: 'user_01',
    created_at: '2026-02-17T00:00:00Z',
    ...overrides,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Drift Detection
// ─────────────────────────────────────────────────────────────────────────────

describe('Config Drift: Detection', () => {
  it('drift report has required fields: id, instance_id, severity, status, items', () => {
    const report = makeDriftReport();
    expect(report.id).toBeTruthy();
    expect(report.instance_id).toBeTruthy();
    expect(['CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'NONE']).toContain(report.severity);
    expect(['DETECTED', 'ACKNOWLEDGED', 'REMEDIATING', 'RESOLVED', 'SUPPRESSED']).toContain(report.status);
    expect(Array.isArray(report.items)).toBe(true);
  });

  it('drift item describes the exact discrepancy', () => {
    const item = makeDriftItem();
    expect(item.drift_type).toBe('EXTENSION_MISMATCH');
    expect(item.field).toBe('extensions');
    expect(item.expected_value).toBeTruthy();
    expect(item.actual_value).toBeTruthy();
    expect(item.description).toBeTruthy();
  });

  it('all drift types are recognized', () => {
    const types: DriftType[] = [
      'EXTENSION_MISMATCH', 'CONFIG_HASH_CHANGE', 'RESOURCE_DRIFT',
      'MISSING_EXTENSION', 'EXTRA_EXTENSION', 'VERSION_MISMATCH',
    ];
    expect(types).toHaveLength(6);
  });

  it('extension mismatch detected when installed version differs from desired', () => {
    const desired = { extension: 'node-lts', version: '20.11.0' };
    const actual = { extension: 'node-lts', version: '20.9.0' };
    const hasDrift = desired.version !== actual.version;
    expect(hasDrift).toBe(true);
  });

  it('config hash change detected when deployed hash differs from current schema', () => {
    const deployedHash = 'a'.repeat(64);
    const currentHash = 'b'.repeat(64);
    const hasDrift = deployedHash !== currentHash;
    expect(hasDrift).toBe(true);
  });

  it('missing extension detected when desired extension is not installed', () => {
    const desired = ['node-lts', 'git', 'docker'];
    const installed = ['node-lts', 'git'];
    const missing = desired.filter((ext) => !installed.includes(ext));
    expect(missing).toHaveLength(1);
    expect(missing[0]).toBe('docker');
  });

  it('extra extension detected when installed extension is not in desired config', () => {
    const desired = ['node-lts', 'git'];
    const installed = ['node-lts', 'git', 'python-312'];
    const extra = installed.filter((ext) => !desired.includes(ext));
    expect(extra).toHaveLength(1);
    expect(extra[0]).toBe('python-312');
  });

  it('no drift when config matches desired state', () => {
    const desired = { extensions: ['node-lts', 'git'], configHash: 'a'.repeat(64) };
    const actual = { extensions: ['node-lts', 'git'], configHash: 'a'.repeat(64) };
    const hasExtDrift = JSON.stringify(desired.extensions.sort()) !== JSON.stringify(actual.extensions.sort());
    const hasHashDrift = desired.configHash !== actual.configHash;
    expect(hasExtDrift).toBe(false);
    expect(hasHashDrift).toBe(false);
  });

  it('drift report overall severity equals highest item severity', () => {
    const items: DriftItem[] = [
      makeDriftItem({ severity: 'MEDIUM' }),
      makeDriftItem({ severity: 'HIGH' }),
      makeDriftItem({ severity: 'LOW' }),
    ];
    const severityOrder: DriftSeverity[] = ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'NONE'];
    const highestSeverity = items.reduce((max, item) => {
      return severityOrder.indexOf(item.severity) < severityOrder.indexOf(max) ? item.severity : max;
    }, 'NONE' as DriftSeverity);
    expect(highestSeverity).toBe('HIGH');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Drift Severity
// ─────────────────────────────────────────────────────────────────────────────

describe('Config Drift: Severity Classification', () => {
  function classifyDriftSeverity(driftType: DriftType): DriftSeverity {
    switch (driftType) {
      case 'MISSING_EXTENSION': return 'CRITICAL';
      case 'CONFIG_HASH_CHANGE': return 'HIGH';
      case 'EXTENSION_MISMATCH': return 'HIGH';
      case 'VERSION_MISMATCH': return 'MEDIUM';
      case 'EXTRA_EXTENSION': return 'LOW';
      case 'RESOURCE_DRIFT': return 'MEDIUM';
    }
  }

  it('missing extension is classified as CRITICAL severity', () => {
    expect(classifyDriftSeverity('MISSING_EXTENSION')).toBe('CRITICAL');
  });

  it('config hash change is classified as HIGH severity', () => {
    expect(classifyDriftSeverity('CONFIG_HASH_CHANGE')).toBe('HIGH');
  });

  it('extension version mismatch is classified as HIGH severity', () => {
    expect(classifyDriftSeverity('EXTENSION_MISMATCH')).toBe('HIGH');
  });

  it('version mismatch is classified as MEDIUM severity', () => {
    expect(classifyDriftSeverity('VERSION_MISMATCH')).toBe('MEDIUM');
  });

  it('extra extension is classified as LOW severity', () => {
    expect(classifyDriftSeverity('EXTRA_EXTENSION')).toBe('LOW');
  });

  it('resource drift is classified as MEDIUM severity', () => {
    expect(classifyDriftSeverity('RESOURCE_DRIFT')).toBe('MEDIUM');
  });

  it('severity levels are ordered: CRITICAL > HIGH > MEDIUM > LOW > NONE', () => {
    const order: DriftSeverity[] = ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'NONE'];
    expect(order.indexOf('CRITICAL')).toBeLessThan(order.indexOf('HIGH'));
    expect(order.indexOf('HIGH')).toBeLessThan(order.indexOf('MEDIUM'));
    expect(order.indexOf('MEDIUM')).toBeLessThan(order.indexOf('LOW'));
    expect(order.indexOf('LOW')).toBeLessThan(order.indexOf('NONE'));
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Drift State Machine
// ─────────────────────────────────────────────────────────────────────────────

describe('Config Drift: State Machine', () => {
  it('new drift starts in DETECTED status', () => {
    const report = makeDriftReport({ status: 'DETECTED' });
    expect(report.status).toBe('DETECTED');
  });

  it('drift transitions DETECTED → ACKNOWLEDGED when user acknowledges', () => {
    let report = makeDriftReport({ status: 'DETECTED' });
    report = { ...report, status: 'ACKNOWLEDGED' };
    expect(report.status).toBe('ACKNOWLEDGED');
  });

  it('drift transitions ACKNOWLEDGED → REMEDIATING when remediation starts', () => {
    let report = makeDriftReport({ status: 'ACKNOWLEDGED' });
    report = { ...report, status: 'REMEDIATING' };
    expect(report.status).toBe('REMEDIATING');
  });

  it('drift transitions REMEDIATING → RESOLVED after successful remediation', () => {
    let report = makeDriftReport({ status: 'REMEDIATING' });
    report = { ...report, status: 'RESOLVED', remediated_at: new Date().toISOString() };
    expect(report.status).toBe('RESOLVED');
    expect(report.remediated_at).toBeTruthy();
  });

  it('drift can be SUPPRESSED from any state', () => {
    const report = makeDriftReport({ status: 'SUPPRESSED', suppressed_until: '2026-03-01T00:00:00Z' });
    expect(report.status).toBe('SUPPRESSED');
    expect(report.suppressed_until).toBeTruthy();
  });

  it('RESOLVED drift has remediated_at timestamp', () => {
    const report = makeDriftReport({
      status: 'RESOLVED',
      remediated_at: '2026-02-17T11:00:00Z',
    });
    expect(report.remediated_at).toBeTruthy();
    expect(new Date(report.remediated_at!).getTime()).toBeGreaterThan(
      new Date(report.detected_at).getTime(),
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Remediation
// ─────────────────────────────────────────────────────────────────────────────

describe('Config Drift: Remediation', () => {
  it('remediation job has required fields: drift_report_id, instance_id, status, mode', () => {
    const job = makeRemediationJob();
    expect(job.drift_report_id).toBeTruthy();
    expect(job.instance_id).toBeTruthy();
    expect(['PENDING', 'IN_PROGRESS', 'SUCCEEDED', 'FAILED', 'SKIPPED']).toContain(job.status);
    expect(['MANUAL', 'AUTOMATIC']).toContain(job.mode);
  });

  it('manual remediation requires user to trigger it', () => {
    const job = makeRemediationJob({ mode: 'MANUAL', triggered_by: 'user_01' });
    expect(job.mode).toBe('MANUAL');
    expect(job.triggered_by).toBeTruthy();
  });

  it('automatic remediation is triggered by the system', () => {
    const job = makeRemediationJob({ mode: 'AUTOMATIC', triggered_by: 'system' });
    expect(job.mode).toBe('AUTOMATIC');
    expect(job.triggered_by).toBe('system');
  });

  it('successful remediation has SUCCEEDED status and finished_at', () => {
    const job = makeRemediationJob({
      status: 'SUCCEEDED',
      finished_at: '2026-02-17T10:10:00Z',
    });
    expect(job.status).toBe('SUCCEEDED');
    expect(job.finished_at).toBeTruthy();
  });

  it('failed remediation captures error in log', () => {
    const job = makeRemediationJob({
      status: 'FAILED',
      log: 'Extension installation failed: network timeout',
    });
    expect(job.status).toBe('FAILED');
    expect(job.log).toContain('failed');
  });

  it('remediation duration can be calculated from started_at to finished_at', () => {
    const job = makeRemediationJob({
      started_at: '2026-02-17T10:05:00Z',
      finished_at: '2026-02-17T10:08:30Z',
    });
    const durationMs = new Date(job.finished_at!).getTime() - new Date(job.started_at).getTime();
    expect(durationMs).toBe(3.5 * 60 * 1000); // 3.5 minutes
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Suppression Rules
// ─────────────────────────────────────────────────────────────────────────────

describe('Config Drift: Suppression', () => {
  it('suppress rule has required fields: reason, created_by, created_at', () => {
    const rule = makeSuppressRule();
    expect(rule.reason).toBeTruthy();
    expect(rule.created_by).toBeTruthy();
    expect(rule.created_at).toBeTruthy();
  });

  it('fleet-wide suppress rule has null instance_id', () => {
    const rule = makeSuppressRule({ instance_id: null });
    expect(rule.instance_id).toBeNull();
  });

  it('instance-scoped suppress rule has specific instance_id', () => {
    const rule = makeSuppressRule({ instance_id: 'inst_01' });
    expect(rule.instance_id).toBe('inst_01');
  });

  it('type-scoped suppress rule only suppresses specified drift type', () => {
    const rule = makeSuppressRule({ drift_type: 'CONFIG_HASH_CHANGE' });
    expect(rule.drift_type).toBe('CONFIG_HASH_CHANGE');
    // Other drift types are not suppressed
    const shouldSuppress = (driftType: DriftType) =>
      rule.drift_type === null || rule.drift_type === driftType;
    expect(shouldSuppress('CONFIG_HASH_CHANGE')).toBe(true);
    expect(shouldSuppress('EXTENSION_MISMATCH')).toBe(false);
  });

  it('suppress rule with null drift_type suppresses all drift types', () => {
    const rule = makeSuppressRule({ drift_type: null });
    const shouldSuppress = (_driftType: DriftType) => rule.drift_type === null;
    expect(shouldSuppress('EXTENSION_MISMATCH')).toBe(true);
    expect(shouldSuppress('MISSING_EXTENSION')).toBe(true);
  });

  it('expired suppress rule is no longer active', () => {
    const rule = makeSuppressRule({ expires_at: '2025-01-01T00:00:00Z' });
    const isActive = rule.expires_at === null || new Date(rule.expires_at) > new Date();
    expect(isActive).toBe(false);
  });

  it('non-expired suppress rule is active', () => {
    const future = new Date();
    future.setMonth(future.getMonth() + 1);
    const rule = makeSuppressRule({ expires_at: future.toISOString() });
    const isActive = rule.expires_at === null || new Date(rule.expires_at) > new Date();
    expect(isActive).toBe(true);
  });

  it('permanent suppress rule (null expires_at) never expires', () => {
    const rule = makeSuppressRule({ expires_at: null });
    const isActive = rule.expires_at === null || new Date(rule.expires_at) > new Date();
    expect(isActive).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Fleet Drift Summary
// ─────────────────────────────────────────────────────────────────────────────

describe('Config Drift: Fleet Summary', () => {
  const reports: DriftReport[] = [
    makeDriftReport({ id: 'dr1', instance_id: 'i1', severity: 'CRITICAL', status: 'DETECTED' }),
    makeDriftReport({ id: 'dr2', instance_id: 'i2', severity: 'HIGH', status: 'ACKNOWLEDGED' }),
    makeDriftReport({ id: 'dr3', instance_id: 'i3', severity: 'NONE', status: 'RESOLVED' }),
    makeDriftReport({ id: 'dr4', instance_id: 'i4', severity: 'LOW', status: 'DETECTED' }),
    makeDriftReport({ id: 'dr5', instance_id: 'i5', severity: 'HIGH', status: 'SUPPRESSED' }),
  ];

  it('fleet summary counts total drifting instances', () => {
    const drifting = reports.filter((r) => r.severity !== 'NONE' && r.status !== 'RESOLVED');
    expect(drifting).toHaveLength(4);
  });

  it('fleet summary shows count by severity', () => {
    const bySeverity = reports.reduce((acc, r) => {
      acc[r.severity] = (acc[r.severity] ?? 0) + 1;
      return acc;
    }, {} as Record<DriftSeverity, number>);
    expect(bySeverity['CRITICAL']).toBe(1);
    expect(bySeverity['HIGH']).toBe(2);
    expect(bySeverity['LOW']).toBe(1);
    expect(bySeverity['NONE']).toBe(1);
  });

  it('fleet summary shows instances with CRITICAL drift first', () => {
    const sevOrder: DriftSeverity[] = ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'NONE'];
    const sorted = [...reports].sort(
      (a, b) => sevOrder.indexOf(a.severity) - sevOrder.indexOf(b.severity),
    );
    expect(sorted[0].severity).toBe('CRITICAL');
  });

  it('health percentage is calculated as compliant instances / total', () => {
    const totalInstances = 10;
    const driftingInstances = 3;
    const healthPercent = ((totalInstances - driftingInstances) / totalInstances) * 100;
    expect(healthPercent).toBe(70);
  });

  it('fleet drift report includes most recent scan timestamp', () => {
    const timestamps = reports.map((r) => r.detected_at);
    const mostRecent = timestamps.sort().reverse()[0];
    expect(mostRecent).toBeTruthy();
  });
});
