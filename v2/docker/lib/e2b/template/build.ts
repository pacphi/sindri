/**
 * E2B Template Build Script for Sindri
 *
 * This script builds an E2B template from Sindri's Dockerfile,
 * configuring it with the appropriate profile and extensions.
 *
 * Usage:
 *   npx tsx build.ts [options]
 *
 * Options are provided via environment variables or .e2b/config.json
 *
 * @see ./config.ts for configuration options
 */

import 'dotenv/config';
import { Template, defaultBuildLogger } from 'e2b';
import { createSindriTemplate, createMinimalSindriTemplate } from './template';
import { loadSindriConfig, saveE2BConfig } from './config';

/**
 * Command-line argument parsing
 */
interface BuildOptions {
  minimal: boolean;
  dryRun: boolean;
  verbose: boolean;
  saveConfig: boolean;
}

function parseArgs(): BuildOptions {
  const args = process.argv.slice(2);
  return {
    minimal: args.includes('--minimal'),
    dryRun: args.includes('--dry-run'),
    verbose: args.includes('--verbose') || args.includes('-v'),
    saveConfig: args.includes('--save-config'),
  };
}

/**
 * Prints build configuration summary
 */
function printConfig(config: ReturnType<typeof loadSindriConfig>): void {
  console.log('\n--- Sindri E2B Template Configuration ---');
  console.log(`Template Alias:  ${config.templateAlias}`);
  console.log(`Profile:         ${config.profile}`);
  console.log(`Extensions:      ${config.additionalExtensions || '(none)'}`);
  console.log(`vCPUs:           ${config.cpus}`);
  console.log(`Memory:          ${config.memoryMB}MB`);
  console.log(`Timeout:         ${config.timeout}s`);
  console.log(`Auto-Pause:      ${config.autoPause}`);
  console.log(`Reuse Template:  ${config.reuseTemplate}`);
  console.log('-----------------------------------------\n');
}

/**
 * Main build function
 */
async function main(): Promise<void> {
  const options = parseArgs();

  // Validate E2B API key
  if (!process.env.E2B_API_KEY) {
    console.error('Error: E2B_API_KEY environment variable is required');
    console.error('Set it via: export E2B_API_KEY=your_api_key');
    console.error('Get your API key at: https://e2b.dev/dashboard');
    process.exit(1);
  }

  // Load configuration
  let config;
  try {
    config = loadSindriConfig();
  } catch (error) {
    console.error('Configuration error:', error);
    process.exit(1);
  }

  // Print configuration
  printConfig(config);

  // Save configuration if requested
  if (options.saveConfig) {
    saveE2BConfig(config);
  }

  // Dry run mode - just show what would be built
  if (options.dryRun) {
    console.log('Dry run mode - no template will be built');
    console.log('\nTemplate definition would be created with:');
    console.log(`  - Dockerfile: Dockerfile (project root)`);
    console.log(`  - Environment: HOME=/alt/home/developer`);
    console.log(`  - Profile: ${config.profile}`);
    console.log(`  - Extensions: ${config.additionalExtensions || 'none'}`);
    console.log(`  - User: developer`);
    console.log(`  - Workdir: /alt/home/developer/workspace`);
    return;
  }

  // Create template definition
  console.log(`Creating ${options.minimal ? 'minimal ' : ''}template definition...`);
  const template = options.minimal
    ? createMinimalSindriTemplate(config)
    : createSindriTemplate(config);

  // Build the template
  console.log(`\nBuilding E2B template: ${config.templateAlias}`);
  console.log('This may take several minutes for the first build...\n');

  try {
    const buildResult = await Template.build(template, {
      alias: config.templateAlias,
      cpuCount: config.cpus,
      memoryMB: config.memoryMB,
      onBuildLogs: options.verbose
        ? defaultBuildLogger()
        : (log) => {
            // Simplified logging - show only key milestones
            if (
              log.includes('Step') ||
              log.includes('Successfully') ||
              log.includes('Error')
            ) {
              console.log(log);
            }
          },
    });

    console.log('\n--- Build Complete ---');
    console.log(`Template ID:     ${buildResult.templateId}`);
    console.log(`Template Alias:  ${config.templateAlias}`);
    console.log(`Build Duration:  (see logs above)`);
    console.log('----------------------\n');

    console.log('Next steps:');
    console.log(`  1. Create sandbox: e2b sandbox create ${config.templateAlias}`);
    console.log(`  2. Or deploy via: sindri deploy --provider e2b`);
    console.log('');
  } catch (error) {
    console.error('\nTemplate build failed:', error);

    // Provide helpful error messages
    if (error instanceof Error) {
      if (error.message.includes('authentication')) {
        console.error('\nAuthentication error. Check your E2B_API_KEY.');
      } else if (error.message.includes('Dockerfile')) {
        console.error(
          '\nDockerfile error. Ensure Dockerfile exists in the project root.'
        );
      } else if (error.message.includes('timeout')) {
        console.error(
          '\nBuild timed out. The Sindri image may be too large or complex.'
        );
        console.error('Try using --minimal flag for a faster build.');
      }
    }

    process.exit(1);
  }
}

// Run main function
main().catch((error) => {
  console.error('Unexpected error:', error);
  process.exit(1);
});
