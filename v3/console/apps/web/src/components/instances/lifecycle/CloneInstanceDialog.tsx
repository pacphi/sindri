import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { lifecycleApi } from "@/api/lifecycle";
import type { Instance } from "@/types/instance";

interface CloneInstanceDialogProps {
  instance: Instance;
  open: boolean;
  onClose: () => void;
  onSuccess?: (clonedId: string) => void;
}

const PROVIDERS = ["fly", "docker", "devpod", "e2b", "kubernetes"] as const;

export function CloneInstanceDialog({
  instance,
  open,
  onClose,
  onSuccess,
}: CloneInstanceDialogProps) {
  const queryClient = useQueryClient();
  const [name, setName] = useState(`${instance.name}-clone`);
  const [provider, setProvider] = useState(instance.provider);
  const [region, setRegion] = useState(instance.region ?? "");
  const [nameError, setNameError] = useState<string | null>(null);

  const { mutate: cloneInstance, isPending } = useMutation({
    mutationFn: () =>
      lifecycleApi.clone(instance.id, {
        name,
        provider: provider as (typeof PROVIDERS)[number],
        region: region || undefined,
      }),
    onSuccess: (data) => {
      void queryClient.invalidateQueries({ queryKey: ["instances"] });
      onSuccess?.(data.id);
      onClose();
    },
  });

  function validateName(value: string): string | null {
    if (!value) return "Name is required";
    if (!/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/.test(value)) {
      return "Name must be lowercase alphanumeric and hyphens, starting and ending with alphanumeric";
    }
    if (value.length > 128) return "Name must be 128 characters or fewer";
    return null;
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const error = validateName(name);
    if (error) {
      setNameError(error);
      return;
    }
    setNameError(null);
    cloneInstance();
  }

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />
      <div className="relative z-10 w-full max-w-md mx-4 rounded-lg border border-border bg-background shadow-lg">
        <div className="px-6 py-4 border-b border-border">
          <h2 className="text-lg font-semibold">Clone Instance</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Create a copy of <span className="font-mono font-medium">{instance.name}</span> with
            optional provider and region overrides.
          </p>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="px-6 py-4 space-y-4">
            <div className="space-y-1.5">
              <label className="text-sm font-medium" htmlFor="clone-name">
                New Instance Name
              </label>
              <Input
                id="clone-name"
                value={name}
                onChange={(e) => {
                  setName(e.target.value);
                  if (nameError) setNameError(validateName(e.target.value));
                }}
                placeholder="my-instance-clone"
                className="font-mono"
                disabled={isPending}
              />
              {nameError && <p className="text-xs text-destructive">{nameError}</p>}
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium" htmlFor="clone-provider">
                Provider
              </label>
              <select
                id="clone-provider"
                value={provider}
                onChange={(e) => setProvider(e.target.value)}
                disabled={isPending}
                className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {PROVIDERS.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </select>
              <p className="text-xs text-muted-foreground">
                Source: <span className="font-mono">{instance.provider}</span>
              </p>
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium" htmlFor="clone-region">
                Region <span className="text-muted-foreground font-normal">(optional)</span>
              </label>
              <Input
                id="clone-region"
                value={region}
                onChange={(e) => setRegion(e.target.value)}
                placeholder={instance.region ?? "e.g. us-east-1"}
                disabled={isPending}
              />
            </div>

            <div className="rounded-md bg-muted/50 px-3 py-2 text-xs text-muted-foreground space-y-1">
              <p className="font-medium text-foreground">Cloning includes:</p>
              <ul className="list-disc list-inside space-y-0.5">
                <li>All {instance.extensions.length} extension(s)</li>
                <li>Configuration hash</li>
                <li>Provider and region (overridable)</li>
              </ul>
              <p className="mt-1">SSH endpoint will be assigned when the new instance registers.</p>
            </div>
          </div>

          <div className="px-6 py-4 border-t border-border flex justify-end gap-3">
            <Button type="button" variant="outline" onClick={onClose} disabled={isPending}>
              Cancel
            </Button>
            <Button type="submit" disabled={isPending}>
              {isPending ? "Cloning..." : "Clone Instance"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
