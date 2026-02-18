import { useState } from "react";
import {
  PauseCircle,
  PlayCircle,
  Trash2,
  X,
  CheckSquare,
  AlertCircle,
  CheckCircle,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { VolumeBackupSelector } from "./VolumeBackupSelector";
import { instancesApi } from "@/lib/api";
import type { Instance } from "@/types/instance";
import { cn } from "@/lib/utils";

interface BulkOperationsProps {
  selectedInstances: Instance[];
  onClearSelection: () => void;
  onSuccess?: () => void;
}

type BulkAction = "suspend" | "resume" | "destroy";

interface BulkResult {
  id: string;
  name: string;
  success: boolean;
  error?: string | null;
  newStatus?: string | null;
}

export function BulkOperations({
  selectedInstances,
  onClearSelection,
  onSuccess,
}: BulkOperationsProps) {
  const [action, setAction] = useState<BulkAction | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [results, setResults] = useState<BulkResult[] | null>(null);
  const [backupVolume, setBackupVolume] = useState(false);
  const [backupLabel, setBackupLabel] = useState("");

  const count = selectedInstances.length;

  if (count === 0) return null;

  function handleClose() {
    setAction(null);
    setResults(null);
    setBackupVolume(false);
    setBackupLabel("");
  }

  async function handleConfirm() {
    if (!action) return;
    setIsLoading(true);
    try {
      const ids = selectedInstances.map((i) => i.id);
      const response = await instancesApi.bulkAction(ids, action, {
        backupVolume: action === "destroy" ? backupVolume : false,
      });
      setResults(response.results);
      onSuccess?.();
    } catch (err) {
      setResults(
        selectedInstances.map((i) => ({
          id: i.id,
          name: i.name,
          success: false,
          error: err instanceof Error ? err.message : "Operation failed",
        })),
      );
    } finally {
      setIsLoading(false);
    }
  }

  function handleDone() {
    handleClose();
    onClearSelection();
  }

  const hasResults = results !== null;
  const succeeded = results?.filter((r) => r.success).length ?? 0;
  const failed = results?.filter((r) => !r.success).length ?? 0;

  return (
    <>
      {/* Selection toolbar */}
      <div className="flex items-center gap-3 rounded-lg border bg-card px-4 py-2.5 shadow-sm">
        <div className="flex items-center gap-2 text-sm font-medium">
          <CheckSquare className="h-4 w-4 text-primary" />
          <span>{count} selected</span>
        </div>

        <div className="h-4 w-px bg-border" />

        <div className="flex items-center gap-1.5">
          <Button
            variant="ghost"
            size="sm"
            className="h-8 text-amber-600 hover:text-amber-700 hover:bg-amber-50 dark:hover:bg-amber-950"
            onClick={() => setAction("suspend")}
          >
            <PauseCircle className="h-3.5 w-3.5" />
            Suspend
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-8 text-green-600 hover:text-green-700 hover:bg-green-50 dark:hover:bg-green-950"
            onClick={() => setAction("resume")}
          >
            <PlayCircle className="h-3.5 w-3.5" />
            Resume
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-8 text-destructive hover:text-destructive hover:bg-destructive/10"
            onClick={() => setAction("destroy")}
          >
            <Trash2 className="h-3.5 w-3.5" />
            Destroy
          </Button>
        </div>

        <div className="flex-1" />

        <Button
          variant="ghost"
          size="sm"
          className="h-7 w-7 p-0"
          onClick={onClearSelection}
          aria-label="Clear selection"
        >
          <X className="h-4 w-4" />
        </Button>
      </div>

      {/* Confirmation dialog */}
      <Dialog open={action !== null} onOpenChange={(open) => !open && handleClose()}>
        <DialogContent>
          {!hasResults ? (
            <>
              <DialogHeader>
                <DialogTitle className="flex items-center gap-2">
                  {action === "suspend" && <PauseCircle className="h-5 w-5 text-amber-500" />}
                  {action === "resume" && <PlayCircle className="h-5 w-5 text-green-500" />}
                  {action === "destroy" && <Trash2 className="h-5 w-5 text-destructive" />}
                  Bulk{" "}
                  {action === "suspend" ? "Suspend" : action === "resume" ? "Resume" : "Destroy"}
                </DialogTitle>
                <DialogDescription>
                  {action === "destroy"
                    ? `Permanently destroy ${count} instance${count > 1 ? "s" : ""}? This cannot be undone.`
                    : `${action === "suspend" ? "Suspend" : "Resume"} ${count} instance${count > 1 ? "s" : ""}?`}
                </DialogDescription>
              </DialogHeader>

              <div className="space-y-3">
                {/* Instance list preview */}
                <div className="max-h-40 overflow-y-auto rounded-md border divide-y">
                  {selectedInstances.map((instance) => (
                    <div key={instance.id} className="flex items-center gap-2 px-3 py-2 text-sm">
                      <span className="font-medium truncate">{instance.name}</span>
                      <span className="ml-auto text-xs text-muted-foreground capitalize">
                        {instance.status.toLowerCase()}
                      </span>
                    </div>
                  ))}
                </div>

                {action === "destroy" && (
                  <VolumeBackupSelector
                    enabled={backupVolume}
                    onToggle={setBackupVolume}
                    label={backupLabel}
                    onLabelChange={setBackupLabel}
                  />
                )}
              </div>

              <DialogFooter>
                <Button variant="outline" onClick={handleClose} disabled={isLoading}>
                  Cancel
                </Button>
                <Button
                  variant={action === "destroy" ? "destructive" : "default"}
                  className={cn(
                    action === "suspend" && "bg-amber-500 hover:bg-amber-600 text-white",
                  )}
                  onClick={() => void handleConfirm()}
                  disabled={isLoading}
                >
                  {isLoading
                    ? "Processing..."
                    : `${action === "suspend" ? "Suspend" : action === "resume" ? "Resume" : "Destroy"} ${count} Instance${count > 1 ? "s" : ""}`}
                </Button>
              </DialogFooter>
            </>
          ) : (
            <>
              <DialogHeader>
                <DialogTitle>Bulk Operation Results</DialogTitle>
                <DialogDescription>
                  {succeeded} succeeded, {failed} failed
                </DialogDescription>
              </DialogHeader>

              <div className="max-h-60 overflow-y-auto rounded-md border divide-y">
                {results?.map((result) => (
                  <div key={result.id} className="flex items-center gap-2 px-3 py-2 text-sm">
                    {result.success ? (
                      <CheckCircle className="h-4 w-4 text-green-500 shrink-0" />
                    ) : (
                      <AlertCircle className="h-4 w-4 text-destructive shrink-0" />
                    )}
                    <span className="font-medium truncate flex-1">{result.name}</span>
                    {result.success ? (
                      <span className="text-xs text-muted-foreground capitalize">
                        {result.newStatus?.toLowerCase() ?? "done"}
                      </span>
                    ) : (
                      <span className="text-xs text-destructive truncate max-w-[160px]">
                        {result.error}
                      </span>
                    )}
                  </div>
                ))}
              </div>

              <DialogFooter>
                <Button onClick={handleDone}>Done</Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
