/**
 * Provider catalog routes.
 *
 * GET /api/v1/providers                    — list all supported providers
 * GET /api/v1/providers/:provider/regions  — list regions for a provider
 * GET /api/v1/providers/:provider/vm-sizes — list VM sizes for a provider
 */

import { Hono } from "hono";
import { authMiddleware } from "../middleware/auth.js";
import { rateLimitDefault } from "../middleware/rateLimit.js";

// ─────────────────────────────────────────────────────────────────────────────
// Static provider catalog (extend with live API calls per provider as needed)
// ─────────────────────────────────────────────────────────────────────────────

const PROVIDERS = [
  {
    id: "fly",
    name: "Fly.io",
    description: "Global app hosting platform with edge deployments",
    regions: [
      { id: "iad", name: "Ashburn, VA", location: "US East" },
      { id: "lax", name: "Los Angeles, CA", location: "US West" },
      { id: "ord", name: "Chicago, IL", location: "US Central" },
      { id: "lhr", name: "London", location: "EU West" },
      { id: "fra", name: "Frankfurt", location: "EU Central" },
      { id: "nrt", name: "Tokyo", location: "Asia Pacific" },
      { id: "syd", name: "Sydney", location: "Oceania" },
    ],
    vm_sizes: [
      {
        id: "shared-cpu-1x",
        name: "Shared CPU 1x",
        vcpus: 1,
        memory_gb: 0.25,
        storage_gb: 1,
        price_per_hour: 0.001,
      },
      {
        id: "shared-cpu-2x",
        name: "Shared CPU 2x",
        vcpus: 2,
        memory_gb: 0.5,
        storage_gb: 1,
        price_per_hour: 0.002,
      },
      {
        id: "performance-2x",
        name: "Performance 2x",
        vcpus: 2,
        memory_gb: 4,
        storage_gb: 40,
        price_per_hour: 0.05,
      },
      {
        id: "performance-4x",
        name: "Performance 4x",
        vcpus: 4,
        memory_gb: 8,
        storage_gb: 80,
        price_per_hour: 0.1,
      },
      {
        id: "performance-8x",
        name: "Performance 8x",
        vcpus: 8,
        memory_gb: 16,
        storage_gb: 160,
        price_per_hour: 0.2,
      },
    ],
  },
  {
    id: "docker",
    name: "Docker",
    description: "Local Docker container deployment",
    regions: [{ id: "local", name: "Local", location: "Local Machine" }],
    vm_sizes: [
      { id: "small", name: "Small", vcpus: 1, memory_gb: 1, storage_gb: 10, price_per_hour: 0 },
      { id: "medium", name: "Medium", vcpus: 2, memory_gb: 4, storage_gb: 20, price_per_hour: 0 },
      { id: "large", name: "Large", vcpus: 4, memory_gb: 8, storage_gb: 40, price_per_hour: 0 },
    ],
  },
  {
    id: "devpod",
    name: "DevPod",
    description: "Remote development environments via DevPod",
    regions: [
      { id: "local", name: "Local", location: "Local Machine" },
      { id: "ssh", name: "SSH Remote", location: "Remote Server" },
    ],
    vm_sizes: [
      { id: "small", name: "Small", vcpus: 1, memory_gb: 2, storage_gb: 20, price_per_hour: 0 },
      { id: "medium", name: "Medium", vcpus: 2, memory_gb: 4, storage_gb: 40, price_per_hour: 0 },
      { id: "large", name: "Large", vcpus: 4, memory_gb: 8, storage_gb: 80, price_per_hour: 0 },
    ],
  },
  {
    id: "e2b",
    name: "E2B",
    description: "Cloud sandboxes for AI agents",
    regions: [
      { id: "us-east-1", name: "US East", location: "AWS us-east-1" },
      { id: "eu-west-1", name: "EU West", location: "AWS eu-west-1" },
    ],
    vm_sizes: [
      { id: "nano", name: "Nano", vcpus: 2, memory_gb: 0.5, storage_gb: 1, price_per_hour: 0.004 },
      { id: "small", name: "Small", vcpus: 2, memory_gb: 1, storage_gb: 5, price_per_hour: 0.008 },
      {
        id: "medium",
        name: "Medium",
        vcpus: 4,
        memory_gb: 2,
        storage_gb: 10,
        price_per_hour: 0.016,
      },
    ],
  },
  {
    id: "kubernetes",
    name: "Kubernetes",
    description: "Deploy to any Kubernetes cluster",
    regions: [
      { id: "default", name: "Default Namespace", location: "Cluster Default" },
      { id: "production", name: "Production", location: "Cluster Production" },
      { id: "staging", name: "Staging", location: "Cluster Staging" },
    ],
    vm_sizes: [
      { id: "small", name: "Small", vcpus: 0.5, memory_gb: 0.5, storage_gb: 5, price_per_hour: 0 },
      { id: "medium", name: "Medium", vcpus: 1, memory_gb: 2, storage_gb: 10, price_per_hour: 0 },
      { id: "large", name: "Large", vcpus: 2, memory_gb: 4, storage_gb: 20, price_per_hour: 0 },
      { id: "xlarge", name: "XLarge", vcpus: 4, memory_gb: 8, storage_gb: 50, price_per_hour: 0 },
    ],
  },
  {
    id: "runpod",
    name: "RunPod",
    description: "GPU cloud for AI/ML workloads",
    regions: [
      { id: "us-east-1", name: "US East", location: "US East Coast" },
      { id: "us-west-2", name: "US West", location: "US West Coast" },
      { id: "eu-central-1", name: "EU Central", location: "Europe" },
    ],
    vm_sizes: [
      {
        id: "cpu-1x",
        name: "CPU 1x",
        vcpus: 2,
        memory_gb: 4,
        storage_gb: 20,
        price_per_hour: 0.025,
      },
      {
        id: "cpu-2x",
        name: "CPU 2x",
        vcpus: 4,
        memory_gb: 8,
        storage_gb: 40,
        price_per_hour: 0.05,
      },
      {
        id: "gpu-3090",
        name: "RTX 3090",
        vcpus: 16,
        memory_gb: 24,
        storage_gb: 200,
        price_per_hour: 0.44,
      },
      {
        id: "gpu-4090",
        name: "RTX 4090",
        vcpus: 16,
        memory_gb: 24,
        storage_gb: 200,
        price_per_hour: 0.69,
      },
      {
        id: "gpu-a100",
        name: "A100 80GB",
        vcpus: 32,
        memory_gb: 80,
        storage_gb: 500,
        price_per_hour: 1.99,
      },
    ],
  },
] as const;

