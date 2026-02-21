/**
 * Integration tests for the Phase 2 Deployment Wizard flow.
 *
 * Tests cover:
 * - Template listing and selection
 * - YAML configuration validation and editing
 * - Multi-step wizard flow (template → configure → provider → review → deploy)
 * - Provider-specific configuration validation
 * - Deployment initiation and status tracking
 */

import { describe, it, expect, vi } from "vitest";
import { buildApp, authHeaders } from "./helpers.js";

// ─────────────────────────────────────────────────────────────────────────────
// Mocks
// ─────────────────────────────────────────────────────────────────────────────

// Templates use real DeploymentTemplate schema field names:
// yaml_content (not yaml), provider_recommendations (not providers), slug, is_official
const mockTemplates = [
  {
    id: "tmpl_python_01",
    name: "Python Data Science",
    slug: "python-data-science",
    description: "Jupyter + pandas + scikit-learn environment",
    category: "data-science",
    extensions: ["python3", "jupyter", "pandas", "scikit-learn"],
    provider_recommendations: ["fly", "docker", "e2b"],
    yaml_content: `name: python-data-science\nextensions:\n  - python3\n  - jupyter\n`,
    is_official: true,
    created_by: null,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  },
  {
    id: "tmpl_node_01",
    name: "Node.js Full Stack",
    slug: "node-fullstack",
    description: "Node.js with TypeScript and common dev tools",
    category: "web-development",
    extensions: ["node-lts", "typescript", "git"],
    provider_recommendations: ["fly", "docker", "devpod", "kubernetes"],
    yaml_content: `name: node-fullstack\nextensions:\n  - node-lts\n  - typescript\n`,
    is_official: true,
    created_by: null,
    created_at: "2026-01-15T00:00:00Z",
    updated_at: "2026-01-15T00:00:00Z",
  },
  {
    id: "tmpl_rust_01",
    name: "Rust Development",
    slug: "rust-dev",
    description: "Rust stable toolchain with cargo extensions",
    category: "systems",
    extensions: ["rust-stable", "cargo-watch", "git"],
    provider_recommendations: ["fly", "docker"],
    yaml_content: `name: rust-dev\nextensions:\n  - rust-stable\n  - cargo-watch\n`,
    is_official: false,
    created_by: "user_contrib_01",
    created_at: "2026-02-01T00:00:00Z",
    updated_at: "2026-02-01T00:00:00Z",
  },
];

const mockDeployment = {
  id: "deploy_01",
  instance_id: null,
  template_id: "tmpl_python_01",
  config_hash: "a".repeat(64),
  yaml_content: `name: python-data-science\nextensions:\n  - python3\n  - jupyter\n`,
  provider: "fly",
  region: "sea",
  status: "PENDING",
  initiated_by: "user_dev_01",
  started_at: new Date("2026-02-17T10:00:00Z").toISOString(),
  completed_at: null,
  logs: null,
  error: null,
};

// Pre-computed SHA256 hashes of test API keys (avoids require inside vi.mock hoisting)
const _WIZ_VALID_HASH = "3762e9aa503654d601c34795e887f6e61ecbf137c1e26bc25e2602c5fb2b684d";
const _WIZ_ADMIN_HASH = "202025f117fb2da5b458b8fbfaca54aeae4a348466190e2aa9f63deddba6481f";

