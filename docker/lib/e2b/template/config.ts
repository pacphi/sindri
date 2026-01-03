/**
 * E2B Template Configuration Loader
 *
 * This module handles loading Sindri configuration from various sources:
 * - Environment variables
 * - .e2b/config.json file
 * - sindri.yaml (via yq parsing)
 * - Default values
 *
 * Configuration priority (highest to lowest):
 * 1. Environment variables
 * 2. .e2b/config.json
 * 3. sindri.yaml
 * 4. Default values
 */

import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';
import type { SindriConfig } from './template';

/**
 * Extended configuration including E2B-specific options
 */
export interface E2BConfig extends SindriConfig {
  /** E2B API key for authentication */
  apiKey?: string;
  /** Sandbox timeout in seconds */
  timeout: number;
  /** Enable auto-pause on timeout */
  autoPause: boolean;
  /** Enable auto-resume on connect */
  autoResume: boolean;
  /** Reuse existing template if available */
  reuseTemplate: boolean;
  /** Force template rebuild */
  buildOnDeploy: boolean;
  /** Metadata for sandbox identification */
  metadata: Record<string, string>;
}

/**
 * Default configuration values
 */
const DEFAULT_CONFIG: E2BConfig = {
  profile: 'default',
  additionalExtensions: '',
  cpus: 2,
  memoryMB: 2048,
  templateAlias: 'sindri-dev',
  timeout: 300, // 5 minutes
  autoPause: true,
  autoResume: true,
  reuseTemplate: true,
  buildOnDeploy: false,
  metadata: {},
};

/**
 * Path to the .e2b configuration directory
 */
function getE2BConfigDir(): string {
  // Check for E2B_CONFIG_DIR environment variable
  if (process.env.E2B_CONFIG_DIR) {
    return process.env.E2B_CONFIG_DIR;
  }

  // Default to .e2b in project root
  // Walk up from current directory to find project root (contains sindri.yaml or package.json)
  let currentDir = process.cwd();
  const maxDepth = 10;
  let depth = 0;

  while (depth < maxDepth) {
    if (
      fs.existsSync(path.join(currentDir, 'sindri.yaml')) ||
      fs.existsSync(path.join(currentDir, 'package.json'))
    ) {
      return path.join(currentDir, '.e2b');
    }
    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      break; // Reached root
    }
    currentDir = parentDir;
    depth++;
  }

  // Fallback to current directory
  return path.join(process.cwd(), '.e2b');
}

/**
 * Loads configuration from .e2b/config.json if it exists
 */
