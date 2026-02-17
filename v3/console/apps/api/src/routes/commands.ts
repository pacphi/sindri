/**
 * Command execution routes.
 *
 * POST   /api/v1/commands           — dispatch a command to one instance
 * POST   /api/v1/commands/bulk      — dispatch a command to multiple instances in parallel
 * POST   /api/v1/commands/script    — upload and execute a script on one or more instances
 * GET    /api/v1/commands/history   — list recent command executions
 * GET    /api/v1/commands/:id       — get a specific command execution record
 *
 * Command dispatch sends a `command:exec` envelope over WebSocket to the
 * connected agent and awaits a `command:result` reply via a Redis sub channel.
 * Results are persisted in the `CommandExecution` table.
 */

import { Hono } from "hono";
import { z } from "zod";
import { v4 as uuidv4 } from "uuid";
import { authMiddleware, requireRole } from "../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../middleware/rateLimit.js";
import { db } from "../lib/db.js";
import { redis } from "../lib/redis.js";
import { logger } from "../lib/logger.js";
import { makeEnvelope, CHANNEL, MESSAGE_TYPE } from "../websocket/channels.js";
import { agentConnections } from "../agents/gateway.js";

// ─────────────────────────────────────────────────────────────────────────────
// Zod schemas
// ─────────────────────────────────────────────────────────────────────────────

const CommandSchema = z.object({
  instanceId: z.string().min(1).max(128),
  command: z.string().min(1).max(4096),
  args: z.array(z.string().max(1024)).max(64).optional(),
  env: z.record(z.string().max(256), z.string().max(4096)).optional(),
  workingDir: z.string().max(512).optional(),
  timeoutMs: z.number().int().min(1000).max(300_000).optional().default(30_000),
});

const BulkCommandSchema = z.object({
  instanceIds: z.array(z.string().min(1).max(128)).min(1).max(50),
  command: z.string().min(1).max(4096),
  args: z.array(z.string().max(1024)).max(64).optional(),
  env: z.record(z.string().max(256), z.string().max(4096)).optional(),
  workingDir: z.string().max(512).optional(),
  timeoutMs: z.number().int().min(1000).max(300_000).optional().default(30_000),
});

const ScriptSchema = z.object({
  instanceIds: z.array(z.string().min(1).max(128)).min(1).max(50),
  script: z.string().min(1).max(65_536),
  interpreter: z.string().max(128).optional().default("/bin/bash"),
  timeoutMs: z.number().int().min(1000).max(300_000).optional().default(60_000),
});

const HistoryQuerySchema = z.object({
  instanceId: z.string().max(128).optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
  status: z.enum(["PENDING", "RUNNING", "SUCCEEDED", "FAILED", "TIMEOUT"]).optional(),
});

// ─────────────────────────────────────────────────────────────────────────────
// Command dispatch helpers
// ─────────────────────────────────────────────────────────────────────────────

interface CommandResult {
  exitCode: number;
  stdout: string;
  stderr: string;
  durationMs: number;
}

/**
 * Sends a command to an agent and waits for the result.
 * Returns the result or throws on timeout / agent unavailable.
 */
async function dispatchCommand(
  instanceId: string,
  command: string,
  opts: {
    args?: string[];
    env?: Record<string, string>;
    workingDir?: string;
    timeoutMs: number;
    correlationId: string;
  },
): Promise<CommandResult> {
  const agentWs = agentConnections.get(instanceId);
  if (!agentWs || agentWs.ws.readyState !== 1 /* OPEN */) {
    throw new Error(`Agent for instance '${instanceId}' is not connected`);
  }

  const envelope = makeEnvelope(
    CHANNEL.COMMANDS,
    MESSAGE_TYPE.COMMAND_EXEC,
    {
      command,
      args: opts.args,
      env: opts.env,
      workingDir: opts.workingDir,
      timeout: opts.timeoutMs,
    },
    { instanceId, correlationId: opts.correlationId },
  );

  // Subscribe to the result channel before sending so we don't miss the reply
  const resultKey = `sindri:cmd:result:${opts.correlationId}`;

  return new Promise((resolve, reject) => {
    let settled = false;

    function settle(fn: () => void) {
      if (settled) return;
      settled = true;
      clearInterval(pollInterval);
      clearTimeout(timer);
      fn();
    }

    const timer = setTimeout(() => {
      settle(() => reject(new Error(`Command timed out after ${opts.timeoutMs}ms`)));
    }, opts.timeoutMs + 5_000); // extra buffer beyond agent timeout

    // Poll Redis for the result (agent publishes to this key)
    const pollInterval = setInterval(async () => {
      try {
        const raw = await redis.get(resultKey);
        if (raw) {
          settle(() => resolve(JSON.parse(raw) as CommandResult));
        }
      } catch {
        // continue polling
      }
    }, 200);

    // Send the command to the agent
    agentWs.ws.send(JSON.stringify(envelope));
  });
}

