import { useEffect, useState } from 'react'
import { useMutation, useQuery } from '@tanstack/react-query'
import { teamsApi, usersApi } from '@/api/rbac'
import type { Team, TeamMemberRole } from '@/types/rbac'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

interface TeamEditorProps {
  open: boolean
  team: Team | null
  onClose: () => void
  onSave: () => void
}

export function TeamEditor({ open, team, onClose, onSave }: TeamEditorProps) {
  const isEditing = team !== null

  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [addUserId, setAddUserId] = useState('')
  const [addUserRole, setAddUserRole] = useState<TeamMemberRole>('DEVELOPER')
  const [error, setError] = useState<string | null>(null)

  // Fetch users for the "add member" dropdown
  const { data: usersData } = useQuery({
    queryKey: ['admin-users-for-team'],
    queryFn: () => usersApi.listUsers({}, 1, 100),
    enabled: open && isEditing,
  })

  useEffect(() => {
    if (team) {
      setName(team.name)
      setDescription(team.description ?? '')
    } else {
      setName('')
      setDescription('')
    }
    setAddUserId('')
    setAddUserRole('DEVELOPER')
    setError(null)
  }, [team, open])

  const createMutation = useMutation({
    mutationFn: () => teamsApi.createTeam({ name, description: description || undefined }),
    onSuccess: onSave,
    onError: (err: Error) => setError(err.message),
  })

  const updateMutation = useMutation({
    mutationFn: () => teamsApi.updateTeam(team!.id, { name, description: description || undefined }),
    onSuccess: onSave,
    onError: (err: Error) => setError(err.message),
  })

  const addMemberMutation = useMutation({
    mutationFn: () => teamsApi.addMember(team!.id, addUserId, addUserRole),
    onSuccess: () => {
      setAddUserId('')
      setAddUserRole('DEVELOPER')
    },
    onError: (err: Error) => setError(err.message),
  })

  const handleSave = () => {
    setError(null)
    if (!name.trim()) {
      setError('Team name is required')
      return
    }
    if (isEditing) {
      updateMutation.mutate()
    } else {
      createMutation.mutate()
    }
  }

  const isPending = createMutation.isPending || updateMutation.isPending

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{isEditing ? 'Edit Team' : 'Create Team'}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {error && (
            <div className="rounded-md bg-destructive/10 border border-destructive/20 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          <div className="space-y-1.5">
            <Label htmlFor="team-name">Team Name</Label>
            <Input
              id="team-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Platform Team"
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="team-desc">Description</Label>
            <Textarea
              id="team-desc"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Optional description..."
              rows={2}
            />
          </div>

          {/* Add member section (only when editing) */}
          {isEditing && usersData && (
            <div className="space-y-2 pt-2 border-t border-border">
              <Label>Add Member</Label>
              <div className="flex gap-2">
                <Select value={addUserId} onValueChange={setAddUserId}>
                  <SelectTrigger className="flex-1">
                    <SelectValue placeholder="Select user..." />
                  </SelectTrigger>
                  <SelectContent>
                    {usersData.users.map((u) => (
                      <SelectItem key={u.id} value={u.id}>
                        {u.name ?? u.email}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Select value={addUserRole} onValueChange={(v) => setAddUserRole(v as TeamMemberRole)}>
                  <SelectTrigger className="w-32">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="ADMIN">Admin</SelectItem>
                    <SelectItem value="OPERATOR">Operator</SelectItem>
                    <SelectItem value="DEVELOPER">Developer</SelectItem>
                    <SelectItem value="VIEWER">Viewer</SelectItem>
                  </SelectContent>
                </Select>
                <Button
                  variant="outline"
                  onClick={() => addUserId && addMemberMutation.mutate()}
                  disabled={!addUserId || addMemberMutation.isPending}
                >
                  Add
                </Button>
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={isPending}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={isPending}>
            {isPending ? 'Saving...' : isEditing ? 'Save Changes' : 'Create Team'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
