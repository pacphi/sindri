import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

interface Port {
  port: number;
  protocol: "TCP" | "UDP";
  service: string;
  status: "open" | "closed" | "filtered";
  risk: "high" | "medium" | "low";
}

interface Props {
  instanceName?: string;
  ports?: Port[];
  loading?: boolean;
}

// Default well-known ports for demonstration
const DEFAULT_PORTS: Port[] = [
  { port: 22, protocol: "TCP", service: "SSH", status: "open", risk: "medium" },
  { port: 80, protocol: "TCP", service: "HTTP", status: "open", risk: "low" },
  { port: 443, protocol: "TCP", service: "HTTPS", status: "open", risk: "low" },
  { port: 5432, protocol: "TCP", service: "PostgreSQL", status: "filtered", risk: "high" },
  { port: 6379, protocol: "TCP", service: "Redis", status: "filtered", risk: "high" },
  { port: 8080, protocol: "TCP", service: "HTTP Alt", status: "closed", risk: "low" },
];

function portRiskVariant(risk: string): "error" | "warning" | "success" | "muted" {
  switch (risk) {
    case "high":
      return "error";
    case "medium":
      return "warning";
    case "low":
      return "success";
    default:
      return "muted";
  }
}

function statusColor(status: string): string {
  switch (status) {
    case "open":
      return "text-emerald-500";
    case "filtered":
      return "text-amber-500";
    default:
      return "text-muted-foreground";
  }
}

export function NetworkExposure({ instanceName, ports = DEFAULT_PORTS, loading }: Props) {
  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Network Exposure</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-2">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="h-10 bg-muted rounded" />
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  const openCount = ports.filter((p) => p.status === "open").length;
  const highRisk = ports.filter((p) => p.risk === "high" && p.status === "open").length;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Network Exposure</CardTitle>
          <div className="flex items-center gap-2">
            {highRisk > 0 && <Badge variant="error">{highRisk} high risk</Badge>}
            <span className="text-sm text-muted-foreground">{openCount} open ports</span>
          </div>
        </div>
        {instanceName && <p className="text-xs text-muted-foreground mt-1">{instanceName}</p>}
      </CardHeader>
      <CardContent>
        <div className="space-y-1.5">
          <div className="grid grid-cols-[auto_1fr_auto_auto] gap-3 text-xs text-muted-foreground font-medium pb-1 border-b">
            <span>Port</span>
            <span>Service</span>
            <span>Status</span>
            <span>Risk</span>
          </div>
          {ports.map((port) => (
            <div
              key={`${port.port}:${port.protocol}`}
              className="grid grid-cols-[auto_1fr_auto_auto] gap-3 text-sm items-center py-1"
            >
              <span className="font-mono text-xs w-12">
                {port.port}/{port.protocol}
              </span>
              <span className="text-sm">{port.service}</span>
              <span className={`text-xs font-medium ${statusColor(port.status)}`}>
                {port.status}
              </span>
              <Badge variant={portRiskVariant(port.risk)} className="text-xs">
                {port.risk}
              </Badge>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
