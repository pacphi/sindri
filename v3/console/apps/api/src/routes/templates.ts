/**
 * Deployment template routes.
 *
 * GET    /api/v1/templates          — list templates (filter by category, isOfficial)
 * POST   /api/v1/templates          — create a template (ADMIN/OPERATOR only)
 * GET    /api/v1/templates/:idOrSlug — get template by ID or slug
 * DELETE /api/v1/templates/:id       — delete template (ADMIN only)
 *
 * Templates are reusable sindri.yaml snapshots shown in the deployment wizard.
 * Official templates are curated by the Sindri team; user-uploaded templates
 * are scoped to the creating user.
 */

import { Hono } from 'hono';
import { z } from 'zod';
import { authMiddleware, requireRole } from '../middleware/auth.js';
import { rateLimitDefault, rateLimitStrict } from '../middleware/rateLimit.js';
import { db } from '../lib/db.js';
import { logger } from '../lib/logger.js';

// ─────────────────────────────────────────────────────────────────────────────
// Zod schemas
// ─────────────────────────────────────────────────────────────────────────────

const CreateTemplateSchema = z.object({
  name: z.string().min(1).max(128),
  slug: z
    .string()
    .min(1)
    .max(128)
    .regex(/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/, 'Slug must be lowercase alphanumeric and hyphens'),
  category: z.string().min(1).max(64),
  description: z.string().min(1).max(1024),
  yamlContent: z.string().min(1).max(65536),
  extensions: z.array(z.string().min(1).max(128)).max(200).default([]),
  providerRecommendations: z
    .array(z.enum(['fly', 'docker', 'devpod', 'e2b', 'kubernetes', 'runpod', 'northflank']))
    .default([]),
  isOfficial: z.boolean().default(false),
});

const ListTemplatesQuerySchema = z.object({
  category: z.string().max(64).optional(),
  isOfficial: z
    .string()
    .transform((v) => v === 'true')
    .pipe(z.boolean())
    .optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

// ─────────────────────────────────────────────────────────────────────────────
// Serializer
// ─────────────────────────────────────────────────────────────────────────────

function serializeTemplate(t: {
  id: string;
  name: string;
  slug: string;
  category: string;
  description: string;
  yaml_content: string;
  extensions: string[];
  provider_recommendations: string[];
  is_official: boolean;
  created_by: string | null;
  created_at: Date;
  updated_at: Date;
}) {
  return {
    id: t.id,
    name: t.name,
    slug: t.slug,
    category: t.category,
    description: t.description,
    yamlContent: t.yaml_content,
    extensions: t.extensions,
    providerRecommendations: t.provider_recommendations,
    isOfficial: t.is_official,
    createdBy: t.created_by,
    createdAt: t.created_at.toISOString(),
    updatedAt: t.updated_at.toISOString(),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const templates = new Hono();

templates.use('*', authMiddleware);

// ─── GET /api/v1/templates ────────────────────────────────────────────────────

templates.get('/', rateLimitDefault, async (c) => {
  const queryResult = ListTemplatesQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!queryResult.success) {
    return c.json(
      { error: 'Validation Error', message: 'Invalid query parameters', details: queryResult.error.flatten() },
      422,
    );
  }

  const { category, isOfficial, page, pageSize } = queryResult.data;
  const skip = (page - 1) * pageSize;

  const where: { category?: string; is_official?: boolean } = {};
  if (category) where.category = category;
  if (isOfficial !== undefined) where.is_official = isOfficial;

  try {
    const [templateList, total] = await Promise.all([
      db.deploymentTemplate.findMany({
        where,
        skip,
        take: pageSize,
        orderBy: [{ is_official: 'desc' }, { created_at: 'desc' }],
      }),
      db.deploymentTemplate.count({ where }),
    ]);

    return c.json({
      templates: templateList.map(serializeTemplate),
      pagination: {
        total,
        page,
        pageSize,
        totalPages: Math.ceil(total / pageSize),
      },
    });
  } catch (err) {
    logger.error({ err }, 'Failed to list templates');
    return c.json({ error: 'Internal Server Error', message: 'Failed to list templates' }, 500);
  }
});

// ─── POST /api/v1/templates ───────────────────────────────────────────────────

templates.post('/', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: 'Bad Request', message: 'Request body must be valid JSON' }, 400);
  }

  const parseResult = CreateTemplateSchema.safeParse(body);
  if (!parseResult.success) {
    return c.json(
      { error: 'Validation Error', message: 'Invalid request body', details: parseResult.error.flatten() },
      422,
    );
  }

  const auth = c.get('auth');
  const { name, slug, category, description, yamlContent, extensions, providerRecommendations, isOfficial } =
    parseResult.data;

  // Only ADMIN can mark a template as official
  const effectiveIsOfficial = auth.role === 'ADMIN' ? isOfficial : false;

  try {
    const template = await db.deploymentTemplate.create({
      data: {
        name,
        slug,
        category,
        description,
        yaml_content: yamlContent,
        extensions,
        provider_recommendations: providerRecommendations,
        is_official: effectiveIsOfficial,
        created_by: auth.userId,
      },
    });

    logger.info({ templateId: template.id, slug, userId: auth.userId }, 'Template created');
    return c.json(serializeTemplate(template), 201);
  } catch (err: unknown) {
    // Unique constraint on slug
    if (
      typeof err === 'object' &&
      err !== null &&
      'code' in err &&
      (err as { code: string }).code === 'P2002'
    ) {
      return c.json({ error: 'Conflict', message: `Template with slug '${slug}' already exists` }, 409);
    }
    logger.error({ err }, 'Failed to create template');
    return c.json({ error: 'Internal Server Error', message: 'Failed to create template' }, 500);
  }
});

// ─── GET /api/v1/templates/:idOrSlug ─────────────────────────────────────────

templates.get('/:idOrSlug', rateLimitDefault, async (c) => {
  const idOrSlug = c.req.param('idOrSlug');
  if (!idOrSlug || idOrSlug.length > 128) {
    return c.json({ error: 'Bad Request', message: 'Invalid template identifier' }, 400);
  }

  try {
    // Try by ID first, then by slug
    const template =
      (await db.deploymentTemplate.findUnique({ where: { id: idOrSlug } })) ??
      (await db.deploymentTemplate.findUnique({ where: { slug: idOrSlug } }));

    if (!template) {
      return c.json({ error: 'Not Found', message: `Template '${idOrSlug}' not found` }, 404);
    }

    return c.json(serializeTemplate(template));
  } catch (err) {
    logger.error({ err, idOrSlug }, 'Failed to fetch template');
    return c.json({ error: 'Internal Server Error', message: 'Failed to fetch template' }, 500);
  }
});

// ─── DELETE /api/v1/templates/:id ────────────────────────────────────────────

templates.delete('/:id', rateLimitStrict, requireRole('ADMIN'), async (c) => {
  const id = c.req.param('id');
  if (!id || id.length > 128) {
    return c.json({ error: 'Bad Request', message: 'Invalid template ID' }, 400);
  }

  try {
    const existing = await db.deploymentTemplate.findUnique({ where: { id } });
    if (!existing) {
      return c.json({ error: 'Not Found', message: `Template '${id}' not found` }, 404);
    }

    await db.deploymentTemplate.delete({ where: { id } });
    logger.info({ templateId: id, slug: existing.slug }, 'Template deleted');
    return c.json({ message: 'Template deleted', id, slug: existing.slug });
  } catch (err) {
    logger.error({ err, templateId: id }, 'Failed to delete template');
    return c.json({ error: 'Internal Server Error', message: 'Failed to delete template' }, 500);
  }
});

export { templates as templatesRouter };
