import { useState } from "react";
import { Card, CardContent, CardFooter, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { WizardStepper, type WizardStep } from "./WizardStepper";
import { Step1Configuration } from "./Step1Configuration";
import { Step2ProviderRegion } from "./Step2ProviderRegion";
import { Step3Resources } from "./Step3Resources";
import { Step4Review } from "./Step4Review";
import { DeploymentProgress } from "./DeploymentProgress";
import { deploymentsApi } from "@/api/deployments";
import type { DeploymentConfig } from "@/types/deployment";

const WIZARD_STEPS: WizardStep[] = [
  { id: 1, label: "Configuration", description: "Template & YAML" },
  { id: 2, label: "Provider & Region", description: "Where to deploy" },
  { id: 3, label: "Resources & Secrets", description: "VM size & secrets" },
  { id: 4, label: "Review & Deploy", description: "Confirm & launch" },
];

const DEFAULT_CONFIG: DeploymentConfig = {
  name: "",
  templateId: null,
  yamlConfig: "",
  provider: "",
  region: "",
  vmSize: "",
  memoryGb: 0,
  storageGb: 0,
  secrets: [],
};

function validateStep(step: number, config: DeploymentConfig): string | null {
  if (step === 1) {
    if (!config.name.trim()) return "Please enter a deployment name";
    if (!/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/.test(config.name)) {
      return "Name must be lowercase alphanumeric and hyphens only";
    }
    return null;
  }
  if (step === 2) {
    if (!config.provider) return "Please select a provider";
    if (!config.region) return "Please select a region";
    return null;
  }
  if (step === 3) {
    if (!config.vmSize) return "Please select a VM size";
    return null;
  }
  return null;
}

interface DeploymentWizardProps {
  onClose?: () => void;
  onDeployed?: (instanceId: string) => void;
}

export function DeploymentWizard({ onClose, onDeployed }: DeploymentWizardProps) {
  const [currentStep, setCurrentStep] = useState(1);
  const [config, setConfig] = useState<DeploymentConfig>(DEFAULT_CONFIG);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [isDeploying, setIsDeploying] = useState(false);
  const [deploymentId, setDeploymentId] = useState<string | null>(null);
  const [deployError, setDeployError] = useState<string | null>(null);

  function handleConfigChange(updates: Partial<DeploymentConfig>) {
    setConfig((prev) => ({ ...prev, ...updates }));
    if (validationError) setValidationError(null);
  }

  function handleNext() {
    const error = validateStep(currentStep, config);
    if (error) {
      setValidationError(error);
      return;
    }
    setValidationError(null);
    setCurrentStep((prev) => Math.min(prev + 1, 4));
  }

  function handleBack() {
    setValidationError(null);
    setCurrentStep((prev) => Math.max(prev - 1, 1));
  }

  async function handleDeploy() {
    const error = validateStep(4, config);
    if (error) {
      setValidationError(error);
      return;
    }

    setIsDeploying(true);
    setDeployError(null);

    try {
      const secretsRecord = config.secrets.reduce<Record<string, string>>((acc, s) => {
        acc[s.key] = s.value;
        return acc;
      }, {});

      const response = await deploymentsApi.create({
        name: config.name,
        provider: config.provider,
        region: config.region,
        vm_size: config.vmSize,
        memory_gb: config.memoryGb,
        storage_gb: config.storageGb,
        yaml_config: config.yamlConfig,
        template_id: config.templateId ?? undefined,
        secrets: Object.keys(secretsRecord).length > 0 ? secretsRecord : undefined,
      });

      setDeploymentId(response.deployment.id);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to initiate deployment";
      setDeployError(message);
      setIsDeploying(false);
    }
  }

  function handleDeployComplete(instanceId: string) {
    setIsDeploying(false);
    onDeployed?.(instanceId);
  }

  function handleDeployError(message: string) {
    setDeployError(message);
    setIsDeploying(false);
  }

  function handleCancelDeployment() {
    setDeploymentId(null);
    setIsDeploying(false);
    setCurrentStep(4);
  }

  if (deploymentId) {
    return (
      <div className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold">Deploying Instance</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Deployment is in progress. Do not close this window.
          </p>
        </div>
        <DeploymentProgress
          deploymentId={deploymentId}
          onComplete={handleDeployComplete}
          onError={handleDeployError}
          onCancel={handleCancelDeployment}
        />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">New Deployment</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Configure and deploy a new Sindri instance
        </p>
      </div>

      <WizardStepper steps={WIZARD_STEPS} currentStep={currentStep} />

      <Card>
        <CardHeader className="pb-4">
          <div>
            <h3 className="font-semibold">
              Step {currentStep}: {WIZARD_STEPS[currentStep - 1].label}
            </h3>
            <p className="text-sm text-muted-foreground mt-0.5">
              {WIZARD_STEPS[currentStep - 1].description}
            </p>
          </div>
        </CardHeader>

        <CardContent>
          {currentStep === 1 && (
            <Step1Configuration config={config} onChange={handleConfigChange} />
          )}
          {currentStep === 2 && (
            <Step2ProviderRegion config={config} onChange={handleConfigChange} />
          )}
          {currentStep === 3 && <Step3Resources config={config} onChange={handleConfigChange} />}
          {currentStep === 4 && (
            <Step4Review
              config={config}
              onDeploy={() => {
                void handleDeploy();
              }}
              isDeploying={isDeploying}
            />
          )}

          {(validationError || deployError) && (
            <div className="mt-4 p-3 bg-destructive/10 border border-destructive/30 rounded-md">
              <p className="text-sm text-destructive">{validationError ?? deployError}</p>
            </div>
          )}
        </CardContent>

        <CardFooter className="flex justify-between border-t pt-4">
          <div className="flex gap-2">
            {onClose && (
              <Button variant="ghost" onClick={onClose}>
                Cancel
              </Button>
            )}
            <Button variant="outline" onClick={handleBack} disabled={currentStep === 1}>
              Back
            </Button>
          </div>

          {currentStep < 4 ? (
            <Button onClick={handleNext}>
              Next
              <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5l7 7-7 7"
                />
              </svg>
            </Button>
          ) : null}
        </CardFooter>
      </Card>
    </div>
  );
}
