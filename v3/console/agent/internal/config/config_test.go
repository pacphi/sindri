package config

import (
	"os"
	"testing"
	"time"
)

func TestLoad_RequiredEnvVars(t *testing.T) {
	clearEnv()

	_, err := Load()
	if err == nil {
		t.Fatal("expected error when SINDRI_CONSOLE_URL is missing")
	}

	t.Setenv("SINDRI_CONSOLE_URL", "http://console.test")
	_, err = Load()
	if err == nil {
		t.Fatal("expected error when SINDRI_CONSOLE_API_KEY is missing")
	}

	t.Setenv("SINDRI_CONSOLE_API_KEY", "test-key")
	cfg, err := Load()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.ConsoleURL != "http://console.test" {
		t.Errorf("ConsoleURL = %q, want %q", cfg.ConsoleURL, "http://console.test")
	}
}

func TestLoad_Defaults(t *testing.T) {
	clearEnv()
	setRequired(t)

	cfg, err := Load()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.HeartbeatInterval != 30*time.Second {
		t.Errorf("HeartbeatInterval = %v, want 30s", cfg.HeartbeatInterval)
	}
	if cfg.MetricsInterval != 60*time.Second {
		t.Errorf("MetricsInterval = %v, want 60s", cfg.MetricsInterval)
	}
	if cfg.Shell != "/bin/bash" {
		t.Errorf("Shell = %q, want /bin/bash", cfg.Shell)
	}
	if cfg.LogLevel != "info" {
		t.Errorf("LogLevel = %q, want info", cfg.LogLevel)
	}
	if cfg.Version == "" {
		t.Error("Version must not be empty")
	}
}

func TestLoad_Intervals(t *testing.T) {
	clearEnv()
	setRequired(t)

	t.Setenv("SINDRI_AGENT_HEARTBEAT", "15")
	t.Setenv("SINDRI_AGENT_METRICS", "120")

	cfg, err := Load()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.HeartbeatInterval != 15*time.Second {
		t.Errorf("HeartbeatInterval = %v, want 15s", cfg.HeartbeatInterval)
	}
	if cfg.MetricsInterval != 120*time.Second {
		t.Errorf("MetricsInterval = %v, want 120s", cfg.MetricsInterval)
	}
}

func TestLoad_InvalidInterval(t *testing.T) {
	clearEnv()
	setRequired(t)

	t.Setenv("SINDRI_AGENT_HEARTBEAT", "not-a-number")
	_, err := Load()
	if err == nil {
		t.Fatal("expected error for invalid heartbeat interval")
	}
}

func TestLoad_Tags(t *testing.T) {
	clearEnv()
	setRequired(t)

	t.Setenv("SINDRI_AGENT_TAGS", "env=production, team=platform")

	cfg, err := Load()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.Tags["env"] != "production" {
		t.Errorf("Tags[env] = %q, want production", cfg.Tags["env"])
	}
	if cfg.Tags["team"] != "platform" {
		t.Errorf("Tags[team] = %q, want platform", cfg.Tags["team"])
	}
}

func TestLoad_TrailingSlashStripped(t *testing.T) {
	clearEnv()
	t.Setenv("SINDRI_CONSOLE_URL", "https://console.example.com/")
	t.Setenv("SINDRI_CONSOLE_API_KEY", "key")

	cfg, err := Load()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.ConsoleURL != "https://console.example.com" {
		t.Errorf("ConsoleURL = %q, trailing slash not stripped", cfg.ConsoleURL)
	}
}

func TestWebSocketURL(t *testing.T) {
	tests := []struct {
		consoleURL string
		wantWS     string
	}{
		{"https://console.example.com", "wss://console.example.com/ws/agent"},
		{"http://localhost:3000", "ws://localhost:3000/ws/agent"},
	}
	for _, tt := range tests {
		cfg := &Config{ConsoleURL: tt.consoleURL}
		got := cfg.WebSocketURL()
		if got != tt.wantWS {
			t.Errorf("WebSocketURL(%q) = %q, want %q", tt.consoleURL, got, tt.wantWS)
		}
	}
}

func TestRegistrationURL(t *testing.T) {
	cfg := &Config{ConsoleURL: "https://console.example.com"}
	want := "https://console.example.com/api/v1/instances"
	if got := cfg.RegistrationURL(); got != want {
		t.Errorf("RegistrationURL() = %q, want %q", got, want)
	}
}

// clearEnv removes all SINDRI_ environment variables used by the agent.
func clearEnv() {
	vars := []string{
		"SINDRI_CONSOLE_URL",
		"SINDRI_CONSOLE_API_KEY",
		"SINDRI_INSTANCE_ID",
		"SINDRI_PROVIDER",
		"SINDRI_REGION",
		"SINDRI_AGENT_SHELL",
		"SINDRI_LOG_LEVEL",
		"SINDRI_AGENT_HEARTBEAT",
		"SINDRI_AGENT_METRICS",
		"SINDRI_AGENT_TAGS",
	}
	for _, v := range vars {
		_ = os.Unsetenv(v)
	}
}

func setRequired(t *testing.T) {
	t.Helper()
	t.Setenv("SINDRI_CONSOLE_URL", "https://console.test")
	t.Setenv("SINDRI_CONSOLE_API_KEY", "test-api-key")
}
