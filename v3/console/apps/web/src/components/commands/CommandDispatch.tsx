import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Terminal, FileCode, List, Play, Loader2, Settings2 } from "lucide-react";
import { dispatchCommand, dispatchBulkCommand, dispatchScript } from "@/api/commands";
import type { BulkCommandResult, CommandExecution } from "@/types/command";
import { CommandEditor } from "./CommandEditor";
import { ScriptUpload } from "./ScriptUpload";
import { CommandOutput } from "./CommandOutput";
import { CommandHistory } from "./CommandHistory";
import { InstanceSelector } from "./InstanceSelector";
import { OutputAggregator } from "./OutputAggregator";
import { cn } from "@/lib/utils";

// Fetch instances for the selector
async function fetchInstances() {
  const response = await fetch("/api/v1/instances?pageSize=100");
  if (!response.ok) throw new Error("Failed to fetch instances");
  const data = (await response.json()) as { instances: import("@/types/instance").Instance[] };
  return data.instances;
}

type Tab = "command" | "script" | "history";

interface EnvEntry {
  key: string;
  value: string;
}

export function CommandDispatch() {
  const [activeTab, setActiveTab] = useState<Tab>("command");

  // Command state
  const [command, setCommand] = useState("");
  const [selectedInstanceIds, setSelectedInstanceIds] = useState<string[]>([]);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [workingDir, setWorkingDir] = useState("");
  const [timeoutMs, setTimeoutMs] = useState(30_000);
  const [envEntries, setEnvEntries] = useState<EnvEntry[]>([]);

  // Script state
  const [script, setScript] = useState("");
  const [interpreter, setInterpreter] = useState("/bin/bash");
  const [scriptInstanceIds, setScriptInstanceIds] = useState<string[]>([]);

  // Results
  const [singleResult, setSingleResult] = useState<CommandExecution | null>(null);
  const [bulkResults, setBulkResults] = useState<BulkCommandResult[] | null>(null);

  const { data: instances = [] } = useQuery({
    queryKey: ["instances", "all"],
    queryFn: fetchInstances,
    staleTime: 30_000,
  });

  const env = envEntries.reduce<Record<string, string>>((acc, e) => {
    if (e.key.trim()) acc[e.key.trim()] = e.value;
    return acc;
  }, {});

  const singleDispatch = useMutation({
    mutationFn: () =>
      dispatchCommand({
        instanceId: selectedInstanceIds[0],
        command: command.trim(),
        env: Object.keys(env).length ? env : undefined,
        workingDir: workingDir.trim() || undefined,
        timeoutMs,
      }),
    onSuccess: (data) => {
      setSingleResult(data);
      setBulkResults(null);
    },
  });

  const bulkDispatch = useMutation({
    mutationFn: () =>
      dispatchBulkCommand({
        instanceIds: selectedInstanceIds,
        command: command.trim(),
        env: Object.keys(env).length ? env : undefined,
        workingDir: workingDir.trim() || undefined,
        timeoutMs,
      }),
    onSuccess: (data) => {
      setBulkResults(data.results);
      setSingleResult(null);
    },
  });

  const scriptDispatch = useMutation({
    mutationFn: () =>
      dispatchScript({
        instanceIds: scriptInstanceIds,
        script,
        interpreter,
        timeoutMs,
      }),
    onSuccess: (data) => {
      setBulkResults(data.results);
      setSingleResult(null);
    },
  });

  function handleRun() {
    if (activeTab === "command") {
      if (selectedInstanceIds.length === 1) {
        singleDispatch.mutate();
      } else {
        bulkDispatch.mutate();
      }
    } else if (activeTab === "script") {
      scriptDispatch.mutate();
    }
  }

  const currentInstanceIds = activeTab === "script" ? scriptInstanceIds : selectedInstanceIds;
  const canRun =
    activeTab === "history"
      ? false
      : activeTab === "command"
        ? command.trim().length > 0 && selectedInstanceIds.length > 0
        : script.trim().length > 0 && scriptInstanceIds.length > 0;

  const isRunning = singleDispatch.isPending || bulkDispatch.isPending || scriptDispatch.isPending;
  const runError = singleDispatch.error ?? bulkDispatch.error ?? scriptDispatch.error;

  function addEnvEntry() {
    setEnvEntries((e) => [...e, { key: "", value: "" }]);
  }

  function removeEnvEntry(index: number) {
    setEnvEntries((e) => e.filter((_, i) => i !== index));
  }

  function updateEnvEntry(index: number, field: "key" | "value", val: string) {
    setEnvEntries((e) => e.map((entry, i) => (i === index ? { ...entry, [field]: val } : entry)));
  }

  return (
    <div className="flex flex-col gap-4">
      {/* Tab bar */}
      <div className="flex items-center gap-1 border-b pb-0">
        {(
          [
            { id: "command", label: "Command", Icon: Terminal },
            { id: "script", label: "Script", Icon: FileCode },
            { id: "history", label: "History", Icon: List },
          ] as { id: Tab; label: string; Icon: React.ElementType }[]
        ).map(({ id, label, Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => setActiveTab(id)}
            className={cn(
              "flex items-center gap-1.5 border-b-2 px-3 py-2 text-sm font-medium transition-colors",
              activeTab === id
                ? "border-primary text-foreground"
                : "border-transparent text-muted-foreground hover:text-foreground",
            )}
          >
            <Icon className="h-4 w-4" />
            {label}
          </button>
        ))}
      </div>

      {/* Command tab */}
      {activeTab === "command" && (
        <div className="space-y-3">
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">
              Target Instances
            </label>
            <InstanceSelector
              instances={instances}
              selectedIds={selectedInstanceIds}
              onChange={setSelectedInstanceIds}
              placeholder="Select one or more running instances..."
            />
          </div>

          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">
              Command <span className="text-muted-foreground/60">(Ctrl+Enter to run)</span>
            </label>
            <CommandEditor
              value={command}
              onChange={setCommand}
              onSubmit={handleRun}
              placeholder="e.g. ls -la /var/log"
              disabled={isRunning}
            />
          </div>

          {/* Advanced options toggle */}
          <button
            type="button"
            onClick={() => setShowAdvanced((v) => !v)}
            className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Settings2 className="h-3.5 w-3.5" />
            {showAdvanced ? "Hide" : "Show"} advanced options
          </button>

          {showAdvanced && (
            <div className="rounded-md border p-3 space-y-3 bg-muted/20">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="mb-1 block text-xs font-medium text-muted-foreground">
                    Working directory
                  </label>
                  <input
                    type="text"
                    value={workingDir}
                    onChange={(e) => setWorkingDir(e.target.value)}
                    placeholder="/home/user"
                    className="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-xs font-medium text-muted-foreground">
                    Timeout (seconds)
                  </label>
                  <input
                    type="number"
                    value={timeoutMs / 1000}
                    onChange={(e) =>
                      setTimeoutMs(Math.min(300, Math.max(1, Number(e.target.value))) * 1000)
                    }
                    min={1}
                    max={300}
                    className="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                </div>
              </div>

              {/* Environment variables */}
              <div>
                <div className="mb-1 flex items-center justify-between">
                  <label className="text-xs font-medium text-muted-foreground">
                    Environment variables
                  </label>
                  <button
                    type="button"
                    onClick={addEnvEntry}
                    className="text-xs text-primary hover:underline"
                  >
                    + Add
                  </button>
                </div>
                {envEntries.map((entry, i) => (
                  <div key={i} className="flex items-center gap-2 mb-1">
                    <input
                      placeholder="KEY"
                      value={entry.key}
                      onChange={(e) => updateEnvEntry(i, "key", e.target.value)}
                      className="w-1/3 rounded border border-input bg-background px-2 py-1 font-mono text-xs focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                    <span className="text-muted-foreground">=</span>
                    <input
                      placeholder="value"
                      value={entry.value}
                      onChange={(e) => updateEnvEntry(i, "value", e.target.value)}
                      className="flex-1 rounded border border-input bg-background px-2 py-1 font-mono text-xs focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                    <button
                      type="button"
                      onClick={() => removeEnvEntry(i)}
                      className="text-xs text-muted-foreground hover:text-destructive"
                    >
                      ×
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Script tab */}
      {activeTab === "script" && (
        <div className="space-y-3">
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">
              Target Instances
            </label>
            <InstanceSelector
              instances={instances}
              selectedIds={scriptInstanceIds}
              onChange={setScriptInstanceIds}
              placeholder="Select one or more running instances..."
            />
          </div>
          <ScriptUpload
            value={script}
            onChange={(s) => setScript(s)}
            interpreter={interpreter}
            onInterpreterChange={setInterpreter}
            disabled={isRunning}
          />
        </div>
      )}

      {/* Run button (not shown in history tab) */}
      {activeTab !== "history" && (
        <div className="flex items-center justify-between">
          <div className="text-xs text-muted-foreground">
            {currentInstanceIds.length > 0 ? (
              <span>{currentInstanceIds.length} instance(s) targeted</span>
            ) : (
              <span>No instances selected</span>
            )}
          </div>
          <button
            type="button"
            onClick={handleRun}
            disabled={!canRun || isRunning}
            className={cn(
              "flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium",
              "bg-primary text-primary-foreground hover:bg-primary/90",
              "disabled:cursor-not-allowed disabled:opacity-50",
              "focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
            )}
          >
            {isRunning ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Play className="h-4 w-4" />
            )}
            {isRunning ? "Running..." : "Run"}
          </button>
        </div>
      )}

      {/* Error display */}
      {runError && (
        <div className="rounded-md border border-red-500/30 bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-400">
          {runError.message}
        </div>
      )}

      {/* Results */}
      {singleResult && activeTab !== "history" && <CommandOutput execution={singleResult} />}

      {bulkResults && activeTab !== "history" && (
        <OutputAggregator results={bulkResults} instances={instances} />
      )}

      {/* History tab */}
      {activeTab === "history" && <CommandHistory />}
    </div>
  );
}
