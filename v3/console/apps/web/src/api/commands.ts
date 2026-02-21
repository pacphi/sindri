import type {
  CommandExecution,
  BulkCommandResult,
  DispatchCommandRequest,
  BulkCommandRequest,
  ScriptRequest,
} from "@/types/command";

const API_BASE = "/api/v1";

export async function dispatchCommand(req: DispatchCommandRequest): Promise<CommandExecution> {
  const response = await fetch(`${API_BASE}/commands`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({ message: "Command dispatch failed" }));
    throw new Error((err as { message?: string }).message ?? "Command dispatch failed");
  }

  return response.json() as Promise<CommandExecution>;
}

export async function dispatchBulkCommand(
  req: BulkCommandRequest,
): Promise<{ results: BulkCommandResult[] }> {
  const response = await fetch(`${API_BASE}/commands/bulk`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({ message: "Bulk command dispatch failed" }));
    throw new Error((err as { message?: string }).message ?? "Bulk command dispatch failed");
  }

  return response.json() as Promise<{ results: BulkCommandResult[] }>;
}

export async function dispatchScript(
  req: ScriptRequest,
): Promise<{ results: BulkCommandResult[] }> {
  const response = await fetch(`${API_BASE}/commands/script`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({ message: "Script dispatch failed" }));
    throw new Error((err as { message?: string }).message ?? "Script dispatch failed");
  }

  return response.json() as Promise<{ results: BulkCommandResult[] }>;
}

export async function getCommandHistory(params?: {
  instanceId?: string;
  page?: number;
  pageSize?: number;
  status?: string;
}): Promise<{
  executions: CommandExecution[];
  pagination: { total: number; page: number; pageSize: number; totalPages: number };
}> {
  const search = new URLSearchParams();
  if (params?.instanceId) search.set("instanceId", params.instanceId);
  if (params?.page) search.set("page", String(params.page));
  if (params?.pageSize) search.set("pageSize", String(params.pageSize));
  if (params?.status) search.set("status", params.status);

  const response = await fetch(`${API_BASE}/commands/history?${search.toString()}`);

  if (!response.ok) {
    throw new Error("Failed to fetch command history");
  }

  return response.json();
}

export async function getCommandExecution(id: string): Promise<CommandExecution> {
  const response = await fetch(`${API_BASE}/commands/${id}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch command execution ${id}`);
  }

  return response.json() as Promise<CommandExecution>;
}
