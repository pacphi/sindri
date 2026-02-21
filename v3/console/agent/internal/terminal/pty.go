// Package terminal manages PTY sessions for web shell access.
package terminal

import (
	"fmt"
	"io"
	"log/slog"
	"os"
	"os/exec"
	"sync"

	"github.com/creack/pty"
	"github.com/pacphi/sindri/v3/console/agent/pkg/protocol"
)

// OutputSender pushes PTY output bytes back to the Console.
type OutputSender interface {
	Send(env protocol.Envelope) error
}

// Session represents a single active PTY session.
type Session struct {
	id     string
	ptmx   *os.File
	cmd    *exec.Cmd
	done   chan struct{}
	once   sync.Once
	logger *slog.Logger
}

// Manager tracks active PTY sessions keyed by session ID.
type Manager struct {
	mu       sync.RWMutex
	sessions map[string]*Session
	sender   OutputSender
	shell    string
	logger   *slog.Logger
}

// NewManager creates a Manager. shell is the default shell (e.g. /bin/bash).
func NewManager(shell string, sender OutputSender, logger *slog.Logger) *Manager {
	return &Manager{
		sessions: make(map[string]*Session),
		sender:   sender,
		shell:    shell,
		logger:   logger,
	}
}

// Create spawns a new PTY for the given session and starts streaming output.
// If req.Shell is empty, the manager's default shell is used.
func (m *Manager) Create(req *protocol.TerminalCreatePayload) error {
	shell := req.Shell
	if shell == "" {
		shell = m.shell
	}
	if shell == "" {
		shell = os.Getenv("SHELL")
	}
	if shell == "" {
		shell = "/bin/bash"
	}

	cmd := exec.Command(shell)
	cmd.Env = append(os.Environ(), "TERM=xterm-256color")

	ptmx, err := pty.Start(cmd)
	if err != nil {
		return fmt.Errorf("starting PTY: %w", err)
	}

	if err := pty.Setsize(ptmx, &pty.Winsize{
		Rows: req.Rows,
		Cols: req.Cols,
	}); err != nil {
		// Non-fatal — default terminal size will be used.
		m.logger.Warn("PTY setsize failed", "session_id", req.SessionID, "error", err)
	}

	sess := &Session{
		id:     req.SessionID,
		ptmx:   ptmx,
		cmd:    cmd,
		done:   make(chan struct{}),
		logger: m.logger,
	}

	m.mu.Lock()
	m.sessions[req.SessionID] = sess
	m.mu.Unlock()

	go m.stream(sess)
	return nil
}

// Write sends keystrokes to the PTY of the named session.
func (m *Manager) Write(sessionID string, data []byte) error {
	sess := m.get(sessionID)
	if sess == nil {
		return fmt.Errorf("session %q not found", sessionID)
	}
	_, err := sess.ptmx.Write(data)
	return err
}

// Resize updates the PTY window dimensions.
func (m *Manager) Resize(sessionID string, cols, rows uint16) error {
	sess := m.get(sessionID)
	if sess == nil {
		return fmt.Errorf("session %q not found", sessionID)
	}
	return pty.Setsize(sess.ptmx, &pty.Winsize{Rows: rows, Cols: cols})
}

// Close terminates a PTY session.
func (m *Manager) Close(sessionID string) {
	m.mu.Lock()
	sess, ok := m.sessions[sessionID]
	if ok {
		delete(m.sessions, sessionID)
	}
	m.mu.Unlock()

	if ok {
		sess.close(0)
	}
}

// CloseAll terminates every active session; called on agent shutdown.
func (m *Manager) CloseAll() {
	m.mu.Lock()
	sessions := make([]*Session, 0, len(m.sessions))
	for _, s := range m.sessions {
		sessions = append(sessions, s)
	}
	m.sessions = make(map[string]*Session)
	m.mu.Unlock()

	for _, s := range sessions {
		s.close(0)
	}
}

// stream copies PTY output to the WebSocket channel until the process exits.
func (m *Manager) stream(sess *Session) {
	buf := make([]byte, 4096)
	for {
		n, err := sess.ptmx.Read(buf)
		if n > 0 {
			data := make([]byte, n)
			copy(data, buf[:n])
			env := protocol.Envelope{
				Type:      protocol.MsgTerminalOutput,
				SessionID: sess.id,
				Payload: protocol.TerminalOutputPayload{
					SessionID: sess.id,
					Data:      data,
				},
			}
			if sendErr := m.sender.Send(env); sendErr != nil {
				m.logger.Warn("terminal output send failed", "session_id", sess.id, "error", sendErr)
			}
		}
		if err != nil {
			if err != io.EOF {
				m.logger.Debug("PTY read finished", "session_id", sess.id, "error", err)
			}
			break
		}
	}

	// Wait for the child to exit and collect the exit code.
	exitCode := 0
	if waitErr := sess.cmd.Wait(); waitErr != nil {
		if exitErr, ok := waitErr.(*exec.ExitError); ok {
			exitCode = exitErr.ExitCode()
		}
	}

	// Remove session from the registry.
	m.mu.Lock()
	delete(m.sessions, sess.id)
	m.mu.Unlock()

	// Notify Console that the session has ended.
	env := protocol.Envelope{
		Type:      protocol.MsgTerminalClosed,
		SessionID: sess.id,
		Payload: protocol.TerminalClosedPayload{
			SessionID: sess.id,
			ExitCode:  exitCode,
		},
	}
	if err := m.sender.Send(env); err != nil {
		m.logger.Warn("terminal closed send failed", "session_id", sess.id, "error", err)
	}

	sess.close(exitCode)
}

func (m *Manager) get(sessionID string) *Session {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.sessions[sessionID]
}

func (s *Session) close(exitCode int) {
	s.once.Do(func() {
		_ = exitCode // available for future use
		_ = s.ptmx.Close()
		// Kill the process if it's still running.
		if s.cmd.Process != nil {
			_ = s.cmd.Process.Kill()
		}
		close(s.done)
	})
}
