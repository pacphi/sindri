import { createFileRoute } from "@tanstack/react-router";
import { TasksPage } from "@/pages/TasksPage";

export const Route = createFileRoute("/tasks")({
  component: TasksPage,
});
