import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { lifecycleApi } from "@/api/lifecycle";
import { ConfigDiff } from "./ConfigDiff";
import type { Instance } from "@/types/instance";

interface RedeployDialogProps {
  instance: Instance;
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

export function RedeployDialog({ instance, open, onClose, onSuccess }: RedeployDialogProps) {
  const queryClient = useQueryClient();
  const [force, setForce] = useState(false);
  const [editedConfig, setEditedConfig] = useState<string | null>(null);
  const [showDiff, setShowDiff] = useState(false);

  const { data: configData, isLoading: isLoadingConfig } = useQuery({
    queryKey: ["instances", instance.id, "config"],
    queryFn: () => lifecycleApi.getConfig(instance.id),
    enabled: open,
  });

  const {
    mutate: redeploy,
    isPending,
    error,
  } = useMutation({
    mutationFn: () =>
      lifecycleApi.redeploy(instance.id, {
        config: editedConfig ?? undefined,
        force,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["instances", instance.id] });
      void queryClient.invalidateQueries({ queryKey: ["instances"] });
      onSuccess?.();
      onClose();
    },
  });

  const currentConfig = configData?.config ?? "";
  const configToRedeploy = editedConfig ?? currentConfig;
  const hasChanges = editedConfig !== null && editedConfig !== currentConfig;

  const isBlockedStatus = instance.status === "DEPLOYING" || instance.status === "DESTROYING";

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />
      <div className="relative z-10 w-full max-w-lg mx-4 rounded-lg border border-border bg-background shadow-lg max-h-[90vh] flex flex-col">
        <div className="px-6 py-4 border-b border-border shrink-0">
          <h2 className="text-lg font-semibold">Redeploy Instance</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Redeploy <span className="font-mono font-medium">{instance.name}</span> using its
            current configuration.
          </p>
        </div>

        <div className="px-6 py-4 space-y-4 overflow-y-auto flex-1">
          {isBlockedStatus && (
            <div className="rounded-md bg-yellow-500/10 border border-yellow-500/20 px-3 py-2 text-sm text-yellow-700 dark:text-yellow-400">
              Instance is currently in{" "}
              <span className="font-mono font-medium">{instance.status}</span> state. You must
              enable force redeploy to continue.
            </div>
          )}

          {error && (
            <div className="rounded-md bg-destructive/10 border border-destructive/20 px-3 py-2 text-sm text-destructive">
              {error instanceof Error ? error.message : "Failed to trigger redeploy"}
            </div>
          )}

          {isLoadingConfig ? (
            <div className="text-sm text-muted-foreground py-4 text-center">
              Loading configuration...
            </div>
          ) : (
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <p className="text-sm font-medium">Current Configuration</p>
                <button
                  type="button"
                  onClick={() => setShowDiff(!showDiff)}
                  className="text-xs text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
                >
                  {showDiff ? "Hide diff" : "Show diff"}
                </button>
              </div>

              <textarea
                value={editedConfig ?? currentConfig}
                onChange={(e) => setEditedConfig(e.target.value)}
                rows={10}
                spellCheck={false}
                className="w-full rounded-md border border-input bg-muted/30 px-3 py-2 text-xs font-mono resize-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              />

              {hasChanges && (
                <button
                  type="button"
                  onClick={() => setEditedConfig(null)}
                  className="text-xs text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
                >
                  Reset to original
                </button>
              )}

              {showDiff && hasChanges && (
                <ConfigDiff
                  original={currentConfig}
                  modified={configToRedeploy}
                  label="Config diff"
                />
              )}

              {showDiff && !hasChanges && (
                <p className="text-xs text-muted-foreground">No config changes detected.</p>
              )}
            </div>
          )}

          <div className="flex items-start gap-3 rounded-md border border-border px-3 py-2.5">
            <input
              id="force-redeploy"
              type="checkbox"
              checked={force}
              onChange={(e) => setForce(e.target.checked)}
              className="mt-0.5 h-4 w-4 rounded border-input"
            />
            <div>
              <label htmlFor="force-redeploy" className="text-sm font-medium cursor-pointer">
                Force redeploy
              </label>
              <p className="text-xs text-muted-foreground mt-0.5">
                Override deployment lock even if the instance is currently deploying or destroying.
              </p>
            </div>
          </div>
        </div>

        <div className="px-6 py-4 border-t border-border flex justify-end gap-3 shrink-0">
          <Button type="button" variant="outline" onClick={onClose} disabled={isPending}>
            Cancel
          </Button>
          <Button
            type="button"
            onClick={() => redeploy()}
            disabled={isPending || (isBlockedStatus && !force) || isLoadingConfig}
          >
            {isPending ? "Redeploying..." : "Redeploy"}
          </Button>
        </div>
      </div>
    </div>
  );
}
