import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { DeploymentConfig, DeploymentSecret, VmSize } from "@/types/deployment";

const MOCK_VM_SIZES: VmSize[] = [
  {
    id: "shared-cpu-1x",
    name: "Shared CPU 1x",
    vcpus: 1,
    memory_gb: 1,
    storage_gb: 10,
    price_per_hour: 0.01,
  },
  {
    id: "shared-cpu-2x",
    name: "Shared CPU 2x",
    vcpus: 2,
    memory_gb: 2,
    storage_gb: 20,
    price_per_hour: 0.02,
  },
  {
    id: "performance-2x",
    name: "Performance 2x",
    vcpus: 2,
    memory_gb: 4,
    storage_gb: 40,
    price_per_hour: 0.05,
  },
  {
    id: "performance-4x",
    name: "Performance 4x",
    vcpus: 4,
    memory_gb: 8,
    storage_gb: 80,
    price_per_hour: 0.1,
  },
  {
    id: "performance-8x",
    name: "Performance 8x",
    vcpus: 8,
    memory_gb: 16,
    storage_gb: 160,
    price_per_hour: 0.2,
  },
  {
    id: "performance-16x",
    name: "Performance 16x",
    vcpus: 16,
    memory_gb: 32,
    storage_gb: 320,
    price_per_hour: 0.4,
  },
];

interface Step3ResourcesProps {
  config: DeploymentConfig;
  onChange: (updates: Partial<DeploymentConfig>) => void;
}

export function Step3Resources({ config, onChange }: Step3ResourcesProps) {
  const [newSecretKey, setNewSecretKey] = useState("");
  const [newSecretValue, setNewSecretValue] = useState("");

  const selectedVmSize = MOCK_VM_SIZES.find((s) => s.id === config.vmSize);

  function handleAddSecret() {
    if (!newSecretKey.trim() || !newSecretValue.trim()) return;
    const updated: DeploymentSecret[] = [
      ...config.secrets,
      { key: newSecretKey.trim(), value: newSecretValue.trim() },
    ];
    onChange({ secrets: updated });
    setNewSecretKey("");
    setNewSecretValue("");
  }

  function handleRemoveSecret(index: number) {
    const updated = config.secrets.filter((_, i) => i !== index);
    onChange({ secrets: updated });
  }

  function handleSecretKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      handleAddSecret();
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-sm font-medium mb-3">VM Size</h3>
        <div className="grid grid-cols-2 gap-3">
          {MOCK_VM_SIZES.map((size) => (
            <Card
              key={size.id}
              className={cn(
                "cursor-pointer transition-colors hover:border-primary",
                config.vmSize === size.id && "border-primary bg-primary/5",
              )}
              onClick={() =>
                onChange({ vmSize: size.id, memoryGb: size.memory_gb, storageGb: size.storage_gb })
              }
            >
              <CardHeader className="p-3 pb-1">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-sm">{size.name}</CardTitle>
                  {config.vmSize === size.id && (
                    <svg
                      className="w-4 h-4 text-primary shrink-0"
                      fill="currentColor"
                      viewBox="0 0 20 20"
                    >
                      <path
                        fillRule="evenodd"
                        d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                        clipRule="evenodd"
                      />
                    </svg>
                  )}
                </div>
              </CardHeader>
              <CardContent className="p-3 pt-0">
                <div className="flex gap-3 text-xs text-muted-foreground mt-1">
                  <span>
                    {size.vcpus} vCPU{size.vcpus !== 1 ? "s" : ""}
                  </span>
                  <span>{size.memory_gb} GB RAM</span>
                  <span>{size.storage_gb} GB</span>
                </div>
                <p className="text-xs font-medium mt-1">${size.price_per_hour.toFixed(3)}/hr</p>
              </CardContent>
            </Card>
          ))}
        </div>

        {selectedVmSize && (
          <div className="mt-3 p-3 bg-muted rounded-md">
            <p className="text-sm font-medium">Selected: {selectedVmSize.name}</p>
            <p className="text-xs text-muted-foreground mt-0.5">
              {selectedVmSize.vcpus} vCPUs · {selectedVmSize.memory_gb} GB RAM ·{" "}
              {selectedVmSize.storage_gb} GB storage · ~$
              {(selectedVmSize.price_per_hour * 730).toFixed(0)}/month
            </p>
          </div>
        )}
      </div>

      <div>
        <h3 className="text-sm font-medium mb-3">Environment Secrets</h3>
        <CardDescription className="text-xs mb-3">
          Add environment variables and secrets. Values are encrypted at rest and never logged.
        </CardDescription>

        {config.secrets.length > 0 && (
          <div className="space-y-2 mb-3">
            {config.secrets.map((secret, index) => (
              <div
                key={index}
                className="flex items-center gap-2 p-2 rounded-md border border-input bg-background"
              >
                <code className="text-xs font-mono flex-1 truncate">{secret.key}</code>
                <code className="text-xs font-mono text-muted-foreground flex-1 truncate">
                  {"*".repeat(Math.min(secret.value.length, 12))}
                </code>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 shrink-0"
                  onClick={() => handleRemoveSecret(index)}
                  aria-label={`Remove ${secret.key}`}
                >
                  <svg
                    className="w-3.5 h-3.5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </Button>
              </div>
            ))}
          </div>
        )}

        <div className="flex gap-2">
          <Input
            placeholder="SECRET_KEY"
            className="font-mono text-sm flex-1"
            value={newSecretKey}
            onChange={(e) => setNewSecretKey(e.target.value.toUpperCase())}
            onKeyDown={handleSecretKeyDown}
          />
          <Input
            placeholder="value"
            className="font-mono text-sm flex-1"
            type="password"
            value={newSecretValue}
            onChange={(e) => setNewSecretValue(e.target.value)}
            onKeyDown={handleSecretKeyDown}
          />
          <Button
            variant="outline"
            onClick={handleAddSecret}
            disabled={!newSecretKey.trim() || !newSecretValue.trim()}
          >
            Add
          </Button>
        </div>
      </div>
    </div>
  );
}
