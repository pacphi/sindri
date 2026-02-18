import { createFileRoute } from "@tanstack/react-router";
import { SecurityDashboard } from "@/components/security/SecurityDashboard";

export const Route = createFileRoute("/security")({
  component: SecurityDashboard,
});
