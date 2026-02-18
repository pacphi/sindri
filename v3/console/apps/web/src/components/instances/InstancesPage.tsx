import { useNavigate } from "@tanstack/react-router";
import type { Instance } from "@/types/instance";
import { InstanceList } from "./InstanceList";

export function InstancesPage() {
  const navigate = useNavigate();

  function handleSelectInstance(instance: Instance) {
    void navigate({ to: "/instances/$id", params: { id: instance.id } });
  }

  return (
    <div className="p-6 space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">Instances</h1>
        <p className="text-sm text-muted-foreground mt-1">
          All registered Sindri environments across providers
        </p>
      </div>

      <InstanceList onSelectInstance={handleSelectInstance} />
    </div>
  );
}
