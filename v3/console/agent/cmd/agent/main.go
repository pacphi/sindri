// Command agent is the Sindri Console instance agent.
//
// It runs on each deployed Sindri instance, providing:
//   - Auto-registration with the Console
//   - Periodic heartbeat pings
//   - System metrics collection and streaming
//   - Interactive PTY sessions over WebSocket
//
// Configuration is entirely via environment variables; see internal/config for details.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/pacphi/sindri/v3/console/agent/internal/config"
	"github.com/pacphi/sindri/v3/console/agent/internal/heartbeat"
	"github.com/pacphi/sindri/v3/console/agent/internal/metrics"
	"github.com/pacphi/sindri/v3/console/agent/internal/registration"
	"github.com/pacphi/sindri/v3/console/agent/internal/terminal"
	agentws "github.com/pacphi/sindri/v3/console/agent/internal/websocket"
	"github.com/pacphi/sindri/v3/console/agent/pkg/protocol"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintf(os.Stderr, "agent: fatal error: %v\n", err)
		os.Exit(1)
	}
}

func run() error {
	cfg, err := config.Load()
	if err != nil {
		return fmt.Errorf("loading config: %w", err)
	}

	logger := newLogger(cfg.LogLevel)
	logger.Info("sindri agent starting",
		"version", cfg.Version,
		"instance_id", cfg.InstanceID,
		"console_url", cfg.ConsoleURL,
		"heartbeat_interval", cfg.HeartbeatInterval,
		"metrics_interval", cfg.MetricsInterval,
	)

	// Root context — cancelled on SIGINT / SIGTERM.
	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer cancel()

	// --- Auto-registration ---
	reg := registration.New(cfg)
	if err := reg.Register(ctx); err != nil {
		// Registration failure is non-fatal: log and continue.
		// The agent can still collect metrics and wait for the Console to come up.
		logger.Warn("initial registration failed; will retry on reconnect", "error", err)
	} else {
		logger.Info("registered with Console")
	}

	// --- WebSocket connection ---
	collector := metrics.NewCollector(cfg.InstanceID)

	var wsClient *agentws.Client
	termMgr := terminal.NewManager(cfg.Shell, nil, logger) // sender set below

	// Build the inbound message handler.
	handler := buildHandler(cfg, termMgr, logger)

	wsClient = agentws.NewClient(cfg.WebSocketURL(), cfg.APIKey, handler, logger)

	// Wire the terminal manager's sender to the WebSocket client.
	// We use a thin adapter so terminal.Manager stays decoupled from the WS package.
	termMgr = terminal.NewManager(cfg.Shell, (*wsSender)(wsClient), logger)

	// Rebuild handler with the properly wired terminal manager.
	handler = buildHandler(cfg, termMgr, logger)
	wsClient = agentws.NewClient(cfg.WebSocketURL(), cfg.APIKey, handler, logger)

	// Run the WebSocket loop in the background.
	go wsClient.Run(ctx)

	// --- Heartbeat ---
	hbMgr := heartbeat.New(cfg.InstanceID, cfg.HeartbeatInterval, (*wsSender)(wsClient), logger)
	go hbMgr.Run(ctx)

	// --- Metrics loop ---
	go runMetricsLoop(ctx, cfg, collector, wsClient, logger)

	// Block until context is cancelled.
	<-ctx.Done()
	logger.Info("agent shutting down")
	termMgr.CloseAll()
	return nil
}

// buildHandler returns a Handler that dispatches inbound Console messages.
func buildHandler(_ *config.Config, termMgr *terminal.Manager, logger *slog.Logger) agentws.Handler {
	return func(env protocol.Envelope) error {
		switch env.Type {
		case protocol.MsgTerminalCreate:
			return handleTerminalCreate(env, termMgr, logger)
		case protocol.MsgTerminalInput:
			return handleTerminalInput(env, termMgr, logger)
		case protocol.MsgTerminalResize:
			return handleTerminalResize(env, termMgr, logger)
		case protocol.MsgTerminalClose:
			termMgr.Close(env.SessionID)
			return nil
		default:
			logger.Debug("unhandled message type", "type", env.Type)
			return nil
		}
	}
}

func handleTerminalCreate(env protocol.Envelope, mgr *terminal.Manager, logger *slog.Logger) error {
	req, err := decodePayload[protocol.TerminalCreatePayload](env.Payload)
	if err != nil {
		return fmt.Errorf("terminal:create payload: %w", err)
	}
	if req.SessionID == "" {
		req.SessionID = env.SessionID
	}
	if req.Cols == 0 {
		req.Cols = 80
	}
	if req.Rows == 0 {
		req.Rows = 24
	}
	logger.Info("creating terminal session", "session_id", req.SessionID)
	return mgr.Create(&req)
}

func handleTerminalInput(env protocol.Envelope, mgr *terminal.Manager, _ *slog.Logger) error {
	req, err := decodePayload[protocol.TerminalInputPayload](env.Payload)
	if err != nil {
		return fmt.Errorf("terminal:input payload: %w", err)
	}
	sid := req.SessionID
	if sid == "" {
		sid = env.SessionID
	}
	return mgr.Write(sid, req.Data)
}

func handleTerminalResize(env protocol.Envelope, mgr *terminal.Manager, _ *slog.Logger) error {
	req, err := decodePayload[protocol.TerminalResizePayload](env.Payload)
	if err != nil {
		return fmt.Errorf("terminal:resize payload: %w", err)
	}
	sid := req.SessionID
	if sid == "" {
		sid = env.SessionID
	}
	return mgr.Resize(sid, req.Cols, req.Rows)
}

// runMetricsLoop collects and sends system metrics at the configured interval.
func runMetricsLoop(
	ctx context.Context,
	cfg *config.Config,
	collector *metrics.Collector,
	sender *agentws.Client,
	logger *slog.Logger,
) {
	ticker := time.NewTicker(cfg.MetricsInterval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			payload, err := collector.Collect(ctx)
			if err != nil {
				logger.Warn("metrics collection failed", "error", err)
				continue
			}
			env := protocol.Envelope{
				Type:    protocol.MsgMetrics,
				Payload: payload,
			}
			if err := sender.Send(env); err != nil {
				logger.Warn("metrics send failed", "error", err)
			} else {
				logger.Debug("metrics sent")
			}
		case <-ctx.Done():
			return
		}
	}
}

// wsSender adapts *agentws.Client to the terminal.OutputSender and heartbeat.Sender interfaces.
type wsSender agentws.Client

func (w *wsSender) Send(env protocol.Envelope) error {
	return (*agentws.Client)(w).Send(env)
}

// decodePayload re-marshals an interface{} payload (originally decoded from JSON)
// into the target type T.
func decodePayload[T any](raw interface{}) (T, error) {
	var zero T
	data, err := json.Marshal(raw)
	if err != nil {
		return zero, err
	}
	var out T
	if err := json.Unmarshal(data, &out); err != nil {
		return zero, err
	}
	return out, nil
}

// newLogger builds a structured slog.Logger for the given level string.
func newLogger(level string) *slog.Logger {
	var l slog.Level
	switch level {
	case "debug":
		l = slog.LevelDebug
	case "warn":
		l = slog.LevelWarn
	case "error":
		l = slog.LevelError
	default:
		l = slog.LevelInfo
	}
	return slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: l}))
}
