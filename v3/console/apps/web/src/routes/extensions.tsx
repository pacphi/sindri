import { createFileRoute } from "@tanstack/react-router";
import { ExtensionsPage } from "@/pages/ExtensionsPage";

export const Route = createFileRoute("/extensions")({
  component: ExtensionsPage,
});
