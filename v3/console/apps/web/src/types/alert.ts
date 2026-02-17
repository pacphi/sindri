// ─────────────────────────────────────────────────────────────────────────────
// Alert types
// ─────────────────────────────────────────────────────────────────────────────

export type AlertRuleType = 'THRESHOLD' | 'ANOMALY' | 'LIFECYCLE' | 'SECURITY' | 'COST'
export type AlertSeverity = 'CRITICAL' | 'HIGH' | 'MEDIUM' | 'LOW' | 'INFO'
export type AlertStatus = 'ACTIVE' | 'ACKNOWLEDGED' | 'RESOLVED' | 'SILENCED'
export type NotificationChannelType = 'WEBHOOK' | 'SLACK' | 'EMAIL' | 'IN_APP'

// ─────────────────────────────────────────────────────────────────────────────
// Conditions
// ─────────────────────────────────────────────────────────────────────────────

export interface ThresholdCondition {
  metric: 'cpu_percent' | 'mem_percent' | 'disk_percent' | 'load_avg_1' | 'load_avg_5'
  operator: 'gt' | 'gte' | 'lt' | 'lte'
  threshold: number
  duration_sec?: number
}

export interface AnomalyCondition {
  metric: 'cpu_percent' | 'mem_percent' | 'net_bytes_recv' | 'net_bytes_sent'
  deviation_percent: number
  window_sec: number
}

export interface LifecycleCondition {
  event: 'heartbeat_lost' | 'unresponsive' | 'deploy_failed' | 'status_changed'
  timeout_sec?: number
  target_statuses?: string[]
}

export interface SecurityCondition {
  check: 'cve_detected' | 'secret_expired' | 'unauthorized_access'
  severity_threshold?: 'CRITICAL' | 'HIGH' | 'MEDIUM'
}

export interface CostCondition {
  budget_usd: number
  period: 'daily' | 'weekly' | 'monthly'
  threshold_percent: number
}

export type AlertConditions =
  | ThresholdCondition
  | AnomalyCondition
  | LifecycleCondition
  | SecurityCondition
  | CostCondition

// ─────────────────────────────────────────────────────────────────────────────
// Alert Rule
// ─────────────────────────────────────────────────────────────────────────────

export interface AlertRule {
  id: string
  name: string
  description: string | null
  type: AlertRuleType
  severity: AlertSeverity
  enabled: boolean
  instanceId: string | null
  conditions: AlertConditions
  cooldownSec: number
  createdBy: string | null
  createdAt: string
  updatedAt: string
  channels: Array<{ id: string; name: string; type: string }>
  alertCount: number
}

export interface CreateAlertRuleInput {
  name: string
  description?: string
  type: AlertRuleType
  severity: AlertSeverity
  instanceId?: string
  conditions: AlertConditions
  cooldownSec?: number
  channelIds?: string[]
}

export interface UpdateAlertRuleInput extends Partial<CreateAlertRuleInput> {
  enabled?: boolean
}

export interface AlertRuleListResponse {
  rules: AlertRule[]
  total: number
  page: number
  pageSize: number
  totalPages: number
}

export interface AlertRuleFilters {
  type?: AlertRuleType
  severity?: AlertSeverity
  enabled?: boolean
  instanceId?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Alert
// ─────────────────────────────────────────────────────────────────────────────

export interface Alert {
  id: string
  ruleId: string
  instanceId: string | null
  status: AlertStatus
  severity: AlertSeverity
  title: string
  message: string
  metadata: Record<string, unknown> | null
  firedAt: string
  acknowledgedAt: string | null
  acknowledgedBy: string | null
  resolvedAt: string | null
  resolvedBy: string | null
  dedupeKey: string
  rule: { id: string; name: string; type: string } | null
  notifications?: Array<{
    id: string
    sentAt: string
    success: boolean
    error: string | null
    channel: { id: string; name: string; type: string }
  }>
  notificationCount: number | null
}

export interface AlertListResponse {
  alerts: Alert[]
  total: number
  page: number
  pageSize: number
  totalPages: number
}

export interface AlertFilters {
  ruleId?: string
  instanceId?: string
  status?: AlertStatus
  severity?: AlertSeverity
  from?: string
  to?: string
}

export interface AlertSummary {
  bySeverity: Partial<Record<AlertSeverity, number>>
  byStatus: Partial<Record<AlertStatus, number>>
}

// ─────────────────────────────────────────────────────────────────────────────
// Notification channels
// ─────────────────────────────────────────────────────────────────────────────

export interface WebhookChannelConfig {
  url: string
  method?: 'POST' | 'PUT'
  headers?: Record<string, string>
  secret?: string
}

export interface SlackChannelConfig {
  webhook_url: string
  channel?: string
  username?: string
  icon_emoji?: string
}

export interface EmailChannelConfig {
  recipients: string[]
  subject_prefix?: string
}

export interface InAppChannelConfig {
  user_ids?: string[]
}

export type ChannelConfig =
  | WebhookChannelConfig
  | SlackChannelConfig
  | EmailChannelConfig
  | InAppChannelConfig

export interface NotificationChannel {
  id: string
  name: string
  type: NotificationChannelType
  config: ChannelConfig
  enabled: boolean
  createdBy: string | null
  createdAt: string
  updatedAt: string
  ruleCount: number
}

export interface CreateChannelInput {
  name: string
  type: NotificationChannelType
  config: ChannelConfig
}

export interface UpdateChannelInput {
  name?: string
  config?: ChannelConfig
  enabled?: boolean
}