const VALID_PROVIDER_IDS = PROVIDERS.map((p) => p.id);

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const providers = new Hono();

providers.use("*", authMiddleware);

// ─── GET /api/v1/providers ────────────────────────────────────────────────────

providers.get("/", rateLimitDefault, (c) => {
  const list = PROVIDERS.map(({ vm_sizes: _vmSizes, regions: _regions, ...rest }) => rest);
  return c.json({ providers: list });
});

// ─── GET /api/v1/providers/:provider/regions ─────────────────────────────────

providers.get("/:provider/regions", rateLimitDefault, (c) => {
  const providerId = c.req.param("provider");

  if (!VALID_PROVIDER_IDS.includes(providerId as (typeof VALID_PROVIDER_IDS)[number])) {
    return c.json({ error: "Not Found", message: `Provider '${providerId}' not found` }, 404);
  }

  const provider = PROVIDERS.find((p) => p.id === providerId);
  return c.json({ regions: provider?.regions ?? [] });
});

// ─── GET /api/v1/providers/:provider/vm-sizes ────────────────────────────────

providers.get("/:provider/vm-sizes", rateLimitDefault, (c) => {
  const providerId = c.req.param("provider");

  if (!VALID_PROVIDER_IDS.includes(providerId as (typeof VALID_PROVIDER_IDS)[number])) {
    return c.json({ error: "Not Found", message: `Provider '${providerId}' not found` }, 404);
  }

  const provider = PROVIDERS.find((p) => p.id === providerId);
  return c.json({ vm_sizes: provider?.vm_sizes ?? [] });
});

export { providers as providersRouter };
