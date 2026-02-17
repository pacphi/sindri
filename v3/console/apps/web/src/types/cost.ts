// ─────────────────────────────────────────────────────────────────────────────
// Cost tracking types
// ─────────────────────────────────────────────────────────────────────────────

export type BudgetPeriod = 'DAILY' | 'WEEKLY' | 'MONTHLY'

export type CostGranularity = 'day' | 'week' | 'month'

// ─────────────────────────────────────────────────────────────────────────────
// Cost entries
// ─────────────────────────────────────────────────────────────────────────────

export interface CostEntry {
  id: string
  instanceId: string
  instanceName?: string
  provider: string
  periodStart: string
  periodEnd: string
  computeUsd: number
  storageUsd: number
  networkUsd: number
  totalUsd: number
  currency: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Summary / trends
// ─────────────────────────────────────────────────────────────────────────────

export interface CostTrendPoint {
  date: string      // ISO date string (YYYY-MM-DD)
  totalUsd: number
  computeUsd: number
  storageUsd: number
  networkUsd: number
}

export interface ProviderCostPoint {
  date: string
  provider: string
  totalUsd: number
}

export interface TeamCostShare {
  team: string
  totalUsd: number
  percentage: number
}

export interface CostSummary {
  totalUsd: number
  computeUsd: number
  storageUsd: number
  networkUsd: number
  periodStart: string
  periodEnd: string
  instanceCount: number
  changePercent: number | null  // vs previous period, null when no prior data
}

export interface CostTrendsResponse {
  granularity: CostGranularity
  points: CostTrendPoint[]
  byProvider: ProviderCostPoint[]
  byTeam: TeamCostShare[]
  summary: CostSummary
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-instance breakdown
// ─────────────────────────────────────────────────────────────────────────────

export interface InstanceCostRow {
  instanceId: string
  instanceName: string
  provider: string
  region: string | null
  totalUsd: number
  computeUsd: number
  storageUsd: number
  networkUsd: number
  percentOfTotal: number
}

export interface InstanceCostBreakdownResponse {
  rows: InstanceCostRow[]
  totalUsd: number
  periodStart: string
  periodEnd: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Budgets
// ─────────────────────────────────────────────────────────────────────────────

export interface Budget {
  id: string
  name: string
  amountUsd: number
  period: BudgetPeriod
  instanceId: string | null
  provider: string | null
  alertThreshold: number  // 0..1, e.g. 0.8 = 80%
  alertSent: boolean
  createdBy: string | null
  createdAt: string
  updatedAt: string
  // Computed fields from API
  spentUsd?: number
  spentPercent?: number
}

export interface CreateBudgetInput {
  name: string
  amountUsd: number
  period: BudgetPeriod
  instanceId?: string
  provider?: string
  alertThreshold?: number
}

export interface UpdateBudgetInput {
  name?: string
  amountUsd?: number
  period?: BudgetPeriod
  alertThreshold?: number
}

export interface BudgetListResponse {
  budgets: Budget[]
  total: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Right-sizing recommendations
// ─────────────────────────────────────────────────────────────────────────────

export interface RightSizingRecommendation {
  id: string
  instanceId: string
  instanceName: string
  provider: string
  currentTier: string
  suggestedTier: string
  currentUsdMo: number
  suggestedUsdMo: number
  savingsUsdMo: number
  avgCpuPercent: number
  avgMemPercent: number
  confidence: number   // 0..1
  generatedAt: string
  dismissed: boolean
}

export interface RightSizingResponse {
  recommendations: RightSizingRecommendation[]
  totalSavingsUsdMo: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Idle instances
// ─────────────────────────────────────────────────────────────────────────────

export interface IdleInstance {
  instanceId: string
  instanceName: string
  provider: string
  region: string | null
  status: string
  idleSinceDays: number
  wastedUsdMo: number
  avgCpuPercent: number
  avgMemPercent: number
}

export interface IdleInstancesResponse {
  instances: IdleInstance[]
  totalWastedUsdMo: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Budget alerts
// ─────────────────────────────────────────────────────────────────────────────

export interface CostAlert {
  id: string
  budgetId: string
  budgetName: string
  amountUsd: number
  spentUsd: number
  spentPercent: number
  period: BudgetPeriod
  firedAt: string
}

export interface CostAlertsResponse {
  alerts: CostAlert[]
  total: number
}
