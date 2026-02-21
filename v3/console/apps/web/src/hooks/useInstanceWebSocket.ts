import { useEffect, useRef, useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { HeartbeatMessage, InstanceUpdateMessage, WebSocketMessage } from "@/types/instance";

const WS_URL = `${window.location.protocol === "https:" ? "wss:" : "ws:"}//${window.location.host}/ws/instances`;

export function useInstanceWebSocket() {
  const queryClient = useQueryClient();
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const MAX_RECONNECT_ATTEMPTS = 5;
  const RECONNECT_DELAY_MS = 2000;

  const handleMessage = useCallback(
    (event: MessageEvent) => {
      let msg: WebSocketMessage;
      try {
        msg = JSON.parse(event.data as string) as WebSocketMessage;
      } catch {
        return;
      }

      if (msg.type === "instance_update") {
        const update = msg as InstanceUpdateMessage;
        // Update the specific instance in the cache
        queryClient.setQueryData<{
          instances: { id: string; status: string; updated_at: string }[];
        }>(["instances"], (old) => {
          if (!old) return old;
          return {
            ...old,
            instances: old.instances.map((inst) =>
              inst.id === update.payload.instance_id
                ? { ...inst, status: update.payload.status, updated_at: update.payload.updated_at }
                : inst,
            ),
          };
        });
        // Also invalidate the individual instance query
        void queryClient.invalidateQueries({ queryKey: ["instances", update.payload.instance_id] });
      }

      if (msg.type === "heartbeat") {
        const hb = msg as HeartbeatMessage;
        // Update the cached heartbeat for the instance
        queryClient.setQueryData<{ instances: { id: string; latest_heartbeat: unknown }[] }>(
          ["instances"],
          (old) => {
            if (!old) return old;
            return {
              ...old,
              instances: old.instances.map((inst) =>
                inst.id === hb.payload.instance_id
                  ? { ...inst, latest_heartbeat: hb.payload }
                  : inst,
              ),
            };
          },
        );
      }
    },
    [queryClient],
  );

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    const ws = new WebSocket(WS_URL);
    wsRef.current = ws;

    ws.addEventListener("open", () => {
      reconnectAttemptsRef.current = 0;
    });

    ws.addEventListener("message", handleMessage);

    ws.addEventListener("close", () => {
      wsRef.current = null;
      if (reconnectAttemptsRef.current < MAX_RECONNECT_ATTEMPTS) {
        reconnectAttemptsRef.current++;
        reconnectTimerRef.current = setTimeout(() => {
          connect();
        }, RECONNECT_DELAY_MS * reconnectAttemptsRef.current);
      }
    });

    ws.addEventListener("error", () => {
      ws.close();
    });
  }, [handleMessage]);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current);
      }
      wsRef.current?.close();
    };
  }, [connect]);
}
