import { useEffect, useRef, useCallback } from "react";
import type { Terminal } from "@xterm/xterm";

export type ConnectionStatus = "connecting" | "connected" | "disconnected" | "error";

interface UseTerminalWebSocketOptions {
  sessionId: string | null;
  terminal: Terminal | null;
  onStatusChange: (status: ConnectionStatus) => void;
  onReconnect?: () => void;
  maxReconnectAttempts?: number;
  reconnectDelay?: number;
}

export function useTerminalWebSocket({
  sessionId,
  terminal,
  onStatusChange,
  onReconnect,
  maxReconnectAttempts = 5,
  reconnectDelay = 2000,
}: UseTerminalWebSocketOptions) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectCountRef = useRef(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isUnmountedRef = useRef(false);

  const disconnect = useCallback(() => {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
  }, []);

  const connectRef = useRef<(() => void) | null>(null);

  const connect = useCallback((): void => {
    if (!sessionId || !terminal || isUnmountedRef.current) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${protocol}//${window.location.host}/ws/terminal/${sessionId}`;

    onStatusChange("connecting");

    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";
    wsRef.current = ws;

    ws.onopen = () => {
      if (isUnmountedRef.current) {
        ws.close();
        return;
      }
      reconnectCountRef.current = 0;
      onStatusChange("connected");

      // Send initial terminal size
      const { cols, rows } = terminal;
      ws.send(JSON.stringify({ type: "resize", cols, rows }));
    };

    ws.onmessage = (event) => {
      if (isUnmountedRef.current || !terminal) return;

      if (event.data instanceof ArrayBuffer) {
        terminal.write(new Uint8Array(event.data));
      } else if (typeof event.data === "string") {
        try {
          const msg = JSON.parse(event.data) as { type: string; data?: string };
          if (msg.type === "data") {
            terminal.write(msg.data ?? "");
          }
        } catch {
          terminal.write(event.data);
        }
      }
    };

    ws.onerror = () => {
      if (isUnmountedRef.current) return;
      onStatusChange("error");
    };

    ws.onclose = (event) => {
      if (isUnmountedRef.current) return;

      wsRef.current = null;

      if (event.wasClean) {
        onStatusChange("disconnected");
        return;
      }

      // Attempt reconnection
      if (reconnectCountRef.current < maxReconnectAttempts) {
        reconnectCountRef.current++;
        onStatusChange("connecting");
        reconnectTimerRef.current = setTimeout(() => {
          if (!isUnmountedRef.current) {
            onReconnect?.();
            connectRef.current?.();
          }
        }, reconnectDelay * reconnectCountRef.current);
      } else {
        onStatusChange("disconnected");
      }
    };
  }, [sessionId, terminal, onStatusChange, onReconnect, maxReconnectAttempts, reconnectDelay]);

  connectRef.current = connect;

  // Send data to PTY
  const sendData = useCallback((data: string) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: "data", data }));
    }
  }, []);

  // Send resize event to PTY
  const sendResize = useCallback((cols: number, rows: number) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: "resize", cols, rows }));
    }
  }, []);

  useEffect(() => {
    isUnmountedRef.current = false;

    if (sessionId && terminal) {
      connect();
    }

    return () => {
      isUnmountedRef.current = true;
      disconnect();
    };
  }, [sessionId, terminal, connect, disconnect]);

  return { sendData, sendResize, disconnect };
}
