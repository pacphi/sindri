import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { securityApi } from '@/api/security'
import type { VulnerabilityFilters } from '@/types/security'

// ─────────────────────────────────────────────────────────────────────────────
// Query keys
// ─────────────────────────────────────────────────────────────────────────────

export const securityKeys = {
  all: ['security'] as const,
  summary: (instanceId?: string) => [...securityKeys.all, 'summary', instanceId] as const,
  vulns: ['vulnerabilities'] as const,
  vulnList: (filters: VulnerabilityFilters) => [...securityKeys.vulns, 'list', filters] as const,
  vulnDetail: (id: string) => [...securityKeys.vulns, 'detail', id] as const,
  bom: (instanceId?: string, ecosystem?: string) => [...securityKeys.all, 'bom', instanceId, ecosystem] as const,
  secrets: (instanceId?: string, overdueOnly?: boolean) => [...securityKeys.all, 'secrets', instanceId, overdueOnly] as const,
  sshKeys: (instanceId?: string, status?: string) => [...securityKeys.all, 'ssh-keys', instanceId, status] as const,
  compliance: (instanceId?: string) => [...securityKeys.all, 'compliance', instanceId] as const,
}

// ─────────────────────────────────────────────────────────────────────────────
// Queries
// ─────────────────────────────────────────────────────────────────────────────

export function useSecuritySummary(instanceId?: string) {
  return useQuery({
    queryKey: securityKeys.summary(instanceId),
    queryFn: () => securityApi.getSummary(instanceId).then((r) => r.summary),
    staleTime: 30_000,
    refetchInterval: 60_000,
  })
}

export function useVulnerabilities(filters: VulnerabilityFilters = {}) {
  return useQuery({
    queryKey: securityKeys.vulnList(filters),
    queryFn: () => securityApi.listVulnerabilities(filters),
    staleTime: 30_000,
    refetchInterval: 60_000,
  })
}

export function useVulnerability(id: string) {
  return useQuery({
    queryKey: securityKeys.vulnDetail(id),
    queryFn: () => securityApi.getVulnerability(id),
    enabled: Boolean(id),
  })
}

export function useBom(instanceId?: string, ecosystem?: string) {
  return useQuery({
    queryKey: securityKeys.bom(instanceId, ecosystem),
    queryFn: () => securityApi.getBom(instanceId, ecosystem),
    staleTime: 60_000,
  })
}

export function useSecretRotations(instanceId?: string, overdueOnly = false) {
  return useQuery({
    queryKey: securityKeys.secrets(instanceId, overdueOnly),
    queryFn: () => securityApi.listSecrets(instanceId, overdueOnly).then((r) => r.secrets),
    staleTime: 30_000,
    refetchInterval: 60_000,
  })
}

export function useSshKeys(instanceId?: string, status?: string) {
  return useQuery({
    queryKey: securityKeys.sshKeys(instanceId, status),
    queryFn: () => securityApi.listSshKeys(instanceId, status),
    staleTime: 30_000,
    refetchInterval: 60_000,
  })
}

export function useComplianceReport(instanceId?: string) {
  return useQuery({
    queryKey: securityKeys.compliance(instanceId),
    queryFn: () => securityApi.getComplianceReport(instanceId),
    staleTime: 60_000,
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Mutations
// ─────────────────────────────────────────────────────────────────────────────

export function useTriggerScan() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (instanceId: string) => securityApi.triggerScan(instanceId),
    onSuccess: (_, instanceId) => {
      qc.invalidateQueries({ queryKey: securityKeys.vulnList({}) })
      qc.invalidateQueries({ queryKey: securityKeys.bom(instanceId) })
      qc.invalidateQueries({ queryKey: securityKeys.summary() })
    },
  })
}

export function useAcknowledgeVulnerability() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => securityApi.acknowledgeVulnerability(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: securityKeys.vulns })
      qc.invalidateQueries({ queryKey: securityKeys.summary() })
    },
  })
}

export function useFixVulnerability() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => securityApi.fixVulnerability(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: securityKeys.vulns })
      qc.invalidateQueries({ queryKey: securityKeys.summary() })
    },
  })
}

export function useMarkFalsePositive() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => securityApi.falsePositive(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: securityKeys.vulns })
    },
  })
}

export function useRotateSecret() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => securityApi.rotateSecret(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: securityKeys.secrets() })
      qc.invalidateQueries({ queryKey: securityKeys.summary() })
      qc.invalidateQueries({ queryKey: securityKeys.compliance() })
    },
  })
}

export function useRevokeSshKey() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => securityApi.revokeSshKey(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: securityKeys.sshKeys() })
      qc.invalidateQueries({ queryKey: securityKeys.summary() })
    },
  })
}
