import { createFileRoute } from '@tanstack/react-router'
import { DeploymentPage } from '@/components/deployment/DeploymentPage'

export const Route = createFileRoute('/deployments')({
  component: DeploymentPage,
})
