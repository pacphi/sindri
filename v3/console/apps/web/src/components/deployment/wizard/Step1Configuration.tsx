import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { SindriYamlEditor } from "@/components/deployment/SindriYamlEditor";
import type { DeploymentConfig } from "@/types/deployment";

interface Template {
  id: string;
  name: string;
  description: string;
  category: string;
  yamlContent: string;
}

const BUILT_IN_TEMPLATES: Template[] = [
  {
    id: "blank",
    name: "Blank",
    description: "Start with an empty configuration",
    category: "General",
    yamlContent: `name: my-instance
provider: fly
region: iad

resources:
  vcpus: 2
  memory_gb: 4
  storage_gb: 20

extensions: []
`,
  },
  {
    id: "node-dev",
    name: "Node.js Dev",
    description: "Node.js development environment with common tools",
    category: "Development",
    yamlContent: `name: node-dev
provider: fly
region: iad

resources:
  vcpus: 2
  memory_gb: 4
  storage_gb: 20

extensions:
  - node@20
  - npm
  - git

env:
  NODE_ENV: development
`,
  },
  {
    id: "python-ml",
    name: "Python ML",
    description: "Python environment with ML/AI libraries",
    category: "Data Science",
    yamlContent: `name: python-ml
provider: runpod
region: us-east-1

resources:
  vcpus: 4
  memory_gb: 16
  storage_gb: 50

extensions:
  - python@3.11
  - pip
  - jupyter
  - pytorch
  - transformers
`,
  },
  {
    id: "go-backend",
    name: "Go Backend",
    description: "Go backend service with database tools",
    category: "Development",
    yamlContent: `name: go-backend
provider: fly
region: iad

resources:
  vcpus: 2
  memory_gb: 2
  storage_gb: 10

extensions:
  - go@1.22
  - git
  - postgresql-client
`,
  },
];

interface Step1ConfigurationProps {
  config: DeploymentConfig;
  onChange: (updates: Partial<DeploymentConfig>) => void;
}

export function Step1Configuration({ config, onChange }: Step1ConfigurationProps) {
  const [selectedTemplateId, setSelectedTemplateId] = useState<string | null>(config.templateId);
  const [activeCategory, setActiveCategory] = useState<string>("All");

  const categories = ["All", ...Array.from(new Set(BUILT_IN_TEMPLATES.map((t) => t.category)))];

  const filteredTemplates =
    activeCategory === "All"
      ? BUILT_IN_TEMPLATES
      : BUILT_IN_TEMPLATES.filter((t) => t.category === activeCategory);

  function handleSelectTemplate(template: Template) {
    setSelectedTemplateId(template.id);
    onChange({
      templateId: template.id,
      yamlConfig: template.yamlContent,
      name: config.name || template.name.toLowerCase().replace(/\s+/g, "-"),
    });
  }

  function handleYamlChange(value: string) {
    onChange({ yamlConfig: value });
  }

  return (
    <div className="space-y-6">
      <div>
        <label className="text-sm font-medium" htmlFor="deployment-name">
          Deployment Name
        </label>
        <Input
          id="deployment-name"
          className="mt-1.5"
          placeholder="my-instance"
          value={config.name}
          onChange={(e) => onChange({ name: e.target.value })}
        />
        <p className="text-xs text-muted-foreground mt-1">
          Lowercase letters, numbers, and hyphens only
        </p>
      </div>

      <div>
        <h3 className="text-sm font-medium mb-3">Select a Template</h3>

        <div className="flex gap-2 mb-3 flex-wrap">
          {categories.map((cat) => (
            <Button
              key={cat}
              variant={activeCategory === cat ? "default" : "outline"}
              size="sm"
              onClick={() => setActiveCategory(cat)}
            >
              {cat}
            </Button>
          ))}
        </div>

        <div className="grid grid-cols-2 gap-3">
          {filteredTemplates.map((template) => (
            <Card
              key={template.id}
              className={cn(
                "cursor-pointer transition-colors hover:border-primary",
                selectedTemplateId === template.id && "border-primary bg-primary/5",
              )}
              onClick={() => handleSelectTemplate(template)}
            >
              <CardHeader className="p-4 pb-2">
                <div className="flex items-start justify-between">
                  <CardTitle className="text-sm">{template.name}</CardTitle>
                  {selectedTemplateId === template.id && (
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
                <CardDescription className="text-xs">{template.description}</CardDescription>
              </CardHeader>
              <CardContent className="p-4 pt-0">
                <span className="inline-block text-xs bg-secondary text-secondary-foreground px-2 py-0.5 rounded">
                  {template.category}
                </span>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>

      <div>
        <label className="text-sm font-medium block mb-2">Configuration YAML</label>
        <SindriYamlEditor
          initialValue={config.yamlConfig}
          onChange={handleYamlChange}
          height={320}
        />
      </div>
    </div>
  );
}
