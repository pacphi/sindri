/**
 * Integration tests for /api/v1/templates (deployment template CRUD).
 *
 * Database calls are mocked via vitest vi.mock.
 */

import { describe, it, expect, vi } from 'vitest';
import { buildApp, authHeaders, VALID_API_KEY, ADMIN_API_KEY } from './helpers.js';
import { createHash } from 'crypto';

function sha256(v: string) {
  return createHash('sha256').update(v).digest('hex');
}

const mockTemplate = {
  id: 'tmpl_01',
  name: 'Python ML Stack',
  slug: 'python-ml',
  category: 'ai',
  description: 'Python ML environment with PyTorch and Jupyter',
  yaml_content: 'name: python-ml\nextensions:\n  - python3\n  - pytorch\n',
  extensions: ['python3', 'pytorch'],
  provider_recommendations: ['fly', 'e2b'],
  is_official: true,
  created_by: null,
  created_at: new Date('2026-02-17T00:00:00Z'),
  updated_at: new Date('2026-02-17T00:00:00Z'),
};

// Mock db
vi.mock('../src/lib/db.js', () => {
  const db = {
    apiKey: {
      findUnique: vi.fn(({ where }: { where: { key_hash: string } }) => {
        const keys: Record<string, { id: string; user_id: string; key_hash: string; expires_at: Date | null; user: { role: 'ADMIN' | 'OPERATOR' | 'DEVELOPER' | 'VIEWER' } }> = {
          [sha256('sk-test-valid-key-0001')]: {
            id: 'key_dev_01', user_id: 'user_dev_01', key_hash: where.key_hash,
            expires_at: null, user: { role: 'DEVELOPER' },
          },
          [sha256('sk-test-admin-key-0001')]: {
            id: 'key_admin_01', user_id: 'user_admin_01', key_hash: where.key_hash,
            expires_at: null, user: { role: 'ADMIN' },
          },
        };
        return Promise.resolve(keys[where.key_hash] ?? null);
      }),
      update: vi.fn(() => Promise.resolve({})),
    },
    deploymentTemplate: {
      findMany: vi.fn(() => Promise.resolve([mockTemplate])),
      count: vi.fn(() => Promise.resolve(1)),
      findUnique: vi.fn(({ where }: { where: { id?: string; slug?: string } }) => {
        if (where.id === mockTemplate.id || where.slug === mockTemplate.slug) {
          return Promise.resolve(mockTemplate);
        }
        return Promise.resolve(null);
      }),
      create: vi.fn(() => Promise.resolve(mockTemplate)),
      delete: vi.fn(() => Promise.resolve(mockTemplate)),
    },
    $queryRaw: vi.fn(() => Promise.resolve([{ '?column?': 1 }])),
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
    deploymentProgress: (id: string) => `sindri:deployment:${id}:progress`,
  },
  REDIS_KEYS: {
    instanceOnline: (id: string) => `sindri:instance:${id}:online`,
    activeAgents: 'sindri:agents:active',
  },
  connectRedis: vi.fn(() => Promise.resolve()),
  disconnectRedis: vi.fn(() => Promise.resolve()),
}));

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('GET /api/v1/templates', () => {
  const app = buildApp();

  it('returns a list of templates', async () => {
    const res = await app.request('/api/v1/templates', { headers: authHeaders() });
    expect(res.status).toBe(200);
    const body = await res.json() as { templates: unknown[]; pagination: { total: number } };
    expect(Array.isArray(body.templates)).toBe(true);
    expect(body.pagination.total).toBe(1);
  });

  it('returns 401 without auth', async () => {
    const res = await app.request('/api/v1/templates');
    expect(res.status).toBe(401);
  });

  it('serializes template fields correctly', async () => {
    const res = await app.request('/api/v1/templates', { headers: authHeaders() });
    const body = await res.json() as { templates: Array<{ id: string; slug: string; isOfficial: boolean; yamlContent: string; createdAt: string }> };
    const t = body.templates[0];
    expect(t.id).toBe(mockTemplate.id);
    expect(t.slug).toBe(mockTemplate.slug);
    expect(t.isOfficial).toBe(true);
    expect(typeof t.yamlContent).toBe('string');
    expect(t.createdAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);
  });

  it('accepts category filter', async () => {
    const res = await app.request('/api/v1/templates?category=ai', { headers: authHeaders() });
    expect(res.status).toBe(200);
  });

  it('accepts isOfficial filter', async () => {
    const res = await app.request('/api/v1/templates?isOfficial=true', { headers: authHeaders() });
    expect(res.status).toBe(200);
  });
});

