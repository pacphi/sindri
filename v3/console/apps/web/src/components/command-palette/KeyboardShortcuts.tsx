import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface ShortcutGroup {
  label: string;
  shortcuts: { keys: string[]; description: string }[];
}

const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    label: "Command Palette",
    shortcuts: [
      { keys: ["⌘", "K"], description: "Open command palette" },
      { keys: ["⌘", "P"], description: "Open instance switcher" },
      { keys: ["/"], description: "Quick search" },
      { keys: ["?"], description: "Show keyboard shortcuts" },
    ],
  },
  {
    label: "Navigation",
    shortcuts: [
      { keys: ["↑", "↓"], description: "Move through results" },
      { keys: ["Enter"], description: "Select item" },
      { keys: ["Esc"], description: "Close / go back" },
      { keys: ["Tab"], description: "Next result" },
    ],
  },
  {
    label: "General",
    shortcuts: [
      { keys: ["⌘", "K"], description: "Open command palette" },
      { keys: ["⌘", "/"], description: "Toggle sidebar" },
    ],
  },
];

interface KeyboardShortcutsProps {
  onClose: () => void;
  className?: string;
}

export function KeyboardShortcuts({ onClose, className }: KeyboardShortcutsProps) {
  return (
    <div className={cn("flex flex-col", className)}>
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 className="text-sm font-semibold">Keyboard Shortcuts</h2>
        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onClose}>
          <X className="h-4 w-4" />
        </Button>
      </div>

      {/* Shortcut groups */}
      <div className="flex-1 overflow-y-auto p-4 space-y-6">
        {SHORTCUT_GROUPS.map((group) => (
          <div key={group.label}>
            <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
              {group.label}
            </h3>
            <div className="space-y-1">
              {group.shortcuts.map((shortcut) => (
                <div key={shortcut.description} className="flex items-center justify-between py-1">
                  <span className="text-sm text-foreground">{shortcut.description}</span>
                  <div className="flex items-center gap-1">
                    {shortcut.keys.map((key, i) => (
                      <span key={i} className="flex items-center gap-1">
                        <kbd className="px-1.5 py-0.5 text-xs bg-muted rounded border border-border font-mono min-w-[1.5rem] text-center">
                          {key}
                        </kbd>
                        {i < shortcut.keys.length - 1 && (
                          <span className="text-xs text-muted-foreground">+</span>
                        )}
                      </span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
