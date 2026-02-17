import { useEffect, useRef, useCallback, useState } from "react";
import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { useTerminalWebSocket, type ConnectionStatus } from "@/hooks/useTerminalWebSocket";
import { darkTheme, lightTheme } from "@/lib/terminal-themes";
import "@xterm/xterm/css/xterm.css";

interface TerminalProps {
  sessionId: string | null;
  instanceId: string;
  theme?: "dark" | "light";
  onStatusChange?: (status: ConnectionStatus) => void;
  className?: string;
}

export function Terminal({
  sessionId,
  instanceId: _instanceId,
  theme = "dark",
  onStatusChange,
  className,
}: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  // Track xterm readiness as state so the WebSocket hook re-runs when it's mounted
  const [xtermInstance, setXtermInstance] = useState<XTerm | null>(null);
  const [status, setStatus] = useState<ConnectionStatus>("disconnected");

  const handleStatusChange = useCallback(
    (newStatus: ConnectionStatus) => {
      setStatus(newStatus);
      onStatusChange?.(newStatus);
    },
    [onStatusChange]
  );

  // Initialize xterm.js once on mount
  useEffect(() => {
    if (!containerRef.current) return;

    const termTheme = theme === "dark" ? darkTheme : lightTheme;

    const xterm = new XTerm({
      theme: termTheme,
      fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", "Consolas", monospace',
      fontSize: 14,
      lineHeight: 1.2,
      cursorBlink: true,
      cursorStyle: "block",
      scrollback: 10000,
      allowTransparency: false,
      macOptionIsMeta: true,
      rightClickSelectsWord: true,
    });

    const fitAddon = new FitAddon();
    const searchAddon = new SearchAddon();
    const webLinksAddon = new WebLinksAddon();

    xterm.loadAddon(fitAddon);
    xterm.loadAddon(searchAddon);
    xterm.loadAddon(webLinksAddon);

    xterm.open(containerRef.current);

    // Try WebGL renderer, fall back to canvas
    try {
      const webglAddon = new WebglAddon();
      webglAddon.onContextLoss(() => {
        webglAddon.dispose();
      });
      xterm.loadAddon(webglAddon);
    } catch {
      // WebGL not available; canvas renderer is used as fallback
    }

    fitAddon.fit();

    xtermRef.current = xterm;
    fitAddonRef.current = fitAddon;
    setXtermInstance(xterm);

    return () => {
      xterm.dispose();
      xtermRef.current = null;
      fitAddonRef.current = null;
      setXtermInstance(null);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Only run once on mount; theme is handled separately

  // Update theme when it changes without recreating the terminal
  useEffect(() => {
    if (!xtermRef.current) return;
    const termTheme = theme === "dark" ? darkTheme : lightTheme;
    xtermRef.current.options.theme = termTheme;
  }, [theme]);

  // Handle container resize
  useEffect(() => {
    const handleResize = () => {
      fitAddonRef.current?.fit();
    };

    const resizeObserver = new ResizeObserver(handleResize);
    if (containerRef.current) {
      resizeObserver.observe(containerRef.current);
    }
    window.addEventListener("resize", handleResize);

    return () => {
      resizeObserver.disconnect();
      window.removeEventListener("resize", handleResize);
    };
  }, []);

  // Connect WebSocket once both sessionId and the xterm instance are ready
  const { sendData, sendResize } = useTerminalWebSocket({
    sessionId,
    terminal: xtermInstance,
    onStatusChange: handleStatusChange,
    onReconnect: () => {
      xtermRef.current?.write("\r\n\x1b[33mReconnecting...\x1b[0m\r\n");
    },
  });

  // Wire xterm input events to WebSocket output
  useEffect(() => {
    if (!xtermInstance) return;

    const dataDisposable = xtermInstance.onData((data) => {
      sendData(data);
    });

    const resizeDisposable = xtermInstance.onResize(({ cols, rows }) => {
      sendResize(cols, rows);
    });

    return () => {
      dataDisposable.dispose();
      resizeDisposable.dispose();
    };
  }, [xtermInstance, sendData, sendResize]);

  // Show status overlay when not connected and a session was requested
  const showOverlay = status !== "connected" && sessionId !== null;

  return (
    <div className={`relative flex flex-col h-full bg-[#0d1117] ${className ?? ""}`}>
      {showOverlay && (
        <div className="absolute inset-0 z-10 flex items-center justify-center bg-black/60">
          <div className="flex flex-col items-center gap-3 text-sm text-white">
            {status === "connecting" && (
              <>
                <div className="h-5 w-5 animate-spin rounded-full border-2 border-white border-t-transparent" />
                <span>Connecting to terminal...</span>
              </>
            )}
            {status === "error" && <span className="text-red-400">Connection error. Retrying...</span>}
            {status === "disconnected" && <span className="text-gray-400">Disconnected</span>}
          </div>
        </div>
      )}
      <div ref={containerRef} className="flex-1 overflow-hidden p-2" style={{ minHeight: 0 }} />
    </div>
  );
}
