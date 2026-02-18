// Package protocol defines the WebSocket message types and structures
// used for communication between the Sindri agent and Console.
package protocol

import "time"

// MessageType identifies the kind of WebSocket message.
type MessageType string

const (
	// Inbound (Console → Agent)
	MsgTerminalCreate  MessageType = "terminal:create"
	MsgTerminalClose   MessageType = "terminal:close"
	MsgTerminalInput   MessageType = "terminal:input"
	MsgTerminalResize  MessageType = "terminal:resize"
	MsgCommandDispatch MessageType = "command:dispatch"

	// Outbound (Agent → Console)
	MsgHeartbeat      MessageType = "heartbeat"
	MsgMetrics        MessageType = "metrics"
	MsgTerminalOutput MessageType = "terminal:output"
	MsgTerminalClosed MessageType = "terminal:closed"
	MsgCommandResult  MessageType = "command:result"
	MsgEvent          MessageType = "event"
	MsgRegistration   MessageType = "registration"
)

// Envelope wraps every WebSocket message with routing metadata.
type Envelope struct {
	Type      MessageType `json:"type"`
	SessionID string      `json:"session_id,omitempty"`
	Payload   interface{} `json:"payload"`
}

// HeartbeatPayload is sent every HeartbeatInterval to signal the agent is alive.
type HeartbeatPayload struct {
	InstanceID string    `json:"instance_id"`
	Timestamp  time.Time `json:"timestamp"`
	Uptime     int64     `json:"uptime_seconds"`
}

// MetricsPayload carries a snapshot of system resource usage.
type MetricsPayload struct {
	InstanceID string         `json:"instance_id"`
	Timestamp  time.Time      `json:"timestamp"`
	CPU        CPUMetrics     `json:"cpu"`
	Memory     MemoryMetrics  `json:"memory"`
	Disk       []DiskMetrics  `json:"disk"`
	Network    NetworkMetrics `json:"network"`
}

// CPUMetrics holds CPU usage statistics.
type CPUMetrics struct {
	UsagePercent float64   `json:"usage_percent"`
	LoadAvg1     float64   `json:"load_avg_1"`
	LoadAvg5     float64   `json:"load_avg_5"`
	LoadAvg15    float64   `json:"load_avg_15"`
	CoreCount    int       `json:"core_count"`
	PerCore      []float64 `json:"per_core,omitempty"`
}

// MemoryMetrics holds memory usage statistics.
type MemoryMetrics struct {
	TotalBytes     uint64  `json:"total_bytes"`
	UsedBytes      uint64  `json:"used_bytes"`
	FreeBytes      uint64  `json:"free_bytes"`
	CachedBytes    uint64  `json:"cached_bytes"`
	UsagePercent   float64 `json:"usage_percent"`
	SwapTotalBytes uint64  `json:"swap_total_bytes"`
	SwapUsedBytes  uint64  `json:"swap_used_bytes"`
}

// DiskMetrics holds disk usage for a single mount point.
type DiskMetrics struct {
	MountPoint   string  `json:"mount_point"`
	Device       string  `json:"device"`
	FSType       string  `json:"fs_type"`
	TotalBytes   uint64  `json:"total_bytes"`
	UsedBytes    uint64  `json:"used_bytes"`
	FreeBytes    uint64  `json:"free_bytes"`
	UsagePercent float64 `json:"usage_percent"`
}

// NetworkMetrics aggregates network I/O counters.
type NetworkMetrics struct {
	BytesSent   uint64 `json:"bytes_sent"`
	BytesRecv   uint64 `json:"bytes_recv"`
	PacketsSent uint64 `json:"packets_sent"`
	PacketsRecv uint64 `json:"packets_recv"`
}

// RegistrationPayload is POSTed to the Console on agent boot.
type RegistrationPayload struct {
	InstanceID   string            `json:"instance_id"`
	Hostname     string            `json:"hostname"`
	Provider     string            `json:"provider"`
	Region       string            `json:"region"`
	AgentVersion string            `json:"agent_version"`
	OS           string            `json:"os"`
	Arch         string            `json:"arch"`
	Tags         map[string]string `json:"tags,omitempty"`
	Timestamp    time.Time         `json:"timestamp"`
}

// TerminalCreatePayload requests PTY allocation.
type TerminalCreatePayload struct {
	SessionID string `json:"session_id"`
	Cols      uint16 `json:"cols"`
	Rows      uint16 `json:"rows"`
	Shell     string `json:"shell,omitempty"` // defaults to $SHELL or /bin/bash
}

// TerminalInputPayload carries keystrokes from the browser.
type TerminalInputPayload struct {
	SessionID string `json:"session_id"`
	Data      []byte `json:"data"`
}

// TerminalResizePayload changes PTY dimensions.
type TerminalResizePayload struct {
	SessionID string `json:"session_id"`
	Cols      uint16 `json:"cols"`
	Rows      uint16 `json:"rows"`
}

// TerminalOutputPayload carries PTY output bytes.
type TerminalOutputPayload struct {
	SessionID string `json:"session_id"`
	Data      []byte `json:"data"`
}

// TerminalClosedPayload is sent when a PTY session ends.
type TerminalClosedPayload struct {
	SessionID string `json:"session_id"`
	ExitCode  int    `json:"exit_code"`
	Reason    string `json:"reason,omitempty"`
}

// CommandDispatchPayload requests one-off command execution.
type CommandDispatchPayload struct {
	CommandID string   `json:"command_id"`
	Command   string   `json:"command"`
	Args      []string `json:"args,omitempty"`
	Env       []string `json:"env,omitempty"`
	TimeoutMs int64    `json:"timeout_ms,omitempty"`
}

// CommandResultPayload returns stdout/stderr and exit code.
type CommandResultPayload struct {
	CommandID  string `json:"command_id"`
	Stdout     string `json:"stdout"`
	Stderr     string `json:"stderr"`
	ExitCode   int    `json:"exit_code"`
	DurationMs int64  `json:"duration_ms"`
}

// EventPayload carries lifecycle events from the agent.
type EventPayload struct {
	InstanceID string            `json:"instance_id"`
	EventType  string            `json:"event_type"`
	Message    string            `json:"message,omitempty"`
	Metadata   map[string]string `json:"metadata,omitempty"`
	Timestamp  time.Time         `json:"timestamp"`
}
