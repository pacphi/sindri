// Package config loads and validates agent configuration from environment variables.
package config

import (
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"
)

const (
	defaultHeartbeatInterval = 30 * time.Second
	defaultMetricsInterval   = 60 * time.Second
	defaultShell             = "/bin/bash"
	defaultLogLevel          = "info"
	agentVersion             = "0.1.0"
)

// Config holds all runtime configuration for the agent.
type Config struct {
	// ConsoleURL is the base URL of the Sindri Console (e.g. https://console.example.com).
	ConsoleURL string

	// APIKey is the shared secret used to authenticate with the Console.
	APIKey string

	// InstanceID uniquely identifies this Sindri instance.
	// Defaults to the system hostname when not set.
	InstanceID string

	// Provider describes which deployment provider hosts this instance (fly, docker, k8s, e2b, devpod).
	Provider string

	// Region is the geographic region of the instance (e.g. sea, iad, us-east-1).
	Region string

	// HeartbeatInterval controls how often the agent pings the Console.
	HeartbeatInterval time.Duration

	// MetricsInterval controls how often system metrics are collected and sent.
	MetricsInterval time.Duration

	// Shell is the default shell spawned for PTY sessions.
	Shell string

	// Tags are arbitrary key=value labels attached to registration metadata.
	Tags map[string]string

	// LogLevel controls verbosity (debug, info, warn, error).
	LogLevel string

	// Version is the compiled-in agent version string.
	Version string
}

// Load reads configuration from environment variables.
// Required: SINDRI_CONSOLE_URL, SINDRI_CONSOLE_API_KEY
func Load() (*Config, error) {
	cfg := &Config{
		ConsoleURL:        os.Getenv("SINDRI_CONSOLE_URL"),
		APIKey:            os.Getenv("SINDRI_CONSOLE_API_KEY"),
		InstanceID:        os.Getenv("SINDRI_INSTANCE_ID"),
		Provider:          os.Getenv("SINDRI_PROVIDER"),
		Region:            os.Getenv("SINDRI_REGION"),
		Shell:             envOrDefault("SINDRI_AGENT_SHELL", defaultShell),
		LogLevel:          envOrDefault("SINDRI_LOG_LEVEL", defaultLogLevel),
		HeartbeatInterval: defaultHeartbeatInterval,
		MetricsInterval:   defaultMetricsInterval,
		Tags:              map[string]string{},
		Version:           agentVersion,
	}

	if cfg.ConsoleURL == "" {
		return nil, fmt.Errorf("SINDRI_CONSOLE_URL is required")
	}
	if cfg.APIKey == "" {
		return nil, fmt.Errorf("SINDRI_CONSOLE_API_KEY is required")
	}

	// Strip trailing slash from console URL.
	cfg.ConsoleURL = strings.TrimRight(cfg.ConsoleURL, "/")

	// Resolve instance ID from hostname when not set explicitly.
	if cfg.InstanceID == "" {
		hostname, err := os.Hostname()
		if err != nil {
			return nil, fmt.Errorf("resolving hostname for instance ID: %w", err)
		}
		cfg.InstanceID = hostname
	}

	// Optional heartbeat interval override (seconds).
	if v := os.Getenv("SINDRI_AGENT_HEARTBEAT"); v != "" {
		secs, err := strconv.ParseFloat(v, 64)
		if err != nil || secs <= 0 {
			return nil, fmt.Errorf("SINDRI_AGENT_HEARTBEAT must be a positive number of seconds, got %q", v)
		}
		cfg.HeartbeatInterval = time.Duration(secs * float64(time.Second))
	}

	// Optional metrics interval override (seconds).
	if v := os.Getenv("SINDRI_AGENT_METRICS"); v != "" {
		secs, err := strconv.ParseFloat(v, 64)
		if err != nil || secs <= 0 {
			return nil, fmt.Errorf("SINDRI_AGENT_METRICS must be a positive number of seconds, got %q", v)
		}
		cfg.MetricsInterval = time.Duration(secs * float64(time.Second))
	}

	// Parse SINDRI_AGENT_TAGS as comma-separated key=value pairs.
	if v := os.Getenv("SINDRI_AGENT_TAGS"); v != "" {
		for _, pair := range strings.Split(v, ",") {
			parts := strings.SplitN(strings.TrimSpace(pair), "=", 2)
			if len(parts) == 2 {
				cfg.Tags[strings.TrimSpace(parts[0])] = strings.TrimSpace(parts[1])
			}
		}
	}

	return cfg, nil
}

// WebSocketURL converts the ConsoleURL to a WebSocket endpoint.
func (c *Config) WebSocketURL() string {
	url := strings.Replace(c.ConsoleURL, "https://", "wss://", 1)
	url = strings.Replace(url, "http://", "ws://", 1)
	return url + "/ws/agent"
}

// RegistrationURL returns the REST endpoint for instance registration.
func (c *Config) RegistrationURL() string {
	return c.ConsoleURL + "/api/v1/instances"
}

func envOrDefault(key, def string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return def
}
