import { useThemeStore } from "@/stores/themeStore";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Sun, Moon, Monitor } from "lucide-react";
import { cn } from "@/lib/utils";

const THEMES = [
  { value: "light" as const, label: "Light", icon: Sun },
  { value: "dark" as const, label: "Dark", icon: Moon },
  { value: "system" as const, label: "System", icon: Monitor },
];

export function SettingsPage() {
  const { theme, setTheme } = useThemeStore();

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">Settings</h1>
        <p className="text-sm text-muted-foreground mt-1">Manage your Console preferences</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Appearance</CardTitle>
          <CardDescription>Choose your preferred color scheme</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex gap-3">
            {THEMES.map(({ value, label, icon: Icon }) => (
              <Button
                key={value}
                variant={theme === value ? "default" : "outline"}
                size="sm"
                onClick={() => setTheme(value)}
                className={cn("gap-2", theme === value && "pointer-events-none")}
              >
                <Icon className="h-4 w-4" />
                {label}
              </Button>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>API Connection</CardTitle>
          <CardDescription>Configure the Sindri Console API endpoint</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-muted-foreground">
            <p>
              The frontend connects to the API via the{" "}
              <code className="px-1 py-0.5 rounded bg-muted font-mono text-xs">/api</code> proxy
              configured in{" "}
              <code className="px-1 py-0.5 rounded bg-muted font-mono text-xs">vite.config.ts</code>
              .
            </p>
            <p className="mt-2">
              In production, configure your reverse proxy to route{" "}
              <code className="px-1 py-0.5 rounded bg-muted font-mono text-xs">/api</code> to the
              API server.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
