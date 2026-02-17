-- Migration: Instance Lifecycle - Add SUSPENDED status and SUSPEND/RESUME event types
-- Adds SUSPENDED to InstanceStatus enum and SUSPEND/RESUME to EventType enum

-- Add SUSPENDED to InstanceStatus enum
ALTER TYPE "InstanceStatus" ADD VALUE IF NOT EXISTS 'SUSPENDED';

-- Add SUSPEND to EventType enum
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'SUSPEND';

-- Add RESUME to EventType enum
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'RESUME';
