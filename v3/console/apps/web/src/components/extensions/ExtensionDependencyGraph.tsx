import { useMemo } from "react";
import { GitBranch, ArrowRight, Package, AlertTriangle } from "lucide-react";
import { useExtension, useExtensionDependencies } from "@/hooks/useExtensions";

interface ExtensionDependencyGraphProps {
  extensionId: string;
}

interface DependencyNode {
  name: string;
  depth: number;
  isDirect: boolean;
}

// Simple tree layout without react-flow dependency
function DependencyTree({ nodes, rootName }: { nodes: DependencyNode[]; rootName: string }) {
  const maxDepth = Math.max(...nodes.map((n) => n.depth), 0);

  const byDepth = useMemo(() => {
    const grouped: Record<number, DependencyNode[]> = {};
    for (const node of nodes) {
      if (!grouped[node.depth]) grouped[node.depth] = [];
      grouped[node.depth].push(node);
    }
    return grouped;
  }, [nodes]);

  if (nodes.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-white/10 py-8">
        <Package className="h-6 w-6 text-gray-600" />
        <p className="mt-2 text-sm text-gray-400">No dependencies</p>
        <p className="mt-0.5 text-xs text-gray-600">This extension has no declared dependencies</p>
      </div>
    );
  }

  return (
    <div className="flex items-start gap-6 overflow-x-auto">
      {/* Root node */}
      <div className="flex flex-col items-center">
        <div className="flex items-center gap-1.5 rounded-lg border border-blue-500/30 bg-blue-500/10 px-3 py-2 text-sm text-blue-300">
          <Package className="h-3.5 w-3.5" />
          {rootName}
        </div>
        <p className="mt-1 text-xs text-gray-600">root</p>
      </div>

      {/* Depth levels */}
      {Array.from({ length: maxDepth + 1 }, (_, i) => i + 1)
        .filter((d) => byDepth[d])
        .map((depth) => (
          <div key={depth} className="flex flex-col items-center gap-2">
            {/* Arrow connector */}
            <div className="flex items-center self-start pt-2.5">
              <ArrowRight className="h-4 w-4 text-gray-600" />
            </div>
            <div className="flex flex-col gap-2">
              {byDepth[depth].map((node) => (
                <div
                  key={node.name}
                  className={`flex items-center gap-1.5 rounded border px-2.5 py-1.5 text-xs ${
                    node.isDirect
                      ? "border-white/20 bg-white/5 text-gray-300"
                      : "border-white/10 bg-white/3 text-gray-500"
                  }`}
                >
                  <Package className="h-3 w-3 shrink-0" />
                  <span>{node.name}</span>
                  {!node.isDirect && <span className="ml-1 text-gray-600">(transitive)</span>}
                </div>
              ))}
            </div>
            <p className="text-xs text-gray-600">depth {depth}</p>
          </div>
        ))}
    </div>
  );
}

export function ExtensionDependencyGraph({ extensionId }: ExtensionDependencyGraphProps) {
  const { data: ext, isLoading: extLoading } = useExtension(extensionId);
  const { data: depsData, isLoading: depsLoading } = useExtensionDependencies(extensionId);

  const nodes = useMemo<DependencyNode[]>(() => {
    if (!ext || !depsData) return [];

    const directDeps = new Set(ext.dependencies);
    return depsData.dependencies.map((name) => ({
      name,
      depth: directDeps.has(name) ? 1 : 2,
      isDirect: directDeps.has(name),
    }));
  }, [ext, depsData]);

  const isLoading = extLoading || depsLoading;

  if (isLoading) {
    return (
      <div className="flex flex-col gap-4">
        <div className="h-6 w-48 animate-pulse rounded bg-white/5" />
        <div className="h-24 animate-pulse rounded-lg bg-white/5" />
      </div>
    );
  }

  if (!ext) {
    return (
      <div className="flex items-center justify-center py-8">
        <p className="text-sm text-gray-400">Extension not found</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <div>
        <div className="flex items-center gap-2">
          <GitBranch className="h-4 w-4 text-gray-400" />
          <h3 className="text-sm font-medium text-white">Dependency Graph</h3>
        </div>
        <p className="mt-0.5 text-xs text-gray-400">
          {nodes.length > 0
            ? `${nodes.length} total dependencies (${ext.dependencies.length} direct)`
            : "No dependencies declared"}
        </p>
      </div>

      {/* Circular dependency warning (simple heuristic) */}
      {depsData?.dependencies.includes(ext.name) && (
        <div className="flex items-center gap-2 rounded-lg border border-orange-500/20 bg-orange-500/10 px-3 py-2 text-xs text-orange-400">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          Circular dependency detected — this extension appears in its own dependency tree
        </div>
      )}

      {/* Graph */}
      <div className="overflow-x-auto rounded-lg border border-white/10 bg-white/3 p-4">
        <DependencyTree nodes={nodes} rootName={ext.name} />
      </div>

      {/* Dependency list */}
      {nodes.length > 0 && (
        <div>
          <h4 className="mb-2 text-xs font-medium text-gray-400">All Dependencies</h4>
          <div className="flex flex-wrap gap-1.5">
            {nodes.map((node) => (
              <span
                key={node.name}
                className={`inline-flex items-center gap-1 rounded border px-2 py-0.5 text-xs ${
                  node.isDirect
                    ? "border-blue-500/20 bg-blue-500/10 text-blue-300"
                    : "border-white/10 text-gray-500"
                }`}
              >
                <Package className="h-2.5 w-2.5" />
                {node.name}
                {node.isDirect && <span className="ml-0.5 text-blue-500/60">direct</span>}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
