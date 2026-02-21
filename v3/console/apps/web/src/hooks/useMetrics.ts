import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useCallback, useState } from "react";
import { metricsApi } from "@/lib/metricsApi";
import type { TimeRange, MetricsStreamMessage, MetricsDataPoint } from "@/types/metrics";

export function useInstanceExtensions(instanceId: string) {
  return useQuery({
    queryKey: ["metrics", "extensions", instanceId],
    queryFn: () => metricsApi.extensions(instanceId),
    staleTime: 30_000,
    refetchInterval: 60_000,
    enabled: Boolean(instanceId),
  });
}

export function useInstanceEvents(instanceId: string, limit = 50) {
  return useQuery({
    queryKey: ["metrics", "events", instanceId, limit],
    queryFn: () => metricsApi.events(instanceId, limit),
    staleTime: 15_000,
    refetchInterval: 30_000,
    enabled: Boolean(instanceId),
  });
}

const MAX_REALTIME_POINTS = 300; // ~5 min at 1s intervals

export function useMetricsTimeSeries(instanceId: string, range: TimeRange) {
  return useQuery({
    queryKey: ["metrics", "timeseries", instanceId, range],
    queryFn: () => metricsApi.timeseries(instanceId, range),
    staleTime: 30_000,
    refetchInterval: 60_000,
    enabled: Boolean(instanceId),
  });
}

export function useProcessList(instanceId: string) {
  return useQuery({
    queryKey: ["metrics", "processes", instanceId],
    queryFn: () => metricsApi.processes(instanceId),
    staleTime: 5_000,
    refetchInterval: 10_000,
    enabled: Boolean(instanceId),
  });
}

/**
 * Connects to the metrics WebSocket stream and appends real-time data points
 * to a local ring buffer. Returns the latest N data points per metric.
 */
export function useMetricsStream(instanceId: string) {
  const [realtimePoints, setRealtimePoints] = useState<{
    cpu: MetricsDataPoint[];
    memory: MetricsDataPoint[];
    disk: MetricsDataPoint[];
    network_in: MetricsDataPoint[];
    network_out: MetricsDataPoint[];
  }>({
    cpu: [],
    memory: [],
    disk: [],
    network_in: [],
    network_out: [],
  });

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const MAX_RECONNECT = 5;

  function appendPoint<K extends keyof typeof realtimePoints>(
    prev: typeof realtimePoints,
    key: K,
    point: MetricsDataPoint,
  ): typeof realtimePoints {
    const series = [...prev[key], point];
    if (series.length > MAX_REALTIME_POINTS) series.splice(0, series.length - MAX_REALTIME_POINTS);
    return { ...prev, [key]: series };
  }

  const handleMessage = useCallback(
    (event: MessageEvent) => {
      let msg: MetricsStreamMessage;
      try {
        msg = JSON.parse(event.data as string) as MetricsStreamMessage;
      } catch {
        return;
      }
      if (msg.type !== "metrics:snapshot" || msg.instance_id !== instanceId) return;

      const ts = msg.ts;
      const timestamp = new Date(ts).toISOString();
      const memPct = msg.memory_total > 0 ? (msg.memory_used / msg.memory_total) * 100 : 0;
      const diskPct = msg.disk_total > 0 ? (msg.disk_used / msg.disk_total) * 100 : 0;

      setRealtimePoints((prev) => {
        let next = appendPoint(prev, "cpu", { ts, timestamp, value: msg.cpu_percent });
        next = appendPoint(next, "memory", { ts, timestamp, value: memPct });
        next = appendPoint(next, "disk", { ts, timestamp, value: diskPct });
        next = appendPoint(next, "network_in", { ts, timestamp, value: msg.network_bytes_in });
        next = appendPoint(next, "network_out", { ts, timestamp, value: msg.network_bytes_out });
        return next;
      });
    },
    [instanceId],
  );

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const ws = new WebSocket(
      `${proto}//${window.location.host}/ws/metrics/stream?instanceId=${instanceId}`,
    );
    wsRef.current = ws;

    ws.addEventListener("open", () => {
      reconnectAttemptsRef.current = 0;
    });
    ws.addEventListener("message", handleMessage);
    ws.addEventListener("close", () => {
      wsRef.current = null;
      if (reconnectAttemptsRef.current < MAX_RECONNECT) {
        reconnectAttemptsRef.current++;
        reconnectTimerRef.current = setTimeout(connect, 2000 * reconnectAttemptsRef.current);
      }
    });
    ws.addEventListener("error", () => ws.close());
  }, [instanceId, handleMessage]);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      wsRef.current?.close();
    };
  }, [connect]);

  return realtimePoints;
}
