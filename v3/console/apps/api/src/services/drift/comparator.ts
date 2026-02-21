/**
 * Drift comparator — compares declared vs actual configuration and returns
 * a list of drifted fields with severity ratings.
 */

import type {
  DeclaredConfig,
  ActualConfig,
  ComparisonResult,
  DriftField,
  DriftSeverity,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

export function compareConfigs(declared: DeclaredConfig, actual: ActualConfig): ComparisonResult {
  const fields: DriftField[] = [];

  compareExtensions(declared, actual, fields);
  compareEnv(declared, actual, fields);
  compareResources(declared, actual, fields);
  compareNetwork(declared, actual, fields);
  compareTopLevel(declared, actual, fields);

  return {
    hasDrift: fields.length > 0,
    fields,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-section comparators
// ─────────────────────────────────────────────────────────────────────────────

function compareExtensions(
  declared: DeclaredConfig,
  actual: ActualConfig,
  out: DriftField[],
): void {
  const declaredExts = declared.extensions ?? [];
  const actualExts = actual.extensions ?? [];

  const actualMap = new Map(actualExts.map((e) => [e.name, e]));
  const declaredMap = new Map(declaredExts.map((e) => [e.name, e]));

  // Check declared extensions are present and version-matched in actual
  for (const dec of declaredExts) {
    const act = actualMap.get(dec.name);
    if (!act) {
      out.push({
        fieldPath: `extensions.${dec.name}.present`,
        declaredVal: "true",
        actualVal: "false",
        severity: "HIGH",
        description: `Extension "${dec.name}" is declared but not installed on the instance`,
      });
      continue;
    }

    if (dec.version && act.version && dec.version !== act.version) {
      out.push({
        fieldPath: `extensions.${dec.name}.version`,
        declaredVal: dec.version,
        actualVal: act.version,
        severity: "MEDIUM",
        description: `Extension "${dec.name}" version mismatch: declared ${dec.version}, actual ${act.version}`,
      });
    }

    if (dec.enabled === false && act.status !== "disabled") {
      out.push({
        fieldPath: `extensions.${dec.name}.enabled`,
        declaredVal: "false",
        actualVal: act.status ?? "unknown",
        severity: "LOW",
        description: `Extension "${dec.name}" is declared disabled but appears active`,
      });
    }
  }

  // Check for extra extensions running that are not declared
  for (const act of actualExts) {
    if (!declaredMap.has(act.name)) {
      out.push({
        fieldPath: `extensions.${act.name}.present`,
        declaredVal: "false",
        actualVal: "true",
        severity: "LOW",
        description: `Extension "${act.name}" is running but not declared in configuration`,
      });
    }
  }
}

function compareEnv(declared: DeclaredConfig, actual: ActualConfig, out: DriftField[]): void {
  const declEnv = declared.env ?? {};
  const actEnv = actual.env ?? {};

  for (const [key, declaredVal] of Object.entries(declEnv)) {
    const actualVal = actEnv[key];
    if (actualVal === undefined) {
      out.push({
        fieldPath: `env.${key}`,
        declaredVal,
        actualVal: null,
        severity: "HIGH",
        description: `Environment variable "${key}" is declared but not set on the instance`,
      });
    } else if (declaredVal !== actualVal) {
      // Detect if this looks like a secret placeholder vs actual mismatch
      const looksLikeSecret =
        key.toLowerCase().includes("secret") ||
        key.toLowerCase().includes("token") ||
        key.toLowerCase().includes("password") ||
        key.toLowerCase().includes("key");

      out.push({
        fieldPath: `env.${key}`,
        declaredVal: looksLikeSecret ? "<redacted>" : declaredVal,
        actualVal: looksLikeSecret ? "<redacted>" : actualVal,
        severity: looksLikeSecret ? "HIGH" : "MEDIUM",
        description: looksLikeSecret
          ? `Secret env var "${key}" value differs from declaration`
          : `Environment variable "${key}" value differs: declared "${declaredVal}", actual "${actualVal}"`,
      });
    }
  }
}

function compareResources(declared: DeclaredConfig, actual: ActualConfig, out: DriftField[]): void {
  if (!declared.resources) return;

  const decRes = declared.resources;
  const actRes = actual.resources ?? {};

  if (decRes.cpu !== undefined && actRes.cpu_count !== undefined) {
    if (decRes.cpu !== actRes.cpu_count) {
      out.push({
        fieldPath: "resources.cpu",
        declaredVal: String(decRes.cpu),
        actualVal: String(actRes.cpu_count),
        severity: "HIGH",
        description: `CPU allocation mismatch: declared ${decRes.cpu} cores, actual ${actRes.cpu_count} cores`,
      });
    }
  }

  if (decRes.memory && actRes.memory_total) {
    const decNorm = normalizeMemory(decRes.memory);
    const actNorm = normalizeMemory(actRes.memory_total);
    if (decNorm !== actNorm) {
      out.push({
        fieldPath: "resources.memory",
        declaredVal: decRes.memory,
        actualVal: actRes.memory_total,
        severity: "MEDIUM",
        description: `Memory allocation mismatch: declared ${decRes.memory}, actual ${actRes.memory_total}`,
      });
    }
  }
}

function compareNetwork(declared: DeclaredConfig, actual: ActualConfig, out: DriftField[]): void {
  const decNet = declared.network ?? {};
  const actNet = actual.network ?? {};

  if (decNet.hostname && actNet.hostname && decNet.hostname !== actNet.hostname) {
    out.push({
      fieldPath: "network.hostname",
      declaredVal: decNet.hostname,
      actualVal: actNet.hostname,
      severity: "MEDIUM",
      description: `Hostname mismatch: declared "${decNet.hostname}", actual "${actNet.hostname}"`,
    });
  }

  if (decNet.ports && decNet.ports.length > 0) {
    const actPorts = new Set(actNet.open_ports ?? []);
    for (const port of decNet.ports) {
      if (!actPorts.has(port)) {
        out.push({
          fieldPath: `network.ports.${port}`,
          declaredVal: String(port),
          actualVal: null,
          severity: "HIGH",
          description: `Port ${port} is declared but not open on the instance`,
        });
      }
    }
  }
}

function compareTopLevel(declared: DeclaredConfig, actual: ActualConfig, out: DriftField[]): void {
  if (declared.provider && actual.provider && declared.provider !== actual.provider) {
    out.push({
      fieldPath: "provider",
      declaredVal: declared.provider,
      actualVal: actual.provider,
      severity: "CRITICAL",
      description: `Provider mismatch: declared "${declared.provider}", actual "${actual.provider}"`,
    });
  }

  if (declared.region && actual.region && declared.region !== actual.region) {
    out.push({
      fieldPath: "region",
      declaredVal: declared.region,
      actualVal: actual.region,
      severity: "HIGH",
      description: `Region mismatch: declared "${declared.region}", actual "${actual.region}"`,
    });
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/** Normalise memory strings like "512mb", "1GB", "2gi" to bytes for comparison */
function normalizeMemory(value: string): number {
  const lower = value.toLowerCase().trim();
  const num = parseFloat(lower);
  if (Number.isNaN(num)) return 0;

  if (lower.endsWith("gi") || lower.endsWith("gib")) return Math.round(num * 1024 ** 3);
  if (lower.endsWith("gb")) return Math.round(num * 1000 ** 3);
  if (lower.endsWith("mi") || lower.endsWith("mib")) return Math.round(num * 1024 ** 2);
  if (lower.endsWith("mb")) return Math.round(num * 1000 ** 2);
  if (lower.endsWith("ki") || lower.endsWith("kib")) return Math.round(num * 1024);
  if (lower.endsWith("kb")) return Math.round(num * 1000);
  return Math.round(num);
}

export function maxSeverity(fields: DriftField[]): DriftSeverity {
  const rank: Record<DriftSeverity, number> = { CRITICAL: 3, HIGH: 2, MEDIUM: 1, LOW: 0 };
  if (fields.length === 0) return "LOW";
  return fields.reduce<DriftSeverity>(
    (max, f) => (rank[f.severity] > rank[max] ? f.severity : max),
    "LOW",
  );
}
