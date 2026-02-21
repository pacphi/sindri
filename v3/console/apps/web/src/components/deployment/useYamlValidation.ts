import { useState, useEffect, useCallback } from "react";
import type { ValidationResult, ValidationError } from "./YamlValidator";

// Simple YAML parser for validation purposes.
// We parse the YAML manually to check required fields and basic structure
// without pulling in a heavy YAML library dependency.

interface ParsedSindriConfig {
  version?: unknown;
  name?: unknown;
  deployment?: {
    provider?: unknown;
    image?: unknown;
    resources?: unknown;
    volumes?: unknown;
  };
  extensions?: {
    profile?: unknown;
    active?: unknown[];
    additional?: unknown[];
    auto_install?: unknown;
  };
  secrets?: unknown[];
  providers?: unknown;
}

function parseSimpleYaml(yaml: string): {
  value: ParsedSindriConfig | null;
  parseError: string | null;
} {
  try {
    // Use a simple line-by-line parser for basic structure validation
    const lines = yaml.split("\n");
    const result: Record<string, unknown> = {};
    const stack: Array<{ obj: Record<string, unknown>; indent: number; key: string | null }> = [
      { obj: result, indent: -1, key: null },
    ];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      // Skip comments and empty lines
      if (!line.trim() || line.trim().startsWith("#")) continue;

      const indent = line.match(/^(\s*)/)?.[1]?.length ?? 0;
      const trimmed = line.trim();

      // Key-value pair
      const kvMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*):\s*(.*)$/);
      if (kvMatch) {
        const [, key, rawVal] = kvMatch;
        const val = rawVal.trim();

        // Pop stack until we find the right parent
        while (stack.length > 1 && stack[stack.length - 1].indent >= indent) {
          stack.pop();
        }
        const parent = stack[stack.length - 1].obj;

        if (val === "" || val === null) {
          // Nested object will follow
          const nested: Record<string, unknown> = {};
          parent[key] = nested;
          stack.push({ obj: nested, indent, key });
        } else if (val === "true" || val === "false") {
          parent[key] = val === "true";
        } else if (!isNaN(Number(val)) && val !== "") {
          parent[key] = Number(val);
        } else {
          // String value (strip quotes)
          parent[key] = val.replace(/^["']|["']$/g, "");
        }
        continue;
      }

      // Array item
      const arrayMatch = trimmed.match(/^-\s*(.*)$/);
      if (arrayMatch) {
        while (stack.length > 1 && stack[stack.length - 1].indent >= indent) {
          stack.pop();
        }
        const parent = stack[stack.length - 1].obj;
        const parentKey = stack[stack.length - 1].key;
        // Find the key for this array
        if (parentKey) {
          const arr = (parent[parentKey] as unknown[]) ?? [];
          parent[parentKey] = arr;
          const itemVal = arrayMatch[1].trim().replace(/^["']|["']$/g, "");
          if (itemVal) arr.push(itemVal);
        }
      }
    }

    return { value: result as ParsedSindriConfig, parseError: null };
  } catch {
    return { value: null, parseError: "Failed to parse YAML" };
  }
}

function validateSindriConfig(config: ParsedSindriConfig, yamlLines: string[]): ValidationError[] {
  const errors: ValidationError[] = [];

  const findLine = (key: string): number | undefined => {
    const idx = yamlLines.findIndex((l) => l.trim().startsWith(`${key}:`));
    return idx >= 0 ? idx + 1 : undefined;
  };

  // Required top-level fields
  if (!config.version) {
    errors.push({
      severity: "error",
      message: 'Missing required field "version"',
      path: "version",
      line: findLine("version"),
    });
  } else if (typeof config.version !== "string" || !/^\d+\.\d+$/.test(String(config.version))) {
    errors.push({
      severity: "error",
      message: 'Field "version" must match pattern "\\d+.\\d+" (e.g. "1.0")',
      path: "version",
      line: findLine("version"),
    });
  }

  if (!config.name) {
    errors.push({
      severity: "error",
      message: 'Missing required field "name"',
      path: "name",
      line: findLine("name"),
    });
  } else if (!/^[a-z][a-z0-9-]*$/.test(String(config.name))) {
    errors.push({
      severity: "error",
      message:
        'Field "name" must start with a lowercase letter and contain only lowercase letters, digits, and hyphens',
      path: "name",
      line: findLine("name"),
    });
  }

  if (!config.deployment) {
    errors.push({
      severity: "error",
      message: 'Missing required field "deployment"',
      path: "deployment",
      line: findLine("deployment"),
    });
  } else {
    const validProviders = [
      "fly",
      "kubernetes",
      "docker-compose",
      "docker",
      "devpod",
      "e2b",
      "runpod",
      "northflank",
    ];
    if (!config.deployment.provider) {
      errors.push({
        severity: "error",
        message: 'Missing required field "deployment.provider"',
        path: "deployment.provider",
        line: findLine("provider"),
      });
    } else if (!validProviders.includes(String(config.deployment.provider))) {
      errors.push({
        severity: "error",
        message: `Invalid provider "${config.deployment.provider}". Must be one of: ${validProviders.join(", ")}`,
        path: "deployment.provider",
        line: findLine("provider"),
      });
    }
  }

  if (!config.extensions) {
    errors.push({
      severity: "error",
      message: 'Missing required field "extensions"',
      path: "extensions",
      line: findLine("extensions"),
    });
  } else {
    const ext = config.extensions;
    const hasProfile = ext.profile !== undefined;
    const hasActive = ext.active !== undefined;

    if (!hasProfile && !hasActive) {
      errors.push({
        severity: "error",
        message: 'Field "extensions" must have either "profile" or "active"',
        path: "extensions",
        line: findLine("extensions"),
      });
    }

    if (hasProfile && hasActive) {
      errors.push({
        severity: "error",
        message: 'Fields "extensions.profile" and "extensions.active" are mutually exclusive',
        path: "extensions",
        line: findLine("profile"),
      });
    }

    if (hasActive && ext.additional) {
      errors.push({
        severity: "error",
        message:
          'Field "extensions.additional" cannot be used with "extensions.active" (only with profile)',
        path: "extensions.additional",
        line: findLine("additional"),
      });
    }

    if (hasProfile) {
      const validProfiles = [
        "minimal",
        "fullstack",
        "anthropic-dev",
        "systems",
        "enterprise",
        "devops",
        "mobile",
        "visionflow-core",
        "visionflow-data-scientist",
        "visionflow-creative",
        "visionflow-full",
      ];
      if (!validProfiles.includes(String(ext.profile))) {
        errors.push({
          severity: "error",
          message: `Invalid profile "${ext.profile}". Must be one of: ${validProfiles.join(", ")}`,
          path: "extensions.profile",
          line: findLine("profile"),
        });
      }
    }
  }

  return errors;
}

export interface YamlValidationOptions {
  debounceMs?: number;
}

export function useYamlValidation(
  yaml: string,
  options: YamlValidationOptions = {},
): ValidationResult {
  const { debounceMs = 300 } = options;
  const [result, setResult] = useState<ValidationResult>({ valid: true, errors: [] });

  const validate = useCallback((input: string): ValidationResult => {
    if (!input.trim()) {
      return { valid: false, errors: [{ severity: "error", message: "YAML content is empty" }] };
    }

    const lines = input.split("\n");
    const { value: config, parseError } = parseSimpleYaml(input);

    if (parseError || !config) {
      return {
        valid: false,
        errors: [{ severity: "error", message: parseError ?? "Failed to parse YAML" }],
      };
    }

    const errors = validateSindriConfig(config, lines);
    return { valid: errors.length === 0, errors };
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => {
      setResult(validate(yaml));
    }, debounceMs);
    return () => clearTimeout(timer);
  }, [yaml, debounceMs, validate]);

  return result;
}
