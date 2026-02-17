/**
 * Unit tests: Frontend utility functions
 *
 * Tests the helper functions in src/lib/utils.ts:
 *   - formatBytes
 *   - formatUptime
 *   - formatRelativeTime
 *   - cn (class name merging)
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { formatBytes, formatUptime, formatRelativeTime, cn } from "../src/lib/utils.js";

// ---------------------------------------------------------------------------
// formatBytes
// ---------------------------------------------------------------------------

describe("formatBytes", () => {
  it("formats 0 bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("formats bytes (< 1 KB)", () => {
    expect(formatBytes(512)).toBe("512 B");
  });

  it("formats kilobytes", () => {
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
  });

  it("formats megabytes", () => {
    expect(formatBytes(1024 * 1024)).toBe("1 MB");
    expect(formatBytes(512 * 1024 * 1024)).toBe("512 MB");
  });

  it("formats gigabytes", () => {
    expect(formatBytes(1024 * 1024 * 1024)).toBe("1 GB");
    expect(formatBytes(10.5 * 1024 * 1024 * 1024)).toBe("10.5 GB");
  });

  it("formats terabytes", () => {
    expect(formatBytes(1024 * 1024 * 1024 * 1024)).toBe("1 TB");
  });

  it("rounds to 1 decimal place", () => {
    // 1.1 KB
    expect(formatBytes(Math.round(1.1 * 1024))).toContain("1.1");
  });
});

// ---------------------------------------------------------------------------
// formatUptime
// ---------------------------------------------------------------------------

describe("formatUptime", () => {
  it("formats seconds under 60", () => {
    expect(formatUptime(0)).toBe("0s");
    expect(formatUptime(1)).toBe("1s");
    expect(formatUptime(59)).toBe("59s");
  });

  it("formats minutes (60s to 3599s)", () => {
    expect(formatUptime(60)).toBe("1m");
    expect(formatUptime(90)).toBe("1m");
    expect(formatUptime(3599)).toBe("59m");
    expect(formatUptime(120)).toBe("2m");
  });

  it("formats hours (3600s to 86399s)", () => {
    expect(formatUptime(3600)).toBe("1h 0m");
    expect(formatUptime(3660)).toBe("1h 1m");
    expect(formatUptime(7200)).toBe("2h 0m");
    expect(formatUptime(86399)).toBe("23h 59m");
  });

  it("formats days (>= 86400s)", () => {
    expect(formatUptime(86400)).toBe("1d 0h");
    expect(formatUptime(90000)).toBe("1d 1h");
    expect(formatUptime(172800)).toBe("2d 0h");
    expect(formatUptime(172900)).toBe("2d 0h");
  });
});

// ---------------------------------------------------------------------------
// formatRelativeTime
// ---------------------------------------------------------------------------

describe("formatRelativeTime", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns "just now" for very recent timestamps', () => {
    vi.useFakeTimers();
    const now = new Date("2024-01-01T12:00:00Z");
    vi.setSystemTime(now);

    const recent = new Date(now.getTime() - 30 * 1000); // 30 seconds ago
    expect(formatRelativeTime(recent.toISOString())).toBe("just now");
  });

  it("returns minutes ago for timestamps 1-59 minutes old", () => {
    vi.useFakeTimers();
    const now = new Date("2024-01-01T12:00:00Z");
    vi.setSystemTime(now);

    const fiveMinutesAgo = new Date(now.getTime() - 5 * 60 * 1000);
    expect(formatRelativeTime(fiveMinutesAgo.toISOString())).toBe("5m ago");

    const thirtyMinutesAgo = new Date(now.getTime() - 30 * 60 * 1000);
    expect(formatRelativeTime(thirtyMinutesAgo.toISOString())).toBe("30m ago");
  });

  it("returns hours ago for timestamps 1-23 hours old", () => {
    vi.useFakeTimers();
    const now = new Date("2024-01-01T12:00:00Z");
    vi.setSystemTime(now);

    const oneHourAgo = new Date(now.getTime() - 3600 * 1000);
    expect(formatRelativeTime(oneHourAgo.toISOString())).toBe("1h ago");

    const twelveHoursAgo = new Date(now.getTime() - 12 * 3600 * 1000);
    expect(formatRelativeTime(twelveHoursAgo.toISOString())).toBe("12h ago");
  });

  it("returns days ago for timestamps 1+ days old", () => {
    vi.useFakeTimers();
    const now = new Date("2024-01-08T12:00:00Z");
    vi.setSystemTime(now);

    const oneDayAgo = new Date(now.getTime() - 24 * 3600 * 1000);
    expect(formatRelativeTime(oneDayAgo.toISOString())).toBe("1d ago");

    const sevenDaysAgo = new Date(now.getTime() - 7 * 24 * 3600 * 1000);
    expect(formatRelativeTime(sevenDaysAgo.toISOString())).toBe("7d ago");
  });
});

// ---------------------------------------------------------------------------
// cn (class name merging)
// ---------------------------------------------------------------------------

describe("cn", () => {
  it("merges class names", () => {
    expect(cn("foo", "bar")).toBe("foo bar");
  });

  it("handles conditional classes", () => {
    const isActive = true;
    const isDisabled = false;
    expect(cn("base", isActive && "active", isDisabled && "disabled")).toBe("base active");
  });

  it("deduplicates conflicting Tailwind classes", () => {
    // tailwind-merge should pick the last one
    const result = cn("text-red-500", "text-blue-500");
    expect(result).toBe("text-blue-500");
  });

  it("handles undefined and null gracefully", () => {
    expect(() => cn("foo", undefined, null as unknown as string, "bar")).not.toThrow();
  });

  it("returns empty string with no arguments", () => {
    expect(cn()).toBe("");
  });

  it("handles object syntax", () => {
    const result = cn({ "text-red-500": true, "text-blue-500": false });
    expect(result).toBe("text-red-500");
  });
});
