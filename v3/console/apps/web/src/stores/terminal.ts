import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { TerminalSession } from '@/types/terminal'

interface TerminalStore {
  // Active sessions per instance (instanceId -> sessionId[])
  sessions: Record<string, TerminalSession[]>
  // Last active tab per instance (instanceId -> sessionId)
  lastActiveSession: Record<string, string>

  addSession: (instanceId: string, session: TerminalSession) => void
  removeSession: (instanceId: string, sessionId: string) => void
  updateSessionStatus: (instanceId: string, sessionId: string, status: TerminalSession['status']) => void
  setLastActiveSession: (instanceId: string, sessionId: string) => void
  getInstanceSessions: (instanceId: string) => TerminalSession[]
  clearInstanceSessions: (instanceId: string) => void
}

export const useTerminalStore = create<TerminalStore>()(
  persist(
    (set, get) => ({
      sessions: {},
      lastActiveSession: {},

      addSession: (instanceId, session) => {
        set((state) => ({
          sessions: {
            ...state.sessions,
            [instanceId]: [...(state.sessions[instanceId] ?? []), session],
          },
        }))
      },

      removeSession: (instanceId, sessionId) => {
        set((state) => ({
          sessions: {
            ...state.sessions,
            [instanceId]: (state.sessions[instanceId] ?? []).filter(
              (s) => s.sessionId !== sessionId
            ),
          },
        }))
      },

      updateSessionStatus: (instanceId, sessionId, status) => {
        set((state) => ({
          sessions: {
            ...state.sessions,
            [instanceId]: (state.sessions[instanceId] ?? []).map((s) =>
              s.sessionId === sessionId ? { ...s, status } : s
            ),
          },
        }))
      },

      setLastActiveSession: (instanceId, sessionId) => {
        set((state) => ({
          lastActiveSession: {
            ...state.lastActiveSession,
            [instanceId]: sessionId,
          },
        }))
      },

      getInstanceSessions: (instanceId) => {
        return get().sessions[instanceId] ?? []
      },

      clearInstanceSessions: (instanceId) => {
        set((state) => {
          const { [instanceId]: _removed, ...remainingSessions } = state.sessions
          const { [instanceId]: _removedActive, ...remainingActive } = state.lastActiveSession
          return {
            sessions: remainingSessions,
            lastActiveSession: remainingActive,
          }
        })
      },
    }),
    {
      name: 'sindri-terminal-store',
      // Only persist lastActiveSession, not live session state
      partialize: (state) => ({ lastActiveSession: state.lastActiveSession }),
    }
  )
)
