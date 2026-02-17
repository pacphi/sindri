/**
 * Extension registry service — CRUD and catalog management.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import type { ListExtensionsFilter, CreateExtensionInput, UpdateExtensionInput } from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Query
// ─────────────────────────────────────────────────────────────────────────────

export async function listExtensions(filter: ListExtensionsFilter = {}) {
  const page = filter.page ?? 1;
  const pageSize = filter.pageSize ?? 50;
  const skip = (page - 1) * pageSize;

  const where = {
    ...(filter.category && { category: filter.category }),
    ...(filter.scope && { scope: filter.scope }),
    ...(filter.isOfficial !== undefined && { is_official: filter.isOfficial }),
    ...(filter.tags?.length && { tags: { hasSome: filter.tags } }),
    ...(filter.search && {
      OR: [
        { name: { contains: filter.search, mode: "insensitive" as const } },
        { display_name: { contains: filter.search, mode: "insensitive" as const } },
        { description: { contains: filter.search, mode: "insensitive" as const } },
        { tags: { hasSome: [filter.search] } },
      ],
    }),
  };

  const [extensions, total] = await Promise.all([
    db.extension.findMany({
      where,
      include: {
        _count: { select: { usages: true, policies: true } },
      },
      orderBy: [{ is_official: "desc" }, { download_count: "desc" }, { name: "asc" }],
      skip,
      take: pageSize,
    }),
    db.extension.count({ where }),
  ]);

  return {
    extensions: extensions.map(formatExtension),
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

export async function getExtensionById(id: string) {
  const ext = await db.extension.findUnique({
    where: { id },
    include: {
      _count: { select: { usages: true, policies: true } },
      usages: {
        where: { removed_at: null },
        select: { instance_id: true, version: true, installed_at: true },
        take: 10,
        orderBy: { installed_at: "desc" },
      },
      policies: {
        select: { instance_id: true, policy: true, pinned_version: true },
      },
    },
  });

  if (!ext) return null;
  return formatExtension(ext);
}

export async function getExtensionByName(name: string) {
  const ext = await db.extension.findUnique({
    where: { name },
    include: {
      _count: { select: { usages: true } },
    },
  });

  if (!ext) return null;
  return formatExtension(ext);
}

export async function createExtension(input: CreateExtensionInput) {
  const ext = await db.extension.create({
    data: {
      name: input.name,
      display_name: input.display_name,
      description: input.description,
      category: input.category,
      version: input.version,
      author: input.author,
      license: input.license,
      homepage_url: input.homepage_url,
      icon_url: input.icon_url,
      tags: input.tags ?? [],
      dependencies: input.dependencies ?? [],
      scope: input.scope ?? "PUBLIC",
      is_official: input.is_official ?? false,
      published_by: input.published_by,
    },
    include: {
      _count: { select: { usages: true } },
    },
  });

  logger.info({ extensionId: ext.id, name: ext.name }, "Extension created");
  return formatExtension(ext);
}

export async function updateExtension(id: string, input: UpdateExtensionInput) {
  const ext = await db.extension.update({
    where: { id },
    data: {
      ...(input.display_name && { display_name: input.display_name }),
      ...(input.description && { description: input.description }),
      ...(input.category && { category: input.category }),
      ...(input.version && { version: input.version }),
      ...(input.author !== undefined && { author: input.author }),
      ...(input.license !== undefined && { license: input.license }),
      ...(input.homepage_url !== undefined && { homepage_url: input.homepage_url }),
      ...(input.icon_url !== undefined && { icon_url: input.icon_url }),
      ...(input.tags && { tags: input.tags }),
      ...(input.dependencies && { dependencies: input.dependencies }),
      ...(input.is_deprecated !== undefined && { is_deprecated: input.is_deprecated }),
    },
    include: {
      _count: { select: { usages: true } },
    },
  });

  logger.info({ extensionId: ext.id }, "Extension updated");
  return formatExtension(ext);
}

export async function deleteExtension(id: string) {
  await db.extension.delete({ where: { id } });
  logger.info({ extensionId: id }, "Extension deleted");
}

export async function listCategories() {
  const result = await db.extension.groupBy({
    by: ["category"],
    _count: { category: true },
    orderBy: { _count: { category: "desc" } },
  });

  return result.map((r) => ({ category: r.category, count: r._count.category }));
}

export async function resolveDependencies(extensionName: string): Promise<string[]> {
  const visited = new Set<string>();
  const resolved: string[] = [];

  async function resolve(name: string) {
    if (visited.has(name)) return;
    visited.add(name);

    const ext = await db.extension.findUnique({
      where: { name },
      select: { dependencies: true },
    });

    if (!ext) return;

    for (const dep of ext.dependencies) {
      await resolve(dep);
    }
    resolved.push(name);
  }

  await resolve(extensionName);
  return resolved.filter((n) => n !== extensionName);
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatters
// ─────────────────────────────────────────────────────────────────────────────

type ExtensionWithCount = {
  _count?: { usages?: number; policies?: number };
  [key: string]: unknown;
};

function formatExtension(ext: ExtensionWithCount) {
  const { _count, ...rest } = ext;
  return {
    ...rest,
    install_count: _count?.usages ?? 0,
  };
}