/**
 * Execute a command on one instance, persist it, and return the DB record.
 */
async function runCommand(
  instanceId: string,
  command: string,
  opts: {
    args?: string[];
    env?: Record<string, string>;
    workingDir?: string;
    timeoutMs: number;
    userId: string;
    scriptContent?: string;
  },
) {
  const correlationId = uuidv4();

  // Persist pending record
  const record = await db.commandExecution.create({
    data: {
      instance_id: instanceId,
      user_id: opts.userId,
      command,
      args: opts.args ?? [],
      env: opts.env ?? {},
      working_dir: opts.workingDir,
      timeout_ms: opts.timeoutMs,
      correlation_id: correlationId,
      status: "RUNNING",
      script_content: opts.scriptContent,
    },
  });

  try {
    const result = await dispatchCommand(instanceId, command, {
      args: opts.args,
      env: opts.env,
      workingDir: opts.workingDir,
      timeoutMs: opts.timeoutMs,
      correlationId,
    });

    const updated = await db.commandExecution.update({
      where: { id: record.id },
      data: {
        status: result.exitCode === 0 ? "SUCCEEDED" : "FAILED",
        exit_code: result.exitCode,
        stdout: result.stdout,
        stderr: result.stderr,
        duration_ms: result.durationMs,
        completed_at: new Date(),
      },
    });
    return updated;
  } catch (err) {
    const isTimeout = err instanceof Error && err.message.includes("timed out");
    await db.commandExecution.update({
      where: { id: record.id },
      data: {
        status: isTimeout ? "TIMEOUT" : "FAILED",
        stderr: err instanceof Error ? err.message : "Unknown error",
        completed_at: new Date(),
      },
    });
    throw err;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const commands = new Hono();

commands.use("*", authMiddleware);

// ─── POST /api/v1/commands ────────────────────────────────────────────────────

commands.post("/", rateLimitStrict, requireRole("DEVELOPER"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = CommandSchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: "Validation Error", details: parsed.error.flatten() }, 422);
  }

  const { instanceId, command, args, env, workingDir, timeoutMs } = parsed.data;
  const auth = c.get("auth");

  // Verify instance exists
  const instance = await db.instance.findUnique({ where: { id: instanceId } });
  if (!instance) {
    return c.json({ error: "Not Found", message: `Instance '${instanceId}' not found` }, 404);
  }

  try {
    const record = await runCommand(instanceId, command, {
      args,
      env,
      workingDir,
      timeoutMs,
      userId: auth.userId,
    });
    return c.json(serializeExecution(record), 201);
  } catch (err) {
    logger.error({ err, instanceId }, "Command dispatch failed");
    const message = err instanceof Error ? err.message : "Command failed";
    return c.json({ error: "Command Failed", message }, 502);
  }
});

// ─── POST /api/v1/commands/bulk ──────────────────────────────────────────────

commands.post("/bulk", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = BulkCommandSchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: "Validation Error", details: parsed.error.flatten() }, 422);
  }

  const { instanceIds, command, args, env, workingDir, timeoutMs } = parsed.data;
  const auth = c.get("auth");

  // Verify all instances exist
  const instances = await db.instance.findMany({ where: { id: { in: instanceIds } } });
  const foundIds = new Set(instances.map((i) => i.id));
  const missing = instanceIds.filter((id) => !foundIds.has(id));
  if (missing.length > 0) {
    return c.json(
      { error: "Not Found", message: `Instances not found: ${missing.join(", ")}` },
      404,
    );
  }

  // Dispatch in parallel
  const results = await Promise.allSettled(
    instanceIds.map((instanceId) =>
      runCommand(instanceId, command, {
        args,
        env,
        workingDir,
        timeoutMs,
        userId: auth.userId,
      }),
    ),
  );

  const response = instanceIds.map((instanceId, i) => {
    const result = results[i];
    if (result.status === "fulfilled") {
      return { instanceId, success: true, execution: serializeExecution(result.value) };
    }
    return {
      instanceId,
      success: false,
      error: result.reason instanceof Error ? result.reason.message : "Unknown error",
    };
  });

  return c.json({ results: response }, 207);
});

