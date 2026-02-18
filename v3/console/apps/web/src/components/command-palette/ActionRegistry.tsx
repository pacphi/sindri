import {
  LayoutDashboard,
  Server,
  Settings,
  Plus,
  RefreshCw,
  Terminal,
  Moon,
  Sun,
  Monitor,
  Keyboard,
  Search,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export interface PaletteAction {
  id: string;
  label: string;
  description?: string;
  icon: LucideIcon;
  category: "navigation" | "action" | "instance" | "system";
  keywords: string[];
  shortcut?: string[];
  execute: () => void;
}

export type ActionRegistryOptions = {
  navigate: (to: string, opts?: { params?: Record<string, string> }) => void;
  setTheme: (theme: "light" | "dark" | "system") => void;
  openShortcuts: () => void;
};

export function buildActionRegistry(opts: ActionRegistryOptions): PaletteAction[] {
  const { navigate, setTheme, openShortcuts } = opts;

  return [
    // Navigation
    {
      id: "nav-dashboard",
      label: "Go to Dashboard",
      description: "Navigate to the main dashboard",
      icon: LayoutDashboard,
      category: "navigation",
      keywords: ["dashboard", "home", "overview"],
      execute: () => navigate("/dashboard"),
    },
    {
      id: "nav-instances",
      label: "Go to Instances",
      description: "Browse all instances",
      icon: Server,
      category: "navigation",
      keywords: ["instances", "servers", "list"],
      execute: () => navigate("/instances"),
    },
    {
      id: "nav-settings",
      label: "Go to Settings",
      description: "Open application settings",
      icon: Settings,
      category: "navigation",
      keywords: ["settings", "config", "preferences"],
      shortcut: [],
      execute: () => navigate("/settings"),
    },

    // Instance actions
    {
      id: "action-new-instance",
      label: "Deploy New Instance",
      description: "Start the deployment wizard",
      icon: Plus,
      category: "action",
      keywords: ["new", "deploy", "create", "instance", "wizard"],
      execute: () => navigate("/instances"),
    },
    {
      id: "action-refresh",
      label: "Refresh Instances",
      description: "Reload instance data",
      icon: RefreshCw,
      category: "action",
      keywords: ["refresh", "reload", "update"],
      execute: () => window.location.reload(),
    },
    {
      id: "action-terminal",
      label: "Open Terminal",
      description: "Open a terminal session",
      icon: Terminal,
      category: "action",
      keywords: ["terminal", "shell", "ssh", "console"],
      execute: () => navigate("/instances"),
    },

    // System
    {
      id: "system-theme-light",
      label: "Set Light Theme",
      icon: Sun,
      category: "system",
      keywords: ["light", "theme", "appearance"],
      execute: () => setTheme("light"),
    },
    {
      id: "system-theme-dark",
      label: "Set Dark Theme",
      icon: Moon,
      category: "system",
      keywords: ["dark", "theme", "appearance"],
      execute: () => setTheme("dark"),
    },
    {
      id: "system-theme-system",
      label: "Set System Theme",
      icon: Monitor,
      category: "system",
      keywords: ["system", "auto", "theme", "appearance"],
      execute: () => setTheme("system"),
    },
    {
      id: "system-shortcuts",
      label: "View Keyboard Shortcuts",
      description: "Show all keyboard shortcuts",
      icon: Keyboard,
      category: "system",
      keywords: ["keyboard", "shortcuts", "hotkeys", "help"],
      shortcut: ["?"],
      execute: () => openShortcuts(),
    },
    {
      id: "system-search",
      label: "Search",
      description: "Search instances and actions",
      icon: Search,
      category: "system",
      keywords: ["search", "find", "filter"],
      shortcut: ["/"],
      execute: () => {},
    },
  ];
}
