import { useEffect, useRef, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { getDeploymentWebSocketUrl } from "@/api/deployments";
import type { DeploymentProgressEvent, DeploymentStatus } from "@/types/deployment";

interface ProgressLogEntry {
  timestamp: Date;
  message: string;
  type: "info" | "error" | "success";
}

interface DeploymentProgressProps {
  deploymentId: string;
  onComplete: (instanceId: string) => void;
  onError: (message: string) => void;
  onCancel: () => void;
}

const STATUS_LABELS: Record<DeploymentStatus, string> = {
  PENDING: "Pending",
  IN_PROGRESS: "Deploying",
  SUCCEEDED: "Succeeded",
  FAILED: "Deployment failed",
  CANCELLED: "Cancelled",
};

export function DeploymentProgress({
  deploymentId,
  onComplete,
  onError,
  onCancel,
}: DeploymentProgressProps) {
  const [status, setStatus] = useState<DeploymentStatus>("PENDING");
  const [progress, setProgress] = useState(0);
  const [logs, setLogs] = useState<ProgressLogEntry[]>([]);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  function addLog(message: string, type: ProgressLogEntry["type"] = "info") {
    setLogs((prev) => [...prev, { timestamp: new Date(), message, type }]);
  }

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  useEffect(() => {
    const url = getDeploymentWebSocketUrl(deploymentId);
    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.addEventListener("open", () => {
      setConnected(true);
      addLog("Connected to deployment stream");
    });

    ws.addEventListener("message", (event: MessageEvent) => {
      let data: DeploymentProgressEvent;
      try {
        data = JSON.parse(event.data as string) as DeploymentProgressEvent;
      } catch {
        addLog(`Received: ${event.data as string}`);
        return;
      }

      addLog(data.message, data.type === "error" ? "error" : "info");

      if (data.status) {
        setStatus(data.status);
      }

      if (data.progress_percent !== undefined) {
        setProgress(data.progress_percent);
      }

      if (data.type === "complete" && data.instance_id) {
        addLog("Deployment complete!", "success");
        setProgress(100);
        setStatus("SUCCEEDED");
        onComplete(data.instance_id);
      }

      if (data.type === "error") {
        setStatus("FAILED");
        onError(data.message);
      }
    });

    ws.addEventListener("close", () => {
      setConnected(false);
      addLog("Disconnected from deployment stream");
    });

    ws.addEventListener("error", () => {
      setConnected(false);
      addLog("Connection error", "error");
    });

    return () => {
      ws.close();
    };
  }, [deploymentId]);

  const isFinal = status === "SUCCEEDED" || status === "FAILED" || status === "CANCELLED";

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm">Deployment Status</CardTitle>
            <div className="flex items-center gap-2">
              <span
                className={cn(
                  "inline-block w-2 h-2 rounded-full",
                  connected ? "bg-green-500 animate-pulse" : "bg-muted-foreground",
                )}
              />
              <span className="text-xs text-muted-foreground">
                {connected ? "Live" : "Disconnected"}
              </span>
            </div>
          </div>
        </CardHeader>
        <CardContent className="pt-0 space-y-3">
          <div className="flex items-center justify-between text-sm">
            <span
              className={cn(
                "font-medium",
                status === "SUCCEEDED" && "text-green-600",
                status === "FAILED" && "text-destructive",
                status === "CANCELLED" && "text-muted-foreground",
              )}
            >
              {STATUS_LABELS[status]}
            </span>
            <span className="text-muted-foreground">{progress}%</span>
          </div>

          <div className="w-full bg-muted rounded-full h-2">
            <div
              className={cn(
                "h-2 rounded-full transition-all duration-500",
                status === "FAILED" ? "bg-destructive" : "bg-primary",
              )}
              style={{ width: `${progress}%` }}
            />
          </div>

          <p className="text-xs text-muted-foreground">ID: {deploymentId}</p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm">Deployment Logs</CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          <div className="bg-black rounded-md p-3 h-48 overflow-y-auto font-mono text-xs">
            {logs.length === 0 ? (
              <p className="text-muted-foreground">Waiting for events...</p>
            ) : (
              logs.map((entry, index) => (
                <div key={index} className="flex gap-2 leading-5">
                  <span className="text-muted-foreground shrink-0">
                    {entry.timestamp.toLocaleTimeString()}
                  </span>
                  <span
                    className={cn(
                      entry.type === "error" && "text-red-400",
                      entry.type === "success" && "text-green-400",
                      entry.type === "info" && "text-gray-300",
                    )}
                  >
                    {entry.message}
                  </span>
                </div>
              ))
            )}
            <div ref={logsEndRef} />
          </div>
        </CardContent>
      </Card>

      {!isFinal && (
        <div className="flex justify-end">
          <Button variant="outline" onClick={onCancel}>
            Cancel Deployment
          </Button>
        </div>
      )}
    </div>
  );
}
