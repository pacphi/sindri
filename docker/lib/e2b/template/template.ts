/**
 * E2B Template Definition for Sindri
 *
 * This module defines the E2B template configuration for building Sindri
 * development environments as E2B sandbox templates.
 *
 * @see https://e2b.dev/docs/sandbox-template
 */

import { Template, waitForTimeout } from 'e2b';

/**
 * Configuration options for creating a Sindri E2B template
 */
export interface SindriConfig {
  /** Extension profile to install (e.g., 'default', 'ai-developer', 'full') */
  profile: string;
  /** Comma-separated list of additional extensions to install */
  additionalExtensions: string;
  /** Number of vCPUs for the sandbox (1-8) */
  cpus: number;
  /** Memory allocation in MB (512-8192) */
  memoryMB: number;
  /** Template alias for registration (e.g., 'sindri-dev') */
  templateAlias: string;
}

/**
 * Creates an E2B template definition for Sindri development environments.
 *
 * The template is built from Sindri's Dockerfile and configured with:
 * - Proper environment variables for home directory and workspace
 * - Extension profile and additional extensions
 * - Developer user context
 * - Initialization via entrypoint script
 *
 * @param config - Sindri template configuration
 * @returns E2B Template definition ready for building
 *
 * @example
 * ```typescript
 * const template = createSindriTemplate({
 *   profile: 'ai-developer',
 *   additionalExtensions: 'rust,golang',
 *   cpus: 2,
 *   memoryMB: 4096,
 *   templateAlias: 'sindri-ai-dev',
 * });
 * ```
 */
export function createSindriTemplate(config: SindriConfig) {
  return (
    Template()
      // Build from Sindri's base Dockerfile
      // Path is relative to the template directory (docker/lib/e2b/template/)
      .fromDockerfile('../../../Dockerfile')

      // Set environment variables for Sindri runtime
      .setEnvs({
        // Core Sindri environment
        HOME: '/alt/home/developer',
        WORKSPACE: '/alt/home/developer/workspace',
        DOCKER_LIB: '/docker/lib',

        // Extension configuration
        INSTALL_PROFILE: config.profile,
        ADDITIONAL_EXTENSIONS: config.additionalExtensions,

        // Initialization flags
        INIT_WORKSPACE: 'true',
        SKIP_AUTO_INSTALL: 'false',

        // E2B-specific marker (allows scripts to detect E2B environment)
        E2B_PROVIDER: 'true',
        SINDRI_PROVIDER: 'e2b',

        // Mise configuration (tool version manager)
        MISE_YES: '1',
        MISE_TRUSTED_CONFIG_PATHS:
          '/alt/home/developer/.config/mise:/alt/home/developer/.config/mise/conf.d',
      })

      // Set working directory to workspace
      .setWorkdir('/alt/home/developer/workspace')

      // Run as developer user (UID 1001)
      .setUser('developer')

      // Run initialization through entrypoint
      // This sets up the home directory, installs extensions, and configures the environment
      .runCmd('/docker/scripts/entrypoint.sh echo "Template initialized"')

      // Set ready command to verify environment is properly initialized
      // The .initialized file is created by the entrypoint after successful setup
      // Timeout after 30 seconds if initialization doesn't complete
      .setReadyCmd(
        'test -f /alt/home/developer/.initialized',
        waitForTimeout(30_000)
      )
  );
}

/**
 * Creates a minimal E2B template for quick ephemeral sandboxes.
 *
 * This template skips extension auto-installation for faster startup,
 * useful for throwaway environments or when extensions will be
 * installed manually.
 *
 * @param config - Sindri template configuration
 * @returns E2B Template definition optimized for fast startup
 */
export function createMinimalSindriTemplate(config: SindriConfig) {
  return (
    Template()
      .fromDockerfile('../../../Dockerfile')

      .setEnvs({
        HOME: '/alt/home/developer',
        WORKSPACE: '/alt/home/developer/workspace',
        DOCKER_LIB: '/docker/lib',
        INSTALL_PROFILE: config.profile,
        ADDITIONAL_EXTENSIONS: config.additionalExtensions,
        INIT_WORKSPACE: 'true',
        // Skip auto-install for faster startup
        SKIP_AUTO_INSTALL: 'true',
        E2B_PROVIDER: 'true',
        SINDRI_PROVIDER: 'e2b',
        MISE_YES: '1',
        MISE_TRUSTED_CONFIG_PATHS:
          '/alt/home/developer/.config/mise:/alt/home/developer/.config/mise/conf.d',
      })

      .setWorkdir('/alt/home/developer/workspace')
      .setUser('developer')

      // Minimal initialization - just ensure home directory exists
      .runCmd('mkdir -p /alt/home/developer/workspace')

      // Quick ready check - just verify bash is available
      .setReadyCmd('which bash', waitForTimeout(5_000))
  );
}
