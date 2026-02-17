import type { CreateSessionResponse } from '@/types/terminal'

const API_BASE = '/api/v1'

export async function createTerminalSession(instanceId: string): Promise<CreateSessionResponse> {
  const response = await fetch(`${API_BASE}/instances/${instanceId}/terminal`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
  })

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Failed to create terminal session' }))
    throw new Error(error.message ?? 'Failed to create terminal session')
  }

  return response.json()
}

export async function closeTerminalSession(instanceId: string, sessionId: string): Promise<void> {
  const response = await fetch(`${API_BASE}/instances/${instanceId}/terminal/${sessionId}`, {
    method: 'DELETE',
  })

  if (!response.ok && response.status !== 404) {
    throw new Error('Failed to close terminal session')
  }
}

export function getTerminalWebSocketUrl(sessionId: string): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const host = window.location.host
  return `${protocol}//${host}/ws/terminal/${sessionId}`
}
