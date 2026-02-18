/**
 * Shared types for the alerting engine.
 */

// ─────────────────────────────────────────────────────────────────────────────
// Alert Rule Types
// ─────────────────────────────────────────────────────────────────────────────

export type AlertRuleType = "THRESHOLD" | "ANOMALY" | "LIFECYCLE" | "SECURITY" | "COST";
export type AlertSeverity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "INFO";
export type AlertStatus = "ACTIVE" | "ACKNOWLEDGED" | "RESOLVED" | "SILENCED";
export type NotificationChannelType = "WEBHOOK" | "SLACK" | "EMAIL" | "IN_APP";

// ─────────────────────────────────────────────────────────────────────────────
// Condition configs per rule type
// ─────────────────────────────────────────────────────────────────────────────

export interface ThresholdCondition {
  metric: "cpu_percent" | "mem_percent" | "disk_percent" | "load_avg_1" | "load_avg_5";
  operator: "gt" | "gte" | "lt" | "lte";
  threshold: number;
  /** Duration in seconds the condition must be sustained before firing */
  duration_sec?: number;
}

export interface AnomalyCondition {
  metric: "cpu_percent" | "mem_percent" | "net_bytes_recv" | "net_bytes_sent";
  /** Percentage deviation from rolling baseline to consider anomalous */
  deviation_percent: number;
  /** Lookback window in seconds for baseline calculation */
  window_sec: number;
}

export interface LifecycleCondition {
  event: "heartbeat_lost" | "unresponsive" | "deploy_failed" | "status_changed";
  /** For heartbeat_lost: seconds without a heartbeat */
  timeout_sec?: number;
  /** For status_changed: which statuses trigger the alert */
  target_statuses?: string[];
}

export interface SecurityCondition {
  check: "cve_detected" | "secret_expired" | "unauthorized_access";
  severity_threshold?: "CRITICAL" | "HIGH" | "MEDIUM";
}

export interface CostCondition {
  budget_usd: number;
  period: "daily" | "weekly" | "monthly";
  /** Fire at this percentage of the budget */
  threshold_percent: number;
}

export type AlertConditions =
  | ThresholdCondition
  | AnomalyCondition
  | LifecycleCondition
  | SecurityCondition
  | CostCondition;

// ─────────────────────────────────────────────────────────────────────────────
// Notification channel configs
// ─────────────────────────────────────────────────────────────────────────────

export interface WebhookChannelConfig {
  url: string;
  method?: "POST" | "PUT";
  headers?: Record<string, string>;
  secret?: string; // HMAC secret for payload signing
}

export interface SlackChannelConfig {
  webhook_url: string;
  channel?: string;
  username?: string;
  icon_emoji?: string;
}

export interface EmailChannelConfig {
  recipients: string[];
  subject_prefix?: string;
}

export interface InAppChannelConfig {
  user_ids?: string[]; // empty = all users
}

export type ChannelConfig =
  | WebhookChannelConfig
  | SlackChannelConfig
  | EmailChannelConfig
  | InAppChannelConfig;

// ─────────────────────────────────────────────────────────────────────────────
// Service inputs
// ─────────────────────────────────────────────────────────────────────────────

export interface CreateAlertRuleInput {
  name: string;
  description?: string;
  type: AlertRuleType;
  severity: AlertSeverity;
  instanceId?: string;
  conditions: AlertConditions;
  cooldownSec?: number;
  channelIds?: string[];
  createdBy?: string;
}

export interface UpdateAlertRuleInput {
  name?: string;
  description?: string;
  severity?: AlertSeverity;
  enabled?: boolean;
  instanceId?: string;
  conditions?: AlertConditions;
  cooldownSec?: number;
  channelIds?: string[];
}

export interface CreateChannelInput {
  name: string;
  type: NotificationChannelType;
  config: ChannelConfig;
  createdBy?: string;
}

export interface UpdateChannelInput {
  name?: string;
  config?: ChannelConfig;
  enabled?: boolean;
}

export interface ListAlertsFilter {
  ruleId?: string;
  instanceId?: string;
  status?: AlertStatus;
  severity?: AlertSeverity;
  from?: Date;
  to?: Date;
  page?: number;
  pageSize?: number;
}

export interface ListRulesFilter {
  type?: AlertRuleType;
  severity?: AlertSeverity;
  enabled?: boolean;
  instanceId?: string;
  page?: number;
  pageSize?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Evaluation context (passed to evaluators)
// ─────────────────────────────────────────────────────────────────────────────

export interface EvaluationContext {
  instanceId: string;
  instanceName: string;
  instanceStatus: string;
  latestMetrics?: {
    cpuPercent: number;
    memPercent: number;
    diskPercent: number;
    loadAvg1: number | null;
    loadAvg5: number | null;
    netBytesSent: bigint | null;
    netBytesRecv: bigint | null;
    timestamp: Date;
  };
  lastHeartbeatAt?: Date;
}

export interface EvaluationResult {
  fired: boolean;
  title: string;
  message: string;
  metadata?: Record<string, unknown>;
}
