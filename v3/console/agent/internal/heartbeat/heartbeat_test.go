package heartbeat

import (
	"context"
	"log/slog"
	"os"
	"sync"
	"testing"
	"time"

	"github.com/pacphi/sindri/v3/console/agent/pkg/protocol"
)

// captureSender records envelopes it receives.
type captureSender struct {
	mu        sync.Mutex
	envelopes []protocol.Envelope
}

func (s *captureSender) Send(env protocol.Envelope) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.envelopes = append(s.envelopes, env)
	return nil
}

func (s *captureSender) count() int {
	s.mu.Lock()
	defer s.mu.Unlock()
	return len(s.envelopes)
}

func (s *captureSender) first() protocol.Envelope {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.envelopes[0]
}

func TestManager_SendsImmediately(t *testing.T) {
	sender := &captureSender{}
	logger := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError}))
	mgr := New("test-id", 10*time.Second, sender, logger)

	ctx, cancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer cancel()

	mgr.Run(ctx)

	if sender.count() < 1 {
		t.Fatal("expected at least one heartbeat to be sent")
	}

	env := sender.first()
	if env.Type != protocol.MsgHeartbeat {
		t.Errorf("envelope type = %q, want %q", env.Type, protocol.MsgHeartbeat)
	}

	hb, ok := env.Payload.(protocol.HeartbeatPayload)
	if !ok {
		t.Fatalf("payload type = %T, want HeartbeatPayload", env.Payload)
	}
	if hb.InstanceID != "test-id" {
		t.Errorf("InstanceID = %q, want test-id", hb.InstanceID)
	}
	if hb.Timestamp.IsZero() {
		t.Error("Timestamp must not be zero")
	}
}

func TestManager_TicksAtInterval(t *testing.T) {
	sender := &captureSender{}
	logger := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError}))
	interval := 50 * time.Millisecond
	mgr := New("test-id", interval, sender, logger)

	ctx, cancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer cancel()

	mgr.Run(ctx)

	// In 200ms with a 50ms interval we expect ~4 ticks (plus 1 immediate).
	if sender.count() < 3 {
		t.Errorf("expected >= 3 heartbeats, got %d", sender.count())
	}
}

func TestManager_StopsOnContextCancel(t *testing.T) {
	sender := &captureSender{}
	logger := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError}))
	mgr := New("test-id", 10*time.Second, sender, logger)

	ctx, cancel := context.WithCancel(context.Background())

	done := make(chan struct{})
	go func() {
		mgr.Run(ctx)
		close(done)
	}()

	cancel()

	select {
	case <-done:
		// good
	case <-time.After(500 * time.Millisecond):
		t.Error("Manager.Run did not stop after context cancel")
	}
}
