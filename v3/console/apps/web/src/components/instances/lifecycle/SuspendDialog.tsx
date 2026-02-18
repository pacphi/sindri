import { useState } from "react";
import { PauseCircle, DollarSign } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { instancesApi } from "@/lib/api";
import type { Instance } from "@/types/instance";

interface SuspendDialogProps {
  instance: Instance;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

export function SuspendDialog({ instance, open, onOpenChange, onSuccess }: SuspendDialogProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSuspend() {
    setIsLoading(true);
    setError(null);
    try {
      await instancesApi.suspend(instance.id);
      onOpenChange(false);
      onSuccess?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to suspend instance");
    } finally {
      setIsLoading(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <PauseCircle className="h-5 w-5 text-amber-500" />
            Suspend Instance
          </DialogTitle>
          <DialogDescription>
            Suspend <span className="font-medium text-foreground">{instance.name}</span>?
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <div className="rounded-md bg-amber-500/10 border border-amber-500/20 px-4 py-3">
            <div className="flex items-start gap-3">
              <DollarSign className="h-4 w-4 text-amber-600 mt-0.5 shrink-0" />
              <div className="text-sm">
                <p className="font-medium text-amber-700 dark:text-amber-400">Cost savings</p>
                <p className="text-amber-600 dark:text-amber-500 mt-0.5">
                  Suspending this instance will pause compute billing while preserving your data and
                  configuration. You can resume it at any time.
                </p>
              </div>
            </div>
          </div>

          <div className="text-sm text-muted-foreground space-y-1">
            <p>While suspended, the instance will:</p>
            <ul className="list-disc list-inside space-y-0.5 ml-1">
              <li>Stop all running processes</li>
              <li>Retain volume data and configuration</li>
              <li>Not accept new connections</li>
            </ul>
          </div>
        </div>

        {error && (
          <div className="rounded-md bg-destructive/10 border border-destructive/20 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isLoading}>
            Cancel
          </Button>
          <Button
            variant="default"
            className="bg-amber-500 hover:bg-amber-600 text-white"
            onClick={() => void handleSuspend()}
            disabled={isLoading}
          >
            {isLoading ? "Suspending..." : "Suspend Instance"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