function loadE2BConfigFile(): Partial<E2BConfig> {
  const configPath = path.join(getE2BConfigDir(), 'config.json');

  if (!fs.existsSync(configPath)) {
    return {};
  }

  try {
    const content = fs.readFileSync(configPath, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    console.warn(`Warning: Failed to parse ${configPath}:`, error);
    return {};
  }
}

/**
 * Loads configuration from sindri.yaml using yq
 */
function loadSindriYaml(): Partial<E2BConfig> {
  // Find sindri.yaml
  let sindriYamlPath: string | null = null;
  let currentDir = process.cwd();
  const maxDepth = 10;
  let depth = 0;

  while (depth < maxDepth) {
    const candidate = path.join(currentDir, 'sindri.yaml');
    if (fs.existsSync(candidate)) {
      sindriYamlPath = candidate;
      break;
    }
    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      break;
    }
    currentDir = parentDir;
    depth++;
  }

  if (!sindriYamlPath) {
    return {};
  }

  try {
    // Use yq to parse YAML (should be available in Sindri environment)
    const config: Partial<E2BConfig> = {};

    // Parse core settings
    const name = execSync(`yq '.name // ""' "${sindriYamlPath}"`, {
      encoding: 'utf-8',
    }).trim();
    if (name) {
      config.templateAlias = `sindri-${name}`;
    }

    // Parse profile
    const profile = execSync(
      `yq '.deployment.extensions.profile // "default"' "${sindriYamlPath}"`,
      { encoding: 'utf-8' }
    ).trim();
    if (profile) {
      config.profile = profile;
    }

    // Parse additional extensions
    const additionalExtensions = execSync(
      `yq '.deployment.extensions.additional // [] | join(",")' "${sindriYamlPath}"`,
      { encoding: 'utf-8' }
    ).trim();
    if (additionalExtensions) {
      config.additionalExtensions = additionalExtensions;
    }

    // Parse resources
    const cpus = execSync(
      `yq '.deployment.resources.cpus // 2' "${sindriYamlPath}"`,
      { encoding: 'utf-8' }
    ).trim();
    if (cpus) {
      config.cpus = parseInt(cpus, 10);
    }

    const memory = execSync(
      `yq '.deployment.resources.memory // "2GB"' "${sindriYamlPath}"`,
      { encoding: 'utf-8' }
    ).trim();
    if (memory) {
      // Convert memory string (e.g., "2GB", "4096MB") to MB
      config.memoryMB = parseMemoryToMB(memory);
    }

    // Parse E2B-specific settings
    const e2bConfig = execSync(
      `yq '.providers.e2b // {}' "${sindriYamlPath}"`,
      { encoding: 'utf-8' }
    ).trim();

    if (e2bConfig && e2bConfig !== '{}') {
      const templateAlias = execSync(
        `yq '.providers.e2b.templateAlias // ""' "${sindriYamlPath}"`,
        { encoding: 'utf-8' }
      ).trim();
      if (templateAlias) {
        config.templateAlias = templateAlias;
      }

      const timeout = execSync(
        `yq '.providers.e2b.timeout // 300' "${sindriYamlPath}"`,
        { encoding: 'utf-8' }
      ).trim();
      if (timeout) {
        config.timeout = parseInt(timeout, 10);
      }

      const autoPause = execSync(
        `yq '.providers.e2b.autoPause // true' "${sindriYamlPath}"`,
        { encoding: 'utf-8' }
      ).trim();
      config.autoPause = autoPause === 'true';

      const autoResume = execSync(
        `yq '.providers.e2b.autoResume // true' "${sindriYamlPath}"`,
        { encoding: 'utf-8' }
      ).trim();
      config.autoResume = autoResume === 'true';

      const reuseTemplate = execSync(
        `yq '.providers.e2b.reuseTemplate // true' "${sindriYamlPath}"`,
        { encoding: 'utf-8' }
      ).trim();
      config.reuseTemplate = reuseTemplate === 'true';

      const buildOnDeploy = execSync(
        `yq '.providers.e2b.buildOnDeploy // false' "${sindriYamlPath}"`,
        { encoding: 'utf-8' }
      ).trim();
      config.buildOnDeploy = buildOnDeploy === 'true';
    }

    return config;
  } catch (error) {
    // yq might not be available during template build in E2B
    console.warn('Warning: Failed to parse sindri.yaml:', error);
    return {};
  }
}

/**
 * Converts memory string to megabytes
 *
 * @param memory - Memory string (e.g., "2GB", "4096MB", "2048")
 * @returns Memory in megabytes
 */
function parseMemoryToMB(memory: string): number {
  const value = parseInt(memory, 10);

  if (memory.toLowerCase().includes('gb')) {
    return value * 1024;
  } else if (memory.toLowerCase().includes('mb')) {
    return value;
  } else {
    // Assume MB if no unit specified
    return value;
  }
}

/**
 * Loads configuration from environment variables
 */
