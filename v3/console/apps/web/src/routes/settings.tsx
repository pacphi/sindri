import { createFileRoute } from '@tanstack/react-router'
import { SettingsPage } from '@/components/admin/SettingsPage'

export const Route = createFileRoute('/settings')({
  component: SettingsPage,
})