describe('GET /api/v1/templates/:idOrSlug', () => {
  const app = buildApp();

  it('returns template by ID', async () => {
    const res = await app.request(`/api/v1/templates/${mockTemplate.id}`, { headers: authHeaders() });
    expect(res.status).toBe(200);
    const body = await res.json() as { id: string };
    expect(body.id).toBe(mockTemplate.id);
  });

  it('returns template by slug', async () => {
    const res = await app.request(`/api/v1/templates/${mockTemplate.slug}`, { headers: authHeaders() });
    expect(res.status).toBe(200);
    const body = await res.json() as { slug: string };
    expect(body.slug).toBe(mockTemplate.slug);
  });

  it('returns 404 for unknown template', async () => {
    const res = await app.request('/api/v1/templates/nonexistent', { headers: authHeaders() });
    expect(res.status).toBe(404);
  });
});

describe('POST /api/v1/templates', () => {
  const app = buildApp();

  const validPayload = {
    name: 'Python ML Stack',
    slug: 'python-ml',
    category: 'ai',
    description: 'Python ML environment with PyTorch and Jupyter',
    yamlContent: 'name: python-ml\nextensions:\n  - python3\n',
    extensions: ['python3'],
    providerRecommendations: ['fly'],
    isOfficial: false,
  };

  it('returns 403 for DEVELOPER role', async () => {
    const res = await app.request('/api/v1/templates', {
      method: 'POST',
      headers: { ...authHeaders(VALID_API_KEY), 'Content-Type': 'application/json' },
      body: JSON.stringify(validPayload),
    });
    expect(res.status).toBe(403);
  });

  it('creates a template as ADMIN', async () => {
    const res = await app.request('/api/v1/templates', {
      method: 'POST',
      headers: { ...authHeaders(ADMIN_API_KEY), 'Content-Type': 'application/json' },
      body: JSON.stringify(validPayload),
    });
    expect(res.status).toBe(201);
    const body = await res.json() as { id: string; slug: string };
    expect(body.slug).toBe(mockTemplate.slug);
  });

  it('returns 422 for invalid slug', async () => {
    const res = await app.request('/api/v1/templates', {
      method: 'POST',
      headers: { ...authHeaders(ADMIN_API_KEY), 'Content-Type': 'application/json' },
      body: JSON.stringify({ ...validPayload, slug: 'Invalid Slug!' }),
    });
    expect(res.status).toBe(422);
  });
});

describe('DELETE /api/v1/templates/:id', () => {
  const app = buildApp();

  it('returns 403 for non-ADMIN', async () => {
    const res = await app.request(`/api/v1/templates/${mockTemplate.id}`, {
      method: 'DELETE',
      headers: authHeaders(VALID_API_KEY),
    });
    expect(res.status).toBe(403);
  });

  it('deletes a template as ADMIN', async () => {
    const res = await app.request(`/api/v1/templates/${mockTemplate.id}`, {
      method: 'DELETE',
      headers: authHeaders(ADMIN_API_KEY),
    });
    expect(res.status).toBe(200);
    const body = await res.json() as { message: string; id: string };
    expect(body.message).toContain('deleted');
    expect(body.id).toBe(mockTemplate.id);
  });

  it('returns 404 for unknown template', async () => {
    const res = await app.request('/api/v1/templates/nonexistent', {
      method: 'DELETE',
      headers: authHeaders(ADMIN_API_KEY),
    });
    expect(res.status).toBe(404);
  });
});
