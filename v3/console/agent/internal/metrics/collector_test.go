package metrics

import (
	"context"
	"testing"
)

func TestCollector_Collect(t *testing.T) {
	c := NewCollector("test-instance")

	payload, err := c.Collect(context.Background())
	if err != nil {
		t.Fatalf("Collect() error: %v", err)
	}

	if payload.InstanceID != "test-instance" {
		t.Errorf("InstanceID = %q, want test-instance", payload.InstanceID)
	}
	if payload.Timestamp.IsZero() {
		t.Error("Timestamp must not be zero")
	}

	// CPU
	if payload.CPU.CoreCount <= 0 {
		t.Errorf("CoreCount = %d, want > 0", payload.CPU.CoreCount)
	}
	if payload.CPU.UsagePercent < 0 || payload.CPU.UsagePercent > 100 {
		t.Errorf("CPU.UsagePercent = %.2f, want 0–100", payload.CPU.UsagePercent)
	}

	// Memory
	if payload.Memory.TotalBytes == 0 {
		t.Error("Memory.TotalBytes must be > 0")
	}
	if payload.Memory.UsagePercent < 0 || payload.Memory.UsagePercent > 100 {
		t.Errorf("Memory.UsagePercent = %.2f, want 0–100", payload.Memory.UsagePercent)
	}
}

func TestCollector_CollectCancelled(t *testing.T) {
	c := NewCollector("test-instance")

	ctx, cancel := context.WithCancel(context.Background())
	cancel() // immediately cancelled

	// Even with a cancelled context, the collector should not panic.
	// CPU collection may fail; we just ensure no panic occurs.
	defer func() {
		if r := recover(); r != nil {
			t.Errorf("Collect panicked with cancelled context: %v", r)
		}
	}()

	_, _ = c.Collect(ctx)
}
