import type { Instance } from "@/types/instance";
import { InstanceList } from "@/components/instances";

export function InstancesPage() {
  function handleSelectInstance(_instance: Instance) {
    // Navigate to instance detail — TanStack Router navigation will be wired up
    // when routes are configured
    // TODO: Implement navigation to /instances/${instance.id}
  }

  return (
    <main className="container mx-auto px-4 py-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold tracking-tight">Instances</h1>
        <p className="text-muted-foreground mt-1">
          Manage and monitor your Sindri instances across all providers.
        </p>
      </div>
      <InstanceList onSelectInstance={handleSelectInstance} />
    </main>
  );
}
