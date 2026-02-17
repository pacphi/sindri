import { createFileRoute } from '@tanstack/react-router'
import { CostsPage } from '@/pages/CostsPage'

export const Route = createFileRoute('/costs')({
  component: CostsPage,
})
