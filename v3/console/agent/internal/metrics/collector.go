// Package metrics collects system resource statistics using gopsutil.
package metrics

import (
	"context"
	"fmt"
	"runtime"
	"time"

	"github.com/pacphi/sindri/v3/console/agent/pkg/protocol"
	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/disk"
	"github.com/shirou/gopsutil/v4/load"
	"github.com/shirou/gopsutil/v4/mem"
	psnet "github.com/shirou/gopsutil/v4/net"
)

// Collector gathers system metrics on demand.
type Collector struct {
	instanceID string
}

// NewCollector creates a new Collector for the given instance.
func NewCollector(instanceID string) *Collector {
	return &Collector{instanceID: instanceID}
}

// Collect gathers a full metrics snapshot.
// It returns an error only if critical subsystems (CPU, memory) fail.
func (c *Collector) Collect(ctx context.Context) (*protocol.MetricsPayload, error) {
	payload := &protocol.MetricsPayload{
		InstanceID: c.instanceID,
		Timestamp:  time.Now().UTC(),
	}

	// CPU usage — sampled over 1 second interval.
	cpuMetrics, err := collectCPU(ctx)
	if err != nil {
		return nil, fmt.Errorf("collecting CPU metrics: %w", err)
	}
	payload.CPU = cpuMetrics

	// Memory.
	memMetrics, err := collectMemory()
	if err != nil {
		return nil, fmt.Errorf("collecting memory metrics: %w", err)
	}
	payload.Memory = memMetrics

	// Disk — best-effort; skip on error.
	diskMetrics, err := collectDisk()
	if err == nil {
		payload.Disk = diskMetrics
	}

	// Network — best-effort; skip on error.
	netMetrics, err := collectNetwork()
	if err == nil {
		payload.Network = netMetrics
	}

	return payload, nil
}

func collectCPU(ctx context.Context) (protocol.CPUMetrics, error) {
	m := protocol.CPUMetrics{
		CoreCount: runtime.NumCPU(),
	}

	// Overall usage percentage (non-blocking; 1s window).
	percents, err := cpu.PercentWithContext(ctx, time.Second, false)
	if err != nil {
		return m, err
	}
	if len(percents) > 0 {
		m.UsagePercent = percents[0]
	}

	// Per-core percentages.
	perCore, err := cpu.PercentWithContext(ctx, 0, true)
	if err == nil {
		m.PerCore = perCore
	}

	// Load averages (not available on Windows; gopsutil returns zeros).
	avg, err := load.Avg()
	if err == nil {
		m.LoadAvg1 = avg.Load1
		m.LoadAvg5 = avg.Load5
		m.LoadAvg15 = avg.Load15
	}

	return m, nil
}

func collectMemory() (protocol.MemoryMetrics, error) {
	v, err := mem.VirtualMemory()
	if err != nil {
		return protocol.MemoryMetrics{}, err
	}
	m := protocol.MemoryMetrics{
		TotalBytes:   v.Total,
		UsedBytes:    v.Used,
		FreeBytes:    v.Free,
		CachedBytes:  v.Cached,
		UsagePercent: v.UsedPercent,
	}

	swap, err := mem.SwapMemory()
	if err == nil {
		m.SwapTotalBytes = swap.Total
		m.SwapUsedBytes = swap.Used
	}

	return m, nil
}

func collectDisk() ([]protocol.DiskMetrics, error) {
	partitions, err := disk.Partitions(false)
	if err != nil {
		return nil, err
	}

	var result []protocol.DiskMetrics
	for _, p := range partitions {
		usage, err := disk.Usage(p.Mountpoint)
		if err != nil {
			continue // skip unreadable mounts
		}
		result = append(result, protocol.DiskMetrics{
			MountPoint:   p.Mountpoint,
			Device:       p.Device,
			FSType:       p.Fstype,
			TotalBytes:   usage.Total,
			UsedBytes:    usage.Used,
			FreeBytes:    usage.Free,
			UsagePercent: usage.UsedPercent,
		})
	}
	return result, nil
}

func collectNetwork() (protocol.NetworkMetrics, error) {
	counters, err := psnet.IOCounters(false) // false = aggregate all interfaces
	if err != nil || len(counters) == 0 {
		return protocol.NetworkMetrics{}, err
	}
	c := counters[0]
	return protocol.NetworkMetrics{
		BytesSent:   c.BytesSent,
		BytesRecv:   c.BytesRecv,
		PacketsSent: c.PacketsSent,
		PacketsRecv: c.PacketsRecv,
	}, nil
}
