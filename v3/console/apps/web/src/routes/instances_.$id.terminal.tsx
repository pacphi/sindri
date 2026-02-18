import { createFileRoute } from "@tanstack/react-router";
import { TerminalManager } from "@/components/terminal";

function InstanceTerminalPage() {
  const { id } = Route.useParams();

  return (
    <div className="flex h-full flex-col">
      <TerminalManager instanceId={id} instanceName={id} theme="dark" className="flex-1" />
    </div>
  );
}

export const Route = createFileRoute("/instances_/$id/terminal")({
  component: InstanceTerminalPage,
});
