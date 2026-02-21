import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { fleetApi } from "@/api/fleet";
import type { FleetWebSocketMessage } from "@/types/fleet";

export function useFleetStats() {
  return useQuery({
    queryKey: ["fleet", "stats"],
    queryFn: () => fleetApi.getStats(),
    staleTime: 15_000,
    refetchInterval: 30_000,
  });
}

export function useFleetGeo() {
  return useQuery({
    queryKey: ["fleet", "geo"],
    queryFn: () => fleetApi.getGeo(),
    staleTime: 60_000,
    refetchInterval: 60_000,
  });
}

export function useFleetDeployments() {
  return useQuery({
    queryKey: ["fleet", "deployments"],
    queryFn: () => fleetApi.getDeployments(),
    staleTime: 60_000,
    refetchInterval: 60_000,
  });
}

export function useFleetWebSocket() {
  const queryClient = useQueryClient();
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const attemptsRef = useRef(0);
  const MAX_ATTEMPTS = 5;

  useEffect(() => {
    function connect() {
      const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
      const ws = new WebSocket(`${protocol}//${window.location.host}/ws/fleet`);
      wsRef.current = ws;

      ws.addEventListener("message", (event) => {
        let msg: FleetWebSocketMessage;
        try {
          msg = JSON.parse(event.data as string) as FleetWebSocketMessage;
        } catch {
          return;
        }
        if (msg.type === "fleet_stats") {
          queryClient.setQueryData(["fleet", "stats"], msg.payload);
        }
        if (msg.type === "session_count") {
          queryClient.setQueryData(
            ["fleet", "stats"],
            (old: Record<string, unknown> | undefined) => {
              if (!old) return old;
              return {
                ...old,
                active_sessions: (msg.payload as { active_sessions: number }).active_sessions,
              };
            },
          );
        }
      });

      ws.addEventListener("open", () => {
        attemptsRef.current = 0;
      });

      ws.addEventListener("close", () => {
        wsRef.current = null;
        if (attemptsRef.current < MAX_ATTEMPTS) {
          attemptsRef.current++;
          reconnectTimerRef.current = setTimeout(connect, 2000 * attemptsRef.current);
        }
      });

      ws.addEventListener("error", () => ws.close());
    }

    connect();

    return () => {
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      wsRef.current?.close();
    };
  }, [queryClient]);
}
