import { createFileRoute } from '@tanstack/react-router'
import { InstancesPage } from '@/components/instances/InstancesPage'
import { z } from 'zod'

const instancesSearchSchema = z.object({
  provider: z.string().optional(),
  region: z.string().optional(),
  status: z.string().optional(),
  search: z.string().optional(),
  page: z.number().int().positive().default(1),
})

export const Route = createFileRoute('/instances')({
  validateSearch: instancesSearchSchema,
  component: InstancesPage,
})
