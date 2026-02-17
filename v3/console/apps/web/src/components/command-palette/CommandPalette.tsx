import { useEffect, useRef, useState, useCallback, useMemo } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Search, X, Command } from "lucide-react";
import { useCommandPaletteStore } from "@/stores/commandPaletteStore";
import { useThemeStore } from "@/stores/themeStore";
import { useInstances } from "@/hooks/useInstances";
import { fuzzySearch } from "./fuzzySearch";
import { buildActionRegistry } from "./ActionRegistry";
import { RecentInstances } from "./RecentInstances";
import { SearchResults } from "./SearchResults";
import { KeyboardShortcuts } from "./KeyboardShortcuts";
import type { PaletteAction } from "./ActionRegistry";
import type { SearchResultItem } from "./SearchResults";
import type { Instance } from "@/types/instance";

export function CommandPalette() {
  const { isOpen, mode, closePalette, openPalette, addRecentInstance, recentInstanceIds } =
    useCommandPaletteStore();
  const { setTheme } = useThemeStore();
  const navigate = useNavigate();

  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showShortcuts, setShowShortcuts] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const { data: instanceData } = useInstances({}, 1, 100);
  const allInstances = instanceData?.instances ?? [];

  const actions = useMemo(
    () =>
      buildActionRegistry({
        navigate: (to) => navigate({ to: to as "/dashboard" | "/instances" | "/settings" }),
        setTheme,
        openShortcuts: () => setShowShortcuts(true),
      }),
    [navigate, setTheme],
  );

  // Compute recent instances (ordered)
  const recentInstances = useMemo(() => {
    return recentInstanceIds
      .map((id) => allInstances.find((i) => i.id === id))
      .filter((i): i is Instance => Boolean(i))
      .slice(0, 5);
  }, [recentInstanceIds, allInstances]);

  // Compute search results
  const searchResults = useMemo((): SearchResultItem[] => {
    if (!query.trim()) return [];

    const instanceResults = fuzzySearch(allInstances, query, (inst) => [
      inst.name,
      inst.provider,
      inst.region ?? "",
      inst.status,
    ]).map(({ item, score }) => ({
      type: "instance" as const,
      id: item.id,
      instance: item,
      score,
    }));

    // Filter by mode
    const isInstanceMode = mode === "instance";

    const actionResults = isInstanceMode
      ? []
      : fuzzySearch(actions, query, (a) => [a.label, a.description ?? "", ...a.keywords]).map(
          ({ item, score }) => ({
            type: "action" as const,
            id: item.id,
            action: item,
            score,
          }),
        );

    return [...instanceResults, ...actionResults].sort((a, b) => b.score - a.score).slice(0, 20);
  }, [query, allInstances, actions, mode]);

  // Total items for keyboard navigation
  const totalItems = query.trim() ? searchResults.length : recentInstances.length;

  // Reset state when palette opens/closes
  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setSelectedIndex(0);
      setShowShortcuts(mode === "shortcuts");
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [isOpen, mode]);

  // Scroll selected item into view
  useEffect(() => {
    if (!listRef.current) return;
    const el = listRef.current.querySelector(`[data-result-index="${selectedIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const handleSelectInstance = useCallback(
    (instance: Instance) => {
      addRecentInstance(instance.id);
      navigate({ to: "/instances/$id", params: { id: instance.id } });
      closePalette();
    },
    [addRecentInstance, navigate, closePalette],
  );

  const handleSelectAction = useCallback(
    (action: PaletteAction) => {
      action.execute();
      if (action.id !== "system-shortcuts") {
        closePalette();
      }
    },
    [closePalette],
  );

  const handleSelectCurrent = useCallback(() => {
    if (query.trim()) {
      const item = searchResults[selectedIndex];
      if (!item) return;
      if (item.instance) handleSelectInstance(item.instance);
      else if (item.action) handleSelectAction(item.action);
    } else {
      const instance = recentInstances[selectedIndex];
      if (instance) handleSelectInstance(instance);
    }
  }, [
    query,
    searchResults,
    recentInstances,
    selectedIndex,
    handleSelectInstance,
    handleSelectAction,
  ]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((i) => Math.min(i + 1, totalItems - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((i) => Math.max(i - 1, 0));
          break;
        case "Tab":
          e.preventDefault();
          setSelectedIndex((i) => (i + 1) % Math.max(totalItems, 1));
          break;
        case "Enter":
          e.preventDefault();
          handleSelectCurrent();
          break;
        case "Escape":
          e.preventDefault();
          if (showShortcuts) {
            setShowShortcuts(false);
          } else if (query) {
            setQuery("");
            setSelectedIndex(0);
          } else {
            closePalette();
          }
          break;
      }
    },
    [totalItems, handleSelectCurrent, showShortcuts, query, closePalette],
  );

  // Reset selection when query changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  // Global keyboard handler
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      const isMac = navigator.platform.toUpperCase().includes("MAC");
      const modKey = isMac ? e.metaKey : e.ctrlKey;

      // Cmd/Ctrl+K — command palette
      if (modKey && e.key === "k") {
        e.preventDefault();
        if (isOpen && mode === "command") {
          closePalette();
        } else {
          openPalette("command");
        }
        return;
      }

      // Cmd/Ctrl+P — instance switcher
      if (modKey && e.key === "p") {
        e.preventDefault();
        if (isOpen && mode === "instance") {
          closePalette();
        } else {
          openPalette("instance");
        }
        return;
      }

      // / — quick search (only when not in an input)
      if (e.key === "/" && !isOpen) {
        const tag = (e.target as HTMLElement).tagName;
        if (tag !== "INPUT" && tag !== "TEXTAREA" && tag !== "SELECT") {
          e.preventDefault();
          openPalette("search");
        }
        return;
      }

      // ? — shortcuts help
      if (e.key === "?" && !isOpen) {
        const tag = (e.target as HTMLElement).tagName;
        if (tag !== "INPUT" && tag !== "TEXTAREA" && tag !== "SELECT") {
          e.preventDefault();
          openPalette("shortcuts");
        }
        return;
      }
    };

    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => window.removeEventListener("keydown", handleGlobalKeyDown);
  }, [isOpen, mode, openPalette, closePalette]);

  if (!isOpen) return null;

  const placeholder =
    mode === "instance"
      ? "Switch to instance..."
      : mode === "search"
        ? "Search..."
        : "Type a command or search...";

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
        onClick={closePalette}
        aria-hidden="true"
      />

      {/* Panel */}
      <div
        className="fixed left-1/2 top-[20%] z-50 w-full max-w-lg -translate-x-1/2"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
      >
        <div className="rounded-xl border border-border bg-card shadow-2xl overflow-hidden">
          {showShortcuts ? (
            <KeyboardShortcuts onClose={() => setShowShortcuts(false)} className="max-h-[60vh]" />
          ) : (
            <>
              {/* Search input */}
              <div className="flex items-center gap-2 border-b border-border px-3">
                <Search className="h-4 w-4 shrink-0 text-muted-foreground" />
                <input
                  ref={inputRef}
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder={placeholder}
                  className="flex-1 bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground"
                  aria-label="Command palette search"
                  autoComplete="off"
                  spellCheck={false}
                />
                {query && (
                  <button
                    onClick={() => {
                      setQuery("");
                      inputRef.current?.focus();
                    }}
                    className="shrink-0 text-muted-foreground hover:text-foreground"
                    aria-label="Clear search"
                  >
                    <X className="h-4 w-4" />
                  </button>
                )}
                <kbd
                  className="shrink-0 px-1.5 py-0.5 text-xs bg-muted rounded border border-border font-mono"
                  title="Press Escape to close"
                >
                  Esc
                </kbd>
              </div>

              {/* Results */}
              <div
                ref={listRef}
                className="max-h-[60vh] overflow-y-auto py-1"
                role="listbox"
                aria-label="Results"
              >
                {query.trim() ? (
                  <SearchResults
                    results={searchResults}
                    selectedIndex={selectedIndex}
                    onSelectInstance={handleSelectInstance}
                    onSelectAction={handleSelectAction}
                  />
                ) : (
                  <>
                    <RecentInstances
                      instances={recentInstances}
                      selectedIndex={selectedIndex}
                      indexOffset={0}
                      onSelect={handleSelectInstance}
                    />
                    {recentInstances.length === 0 && (
                      <div className="px-4 py-6 text-center">
                        <Command className="h-8 w-8 mx-auto mb-2 text-muted-foreground" />
                        <p className="text-sm text-muted-foreground">
                          {mode === "instance"
                            ? "Type to search instances"
                            : "Type a command, search, or navigate"}
                        </p>
                      </div>
                    )}
                  </>
                )}
              </div>

              {/* Footer hint */}
              <div className="flex items-center justify-between border-t border-border px-3 py-2 text-xs text-muted-foreground">
                <span className="flex items-center gap-2">
                  <span>
                    <kbd className="font-mono">↑↓</kbd> navigate
                  </span>
                  <span>
                    <kbd className="font-mono">↵</kbd> select
                  </span>
                  <span>
                    <kbd className="font-mono">Esc</kbd> close
                  </span>
                </span>
                <button
                  className="hover:text-foreground transition-colors"
                  onClick={() => setShowShortcuts(true)}
                >
                  <kbd className="font-mono">?</kbd> shortcuts
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </>
  );
}
