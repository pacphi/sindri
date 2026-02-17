import { TerminalManager } from '@/components/terminal'

interface TerminalPageProps {
  instanceId: string
  instanceName: string
}

/**
 * Full-page terminal view for an instance.
 * Renders the TerminalManager with multi-session tab support.
 */
export function TerminalPage({ instanceId, instanceName }: TerminalPageProps) {
  return (
    <div className="flex h-full flex-col bg-gray-950">
      <div className="flex items-center justify-between border-b border-gray-800 px-4 py-2">
        <div className="flex items-center gap-2 text-sm text-gray-400">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-green-500"
          >
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          <span className="font-medium text-white">{instanceName}</span>
          <span>/</span>
          <span>Terminal</span>
        </div>
      </div>

      <TerminalManager
        instanceId={instanceId}
        instanceName={instanceName}
        theme="dark"
        className="flex-1"
      />
    </div>
  )
}
