// Package registration handles the one-time POST to the Console on agent boot.
package registration

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"runtime"
	"time"

	"github.com/pacphi/sindri/v3/console/agent/internal/config"
	"github.com/pacphi/sindri/v3/console/agent/pkg/protocol"
)

const registrationTimeout = 15 * time.Second

// Registrar posts agent metadata to the Console instance registry.
type Registrar struct {
	cfg    *config.Config
	client *http.Client
}

// New creates a Registrar with sensible HTTP timeouts.
func New(cfg *config.Config) *Registrar {
	return &Registrar{
		cfg: cfg,
		client: &http.Client{
			Timeout: registrationTimeout,
		},
	}
}

// Register sends the registration payload to the Console.
// It retries with exponential back-off up to maxAttempts times.
func (r *Registrar) Register(ctx context.Context) error {
	payload := r.buildPayload()

	body, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("marshalling registration payload: %w", err)
	}

	const maxAttempts = 5
	backoff := 2 * time.Second

	for attempt := 1; attempt <= maxAttempts; attempt++ {
		err = r.post(ctx, body)
		if err == nil {
			return nil
		}

		if attempt == maxAttempts {
			break
		}

		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(backoff):
			backoff *= 2
		}
	}

	return fmt.Errorf("registration failed after %d attempts: %w", maxAttempts, err)
}

func (r *Registrar) post(ctx context.Context, body []byte) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, r.cfg.RegistrationURL(), bytes.NewReader(body))
	if err != nil {
		return fmt.Errorf("building request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+r.cfg.APIKey)
	req.Header.Set("X-Agent-Version", r.cfg.Version)

	resp, err := r.client.Do(req)
	if err != nil {
		return fmt.Errorf("HTTP POST: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode == http.StatusConflict {
		// Already registered — idempotent; treat as success.
		return nil
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("unexpected HTTP status %d", resp.StatusCode)
	}
	return nil
}

func (r *Registrar) buildPayload() protocol.RegistrationPayload {
	return protocol.RegistrationPayload{
		InstanceID:   r.cfg.InstanceID,
		Hostname:     r.cfg.InstanceID,
		Provider:     r.cfg.Provider,
		Region:       r.cfg.Region,
		AgentVersion: r.cfg.Version,
		OS:           runtime.GOOS,
		Arch:         runtime.GOARCH,
		Tags:         r.cfg.Tags,
		Timestamp:    time.Now().UTC(),
	}
}
