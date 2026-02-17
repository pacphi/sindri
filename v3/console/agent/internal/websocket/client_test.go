package websocket

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"sync"
	"testing"
	"time"

	gorillaws "github.com/gorilla/websocket"
	"github.com/pacphi/sindri/v3/console/agent/pkg/protocol"
)

var upgrader = gorillaws.Upgrader{CheckOrigin: func(r *http.Request) bool { return true }}

// echoServer upgrades HTTP to WebSocket and echoes every message back.
func echoServer(t *testing.T, received *[]protocol.Envelope, mu *sync.Mutex) *httptest.Server {
	t.Helper()
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			t.Logf("upgrade error: %v", err)
			return
		}
		defer func() { _ = conn.Close() }()
		for {
			_, data, err := conn.ReadMessage()
			if err != nil {
				return
			}
			var env protocol.Envelope
			if json.Unmarshal(data, &env) == nil {
				mu.Lock()
				*received = append(*received, env)
				mu.Unlock()
			}
		}
	}))
}

func wsURL(srv *httptest.Server) string {
	return strings.Replace(srv.URL, "http://", "ws://", 1)
}

func newTestLogger() *slog.Logger {
	return slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError}))
}

func TestClient_ConnectsAndSends(t *testing.T) {
	var received []protocol.Envelope
	var mu sync.Mutex

	srv := echoServer(t, &received, &mu)
	defer srv.Close()

	handler := func(env protocol.Envelope) error { return nil }
	client := NewClient(wsURL(srv), "test-key", handler, newTestLogger())

	ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
	defer cancel()

	go client.Run(ctx)

	// Wait for connection to establish.
	time.Sleep(100 * time.Millisecond)

	env := protocol.Envelope{
		Type:    protocol.MsgHeartbeat,
		Payload: map[string]string{"test": "value"},
	}
	if err := client.Send(env); err != nil {
		t.Fatalf("Send() error: %v", err)
	}

	// Wait for server to receive.
	time.Sleep(100 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	if len(received) == 0 {
		t.Fatal("server received no messages")
	}
	if received[0].Type != protocol.MsgHeartbeat {
		t.Errorf("received type = %q, want %q", received[0].Type, protocol.MsgHeartbeat)
	}
}

func TestClient_SendBeforeConnect(t *testing.T) {
	handler := func(env protocol.Envelope) error { return nil }
	client := NewClient("ws://localhost:0", "key", handler, newTestLogger())

	err := client.Send(protocol.Envelope{Type: protocol.MsgHeartbeat})
	if err == nil {
		t.Fatal("expected error when sending before connection is established")
	}
}

func TestClient_HandlerCalledForInbound(t *testing.T) {
	var handledTypes []protocol.MessageType
	var mu sync.Mutex

	// Server sends a message right after the client connects.
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			return
		}
		defer func() { _ = conn.Close() }()

		msg, _ := json.Marshal(protocol.Envelope{Type: protocol.MsgCommandDispatch})
		conn.WriteMessage(gorillaws.TextMessage, msg) //nolint:errcheck

		// Keep alive briefly.
		time.Sleep(300 * time.Millisecond)
	}))
	defer srv.Close()

	handler := func(env protocol.Envelope) error {
		mu.Lock()
		handledTypes = append(handledTypes, env.Type)
		mu.Unlock()
		return nil
	}
	client := NewClient(wsURL(srv), "key", handler, newTestLogger())

	ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
	defer cancel()

	go client.Run(ctx)
	time.Sleep(400 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	if len(handledTypes) == 0 {
		t.Fatal("handler was never called for inbound message")
	}
	if handledTypes[0] != protocol.MsgCommandDispatch {
		t.Errorf("handled type = %q, want %q", handledTypes[0], protocol.MsgCommandDispatch)
	}
}

func TestClient_ReconnectsAfterDisconnect(t *testing.T) {
	connections := 0
	var mu sync.Mutex

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		connections++
		mu.Unlock()
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			return
		}
		// Close immediately to force a reconnect.
		_ = conn.Close()
	}))
	defer srv.Close()

	handler := func(env protocol.Envelope) error { return nil }
	client := NewClient(wsURL(srv), "key", handler, newTestLogger())

	// The reconnect base wait is 2s, so we need to run long enough to see
	// at least two connections: one initial + one after the first reconnect delay.
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	go client.Run(ctx)

	// Poll until we have at least 2 connections or the context expires.
	deadline := time.Now().Add(4500 * time.Millisecond)
	for time.Now().Before(deadline) {
		mu.Lock()
		c := connections
		mu.Unlock()
		if c >= 2 {
			return // success
		}
		time.Sleep(100 * time.Millisecond)
	}

	mu.Lock()
	c := connections
	mu.Unlock()
	if c < 2 {
		t.Errorf("expected >= 2 connection attempts (to verify reconnect), got %d", c)
	}
}
