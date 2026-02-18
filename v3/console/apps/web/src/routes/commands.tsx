import { createFileRoute } from "@tanstack/react-router";
import { CommandsPage } from "@/pages/CommandsPage";

export const Route = createFileRoute("/commands")({
  component: CommandsPage,
});
