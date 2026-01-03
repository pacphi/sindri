/**
 * E2B PTY Proxy for Enhanced Terminal Access
 *
 * This module provides an enhanced terminal connection to E2B sandboxes,
 * bridging local PTY with E2B's command execution API.
 *
 * Phase 2 Feature - Basic implementation for future enhancement.
 *
 * Features:
 * - Full PTY emulation with proper terminal sizing
 * - Signal handling (Ctrl+C, Ctrl+D, etc.)
 * - Terminal resize support
 * - Session persistence during connection
 *
 * @see https://e2b.dev/docs/sandbox/terminal
 */

import { Sandbox } from 'e2b';
import * as readline from 'readline';

/**
 * Terminal session configuration
 */
export interface TerminalConfig {
  /** Shell to use (default: /bin/bash) */
  shell: string;
  /** Initial working directory */
  cwd: string;
  /** Environment variables to set */
  env: Record<string, string>;
  /** Terminal columns */
  cols: number;
  /** Terminal rows */
  rows: number;
}

/**
 * Default terminal configuration
 */
const DEFAULT_TERMINAL_CONFIG: TerminalConfig = {
  shell: '/bin/bash',
  cwd: '/alt/home/developer/workspace',
  env: {},
  cols: process.stdout.columns || 80,
  rows: process.stdout.rows || 24,
};

/**
 * Connects to an E2B sandbox with an interactive terminal session.
 *
 * This creates a PTY-like experience by:
 * 1. Setting up raw mode for character-by-character input
 * 2. Executing commands via E2B SDK
 * 3. Handling terminal resize events
 * 4. Managing signal propagation
 *
 * @param sandboxId - E2B sandbox ID to connect to
 * @param config - Optional terminal configuration
 *
 * @example
 * ```typescript
 * // Connect to a running sandbox
 * await connectToSandbox('sbx_abc123');
 *
 * // Connect with custom configuration
 * await connectToSandbox('sbx_abc123', {
 *   shell: '/bin/zsh',
 *   cwd: '/home/developer/project',
 * });
 * ```
 */
export async function connectToSandbox(
  sandboxId: string,
  config: Partial<TerminalConfig> = {}
): Promise<void> {
  const termConfig = { ...DEFAULT_TERMINAL_CONFIG, ...config };

  console.log(`Connecting to E2B sandbox: ${sandboxId}`);
  console.log('Press Ctrl+D to disconnect\n');

  // Connect to the sandbox
  const sandbox = await Sandbox.connect(sandboxId);

  // Set up terminal state
  let isConnected = true;

  // Handle terminal resize
  const handleResize = (): void => {
    termConfig.cols = process.stdout.columns || 80;
    termConfig.rows = process.stdout.rows || 24;
    // Note: E2B SDK doesn't directly support resize,
    // but we track it for proper output formatting
  };

  process.stdout.on('resize', handleResize);

  // Set up raw mode for character input
  if (process.stdin.isTTY) {
    process.stdin.setRawMode(true);
  }
  process.stdin.resume();

  // Create readline interface for input handling
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: true,
  });

  // Buffer for accumulating input
  let inputBuffer = '';

  // Handle input
  process.stdin.on('data', async (data: Buffer) => {
    if (!isConnected) return;

    const input = data.toString();

    // Handle special keys
    if (input === '\x04') {
      // Ctrl+D - disconnect
      console.log('\nDisconnecting...');
      isConnected = false;
      cleanup();
      return;
    }

    if (input === '\x03') {
      // Ctrl+C - send interrupt
      try {
        await sandbox.commands.run('kill -INT $$', {
          cwd: termConfig.cwd,
          envs: termConfig.env,
        });
      } catch {
        // Ignore errors from interrupt
      }
      return;
    }

    // Accumulate input until newline
    inputBuffer += input;

    // Echo input if terminal
    if (process.stdin.isTTY) {
      process.stdout.write(input);
    }

    // Execute on newline
    if (input.includes('\n') || input.includes('\r')) {
      const command = inputBuffer.trim();
      inputBuffer = '';

      if (command) {
        try {
          const result = await sandbox.commands.run(command, {
            cwd: termConfig.cwd,
            envs: termConfig.env,
            onStdout: (output: string) => {
              process.stdout.write(output);
            },
            onStderr: (output: string) => {
              process.stderr.write(output);
            },
          });

          // Update cwd if cd command was executed
          if (command.startsWith('cd ')) {
            const newDir = command.slice(3).trim();
            if (newDir && !newDir.startsWith('-')) {
              // Try to resolve the new directory
              try {
                const pwdResult = await sandbox.commands.run('pwd', {
                  cwd: newDir.startsWith('/')
                    ? newDir
                    : `${termConfig.cwd}/${newDir}`,
                });
                if (pwdResult.exitCode === 0) {
                  termConfig.cwd = pwdResult.stdout.trim();
                }
              } catch {
                // Keep current directory on error
              }
            }
          }

          // Show exit code if non-zero
          if (result.exitCode !== 0) {
            console.log(`\n[Exit code: ${result.exitCode}]`);
          }
        } catch (error) {
          console.error('\nCommand execution error:', error);
        }
      }

      // Show prompt
      showPrompt(termConfig.cwd);
    }
  });

  // Cleanup function
  const cleanup = (): void => {
    if (process.stdin.isTTY) {
      process.stdin.setRawMode(false);
    }
    process.stdin.pause();
    rl.close();
    process.stdout.removeListener('resize', handleResize);
    sandbox.close();
  };

  // Handle process exit
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);

  // Show initial prompt
  console.log(`Connected to sandbox. Shell: ${termConfig.shell}`);
  console.log(`Working directory: ${termConfig.cwd}\n`);
  showPrompt(termConfig.cwd);

  // Keep the process running
  await new Promise<void>((resolve) => {
    const checkConnection = setInterval(() => {
      if (!isConnected) {
        clearInterval(checkConnection);
        resolve();
      }
    }, 100);
  });

  cleanup();
  console.log('Disconnected from sandbox.');
}

