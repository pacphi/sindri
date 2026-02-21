import { useState } from "react";
import { Package } from "lucide-react";
import { ExtensionRegistry } from "./ExtensionRegistry";
import { ExtensionDetail } from "./ExtensionDetail";

export function ExtensionsPage() {
  const [selectedExtensionId, setSelectedExtensionId] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Header */}
      <div>
        <div className="flex items-center gap-3">
          <Package className="h-6 w-6 text-indigo-400" />
          <h1 className="text-2xl font-semibold text-white">Extensions</h1>
        </div>
        <p className="mt-1 text-sm text-gray-400">
          Browse, manage, and monitor extensions across your fleet
        </p>
      </div>

      {selectedExtensionId ? (
        <ExtensionDetail
          extensionId={selectedExtensionId}
          onBack={() => setSelectedExtensionId(null)}
        />
      ) : (
        <ExtensionRegistry onSelectExtension={(id) => setSelectedExtensionId(id)} />
      )}
    </div>
  );
}
