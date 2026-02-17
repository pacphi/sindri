import { type ReactNode } from 'react'
import { Sidebar } from './Sidebar'
import { useUIStore } from '@/stores/uiStore'
import { cn } from '@/lib/utils'
import { useInstanceWebSocket } from '@/hooks/useInstanceWebSocket'

interface AppLayoutProps {
  children: ReactNode
}

export function AppLayout({ children }: AppLayoutProps) {
  // Establish the WebSocket connection at the app level so it persists
  useInstanceWebSocket()

  const sidebarCollapsed = useUIStore((state) => state.sidebarCollapsed)

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <Sidebar />
      <main
        className={cn(
          'flex-1 flex flex-col overflow-hidden transition-all duration-200',
          sidebarCollapsed ? 'ml-0' : 'ml-0',
        )}
      >
        <div className="flex-1 overflow-auto">
          {children}
        </div>
      </main>
    </div>
  )
}
