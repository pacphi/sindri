import { createFileRoute } from "@tanstack/react-router";
import { LogsPage } from "@/pages/LogsPage";

export const Route = createFileRoute("/logs")({
  component: LogsPage,
});