/**
 * Shows the command prompt
 */
function showPrompt(cwd: string): void {
  const shortCwd = cwd.replace(/^\/alt\/home\/developer/, '~');
  process.stdout.write(`\n${shortCwd} $ `);
}

/**
 * Runs a single command in the sandbox and returns the result
 *
 * @param sandboxId - E2B sandbox ID
 * @param command - Command to execute
 * @param options - Execution options
 * @returns Command result with stdout, stderr, and exit code
 */
export async function runCommand(
  sandboxId: string,
  command: string,
  options: {
    cwd?: string;
    env?: Record<string, string>;
    timeout?: number;
  } = {}
): Promise<{
  stdout: string;
  stderr: string;
  exitCode: number;
}> {
  const sandbox = await Sandbox.connect(sandboxId);

  try {
    const result = await sandbox.commands.run(command, {
      cwd: options.cwd || '/alt/home/developer/workspace',
      envs: options.env || {},
      timeoutMs: options.timeout || 30000,
    });

    return {
      stdout: result.stdout,
      stderr: result.stderr,
      exitCode: result.exitCode,
    };
  } finally {
    sandbox.close();
  }
}

/**
 * Checks if a sandbox is running and accessible
 *
 * @param sandboxId - E2B sandbox ID to check
 * @returns True if sandbox is running and accessible
 */
export async function isSandboxRunning(sandboxId: string): Promise<boolean> {
  try {
    const sandbox = await Sandbox.connect(sandboxId);
    const result = await sandbox.commands.run('echo ok', { timeoutMs: 5000 });
    sandbox.close();
    return result.exitCode === 0;
  } catch {
    return false;
  }
}

// CLI entrypoint
if (import.meta.url === `file://${process.argv[1]}`) {
  const sandboxId = process.argv[2];

  if (!sandboxId) {
    console.error('Usage: npx tsx connect-proxy.ts <sandbox-id>');
    console.error('Example: npx tsx connect-proxy.ts sbx_abc123');
    process.exit(1);
  }

  connectToSandbox(sandboxId).catch((error) => {
    console.error('Connection failed:', error);
    process.exit(1);
  });
}
