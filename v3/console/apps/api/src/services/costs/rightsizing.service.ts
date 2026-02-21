/**
 * Right-sizing analyzer — compares average resource utilisation with tier
 * pricing to suggest cheaper tiers where utilisation is low.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import { getProviderPricing } from "./pricing.js";

export interface RightSizingResult {
  id: string;
  instanceId: string;
  instanceName: string;
  provider: string;
  currentTier: string;
  suggestedTier: string;
  currentUsdMo: number;
  suggestedUsdMo: number;
  savingsUsdMo: number;
  avgCpuPercent: number;
  avgMemPercent: number;
  confidence: number;
  generatedAt: string;
  dismissed: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// List recommendations
// ─────────────────────────────────────────────────────────────────────────────

export async function listRecommendations(showDismissed = false): Promise<RightSizingResult[]> {
  const recs = await db.rightSizingRecommendation.findMany({
    where: showDismissed ? {} : { dismissed: false },
    include: { instance: { select: { name: true, provider: true } } },
    orderBy: { savings_usd_mo: "desc" },
  });

  return recs.map((r) => ({
    id: r.id,
    instanceId: r.instance_id,
    instanceName: r.instance.name,
    provider: r.instance.provider,
    currentTier: r.current_tier,
    suggestedTier: r.suggested_tier,
    currentUsdMo: r.current_usd_mo,
    suggestedUsdMo: r.suggested_usd_mo,
    savingsUsdMo: r.savings_usd_mo,
    avgCpuPercent: r.avg_cpu_percent,
    avgMemPercent: r.avg_mem_percent,
    confidence: r.confidence,
    generatedAt: r.generated_at.toISOString(),
    dismissed: r.dismissed,
  }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Dismiss a recommendation
// ─────────────────────────────────────────────────────────────────────────────

export async function dismissRecommendation(id: string): Promise<RightSizingResult | null> {
  const rec = await db.rightSizingRecommendation.findUnique({
    where: { id },
    include: { instance: { select: { name: true, provider: true } } },
  });
  if (!rec) return null;

  const updated = await db.rightSizingRecommendation.update({
    where: { id },
    data: { dismissed: true },
    include: { instance: { select: { name: true, provider: true } } },
  });

  return {
    id: updated.id,
    instanceId: updated.instance_id,
    instanceName: updated.instance.name,
    provider: updated.instance.provider,
    currentTier: updated.current_tier,
    suggestedTier: updated.suggested_tier,
    currentUsdMo: updated.current_usd_mo,
    suggestedUsdMo: updated.suggested_usd_mo,
    savingsUsdMo: updated.savings_usd_mo,
    avgCpuPercent: updated.avg_cpu_percent,
    avgMemPercent: updated.avg_mem_percent,
    confidence: updated.confidence,
    generatedAt: updated.generated_at.toISOString(),
    dismissed: updated.dismissed,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Analyze and generate/refresh recommendations
// ─────────────────────────────────────────────────────────────────────────────

export async function analyzeAndGenerateRecommendations(): Promise<{
  analyzed: number;
  generated: number;
  skipped: number;
}> {
  const lookbackDays = 30;
  const since = new Date(Date.now() - lookbackDays * 24 * 60 * 60 * 1000);

  const instances = await db.instance.findMany({
    where: { status: "RUNNING" },
    select: {
      id: true,
      name: true,
      provider: true,
      metrics: {
        where: { timestamp: { gte: since } },
        select: {
          cpu_percent: true,
          mem_used: true,
          mem_total: true,
        },
      },
      cost_entries: {
        orderBy: { period_start: "desc" },
        take: 30,
        select: { total_usd: true },
      },
    },
  });

  let analyzed = 0;
  let generated = 0;
  let skipped = 0;

  for (const inst of instances) {
    analyzed++;

    if (inst.metrics.length < 10) {
      skipped++; // not enough data
      continue;
    }

    const pricing = getProviderPricing(inst.provider);
    if (!pricing || pricing.computeTiers.length < 2) {
      skipped++;
      continue;
    }

    // Compute averages
    const avgCpu = inst.metrics.reduce((acc, m) => acc + m.cpu_percent, 0) / inst.metrics.length;
    const avgMem =
      inst.metrics.reduce((acc, m) => {
        const pct = Number(m.mem_total) > 0 ? (Number(m.mem_used) / Number(m.mem_total)) * 100 : 0;
        return acc + pct;
      }, 0) / inst.metrics.length;

    // Consider downsizing if avg utilisation < 40%
    const isUnderutilized = avgCpu < 40 && avgMem < 50;

    if (!isUnderutilized) {
      skipped++;
      continue;
    }

    // Estimate current monthly cost from recent entries
    const recentSpend = inst.cost_entries.reduce((acc, e) => acc + e.total_usd, 0);
    const _currentUsdMo = recentSpend > 0 ? (recentSpend / inst.cost_entries.length) * 30 : 0;

    // Find cheapest tier that exceeds a 30% utilisation headroom
    // For simplicity: suggest the tier one step smaller in the pricing table
    const tiers = pricing.computeTiers;
    const currentTierIdx = Math.floor(tiers.length / 2); // default to middle tier
    const suggestedTierIdx = Math.max(0, currentTierIdx - 1);

    if (currentTierIdx === suggestedTierIdx) {
      skipped++;
      continue;
    }

    const currentTier = tiers[currentTierIdx];
    const suggestedTier = tiers[suggestedTierIdx];
    const suggestedUsdMo = suggestedTier.pricePerMonth;
    const savingsUsdMo = Math.max(0, currentTier.pricePerMonth - suggestedUsdMo);

    if (savingsUsdMo <= 0) {
      skipped++;
      continue;
    }

    // Confidence based on data points
    const confidence = Math.min(0.95, inst.metrics.length / 100);

    try {
      await db.rightSizingRecommendation.upsert({
        where: { instance_id: inst.id },
        create: {
          instance_id: inst.id,
          current_tier: currentTier.id,
          suggested_tier: suggestedTier.id,
          current_usd_mo: currentTier.pricePerMonth,
          suggested_usd_mo: suggestedUsdMo,
          savings_usd_mo: Math.round(savingsUsdMo * 100) / 100,
          avg_cpu_percent: Math.round(avgCpu * 10) / 10,
          avg_mem_percent: Math.round(avgMem * 10) / 10,
          confidence: Math.round(confidence * 100) / 100,
          dismissed: false,
        },
        update: {
          current_tier: currentTier.id,
          suggested_tier: suggestedTier.id,
          current_usd_mo: currentTier.pricePerMonth,
          suggested_usd_mo: suggestedUsdMo,
          savings_usd_mo: Math.round(savingsUsdMo * 100) / 100,
          avg_cpu_percent: Math.round(avgCpu * 10) / 10,
          avg_mem_percent: Math.round(avgMem * 10) / 10,
          confidence: Math.round(confidence * 100) / 100,
          generated_at: new Date(),
          dismissed: false,
        },
      });
      generated++;
    } catch (err) {
      logger.error({ err, instanceId: inst.id }, "Failed to upsert right-sizing recommendation");
      skipped++;
    }
  }

  return { analyzed, generated, skipped };
}
