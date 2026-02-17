/**
 * Fleet service — aggregate statistics and geo data for the fleet overview dashboard.
 */

import { db } from "../lib/db.js";

// ─────────────────────────────────────────────────────────────────────────────
// Region → approximate lat/lon mapping for geo visualization
// ─────────────────────────────────────────────────────────────────────────────

const REGION_COORDS: Record<string, { lat: number; lon: number; label: string }> = {
  // Fly.io regions
  iad: { lat: 38.94, lon: -77.46, label: "Ashburn, VA" },
  lax: { lat: 33.94, lon: -118.41, label: "Los Angeles" },
  ord: { lat: 41.97, lon: -87.91, label: "Chicago" },
  lhr: { lat: 51.47, lon: -0.45, label: "London" },
  fra: { lat: 50.11, lon: 8.68, label: "Frankfurt" },
  nrt: { lat: 35.77, lon: 140.39, label: "Tokyo" },
  syd: { lat: -33.94, lon: 151.18, label: "Sydney" },
  sea: { lat: 47.45, lon: -122.3, label: "Seattle" },
  dfw: { lat: 32.9, lon: -97.04, label: "Dallas" },
  sin: { lat: 1.36, lon: 103.99, label: "Singapore" },
  bom: { lat: 19.09, lon: 72.87, label: "Mumbai" },
  gru: { lat: -23.43, lon: -46.47, label: "São Paulo" },
  // AWS / E2B regions
  "us-east-1": { lat: 38.13, lon: -78.45, label: "US East (N. Virginia)" },
  "us-east-2": { lat: 39.96, lon: -83.0, label: "US East (Ohio)" },
  "us-west-1": { lat: 37.35, lon: -121.96, label: "US West (N. California)" },
  "us-west-2": { lat: 45.87, lon: -119.69, label: "US West (Oregon)" },
  "eu-west-1": { lat: 53.34, lon: -6.26, label: "EU West (Ireland)" },
  "eu-central-1": { lat: 50.11, lon: 8.68, label: "EU Central (Frankfurt)" },
  "ap-southeast-1": { lat: 1.36, lon: 103.99, label: "AP (Singapore)" },
  "ap-northeast-1": { lat: 35.68, lon: 139.69, label: "AP (Tokyo)" },
  // Generic fallbacks
  local: { lat: 37.77, lon: -122.42, label: "Local" },
  default: { lat: 40.71, lon: -74.01, label: "Default" },
  production: { lat: 40.71, lon: -74.01, label: "Production" },
  staging: { lat: 37.77, lon: -122.42, label: "Staging" },
  ssh: { lat: 48.86, lon: 2.35, label: "SSH Remote" },
};

function getRegionCoords(
  region: string | null,
): { lat: number; lon: number; label: string } | null {
  if (!region) return null;
  const key = region.toLowerCase();
  return REGION_COORDS[key] ?? null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fleet stats — shape matches FleetStats in apps/web/src/types/fleet.ts
// ─────────────────────────────────────────────────────────────────────────────

export interface FleetStats {
  total: number;
  by_status: Record<string, number>;
  by_provider: { provider: string; count: number }[];
  active_sessions: number;
  updated_at: string;
}

export async function getFleetStats(): Promise<FleetStats> {
  const [statusCounts, providerCounts, sessionCount] = await Promise.all([
    db.instance.groupBy({
      by: ["status"],
      _count: { status: true },
    }),
    db.instance.groupBy({
      by: ["provider"],
      _count: { provider: true },
    }),
    db.terminalSession.count({ where: { status: "ACTIVE" } }),
  ]);

  const byStatus = Object.fromEntries(
    statusCounts.map((r: { status: string; _count: { status: number } }) => [
      r.status,
      r._count.status,
    ]),
  );

  type StatusRow = { status: string; _count: { status: number } };
  type ProviderRow = { provider: string; _count: { provider: number } };

  return {
    total: (statusCounts as StatusRow[]).reduce(
      (sum: number, r: StatusRow) => sum + r._count.status,
      0,
    ),
    by_status: byStatus,
    by_provider: (providerCounts as ProviderRow[]).map((r: ProviderRow) => ({
      provider: r.provider,
      count: r._count.provider,
    })),
    active_sessions: sessionCount,
    updated_at: new Date().toISOString(),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Fleet geo
// ─────────────────────────────────────────────────────────────────────────────

export interface GeoPin {
  region: string;
  lat: number;
  lon: number;
  label: string;
  count: number;
  statuses: Record<string, number>;
}

export async function getFleetGeo(): Promise<GeoPin[]> {
  const instances = await db.instance.findMany({
    select: { id: true, region: true, status: true, provider: true },
  });

  const regionMap = new Map<string, GeoPin>();

  for (const inst of instances) {
    const regionKey = inst.region ?? `${inst.provider}-local`;
    const coords = getRegionCoords(inst.region);
    if (!coords) continue;

    if (!regionMap.has(regionKey)) {
      regionMap.set(regionKey, {
        region: regionKey,
        lat: coords.lat,
        lon: coords.lon,
        label: coords.label,
        count: 0,
        statuses: {},
      });
    }

    const pin = regionMap.get(regionKey)!;
    pin.count += 1;
    pin.statuses[inst.status] = (pin.statuses[inst.status] ?? 0) + 1;
  }

  return Array.from(regionMap.values());
}

// ─────────────────────────────────────────────────────────────────────────────
// Fleet deployments — shape matches FleetDeploymentsResponse in frontend types
// ─────────────────────────────────────────────────────────────────────────────

export interface DeploymentActivity {
  hour: string; // ISO timestamp rounded to the hour
  deployments: number;
  failures: number;
}

export interface FleetDeploymentsResponse {
  activity: DeploymentActivity[];
  total_24h: number;
  success_rate: number;
}

export async function getFleetDeployments(): Promise<FleetDeploymentsResponse> {
  const since = new Date(Date.now() - 24 * 60 * 60 * 1000);

  const deployments = await db.deployment.findMany({
    where: { started_at: { gte: since } },
    select: { started_at: true, status: true },
    orderBy: { started_at: "asc" },
  });

  // Build 24 hourly buckets
  const buckets = new Map<string, DeploymentActivity>();
  const now = new Date();

  for (let h = 23; h >= 0; h--) {
    const d = new Date(now);
    d.setHours(d.getHours() - h, 0, 0, 0);
    const key = d.toISOString().slice(0, 13) + ":00:00.000Z";
    buckets.set(key, { hour: key, deployments: 0, failures: 0 });
  }

  let totalSucceeded = 0;
  let totalFailed = 0;

  for (const dep of deployments) {
    const key = dep.started_at.toISOString().slice(0, 13) + ":00:00.000Z";
    const bucket = buckets.get(key);
    if (!bucket) continue;
    bucket.deployments += 1;
    if (dep.status === "FAILED") {
      bucket.failures += 1;
      totalFailed += 1;
    } else if (dep.status === "SUCCEEDED") {
      totalSucceeded += 1;
    }
  }

  const total = deployments.length;
  const successRate = total > 0 ? Math.round((totalSucceeded / total) * 100) : 100;

  return {
    activity: Array.from(buckets.values()),
    total_24h: total,
    success_rate: successRate,
  };
}
