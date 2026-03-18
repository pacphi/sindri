#!/usr/bin/env node
// E2B Sandbox Remote Command Executor
//
// Executes a command inside an existing E2B sandbox non-interactively.
// Used by the V3 E2B provider workflow for CI extension testing since the
// E2B CLI only supports interactive PTY sessions (e2b sandbox terminal).
//
// Usage:
//   node e2b-exec.mjs <sandbox-id> <command>
//
// Environment:
//   E2B_API_KEY - Required
//
// Exit code matches the command's exit code inside the sandbox.

import { Sandbox } from 'e2b'

const [,, sandboxId, ...commandParts] = process.argv
const command = commandParts.join(' ')

if (!sandboxId || !command) {
  console.error('Usage: e2b-exec.mjs <sandbox-id> <command>')
  process.exit(2)
}

try {
  const sandbox = await Sandbox.connect(sandboxId)

  const result = await sandbox.commands.run(command, {
    timeout: 600,
    onStdout: (data) => process.stdout.write(data),
    onStderr: (data) => process.stderr.write(data)
  })

  process.exit(result.exitCode)
} catch (error) {
  console.error(`E2B exec error: ${error.message}`)
  process.exit(1)
}