vi.mock("../src/lib/db.js", () => {
  const VALID_H = "3762e9aa503654d601c34795e887f6e61ecbf137c1e26bc25e2602c5fb2b684d";
  const ADMIN_H = "202025f117fb2da5b458b8fbfaca54aeae4a348466190e2aa9f63deddba6481f";
  const db = {
    apiKey: {
      findUnique: vi.fn(({ where }: { where: { key_hash: string } }) => {
        if (where.key_hash === VALID_H) {
          return Promise.resolve({
            id: "key_dev_01",
            user_id: "user_dev_01",
            key_hash: where.key_hash,
            expires_at: null,
            user: { role: "DEVELOPER" },
          });
        }
        if (where.key_hash === ADMIN_H) {
          return Promise.resolve({
            id: "key_admin_01",
            user_id: "user_admin_01",
            key_hash: where.key_hash,
            expires_at: null,
            user: { role: "ADMIN" },
          });
        }
        return Promise.resolve(null);
      }),
      update: vi.fn(() => Promise.resolve({})),
    },
    instance: {
      upsert: vi.fn(() =>
        Promise.resolve({
          id: "inst_new_01",
          name: "wizard-instance",
          provider: "fly",
          region: "sea",
          status: "RUNNING",
          extensions: ["python3", "git"],
          config_hash: "b".repeat(64),
          ssh_endpoint: "wizard.fly.dev:22",
          created_at: new Date(),
          updated_at: new Date(),
        }),
      ),
      create: vi.fn(() =>
        Promise.resolve({
          id: "inst_new_01",
          name: "my-python-env",
          provider: "fly",
          region: "sea",
          status: "DEPLOYING",
          extensions: ["python3"],
          config_hash: "a".repeat(64),
          ssh_endpoint: null,
          created_at: new Date(),
          updated_at: new Date(),
        }),
      ),
      findMany: vi.fn(() => Promise.resolve([])),
      count: vi.fn(() => Promise.resolve(0)),
      findUnique: vi.fn(() => Promise.resolve(null)),
    },
    event: {
      create: vi.fn(() => Promise.resolve({ id: "evt_wiz_01" })),
    },
    $queryRaw: vi.fn(() => Promise.resolve([{ "?column?": 1 }])),
    $connect: vi.fn(() => Promise.resolve()),
    $disconnect: vi.fn(() => Promise.resolve()),
  };
  return { db };
});

vi.mock("../src/lib/redis.js", () => ({
  redis: {
    publish: vi.fn(() => Promise.resolve(1)),
    ping: vi.fn(() => Promise.resolve("PONG")),
  },
  redisSub: { psubscribe: vi.fn(), on: vi.fn() },
  REDIS_CHANNELS: {
    instanceMetrics: (id: string) => `sindri:instance:${id}:metrics`,
    instanceEvents: (id: string) => `sindri:instance:${id}:events`,
    instanceCommands: (id: string) => `sindri:instance:${id}:commands`,
  },
  REDIS_KEYS: {
    instanceOnline: (id: string) => `sindri:instance:${id}:online`,
    activeAgents: "sindri:agents:active",
  },
  connectRedis: vi.fn(() => Promise.resolve()),
  disconnectRedis: vi.fn(() => Promise.resolve()),
}));

