import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { DeploymentConfig } from "@/types/deployment";

interface Step4ReviewProps {
  config: DeploymentConfig;
  onDeploy: () => void;
  isDeploying: boolean;
}

interface ReviewRowProps {
  label: string;
  value: string;
}

function ReviewRow({ label, value }: ReviewRowProps) {
  return (
    <div className="flex items-start gap-4 py-2 border-b border-border last:border-0">
      <span className="text-sm text-muted-foreground w-32 shrink-0">{label}</span>
      <span className="text-sm font-medium break-all">{value}</span>
    </div>
  );
}

export function Step4Review({ config, onDeploy, isDeploying }: Step4ReviewProps) {
  const secretCount = config.secrets.length;
  const yamlLines = config.yamlConfig.split("\n").length;

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Configuration</CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          <ReviewRow label="Name" value={config.name || "(not set)"} />
          <ReviewRow label="Template" value={config.templateId ?? "Custom"} />
          <ReviewRow label="YAML Config" value={`${yamlLines} lines`} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Infrastructure</CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          <ReviewRow label="Provider" value={config.provider || "(not set)"} />
          <ReviewRow label="Region" value={config.region || "(not set)"} />
          <ReviewRow label="VM Size" value={config.vmSize || "(not set)"} />
          <ReviewRow
            label="Memory"
            value={config.memoryGb ? `${config.memoryGb} GB` : "(not set)"}
          />
          <ReviewRow
            label="Storage"
            value={config.storageGb ? `${config.storageGb} GB` : "(not set)"}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Secrets</CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          {secretCount === 0 ? (
            <p className="text-sm text-muted-foreground py-1">No secrets configured</p>
          ) : (
            <div>
              {config.secrets.map((secret, index) => (
                <div
                  key={index}
                  className="flex items-center gap-4 py-2 border-b border-border last:border-0"
                >
                  <code className="text-sm font-mono text-muted-foreground w-32 shrink-0 truncate">
                    {secret.key}
                  </code>
                  <code className="text-sm font-mono">
                    {"*".repeat(Math.min(secret.value.length, 16))}
                  </code>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <div className="p-4 bg-muted rounded-md">
        <h4 className="text-sm font-medium mb-1">YAML Configuration Preview</h4>
        <pre className="text-xs font-mono text-muted-foreground overflow-auto max-h-48 whitespace-pre-wrap">
          {config.yamlConfig || "(empty)"}
        </pre>
      </div>

      <div className="flex items-center justify-between pt-2">
        <div>
          <p className="text-sm font-medium">Ready to deploy?</p>
          <p className="text-xs text-muted-foreground mt-0.5">
            This will provision a new instance on {config.provider || "the selected provider"}
          </p>
        </div>
        <Button
          size="lg"
          onClick={onDeploy}
          disabled={isDeploying || !config.name || !config.provider || !config.region}
          className="min-w-[120px]"
        >
          {isDeploying ? (
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                />
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                />
              </svg>
              Deploying...
            </span>
          ) : (
            "Deploy"
          )}
        </Button>
      </div>
    </div>
  );
}
