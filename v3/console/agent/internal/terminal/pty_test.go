package terminal

import (
	"log/slog"
	"os"
	"sync"
	"testing"
	"time"

	"github.com/pacphi/sindri/v3/console/agent/pkg/protocol"
)

type mockSender struct {
	mu        sync.Mutex
	envelopes []protocol.Envelope
	sendErr   error
}

func (m *mockSender) Send(env protocol.Envelope) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.sendErr != nil {
		return m.sendErr
	}
	m.envelopes = append(m.envelopes, env)
	return nil
}

func (m *mockSender) types() []protocol.MessageType {
	m.mu.Lock()
	defer m.mu.Unlock()
	var ts []protocol.MessageType
	for _, e := range m.envelopes {
		ts = append(ts, e.Type)
	}
	return ts
}

func newTestLogger() *slog.Logger {
	return slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError}))
}

func TestManager_CreateAndClose(t *testing.T) {
	sender := &mockSender{}
	mgr := NewManager("/bin/sh", sender, newTestLogger())

	req := &protocol.TerminalCreatePayload{
		SessionID: "sess-1",
		Cols:      80,
		Rows:      24,
	}

	if err := mgr.Create(req); err != nil {
		t.Fatalf("Create() error: %v", err)
	}

	// Session should be registered.
	if mgr.get("sess-1") == nil {
		t.Fatal("expected session sess-1 to be registered")
	}

	mgr.Close("sess-1")

	// Wait briefly for the goroutine to clean up.
	deadline := time.Now().Add(500 * time.Millisecond)
	for time.Now().Before(deadline) {
		if mgr.get("sess-1") == nil {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
}

func TestManager_WriteToNonexistentSession(t *testing.T) {
	sender := &mockSender{}
	mgr := NewManager("/bin/sh", sender, newTestLogger())

	err := mgr.Write("nonexistent", []byte("hello"))
	if err == nil {
		t.Fatal("expected error writing to nonexistent session")
	}
}

func TestManager_ResizeNonexistentSession(t *testing.T) {
	sender := &mockSender{}
	mgr := NewManager("/bin/sh", sender, newTestLogger())

	err := mgr.Resize("nonexistent", 100, 40)
	if err == nil {
		t.Fatal("expected error resizing nonexistent session")
	}
}

func TestManager_Write(t *testing.T) {
	sender := &mockSender{}
	mgr := NewManager("/bin/sh", sender, newTestLogger())

	req := &protocol.TerminalCreatePayload{
		SessionID: "sess-write",
		Cols:      80,
		Rows:      24,
	}
	if err := mgr.Create(req); err != nil {
		t.Fatalf("Create() error: %v", err)
	}

	// Write a newline — should not error.
	if err := mgr.Write("sess-write", []byte("\n")); err != nil {
		t.Errorf("Write() error: %v", err)
	}

	mgr.Close("sess-write")
}

func TestManager_CloseAll(t *testing.T) {
	sender := &mockSender{}
	mgr := NewManager("/bin/sh", sender, newTestLogger())

	for i := 0; i < 3; i++ {
		req := &protocol.TerminalCreatePayload{
			SessionID: string(rune('A' + i)),
			Cols:      80,
			Rows:      24,
		}
		if err := mgr.Create(req); err != nil {
			t.Fatalf("Create() error for session %d: %v", i, err)
		}
	}

	mgr.CloseAll()

	mgr.mu.RLock()
	remaining := len(mgr.sessions)
	mgr.mu.RUnlock()

	if remaining != 0 {
		t.Errorf("expected 0 sessions after CloseAll, got %d", remaining)
	}
}
