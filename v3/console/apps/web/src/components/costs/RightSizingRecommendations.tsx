import { useRightSizingRecommendations, useDismissRecommendation } from "@/hooks/useCosts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { TrendingDown, X, AlertCircle } from "lucide-react";

interface RightSizingRecommendationsProps {
  className?: string;
}

function formatUsd(value: number): string {
  return `$${value.toFixed(2)}`;
}

function ConfidenceBadge({ confidence }: { confidence: number }) {
  const pct = Math.round(confidence * 100);
  const color =
    pct >= 80
      ? "text-emerald-600 bg-emerald-50 dark:bg-emerald-950"
      : pct >= 60
        ? "text-yellow-600 bg-yellow-50 dark:bg-yellow-950"
        : "text-muted-foreground bg-muted";
  return (
    <span className={cn("px-1.5 py-0.5 rounded text-[10px] font-medium", color)}>
      {pct}% confidence
    </span>
  );
}

export function RightSizingRecommendations({ className }: RightSizingRecommendationsProps) {
  const { data, isLoading } = useRightSizingRecommendations();
  const dismiss = useDismissRecommendation();

  const recommendations = data?.recommendations ?? [];
  const totalSavings = data?.totalSavingsUsdMo ?? 0;

  return (
    <Card className={cn("", className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">Right-Sizing Recommendations</CardTitle>
          {totalSavings > 0 && (
            <div className="flex items-center gap-1 text-xs text-emerald-600 font-semibold">
              <TrendingDown className="h-3.5 w-3.5" />
              Save {formatUsd(totalSavings)}/mo
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="h-20 rounded-lg bg-muted animate-pulse" />
            ))}
          </div>
        ) : recommendations.length === 0 ? (
          <div className="h-20 flex flex-col items-center justify-center gap-1 text-xs text-muted-foreground">
            <AlertCircle className="h-4 w-4" />
            No recommendations available
          </div>
        ) : (
          <div className="space-y-3">
            {recommendations.map((rec) => (
              <div key={rec.id} className="rounded-lg border p-3 space-y-2">
                <div className="flex items-start justify-between">
                  <div>
                    <div className="flex items-center gap-1.5">
                      <span className="text-xs font-medium">{rec.instanceName}</span>
                      <span className="text-[10px] text-muted-foreground capitalize">
                        {rec.provider}
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 mt-0.5">
                      <span className="text-[10px] font-mono bg-muted px-1 rounded">
                        {rec.currentTier}
                      </span>
                      <span className="text-[10px] text-muted-foreground">→</span>
                      <span className="text-[10px] font-mono bg-emerald-50 dark:bg-emerald-950 text-emerald-700 dark:text-emerald-300 px-1 rounded">
                        {rec.suggestedTier}
                      </span>
                      <ConfidenceBadge confidence={rec.confidence} />
                    </div>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <div className="text-right">
                      <div className="text-xs font-semibold text-emerald-600">
                        -{formatUsd(rec.savingsUsdMo)}/mo
                      </div>
                      <div className="text-[10px] text-muted-foreground">
                        {formatUsd(rec.currentUsdMo)} → {formatUsd(rec.suggestedUsdMo)}
                      </div>
                    </div>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-6 w-6 p-0 text-muted-foreground"
                      onClick={() => dismiss.mutate(rec.id)}
                      disabled={dismiss.isPending}
                      title="Dismiss"
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                </div>
                {/* Resource utilization */}
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <div className="flex justify-between text-[10px] text-muted-foreground mb-0.5">
                      <span>CPU avg</span>
                      <span>{rec.avgCpuPercent.toFixed(1)}%</span>
                    </div>
                    <div className="h-1 bg-muted rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full bg-blue-500 transition-all"
                        style={{ width: `${Math.min(rec.avgCpuPercent, 100)}%` }}
                      />
                    </div>
                  </div>
                  <div>
                    <div className="flex justify-between text-[10px] text-muted-foreground mb-0.5">
                      <span>Memory avg</span>
                      <span>{rec.avgMemPercent.toFixed(1)}%</span>
                    </div>
                    <div className="h-1 bg-muted rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full bg-emerald-500 transition-all"
                        style={{ width: `${Math.min(rec.avgMemPercent, 100)}%` }}
                      />
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
