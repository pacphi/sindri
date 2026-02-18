import { useState } from "react";
import { Check, ChevronDown, Server, X } from "lucide-react";
import type { Instance } from "@/types/instance";
import { cn } from "@/lib/utils";

interface InstanceSelectorProps {
  instances: Instance[];
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  maxSelections?: number;
  disabled?: boolean;
  placeholder?: string;
}

export function InstanceSelector({
  instances,
  selectedIds,
  onChange,
  maxSelections,
  disabled = false,
  placeholder = "Select instances...",
}: InstanceSelectorProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");

  const running = instances.filter((i) => i.status === "RUNNING");
  const filtered = running.filter(
    (i) =>
      i.name.toLowerCase().includes(search.toLowerCase()) ||
      i.provider.toLowerCase().includes(search.toLowerCase()),
  );

  const selectedInstances = instances.filter((i) => selectedIds.includes(i.id));

  function toggle(id: string) {
    if (selectedIds.includes(id)) {
      onChange(selectedIds.filter((s) => s !== id));
    } else {
      if (maxSelections && selectedIds.length >= maxSelections) return;
      onChange([...selectedIds, id]);
    }
  }

  function selectAll() {
    const runningIds = filtered.map((i) => i.id);
    if (maxSelections) {
      onChange(runningIds.slice(0, maxSelections));
    } else {
      onChange(runningIds);
    }
  }

  function clearAll() {
    onChange([]);
  }

  return (
    <div className="relative">
      {/* Trigger */}
      <button
        type="button"
        disabled={disabled}
        onClick={() => setOpen((o) => !o)}
        className={cn(
          "flex min-h-9 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background",
          "focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
          disabled && "cursor-not-allowed opacity-50",
          !disabled && "hover:bg-accent/30",
        )}
      >
        <div className="flex flex-wrap gap-1 flex-1 min-w-0">
          {selectedInstances.length === 0 ? (
            <span className="text-muted-foreground">{placeholder}</span>
          ) : (
            selectedInstances.map((inst) => (
              <span
                key={inst.id}
                className="inline-flex items-center gap-1 rounded bg-primary/15 px-1.5 py-0.5 text-xs font-medium text-primary"
              >
                {inst.name}
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    toggle(inst.id);
                  }}
                  className="ml-0.5 hover:text-primary/70"
                >
                  <X className="h-3 w-3" />
                </button>
              </span>
            ))
          )}
        </div>
        <ChevronDown
          className={cn(
            "ml-2 h-4 w-4 shrink-0 text-muted-foreground transition-transform",
            open && "rotate-180",
          )}
        />
      </button>

      {/* Dropdown */}
      {open && (
        <div className="absolute z-50 mt-1 w-full rounded-md border bg-popover shadow-md">
          <div className="p-2 border-b">
            <input
              className="w-full rounded-sm border-0 bg-transparent px-2 py-1 text-sm placeholder:text-muted-foreground focus:outline-none"
              placeholder="Search instances..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onClick={(e) => e.stopPropagation()}
              autoFocus
            />
          </div>

          <div className="flex items-center justify-between px-3 py-1.5 border-b text-xs text-muted-foreground">
            <span>
              {selectedIds.length} selected
              {maxSelections ? ` / ${maxSelections}` : ""}
            </span>
            <div className="flex gap-2">
              <button type="button" onClick={selectAll} className="hover:text-foreground">
                Select all
              </button>
              <button type="button" onClick={clearAll} className="hover:text-foreground">
                Clear
              </button>
            </div>
          </div>

          <div className="max-h-48 overflow-y-auto py-1">
            {filtered.length === 0 ? (
              <div className="px-3 py-2 text-sm text-muted-foreground">
                No running instances found
              </div>
            ) : (
              filtered.map((inst) => {
                const selected = selectedIds.includes(inst.id);
                const atLimit =
                  !selected && maxSelections !== undefined && selectedIds.length >= maxSelections;
                return (
                  <button
                    key={inst.id}
                    type="button"
                    disabled={atLimit}
                    onClick={() => toggle(inst.id)}
                    className={cn(
                      "flex w-full items-center gap-2 px-3 py-2 text-left text-sm",
                      selected && "bg-primary/10",
                      !atLimit && "hover:bg-accent",
                      atLimit && "cursor-not-allowed opacity-40",
                    )}
                  >
                    <div
                      className={cn(
                        "flex h-4 w-4 items-center justify-center rounded border",
                        selected
                          ? "border-primary bg-primary text-primary-foreground"
                          : "border-muted-foreground",
                      )}
                    >
                      {selected && <Check className="h-3 w-3" />}
                    </div>
                    <Server className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                    <span className="truncate font-medium">{inst.name}</span>
                    <span className="ml-auto text-xs text-muted-foreground">{inst.provider}</span>
                  </button>
                );
              })
            )}
          </div>
        </div>
      )}

      {/* Close on outside click */}
      {open && (
        <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} aria-hidden="true" />
      )}
    </div>
  );
}
