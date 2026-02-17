import { Link, useRouterState } from "@tanstack/react-router";
import {
  LayoutDashboard,
  Server,
  Settings,
  ChevronLeft,
  ChevronRight,
  Moon,
  Sun,
  Monitor,
  Search,
  Rocket,
  Terminal,
  Calendar,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useUIStore } from "@/stores/uiStore";
import { useThemeStore } from "@/stores/themeStore";
import { useCommandPaletteStore } from "@/stores/commandPaletteStore";
import { Button } from "@/components/ui/button";

const NAV_ITEMS = [
  { to: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { to: "/instances", label: "Instances", icon: Server },
  { to: "/deployments", label: "Deployments", icon: Rocket },
  { to: "/commands", label: "Commands", icon: Terminal },
  { to: "/tasks", label: "Scheduled Tasks", icon: Calendar },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

export function Sidebar() {
  const collapsed = useUIStore((state) => state.sidebarCollapsed);
  const toggleSidebar = useUIStore((state) => state.toggleSidebar);
  const { theme, setTheme } = useThemeStore();
  const openPalette = useCommandPaletteStore((state) => state.openPalette);
  const routerState = useRouterState();
  const currentPath = routerState.location.pathname;

  const themeIcons = {
    light: Sun,
    dark: Moon,
    system: Monitor,
  };

  const themes = ["light", "dark", "system"] as const;
  const nextTheme = themes[(themes.indexOf(theme) + 1) % themes.length];
  const ThemeIcon = themeIcons[theme];

  return (
    <aside
      className={cn(
        "flex flex-col h-full border-r border-border bg-card transition-all duration-200 shrink-0",
        collapsed ? "w-16" : "w-60",
      )}
    >
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-border">
        {!collapsed && (
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 rounded bg-primary flex items-center justify-center">
              <span className="text-primary-foreground text-xs font-bold">S</span>
            </div>
            <span className="font-semibold text-sm">Sindri Console</span>
          </div>
        )}
        {collapsed && (
          <div className="w-6 h-6 rounded bg-primary flex items-center justify-center mx-auto">
            <span className="text-primary-foreground text-xs font-bold">S</span>
          </div>
        )}
        {!collapsed && (
          <Button variant="ghost" size="icon" onClick={toggleSidebar} className="h-7 w-7 shrink-0">
            <ChevronLeft className="h-4 w-4" />
          </Button>
        )}
      </div>

      {/* Search / Command Palette trigger */}
      <div className="px-2 py-2 border-b border-border">
        <button
          onClick={() => openPalette("command")}
          className={cn(
            "flex items-center gap-2 w-full rounded-md px-3 py-1.5 text-sm text-muted-foreground bg-muted/50 hover:bg-muted transition-colors",
            collapsed && "justify-center px-2",
          )}
          title={collapsed ? "Command palette (⌘K)" : undefined}
        >
          <Search className="h-3.5 w-3.5 shrink-0" />
          {!collapsed && (
            <>
              <span className="flex-1 text-left text-xs">Search...</span>
              <kbd className="text-xs bg-background border border-border rounded px-1 font-mono">
                ⌘K
              </kbd>
            </>
          )}
        </button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-2 space-y-1">
        {NAV_ITEMS.map(({ to, label, icon: Icon }) => {
          const isActive = currentPath.startsWith(to);
          return (
            <Link
              key={to}
              to={to}
              className={cn(
                "flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors",
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                collapsed && "justify-center px-2",
              )}
              title={collapsed ? label : undefined}
            >
              <Icon className="h-4 w-4 shrink-0" />
              {!collapsed && <span>{label}</span>}
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-2 border-t border-border space-y-1">
        <Button
          variant="ghost"
          size={collapsed ? "icon" : "sm"}
          onClick={() => setTheme(nextTheme)}
          className={cn("w-full", collapsed ? "h-9 w-9 mx-auto flex" : "justify-start gap-3 px-3")}
          title={`Theme: ${theme}`}
        >
          <ThemeIcon className="h-4 w-4 shrink-0" />
          {!collapsed && <span>Theme: {theme}</span>}
        </Button>

        {collapsed && (
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleSidebar}
            className="h-9 w-9 mx-auto flex"
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        )}
      </div>
    </aside>
  );
}
