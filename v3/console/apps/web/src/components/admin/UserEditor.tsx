import { useEffect, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { usersApi } from '@/api/rbac'
import type { User, UserRole } from '@/types/rbac'
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'

interface UserEditorProps {
  open: boolean
  user: User | null
  onClose: () => void
  onSave: () => void
}

export function UserEditor({ open, user, onClose, onSave }: UserEditorProps) {
  const isEditing = user !== null

  const [email, setEmail] = useState('')
  const [name, setName] = useState('')
  const [password, setPassword] = useState('')
  const [role, setRole] = useState<UserRole>('DEVELOPER')
  const [isActive, setIsActive] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (user) {
      setEmail(user.email)
      setName(user.name ?? '')
      setPassword('')
      setRole(user.role)
      setIsActive(user.isActive)
    } else {
      setEmail('')
      setName('')
      setPassword('')
      setRole('DEVELOPER')
      setIsActive(true)
    }
    setError(null)
  }, [user, open])

  const createMutation = useMutation({
    mutationFn: () =>
      usersApi.createUser({ email, name: name || undefined, password, role }),
    onSuccess: onSave,
    onError: (err: Error) => setError(err.message),
  })

  const updateMutation = useMutation({
    mutationFn: () =>
      usersApi.updateUser(user!.id, {
        email,
        name: name || undefined,
        role,
        is_active: isActive,
        ...(password ? { password } : {}),
      }),
    onSuccess: onSave,
    onError: (err: Error) => setError(err.message),
  })

  const handleSave = () => {
    setError(null)
    if (!email.trim()) {
      setError('Email is required')
      return
    }
    if (!isEditing && !password) {
      setError('Password is required for new users')
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
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{isEditing ? 'Edit User' : 'Create User'}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {error && (
            <div className="rounded-md bg-destructive/10 border border-destructive/20 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          <div className="space-y-1.5">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="user@example.com"
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Full name (optional)"
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="password">
              {isEditing ? 'New Password (leave blank to keep current)' : 'Password'}
            </Label>
            <Input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={isEditing ? 'Leave blank to keep current' : 'Min 8 characters'}
            />
          </div>

          <div className="space-y-1.5">
            <Label>Role</Label>
            <Select value={role} onValueChange={(v) => setRole(v as UserRole)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="ADMIN">Admin — Full access</SelectItem>
                <SelectItem value="OPERATOR">Operator — Deploy, clone, destroy</SelectItem>
                <SelectItem value="DEVELOPER">Developer — Connect, execute commands</SelectItem>
                <SelectItem value="VIEWER">Viewer — Read-only</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {isEditing && (
            <div className="flex items-center gap-3">
              <Switch
                id="active"
                checked={isActive}
                onCheckedChange={setIsActive}
              />
              <Label htmlFor="active">Active account</Label>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={isPending}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={isPending}>
            {isPending ? 'Saving...' : isEditing ? 'Save Changes' : 'Create User'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
