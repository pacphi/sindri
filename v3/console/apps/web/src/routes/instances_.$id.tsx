import { createFileRoute } from '@tanstack/react-router'
import { InstanceDetailPage } from '@/components/instances/InstanceDetailPage'

export const Route = createFileRoute('/instances_/$id')({
  component: InstanceDetailPage,
})
