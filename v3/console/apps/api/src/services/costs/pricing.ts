/**
 * Provider-specific pricing tables.
 * All prices are in USD per month unless noted.
 * These are approximate list prices — actual billing may vary.
 */

export interface ComputeTier {
  id: string;
  label: string;
  vcpu: number;
  memoryGb: number;
  pricePerMonth: number; // USD
  pricePerHour: number; // USD
}

export interface StoragePricing {
  gbPerMonth: number; // USD per GB per month
}

export interface NetworkPricing {
  egressGbFree: number; // free egress GB per month
  egressGbPrice: number; // USD per GB after free tier
}

export interface ProviderPricing {
  name: string;
  computeTiers: ComputeTier[];
  storage: StoragePricing;
  network: NetworkPricing;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fly.io
// ─────────────────────────────────────────────────────────────────────────────

export const flyPricing: ProviderPricing = {
  name: "fly",
  computeTiers: [
    {
      id: "shared-cpu-1x",
      label: "shared-cpu-1x (256 MB)",
      vcpu: 1,
      memoryGb: 0.25,
      pricePerMonth: 1.94,
      pricePerHour: 0.0026,
    },
    {
      id: "shared-cpu-1x-512",
      label: "shared-cpu-1x (512 MB)",
      vcpu: 1,
      memoryGb: 0.5,
      pricePerMonth: 3.88,
      pricePerHour: 0.0053,
    },
    {
      id: "shared-cpu-2x",
      label: "shared-cpu-2x (1 GB)",
      vcpu: 2,
      memoryGb: 1,
      pricePerMonth: 7.76,
      pricePerHour: 0.0106,
    },
    {
      id: "performance-1x",
      label: "performance-1x (2 GB)",
      vcpu: 1,
      memoryGb: 2,
      pricePerMonth: 31.0,
      pricePerHour: 0.0425,
    },
    {
      id: "performance-2x",
      label: "performance-2x (4 GB)",
      vcpu: 2,
      memoryGb: 4,
      pricePerMonth: 62.0,
      pricePerHour: 0.085,
    },
    {
      id: "performance-4x",
      label: "performance-4x (8 GB)",
      vcpu: 4,
      memoryGb: 8,
      pricePerMonth: 124.0,
      pricePerHour: 0.17,
    },
    {
      id: "performance-8x",
      label: "performance-8x (16 GB)",
      vcpu: 8,
      memoryGb: 16,
      pricePerMonth: 248.0,
      pricePerHour: 0.34,
    },
  ],
  storage: { gbPerMonth: 0.15 },
  network: { egressGbFree: 100, egressGbPrice: 0.02 },
};

// ─────────────────────────────────────────────────────────────────────────────
// AWS (EC2 on-demand, us-east-1)
// ─────────────────────────────────────────────────────────────────────────────

export const awsPricing: ProviderPricing = {
  name: "aws",
  computeTiers: [
    {
      id: "t3.micro",
      label: "t3.micro (1 GB)",
      vcpu: 2,
      memoryGb: 1,
      pricePerMonth: 7.59,
      pricePerHour: 0.0104,
    },
    {
      id: "t3.small",
      label: "t3.small (2 GB)",
      vcpu: 2,
      memoryGb: 2,
      pricePerMonth: 15.18,
      pricePerHour: 0.0208,
    },
    {
      id: "t3.medium",
      label: "t3.medium (4 GB)",
      vcpu: 2,
      memoryGb: 4,
      pricePerMonth: 30.37,
      pricePerHour: 0.0416,
    },
    {
      id: "t3.large",
      label: "t3.large (8 GB)",
      vcpu: 2,
      memoryGb: 8,
      pricePerMonth: 60.74,
      pricePerHour: 0.0832,
    },
    {
      id: "m5.xlarge",
      label: "m5.xlarge (16 GB)",
      vcpu: 4,
      memoryGb: 16,
      pricePerMonth: 140.16,
      pricePerHour: 0.192,
    },
    {
      id: "m5.2xlarge",
      label: "m5.2xlarge (32 GB)",
      vcpu: 8,
      memoryGb: 32,
      pricePerMonth: 280.32,
      pricePerHour: 0.384,
    },
  ],
  storage: { gbPerMonth: 0.08 }, // gp3 EBS
  network: { egressGbFree: 1, egressGbPrice: 0.09 },
};

// ─────────────────────────────────────────────────────────────────────────────
// GCP (us-central1, e2 series)
// ─────────────────────────────────────────────────────────────────────────────

export const gcpPricing: ProviderPricing = {
  name: "gcp",
  computeTiers: [
    {
      id: "e2-micro",
      label: "e2-micro (1 GB)",
      vcpu: 0.25,
      memoryGb: 1,
      pricePerMonth: 6.11,
      pricePerHour: 0.0084,
    },
    {
      id: "e2-small",
      label: "e2-small (2 GB)",
      vcpu: 0.5,
      memoryGb: 2,
      pricePerMonth: 12.23,
      pricePerHour: 0.0168,
    },
    {
      id: "e2-medium",
      label: "e2-medium (4 GB)",
      vcpu: 1,
      memoryGb: 4,
      pricePerMonth: 24.46,
      pricePerHour: 0.0335,
    },
    {
      id: "e2-standard-2",
      label: "e2-standard-2 (8 GB)",
      vcpu: 2,
      memoryGb: 8,
      pricePerMonth: 48.91,
      pricePerHour: 0.067,
    },
    {
      id: "e2-standard-4",
      label: "e2-standard-4 (16 GB)",
      vcpu: 4,
      memoryGb: 16,
      pricePerMonth: 97.83,
      pricePerHour: 0.134,
    },
    {
      id: "e2-standard-8",
      label: "e2-standard-8 (32 GB)",
      vcpu: 8,
      memoryGb: 32,
      pricePerMonth: 195.65,
      pricePerHour: 0.268,
    },
  ],
  storage: { gbPerMonth: 0.04 }, // standard persistent disk
  network: { egressGbFree: 1, egressGbPrice: 0.08 },
};

// ─────────────────────────────────────────────────────────────────────────────
// Azure (East US, B series)
// ─────────────────────────────────────────────────────────────────────────────

export const azurePricing: ProviderPricing = {
  name: "azure",
  computeTiers: [
    {
      id: "B1s",
      label: "B1s (1 GB)",
      vcpu: 1,
      memoryGb: 1,
      pricePerMonth: 7.59,
      pricePerHour: 0.0104,
    },
    {
      id: "B1ms",
      label: "B1ms (2 GB)",
      vcpu: 1,
      memoryGb: 2,
      pricePerMonth: 15.11,
      pricePerHour: 0.0207,
    },
    {
      id: "B2s",
      label: "B2s (4 GB)",
      vcpu: 2,
      memoryGb: 4,
      pricePerMonth: 30.37,
      pricePerHour: 0.0416,
    },
    {
      id: "B2ms",
      label: "B2ms (8 GB)",
      vcpu: 2,
      memoryGb: 8,
      pricePerMonth: 60.74,
      pricePerHour: 0.0832,
    },
    {
      id: "B4ms",
      label: "B4ms (16 GB)",
      vcpu: 4,
      memoryGb: 16,
      pricePerMonth: 121.47,
      pricePerHour: 0.1664,
    },
    {
      id: "B8ms",
      label: "B8ms (32 GB)",
      vcpu: 8,
      memoryGb: 32,
      pricePerMonth: 242.94,
      pricePerHour: 0.3328,
    },
  ],
  storage: { gbPerMonth: 0.095 }, // premium SSD
  network: { egressGbFree: 5, egressGbPrice: 0.087 },
};

// ─────────────────────────────────────────────────────────────────────────────
// RunPod (on-demand GPU/CPU pods)
// ─────────────────────────────────────────────────────────────────────────────

export const runpodPricing: ProviderPricing = {
  name: "runpod",
  computeTiers: [
    {
      id: "1-cpu-2gb",
      label: "1 vCPU / 2 GB",
      vcpu: 1,
      memoryGb: 2,
      pricePerMonth: 8.76,
      pricePerHour: 0.012,
    },
    {
      id: "2-cpu-4gb",
      label: "2 vCPU / 4 GB",
      vcpu: 2,
      memoryGb: 4,
      pricePerMonth: 17.52,
      pricePerHour: 0.024,
    },
    {
      id: "4-cpu-8gb",
      label: "4 vCPU / 8 GB",
      vcpu: 4,
      memoryGb: 8,
      pricePerMonth: 35.04,
      pricePerHour: 0.048,
    },
    {
      id: "8-cpu-16gb",
      label: "8 vCPU / 16 GB",
      vcpu: 8,
      memoryGb: 16,
      pricePerMonth: 70.08,
      pricePerHour: 0.096,
    },
  ],
  storage: { gbPerMonth: 0.1 },
  network: { egressGbFree: 0, egressGbPrice: 0.05 },
};

// ─────────────────────────────────────────────────────────────────────────────
// Northflank
// ─────────────────────────────────────────────────────────────────────────────

export const northflankPricing: ProviderPricing = {
  name: "northflank",
  computeTiers: [
    {
      id: "nf-compute-10",
      label: "nf-compute-10 (0.5 GB)",
      vcpu: 0.1,
      memoryGb: 0.5,
      pricePerMonth: 4.0,
      pricePerHour: 0.0055,
    },
    {
      id: "nf-compute-20",
      label: "nf-compute-20 (1 GB)",
      vcpu: 0.2,
      memoryGb: 1,
      pricePerMonth: 8.0,
      pricePerHour: 0.011,
    },
    {
      id: "nf-compute-50",
      label: "nf-compute-50 (2 GB)",
      vcpu: 0.5,
      memoryGb: 2,
      pricePerMonth: 16.0,
      pricePerHour: 0.022,
    },
    {
      id: "nf-compute-100",
      label: "nf-compute-100 (4 GB)",
      vcpu: 1,
      memoryGb: 4,
      pricePerMonth: 32.0,
      pricePerHour: 0.044,
    },
    {
      id: "nf-compute-200",
      label: "nf-compute-200 (8 GB)",
      vcpu: 2,
      memoryGb: 8,
      pricePerMonth: 64.0,
      pricePerHour: 0.088,
    },
  ],
  storage: { gbPerMonth: 0.25 },
  network: { egressGbFree: 100, egressGbPrice: 0.03 },
};

export const PROVIDER_PRICING: Record<string, ProviderPricing> = {
  fly: flyPricing,
  aws: awsPricing,
  gcp: gcpPricing,
  azure: azurePricing,
  runpod: runpodPricing,
  northflank: northflankPricing,
};

export function getProviderPricing(provider: string): ProviderPricing | null {
  return PROVIDER_PRICING[provider.toLowerCase()] ?? null;
}

/**
 * Estimate monthly cost for an instance given its provider, tier, disk (GB),
 * and estimated monthly egress (GB).
 */
export function estimateMonthlyCost(
  provider: string,
  tierId: string,
  diskGb = 20,
  egressGb = 10,
): { compute: number; storage: number; network: number; total: number } | null {
  const pricing = getProviderPricing(provider);
  if (!pricing) return null;

  const tier = pricing.computeTiers.find((t) => t.id === tierId);
  if (!tier) return null;

  const compute = tier.pricePerMonth;
  const storage = diskGb * pricing.storage.gbPerMonth;
  const billableEgress = Math.max(0, egressGb - pricing.network.egressGbFree);
  const network = billableEgress * pricing.network.egressGbPrice;
  const total = compute + storage + network;

  return {
    compute: Math.round(compute * 100) / 100,
    storage: Math.round(storage * 100) / 100,
    network: Math.round(network * 100) / 100,
    total: Math.round(total * 100) / 100,
  };
}