function loadEnvConfig(): Partial<E2BConfig> {
  const config: Partial<E2BConfig> = {};

  if (process.env.E2B_API_KEY) {
    config.apiKey = process.env.E2B_API_KEY;
  }

  if (process.env.INSTALL_PROFILE) {
    config.profile = process.env.INSTALL_PROFILE;
  }

  if (process.env.ADDITIONAL_EXTENSIONS) {
    config.additionalExtensions = process.env.ADDITIONAL_EXTENSIONS;
  }

  if (process.env.E2B_TEMPLATE_ALIAS) {
    config.templateAlias = process.env.E2B_TEMPLATE_ALIAS;
  }

  if (process.env.E2B_CPUS) {
    config.cpus = parseInt(process.env.E2B_CPUS, 10);
  }

  if (process.env.E2B_MEMORY_MB) {
    config.memoryMB = parseInt(process.env.E2B_MEMORY_MB, 10);
  }

  if (process.env.E2B_TIMEOUT) {
    config.timeout = parseInt(process.env.E2B_TIMEOUT, 10);
  }

  if (process.env.E2B_AUTO_PAUSE) {
    config.autoPause = process.env.E2B_AUTO_PAUSE === 'true';
  }

  if (process.env.E2B_AUTO_RESUME) {
    config.autoResume = process.env.E2B_AUTO_RESUME === 'true';
  }

  if (process.env.E2B_REUSE_TEMPLATE) {
    config.reuseTemplate = process.env.E2B_REUSE_TEMPLATE === 'true';
  }

  if (process.env.E2B_BUILD_ON_DEPLOY) {
    config.buildOnDeploy = process.env.E2B_BUILD_ON_DEPLOY === 'true';
  }

  return config;
}

/**
 * Loads and merges Sindri configuration from all sources.
 *
 * Priority (highest to lowest):
 * 1. Environment variables
 * 2. .e2b/config.json
 * 3. sindri.yaml
 * 4. Default values
 *
 * @returns Merged E2B configuration
 *
 * @example
 * ```typescript
 * const config = loadSindriConfig();
 * console.log(`Building template: ${config.templateAlias}`);
 * console.log(`Profile: ${config.profile}`);
 * console.log(`Resources: ${config.cpus} CPUs, ${config.memoryMB}MB RAM`);
 * ```
 */
export function loadSindriConfig(): E2BConfig {
  // Load from all sources
  const sindriYamlConfig = loadSindriYaml();
  const e2bFileConfig = loadE2BConfigFile();
  const envConfig = loadEnvConfig();

  // Merge with priority: env > e2b file > sindri.yaml > defaults
  const config: E2BConfig = {
    ...DEFAULT_CONFIG,
    ...sindriYamlConfig,
    ...e2bFileConfig,
    ...envConfig,
  };

  // Validate required fields
  validateConfig(config);

  return config;
}

/**
 * Validates the configuration
 *
 * @param config - Configuration to validate
 * @throws Error if configuration is invalid
 */
function validateConfig(config: E2BConfig): void {
  // Validate CPUs (E2B supports 1-8 vCPUs)
  if (config.cpus < 1 || config.cpus > 8) {
    throw new Error(`Invalid cpus: ${config.cpus}. Must be between 1 and 8.`);
  }

  // Validate memory (E2B supports 512MB-8GB)
  if (config.memoryMB < 512 || config.memoryMB > 8192) {
    throw new Error(
      `Invalid memoryMB: ${config.memoryMB}. Must be between 512 and 8192.`
    );
  }

  // Validate template alias format
  if (!/^[a-z][a-z0-9-]*$/.test(config.templateAlias)) {
    throw new Error(
      `Invalid templateAlias: ${config.templateAlias}. Must start with lowercase letter and contain only lowercase letters, numbers, and hyphens.`
    );
  }

  // Validate timeout
  if (config.timeout < 60 || config.timeout > 86400) {
    throw new Error(
      `Invalid timeout: ${config.timeout}. Must be between 60 and 86400 seconds.`
    );
  }
}

/**
 * Saves configuration to .e2b/config.json
 *
 * @param config - Configuration to save
 */
export function saveE2BConfig(config: Partial<E2BConfig>): void {
  const configDir = getE2BConfigDir();
  const configPath = path.join(configDir, 'config.json');

  // Create directory if it doesn't exist
  if (!fs.existsSync(configDir)) {
    fs.mkdirSync(configDir, { recursive: true });
  }

  // Load existing config and merge
  const existingConfig = loadE2BConfigFile();
  const mergedConfig = { ...existingConfig, ...config };

  // Don't save API key to file
  delete mergedConfig.apiKey;

  fs.writeFileSync(configPath, JSON.stringify(mergedConfig, null, 2));
  console.log(`Configuration saved to ${configPath}`);
}