// ─── POST /api/v1/commands/script ────────────────────────────────────────────

commands.post("/script", rateLimitStrict, requireRole("DEVELOPER"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = ScriptSchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: "Validation Error", details: parsed.error.flatten() }, 422);
  }

  const { instanceIds, script, interpreter, timeoutMs } = parsed.data;
  const auth = c.get("auth");

  // Verify all instances exist
  const instances = await db.instance.findMany({ where: { id: { in: instanceIds } } });
  const foundIds = new Set(instances.map((i) => i.id));
  const missing = instanceIds.filter((id) => !foundIds.has(id));
  if (missing.length > 0) {
    return c.json(
      { error: "Not Found", message: `Instances not found: ${missing.join(", ")}` },
      404,
    );
  }

  // Use the interpreter to run the script via stdin pipe:
  // e.g. bash -s < <(echo "<script>")
  // We pass the script as base64 via env var to avoid shell injection
  const scriptB64 = Buffer.from(script).toString("base64");

  const results = await Promise.allSettled(
    instanceIds.map((instanceId) =>
      runCommand(instanceId, interpreter, {
        args: ["-s"],
        env: { SINDRI_SCRIPT_B64: scriptB64 },
        timeoutMs,
        userId: auth.userId,
        scriptContent: script,
      }),
    ),
  );

  const response = instanceIds.map((instanceId, i) => {
    const result = results[i];
    if (result.status === "fulfilled") {
      return { instanceId, success: true, execution: serializeExecution(result.value) };
    }
    return {
      instanceId,
      success: false,
      error: result.reason instanceof Error ? result.reason.message : "Unknown error",
    };
  });

  return c.json({ results: response }, 207);
});

// ─── GET /api/v1/commands/history ─────────────────────────────────────────────

commands.get("/history", rateLimitDefault, async (c) => {
  const queryResult = HistoryQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!queryResult.success) {
    return c.json({ error: "Validation Error", details: queryResult.error.flatten() }, 422);
  }

  const { instanceId, page, pageSize, status } = queryResult.data;
  const skip = (page - 1) * pageSize;

  const where = {
    ...(instanceId ? { instance_id: instanceId } : {}),
    ...(status ? { status } : {}),
  };

  const [total, executions] = await Promise.all([
    db.commandExecution.count({ where }),
    db.commandExecution.findMany({
      where,
      orderBy: { created_at: "desc" },
      skip,
      take: pageSize,
    }),
  ]);

  return c.json({
    executions: executions.map(serializeExecution),
    pagination: {
      total,
      page,
      pageSize,
      totalPages: Math.ceil(total / pageSize),
    },
  });
});

// ─── GET /api/v1/commands/:id ─────────────────────────────────────────────────

commands.get("/:id", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  if (!id || id.length > 128) {
    return c.json({ error: "Bad Request", message: "Invalid execution ID" }, 400);
  }

  const execution = await db.commandExecution.findUnique({ where: { id } });
  if (!execution) {
    return c.json({ error: "Not Found", message: `Command execution '${id}' not found` }, 404);
  }

  return c.json(serializeExecution(execution));
});

// ─────────────────────────────────────────────────────────────────────────────
// Serializer
// ─────────────────────────────────────────────────────────────────────────────

function serializeExecution(e: {
  id: string;
  instance_id: string;
  user_id: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  working_dir: string | null;
  timeout_ms: number;
  status: string;
  exit_code: number | null;
  stdout: string | null;
  stderr: string | null;
  duration_ms: number | null;
  correlation_id: string;
  script_content: string | null;
  created_at: Date;
  completed_at: Date | null;
}) {
  return {
    id: e.id,
    instanceId: e.instance_id,
    userId: e.user_id,
    command: e.command,
    args: e.args,
    env: e.env,
    workingDir: e.working_dir,
    timeoutMs: e.timeout_ms,
    status: e.status,
    exitCode: e.exit_code,
    stdout: e.stdout,
    stderr: e.stderr,
    durationMs: e.duration_ms,
    correlationId: e.correlation_id,
    hasScript: e.script_content !== null,
    createdAt: e.created_at.toISOString(),
    completedAt: e.completed_at?.toISOString() ?? null,
  };
}

export { commands as commandsRouter };
