import { useEffect, useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { DeploymentConfig, Provider, Region } from "@/types/deployment";

const MOCK_PROVIDERS: Provider[] = [
  {
    id: "fly",
    name: "Fly.io",
    description: "Global app hosting platform with edge deployments",
    regions: [
      { id: "iad", name: "Ashburn, VA", location: "US East" },
      { id: "lax", name: "Los Angeles, CA", location: "US West" },
      { id: "ord", name: "Chicago, IL", location: "US Central" },
      { id: "lhr", name: "London", location: "EU West" },
      { id: "fra", name: "Frankfurt", location: "EU Central" },
      { id: "nrt", name: "Tokyo", location: "Asia Pacific" },
    ],
  },
  {
    id: "docker",
    name: "Docker",
    description: "Local Docker container deployment",
    regions: [{ id: "local", name: "Local", location: "Local Machine" }],
  },
  {
    id: "devpod",
    name: "DevPod",
    description: "Remote development environments via DevPod",
    regions: [
      { id: "local", name: "Local", location: "Local Machine" },
      { id: "ssh", name: "SSH Remote", location: "Remote Server" },
    ],
  },
  {
    id: "e2b",
    name: "E2B",
    description: "Cloud sandboxes for AI agents",
    regions: [
      { id: "us-east-1", name: "US East", location: "AWS us-east-1" },
      { id: "eu-west-1", name: "EU West", location: "AWS eu-west-1" },
    ],
  },
  {
    id: "kubernetes",
    name: "Kubernetes",
    description: "Deploy to any Kubernetes cluster",
    regions: [
      { id: "default", name: "Default Namespace", location: "Cluster Default" },
      { id: "production", name: "Production", location: "Cluster Production" },
      { id: "staging", name: "Staging", location: "Cluster Staging" },
    ],
  },
  {
    id: "runpod",
    name: "RunPod",
    description: "GPU cloud for AI/ML workloads",
    regions: [
      { id: "us-east-1", name: "US East", location: "US East Coast" },
      { id: "us-west-2", name: "US West", location: "US West Coast" },
      { id: "eu-central-1", name: "EU Central", location: "Europe" },
    ],
  },
];

const PROVIDER_ICONS: Record<string, string> = {
  fly: "F",
  docker: "D",
  devpod: "P",
  e2b: "E",
  kubernetes: "K",
  runpod: "R",
  northflank: "N",
};

interface Step2ProviderRegionProps {
  config: DeploymentConfig;
  onChange: (updates: Partial<DeploymentConfig>) => void;
}

export function Step2ProviderRegion({ config, onChange }: Step2ProviderRegionProps) {
  const [providers] = useState<Provider[]>(MOCK_PROVIDERS);
  const [availableRegions, setAvailableRegions] = useState<Region[]>([]);

  const selectedProvider = providers.find((p) => p.id === config.provider);

  useEffect(() => {
    if (selectedProvider) {
      setAvailableRegions(selectedProvider.regions);
      if (!selectedProvider.regions.find((r) => r.id === config.region)) {
        onChange({ region: selectedProvider.regions[0]?.id ?? "" });
      }
    } else {
      setAvailableRegions([]);
    }
  }, [config.provider, selectedProvider]);

  function handleSelectProvider(providerId: string) {
    const provider = providers.find((p) => p.id === providerId);
    onChange({
      provider: providerId,
      region: provider?.regions[0]?.id ?? "",
    });
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-sm font-medium mb-3">Select Provider</h3>
        <div className="grid grid-cols-2 gap-3">
          {providers.map((provider) => (
            <Card
              key={provider.id}
              className={cn(
                "cursor-pointer transition-colors hover:border-primary",
                config.provider === provider.id && "border-primary bg-primary/5",
              )}
              onClick={() => handleSelectProvider(provider.id)}
            >
              <CardHeader className="p-4 pb-2">
                <div className="flex items-center gap-3">
                  <div
                    className={cn(
                      "w-8 h-8 rounded-md flex items-center justify-center text-sm font-bold",
                      config.provider === provider.id
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted text-muted-foreground",
                    )}
                  >
                    {PROVIDER_ICONS[provider.id] ?? provider.name[0]}
                  </div>
                  <div className="flex-1 min-w-0">
                    <CardTitle className="text-sm">{provider.name}</CardTitle>
                  </div>
                  {config.provider === provider.id && (
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
              <CardContent className="p-4 pt-0">
                <CardDescription className="text-xs">{provider.description}</CardDescription>
                <p className="text-xs text-muted-foreground mt-1">
                  {provider.regions.length} region{provider.regions.length !== 1 ? "s" : ""}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>

      {selectedProvider && availableRegions.length > 0 && (
        <div>
          <h3 className="text-sm font-medium mb-3">Select Region</h3>
          <div className="grid grid-cols-3 gap-2">
            {availableRegions.map((region) => (
              <button
                key={region.id}
                type="button"
                className={cn(
                  "rounded-md border p-3 text-left transition-colors hover:border-primary focus:outline-none focus:ring-2 focus:ring-ring",
                  config.region === region.id
                    ? "border-primary bg-primary/5"
                    : "border-input bg-background",
                )}
                onClick={() => onChange({ region: region.id })}
              >
                <p className="text-sm font-medium">{region.name}</p>
                <p className="text-xs text-muted-foreground mt-0.5">{region.location}</p>
                <p className="text-xs text-muted-foreground font-mono mt-0.5">{region.id}</p>
              </button>
            ))}
          </div>
        </div>
      )}

      {!config.provider && (
        <p className="text-sm text-muted-foreground text-center py-4">
          Select a provider to see available regions
        </p>
      )}
    </div>
  );
}
