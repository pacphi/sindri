import { useState } from "react";
import { PlayCircle } from "lucide-react";
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

interface ResumeDialogProps {
  instance: Instance;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

export function ResumeDialog({ instance, open, onOpenChange, onSuccess }: ResumeDialogProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleResume() {
    setIsLoading(true);
    setError(null);
    try {
      await instancesApi.resume(instance.id);
      onOpenChange(false);
      onSuccess?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to resume instance");
    } finally {
      setIsLoading(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <PlayCircle className="h-5 w-5 text-green-500" />
            Resume Instance
          </DialogTitle>
          <DialogDescription>
            Resume <span className="font-medium text-foreground">{instance.name}</span>?
          </DialogDescription>
        </DialogHeader>

        <p className="text-sm text-muted-foreground">
          The instance will restart and become available again. All configuration and data from
          before suspension will be restored.
        </p>

        {error && (
          <div className="rounded-md bg-destructive/10 border border-destructive/20 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isLoading}>
            Cancel
          </Button>
          <Button variant="default" onClick={() => void handleResume()} disabled={isLoading}>
            {isLoading ? "Resuming..." : "Resume Instance"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
