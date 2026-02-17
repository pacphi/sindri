import { useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { DeploymentWizard } from './wizard'

export function DeploymentPage() {
  const navigate = useNavigate()
  const [showWizard, setShowWizard] = useState(false)

  function handleDeployed(instanceId: string) {
    void navigate({ to: '/instances/$id', params: { id: instanceId } })
  }

  if (showWizard) {
    return (
      <div className="p-6 max-w-3xl mx-auto">
        <DeploymentWizard
          onClose={() => setShowWizard(false)}
          onDeployed={handleDeployed}
        />
      </div>
    )
  }

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Deployments</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Deploy and manage Sindri instances across providers
          </p>
        </div>
        <Button onClick={() => setShowWizard(true)}>
          <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          New Deployment
        </Button>
      </div>

      <div className="rounded-lg border border-dashed border-border p-12 text-center">
        <div className="mx-auto w-12 h-12 bg-muted rounded-full flex items-center justify-center mb-4">
          <svg className="w-6 h-6 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z"
            />
          </svg>
        </div>
        <h3 className="text-sm font-medium">No deployments yet</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Get started by creating your first deployment
        </p>
        <Button className="mt-4" onClick={() => setShowWizard(true)}>
          Create Deployment
        </Button>
      </div>
    </div>
  )
}
