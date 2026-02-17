// Package websocket manages the persistent WebSocket connection from the agent to the Console.
package websocket

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"sync"
	"time"

	gorillaws "github.com/gorilla/websocket"
	"github.com/pacphi/sindri/v3/console/agent/pkg/protocol"
)

const (
	writeTimeout      = 10 * time.Second
	pongWait          = 60 * time.Second
	pingInterval      = 50 * time.Second // must be < pongWait
	maxMessageBytes   = 64 * 1024        // 64 KiB
	reconnectBaseWait = 2 * time.Second
	reconnectMaxWait  = 60 * time.Second
)

// Handler is called for each inbound message from the Console.
type Handler func(env protocol.Envelope) error

// Client maintains a resilient WebSocket connection to the Console.
type Client struct {
	url     string
	apiKey  string
	handler Handler
	logger  *slog.Logger

	mu   sync.Mutex
	conn *gorillaws.Conn
}

// NewClient creates a Client. handler receives all inbound Envelopes.
func NewClient(url, apiKey string, handler Handler, logger *slog.Logger) *Client {
	return &Client{
		url:     url,
		apiKey:  apiKey,
		handler: handler,
		logger:  logger,
	}
}

// Run establishes the WebSocket connection and loops until ctx is cancelled.
// It automatically reconnects with exponential back-off on disconnection.
func (c *Client) Run(ctx context.Context) {
	backoff := reconnectBaseWait

	for {
		if err := c.connect(ctx); err != nil {
			if ctx.Err() != nil {
				return // context cancelled — shut down
			}
			c.logger.Warn("WebSocket connection failed", "error", err, "reconnect_in", backoff)
		} else {
			// Connected — run read loop until error.
			c.readLoop(ctx)
			if ctx.Err() != nil {
				return
			}
			c.logger.Info("WebSocket disconnected; reconnecting", "reconnect_in", backoff)
		}

		// Wait before reconnecting.
		select {
		case <-ctx.Done():
			return
		case <-time.After(backoff):
			backoff = min(backoff*2, reconnectMaxWait)
		}
	}
}

// Send serialises an Envelope and writes it to the WebSocket.
// It is safe to call from multiple goroutines.
func (c *Client) Send(env protocol.Envelope) error {
	c.mu.Lock()
	conn := c.conn
	c.mu.Unlock()

	if conn == nil {
		return fmt.Errorf("not connected")
	}

	data, err := json.Marshal(env)
	if err != nil {
		return fmt.Errorf("marshalling envelope: %w", err)
	}

	conn.SetWriteDeadline(time.Now().Add(writeTimeout)) //nolint:errcheck
	return conn.WriteMessage(gorillaws.TextMessage, data)
}

// connect dials the Console and stores the connection.
func (c *Client) connect(ctx context.Context) error {
	dialer := gorillaws.Dialer{
		HandshakeTimeout: 10 * time.Second,
	}
	headers := http.Header{
		"Authorization": {"Bearer " + c.apiKey},
	}

	conn, _, err := dialer.DialContext(ctx, c.url, headers)
	if err != nil {
		return fmt.Errorf("dial %s: %w", c.url, err)
	}

	conn.SetReadLimit(maxMessageBytes)
	conn.SetReadDeadline(time.Now().Add(pongWait)) //nolint:errcheck
	conn.SetPongHandler(func(string) error {
		conn.SetReadDeadline(time.Now().Add(pongWait)) //nolint:errcheck
		return nil
	})

	c.mu.Lock()
	c.conn = conn
	c.mu.Unlock()

	c.logger.Info("WebSocket connected", "url", c.url)
	go c.pingLoop(ctx, conn)
	return nil
}

// readLoop reads inbound messages and dispatches them to the Handler.
func (c *Client) readLoop(ctx context.Context) {
	c.mu.Lock()
	conn := c.conn
	c.mu.Unlock()

	defer func() {
		c.mu.Lock()
		if c.conn == conn {
			c.conn = nil
		}
		c.mu.Unlock()
		conn.Close()
	}()

	for {
		if ctx.Err() != nil {
			return
		}

		_, data, err := conn.ReadMessage()
		if err != nil {
			if gorillaws.IsUnexpectedCloseError(err, gorillaws.CloseGoingAway, gorillaws.CloseNormalClosure) {
				c.logger.Warn("WebSocket read error", "error", err)
			}
			return
		}

		var env protocol.Envelope
		if err := json.Unmarshal(data, &env); err != nil {
			c.logger.Warn("malformed inbound message", "error", err)
			continue
		}

		if err := c.handler(env); err != nil {
			c.logger.Warn("message handler error", "type", env.Type, "error", err)
		}
	}
}

// pingLoop sends periodic WebSocket pings to keep the connection alive.
func (c *Client) pingLoop(ctx context.Context, conn *gorillaws.Conn) {
	ticker := time.NewTicker(pingInterval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			conn.SetWriteDeadline(time.Now().Add(writeTimeout)) //nolint:errcheck
			if err := conn.WriteMessage(gorillaws.PingMessage, nil); err != nil {
				c.logger.Debug("ping failed", "error", err)
				return
			}
		case <-ctx.Done():
			// Send a clean close frame.
			_ = conn.WriteMessage(gorillaws.CloseMessage,
				gorillaws.FormatCloseMessage(gorillaws.CloseNormalClosure, "agent shutdown"))
			return
		}
	}
}

func min(a, b time.Duration) time.Duration {
	if a < b {
		return a
	}
	return b
}
