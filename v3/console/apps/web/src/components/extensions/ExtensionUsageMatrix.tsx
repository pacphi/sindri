import { useState } from "react";
import { Grid3x3, CheckCircle2, XCircle, MinusCircle, AlertCircle } from "lucide-react";
import { useUsageMatrix } from "@/hooks/useExtensions";

interface ExtensionUsageMatrixProps {
  instanceIds?: string[];
}

function CellStatus({ installed, failed }: { installed?: boolean; failed?: boolean }) {
  if (!installed) {
    return <MinusCircle className="h-4 w-4 text-gray-700" />;
  }
  if (failed) {
    return <XCircle className="h-4 w-4 text-red-400" />;
  }
  return <CheckCircle2 className="h-4 w-4 text-green-400" />;
}

export function ExtensionUsageMatrix({ instanceIds }: ExtensionUsageMatrixProps) {
  const { data, isLoading } = useUsageMatrix(instanceIds);
  const [hoveredCell, setHoveredCell] = useState<{
    instanceId: string;
    extensionId: string;
    version?: string;
    installed_at?: string;
    failed?: boolean;
  } | null>(null);

  if (isLoading) {
    return (
      <div className="flex flex-col gap-4">
        <div className="h-6 w-48 animate-pulse rounded bg-white/5" />
        <div className="h-64 animate-pulse rounded-lg bg-white/5" />
      </div>
    );
  }

  if (!data || data.extensions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-white/10 py-12">
        <Grid3x3 className="h-8 w-8 text-gray-600" />
        <p className="mt-3 text-sm text-gray-400">No extension usage data available</p>
        <p className="mt-1 text-xs text-gray-600">
          Install extensions on your instances to see the matrix
        </p>
      </div>
    );
  }

  const { matrix, extensions, instance_ids } = data;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium text-white">Extension Usage Matrix</h3>
          <p className="mt-0.5 text-xs text-gray-400">
            {instance_ids.length} instances × {extensions.length} extensions
          </p>
        </div>
        {/* Legend */}
        <div className="flex items-center gap-4 text-xs text-gray-400">
          <span className="flex items-center gap-1">
            <CheckCircle2 className="h-3.5 w-3.5 text-green-400" /> Installed
          </span>
          <span className="flex items-center gap-1">
            <XCircle className="h-3.5 w-3.5 text-red-400" /> Failed
          </span>
          <span className="flex items-center gap-1">
            <MinusCircle className="h-3.5 w-3.5 text-gray-700" /> Not installed
          </span>
        </div>
      </div>

      {/* Tooltip */}
      {hoveredCell && (
        <div className="rounded-lg border border-white/10 bg-gray-900 px-3 py-2 text-xs">
          <p className="font-medium text-white">
            {extensions.find((e) => e.id === hoveredCell.extensionId)?.display_name}
          </p>
          <p className="text-gray-400">
            Instance: <span className="text-gray-300">{hoveredCell.instanceId}</span>
          </p>
          {hoveredCell.installed_at && (
            <p className="text-gray-400">
              Installed:{" "}
              <span className="text-gray-300">
                {new Date(hoveredCell.installed_at).toLocaleDateString()}
              </span>
            </p>
          )}
          {hoveredCell.version && (
            <p className="text-gray-400">
              Version: <span className="text-gray-300">v{hoveredCell.version}</span>
            </p>
          )}
          {hoveredCell.failed && (
            <p className="flex items-center gap-1 text-red-400">
              <AlertCircle className="h-3 w-3" /> Install failed
            </p>
          )}
        </div>
      )}

      {/* Matrix table */}
      <div className="overflow-x-auto rounded-lg border border-white/10">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-white/10 bg-white/5">
              <th className="sticky left-0 bg-gray-900 px-3 py-2 text-left text-gray-400 font-medium w-32">
                Instance
              </th>
              {extensions.map((ext) => (
                <th
                  key={ext.id}
                  className="px-2 py-2 text-center font-medium text-gray-400 max-w-20"
                  title={ext.display_name}
                >
                  <div
                    className="overflow-hidden text-ellipsis whitespace-nowrap"
                    style={{ maxWidth: "64px" }}
                  >
                    {ext.name}
                  </div>
                  <div className="text-gray-600 font-normal">{ext.category}</div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {instance_ids.map((instanceId, rowIdx) => (
              <tr
                key={instanceId}
                className={`border-b border-white/5 ${rowIdx % 2 === 0 ? "bg-transparent" : "bg-white/2"}`}
              >
                <td className="sticky left-0 bg-gray-900 px-3 py-2 text-gray-300 font-mono text-xs">
                  <span
                    className="block overflow-hidden text-ellipsis whitespace-nowrap w-28"
                    title={instanceId}
                  >
                    {instanceId.slice(0, 12)}...
                  </span>
                </td>
                {extensions.map((ext) => {
                  const cell = matrix[instanceId]?.[ext.id];
                  return (
                    <td
                      key={ext.id}
                      className="px-2 py-2 text-center"
                      onMouseEnter={() =>
                        cell
                          ? setHoveredCell({
                              instanceId,
                              extensionId: ext.id,
                              version: cell.version,
                              installed_at: cell.installed_at,
                              failed: cell.failed,
                            })
                          : setHoveredCell({ instanceId, extensionId: ext.id })
                      }
                      onMouseLeave={() => setHoveredCell(null)}
                    >
                      <div className="flex justify-center">
                        <CellStatus installed={cell?.installed} failed={cell?.failed} />
                      </div>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