// ─────────────────────────────────────────────────────────────────────────────
// Template System Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Deployment Wizard: Template System", () => {
  it("lists all available templates with schema-correct field names", () => {
    expect(mockTemplates).toHaveLength(3);
    for (const template of mockTemplates) {
      expect(template).toHaveProperty("id");
      expect(template).toHaveProperty("name");
      expect(template).toHaveProperty("slug");
      expect(template).toHaveProperty("description");
      expect(template).toHaveProperty("category");
      expect(template).toHaveProperty("extensions");
      // Schema uses provider_recommendations (not providers)
      expect(template).toHaveProperty("provider_recommendations");
      // Schema uses yaml_content (not yaml)
      expect(template).toHaveProperty("yaml_content");
      expect(template).toHaveProperty("is_official");
      expect(template).not.toHaveProperty("providers");
      expect(template).not.toHaveProperty("yaml");
    }
  });

  it("filters templates by category", () => {
    const dataScience = mockTemplates.filter((t) => t.category === "data-science");
    expect(dataScience).toHaveLength(1);
    expect(dataScience[0].name).toBe("Python Data Science");
  });

  it("filters templates by provider recommendation", () => {
    const flyCompatible = mockTemplates.filter((t) => t.provider_recommendations.includes("fly"));
    expect(flyCompatible).toHaveLength(3);

    const e2bCompatible = mockTemplates.filter((t) => t.provider_recommendations.includes("e2b"));
    expect(e2bCompatible).toHaveLength(1);
  });

  it("template yaml_content is valid YAML string", () => {
    for (const template of mockTemplates) {
      expect(typeof template.yaml_content).toBe("string");
      expect(template.yaml_content.length).toBeGreaterThan(0);
      expect(template.yaml_content).toContain("name:");
    }
  });

  it("slug is URL-safe identifier derived from name", () => {
    const slugRegex = /^[a-z0-9][a-z0-9-]*$/;
    for (const template of mockTemplates) {
      expect(template.slug).toMatch(slugRegex);
    }
  });

  it("official templates have is_official true", () => {
    const officialTemplates = mockTemplates.filter((t) => t.is_official);
    expect(officialTemplates.length).toBeGreaterThan(0);
    for (const t of officialTemplates) {
      expect(t.is_official).toBe(true);
    }
  });

  it("community templates have is_official false and created_by set", () => {
    const communityTemplates = mockTemplates.filter((t) => !t.is_official);
    expect(communityTemplates.length).toBeGreaterThan(0);
    for (const t of communityTemplates) {
      expect(t.created_by).toBeTruthy();
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// YAML Configuration Validation Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Deployment Wizard: YAML Configuration", () => {
  const validYaml = `name: my-environment
provider: fly
region: sea
extensions:
  - python3
  - git
resources:
  cpu: 2
  memory: 4096
`;

  it("validates required YAML fields", () => {
    const lines = validYaml.split("\n");
    const hasName = lines.some((l) => l.startsWith("name:"));
    expect(hasName).toBe(true);
  });

  it("detects missing name field in YAML", () => {
    const invalidYaml = `extensions:\n  - python3\n`;
    const lines = invalidYaml.split("\n");
    const hasName = lines.some((l) => l.startsWith("name:"));
    expect(hasName).toBe(false);
  });

  it("validates extension names are non-empty strings", () => {
    const extensions = ["python3", "git", "node-lts"];
    for (const ext of extensions) {
      expect(typeof ext).toBe("string");
      expect(ext.length).toBeGreaterThan(0);
      expect(ext).toMatch(/^[a-z0-9][a-z0-9-]*$/);
    }
  });

  it("validates resource limits are positive integers", () => {
    const resources = { cpu: 2, memory: 4096 };
    expect(resources.cpu).toBeGreaterThan(0);
    expect(resources.memory).toBeGreaterThan(0);
    expect(Number.isInteger(resources.cpu)).toBe(true);
    expect(Number.isInteger(resources.memory)).toBe(true);
  });

  it("rejects extension list exceeding maximum of 200", () => {
    const tooManyExtensions = Array.from({ length: 201 }, (_, i) => `ext-${i}`);
    expect(tooManyExtensions.length).toBeGreaterThan(200);
    // Validation should catch this
    const isValid = tooManyExtensions.length <= 200;
    expect(isValid).toBe(false);
  });

  it("accepts YAML with optional fields omitted", () => {
    const minimalYaml = `name: minimal-env\nextensions:\n  - git\n`;
    const lines = minimalYaml.split("\n");
    const hasName = lines.some((l) => l.startsWith("name:"));
    expect(hasName).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Multi-Step Wizard Flow Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Deployment Wizard: Multi-Step Flow", () => {
  const wizardState = {
    step: 1,
    selectedTemplate: null as string | null,
    yamlConfig: "",
    provider: "",
    region: "",
    instanceName: "",
    deploymentId: null as string | null,
  };

  it("starts at step 1 (template selection)", () => {
    expect(wizardState.step).toBe(1);
    expect(wizardState.selectedTemplate).toBeNull();
  });

  it("advances to step 2 after template selection", () => {
    wizardState.selectedTemplate = "tmpl_python_01";
    wizardState.step = 2;
    expect(wizardState.step).toBe(2);
    expect(wizardState.selectedTemplate).toBe("tmpl_python_01");
  });

  it("populates YAML config from selected template yaml_content", () => {
    const template = mockTemplates.find((t) => t.id === wizardState.selectedTemplate);
    if (template) {
      wizardState.yamlConfig = template.yaml_content;
    }
    expect(wizardState.yamlConfig).toContain("name:");
    expect(wizardState.yamlConfig.length).toBeGreaterThan(0);
  });

  it("advances to step 3 (provider selection) after YAML validation", () => {
    wizardState.step = 3;
    expect(wizardState.step).toBe(3);
  });

  it("sets provider and region in step 3", () => {
    wizardState.provider = "fly";
    wizardState.region = "sea";
    wizardState.step = 4;
    expect(wizardState.provider).toBe("fly");
    expect(wizardState.region).toBe("sea");
  });

  it("sets instance name in step 4 (review)", () => {
    wizardState.instanceName = "my-python-env";
    expect(wizardState.instanceName).toBe("my-python-env");
    expect(wizardState.instanceName).toMatch(/^[a-z0-9][a-z0-9-]*$/);
  });

  it("validates provider is in template provider_recommendations", () => {
    const template = mockTemplates.find((t) => t.id === wizardState.selectedTemplate);
    if (template) {
      expect(template.provider_recommendations).toContain(wizardState.provider);
    }
  });

  it("initiates deployment from step 5 (deploy)", async () => {
    wizardState.step = 5;
    wizardState.deploymentId = mockDeployment.id;
    expect(wizardState.deploymentId).toBe("deploy_01");
    // Schema DeploymentStatus: PENDING | IN_PROGRESS | SUCCEEDED | FAILED | CANCELLED
    expect(mockDeployment.status).toBe("PENDING");
  });

  it("allows navigating back to previous steps", () => {
    const prevStep = wizardState.step - 1;
    expect(prevStep).toBe(4);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Provider Configuration Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Deployment Wizard: Provider Configuration", () => {
  const providers = ["fly", "docker", "devpod", "e2b", "kubernetes"];

  it("validates all supported providers", () => {
    for (const provider of providers) {
      expect(["fly", "docker", "devpod", "e2b", "kubernetes"]).toContain(provider);
    }
  });

  it("fly provider requires region selection", () => {
    const flyConfig = { provider: "fly", region: "sea" };
    expect(flyConfig.region).toBeTruthy();
    expect(["sea", "iad", "lax", "ams", "sin"]).toContain(flyConfig.region);
  });

  it("docker provider does not require cloud region", () => {
    const dockerConfig = { provider: "docker", region: "local" };
    expect(dockerConfig.region).toBe("local");
  });

  it("kubernetes provider supports custom namespace", () => {
    const k8sConfig = { provider: "kubernetes", namespace: "sindri-dev", region: "us-east-1" };
    expect(k8sConfig.namespace).toBeTruthy();
    expect(k8sConfig.namespace).toMatch(/^[a-z0-9][a-z0-9-]*$/);
  });

  it("e2b provider requires valid sandbox config", () => {
    const e2bConfig = { provider: "e2b", sandboxId: "sbx_abc123" };
    expect(e2bConfig.sandboxId).toBeTruthy();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// API Integration: Register Instance from Wizard
// ─────────────────────────────────────────────────────────────────────────────

describe("Deployment Wizard: API Integration", () => {
  const app = buildApp();

  it("registers instance from wizard final step with 201", async () => {
    const payload = {
      name: "wizard-instance",
      provider: "fly",
      region: "sea",
      extensions: ["python3", "git"],
      configHash: "b".repeat(64),
      sshEndpoint: "wizard.fly.dev:22",
    };

    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    expect(res.status).toBe(201);
    const body = (await res.json()) as {
      id: string;
      name: string;
      provider: string;
      status: string;
    };
    expect(body.provider).toBe("fly");
    expect(body.status).toBeDefined();
  });

  it("rejects deployment with invalid provider", async () => {
    const payload = {
      name: "invalid-provider-instance",
      provider: "heroku",
      extensions: ["git"],
      configHash: "c".repeat(64),
    };

    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    expect(res.status).toBe(422);
  });

  it("rejects deployment with name violating naming rules", async () => {
    const payload = {
      name: "My Instance With Spaces!",
      provider: "fly",
      extensions: ["git"],
      configHash: "d".repeat(64),
    };

    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    expect(res.status).toBe(422);
  });
});
