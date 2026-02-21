import { useState } from "react";
import { Trash2, AlertTriangle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { VolumeBackupSelector } from "./VolumeBackupSelector";
import { instancesApi } from "@/lib/api";
import type { Instance } from "@/types/instance";

interface DestroyDialogProps {
  instance: Instance;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

export function DestroyDialog({ instance, open, onOpenChange, onSuccess }: DestroyDialogProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmName, setConfirmName] = useState("");
  const [backupVolume, setBackupVolume] = useState(false);
  const [backupLabel, setBackupLabel] = useState("");

  const isConfirmed = confirmName === instance.name;

  function handleOpenChange(open: boolean) {
    if (!open) {
      setConfirmName("");
      setBackupVolume(false);
      setBackupLabel("");
      setError(null);
    }
    onOpenChange(open);
  }

  async function handleDestroy() {
    if (!isConfirmed) return;
    setIsLoading(true);
    setError(null);
    try {
      await instancesApi.destroy(instance.id, {
        backupVolume,
        backupLabel: backupLabel || undefined,
      });
      handleOpenChange(false);
      onSuccess?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to destroy instance");
    } finally {
      setIsLoading(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Trash2 className="h-5 w-5 text-destructive" />
            Destroy Instance
          </DialogTitle>
          <DialogDescription>This action cannot be undone.</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="rounded-md bg-destructive/10 border border-destructive/20 px-4 py-3">
            <div className="flex items-start gap-3">
              <AlertTriangle className="h-4 w-4 text-destructive mt-0.5 shrink-0" />
              <div className="text-sm">
                <p className="font-medium text-destructive">Permanent destruction</p>
                <p className="text-destructive/80 mt-0.5">
                  Destroying <span className="font-medium">{instance.name}</span> will permanently
                  remove all associated data. This cannot be reversed.
                </p>
              </div>
            </div>
          </div>

          <VolumeBackupSelector
            enabled={backupVolume}
            onToggle={setBackupVolume}
            label={backupLabel}
            onLabelChange={setBackupLabel}
          />

          <div className="space-y-1.5">
            <label className="text-sm font-medium" htmlFor="confirm-name">
              Type <span className="font-mono text-destructive">{instance.name}</span> to confirm
            </label>
            <Input
              id="confirm-name"
              value={confirmName}
              onChange={(e) => setConfirmName(e.target.value)}
              placeholder={instance.name}
              className="font-mono"
            />
          </div>
        </div>

        {error && (
          <div className="rounded-md bg-destructive/10 border border-destructive/20 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={isLoading}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={() => void handleDestroy()}
            disabled={!isConfirmed || isLoading}
          >
            {isLoading ? "Destroying..." : "Destroy Instance"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
