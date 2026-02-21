import { useState } from "react";
import { useBudgets, useCreateBudget, useUpdateBudget, useDeleteBudget } from "@/hooks/useCosts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { Budget, BudgetPeriod, CreateBudgetInput } from "@/types/cost";
import { Plus, Trash2, Pencil, X, Check, AlertTriangle } from "lucide-react";

const PERIOD_LABELS: Record<BudgetPeriod, string> = {
  DAILY: "Daily",
  WEEKLY: "Weekly",
  MONTHLY: "Monthly",
};

function formatUsd(value: number): string {
  return `$${value.toFixed(2)}`;
}

function BudgetRow({
  budget,
  onDelete,
  onSave,
}: {
  budget: Budget;
  onDelete: (id: string) => void;
  onSave: (id: string, name: string, amount: number, threshold: number) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(budget.name);
  const [amount, setAmount] = useState(String(budget.amountUsd));
  const [threshold, setThreshold] = useState(String(Math.round(budget.alertThreshold * 100)));

  const spent = budget.spentUsd ?? 0;
  const spentPct =
    budget.spentPercent ?? (budget.amountUsd > 0 ? (spent / budget.amountUsd) * 100 : 0);
  const isOver = spentPct >= 100;
  const isWarning = spentPct >= budget.alertThreshold * 100 && !isOver;

  const barColor = isOver ? "bg-destructive" : isWarning ? "bg-yellow-500" : "bg-primary";

  function handleSave() {
    const amt = parseFloat(amount);
    const thr = parseInt(threshold, 10) / 100;
    if (!isNaN(amt) && !isNaN(thr)) {
      onSave(budget.id, name, amt, thr);
      setEditing(false);
    }
  }

  return (
    <div className="rounded-lg border p-3 space-y-2">
      {editing ? (
        <div className="space-y-2">
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Budget name"
            className="h-7 text-xs"
          />
          <div className="flex gap-2">
            <div className="flex-1">
              <label className="text-[10px] text-muted-foreground">Amount (USD)</label>
              <Input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="h-7 text-xs"
              />
            </div>
            <div className="w-24">
              <label className="text-[10px] text-muted-foreground">Alert at %</label>
              <Input
                type="number"
                min="1"
                max="100"
                value={threshold}
                onChange={(e) => setThreshold(e.target.value)}
                className="h-7 text-xs"
              />
            </div>
          </div>
          <div className="flex gap-1">
            <Button size="sm" variant="ghost" className="h-6 px-2" onClick={handleSave}>
              <Check className="h-3 w-3" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="h-6 px-2"
              onClick={() => setEditing(false)}
            >
              <X className="h-3 w-3" />
            </Button>
          </div>
        </div>
      ) : (
        <>
          <div className="flex items-start justify-between">
            <div>
              <div className="flex items-center gap-1.5">
                <span className="text-xs font-medium">{budget.name}</span>
                {(isOver || isWarning) && (
                  <AlertTriangle
                    className={cn("h-3 w-3", isOver ? "text-destructive" : "text-yellow-500")}
                  />
                )}
              </div>
              <span className="text-[10px] text-muted-foreground">
                {PERIOD_LABELS[budget.period]}
                {budget.provider ? ` · ${budget.provider}` : ""}
              </span>
            </div>
            <div className="flex items-center gap-1">
              <Button
                size="sm"
                variant="ghost"
                className="h-6 w-6 p-0"
                onClick={() => setEditing(true)}
              >
                <Pencil className="h-3 w-3" />
              </Button>
              <Button
                size="sm"
                variant="ghost"
                className="h-6 w-6 p-0 text-destructive hover:text-destructive"
                onClick={() => onDelete(budget.id)}
              >
                <Trash2 className="h-3 w-3" />
              </Button>
            </div>
          </div>
          <div className="space-y-1">
            <div className="flex justify-between text-[10px]">
              <span className="text-muted-foreground">
                {formatUsd(spent)} of {formatUsd(budget.amountUsd)}
              </span>
              <span
                className={cn(
                  "font-semibold",
                  isOver && "text-destructive",
                  isWarning && "text-yellow-600",
                )}
              >
                {spentPct.toFixed(1)}%
              </span>
            </div>
            <div className="h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className={cn("h-full rounded-full transition-all", barColor)}
                style={{ width: `${Math.min(spentPct, 100)}%` }}
              />
            </div>
          </div>
        </>
      )}
    </div>
  );
}

interface CreateBudgetFormProps {
  onSubmit: (input: CreateBudgetInput) => void;
  onCancel: () => void;
}

function CreateBudgetForm({ onSubmit, onCancel }: CreateBudgetFormProps) {
  const [name, setName] = useState("");
  const [amount, setAmount] = useState("");
  const [period, setPeriod] = useState<BudgetPeriod>("MONTHLY");
  const [threshold, setThreshold] = useState("80");
  const [provider, setProvider] = useState("");

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const amt = parseFloat(amount);
    if (!name.trim() || isNaN(amt) || amt <= 0) return;
    onSubmit({
      name: name.trim(),
      amountUsd: amt,
      period,
      alertThreshold: parseInt(threshold, 10) / 100,
      provider: provider.trim() || undefined,
    });
  }

  return (
    <form onSubmit={handleSubmit} className="rounded-lg border p-3 space-y-2 bg-muted/30">
      <p className="text-xs font-medium">New Budget</p>
      <Input
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Budget name"
        className="h-7 text-xs"
        required
      />
      <div className="flex gap-2">
        <div className="flex-1">
          <label className="text-[10px] text-muted-foreground">Amount (USD)</label>
          <Input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="100.00"
            className="h-7 text-xs"
            required
          />
        </div>
        <div className="w-28">
          <label className="text-[10px] text-muted-foreground">Period</label>
          <select
            value={period}
            onChange={(e) => setPeriod(e.target.value as BudgetPeriod)}
            className="flex h-7 w-full rounded-md border border-input bg-background px-2 text-xs"
          >
            <option value="DAILY">Daily</option>
            <option value="WEEKLY">Weekly</option>
            <option value="MONTHLY">Monthly</option>
          </select>
        </div>
      </div>
      <div className="flex gap-2">
        <div className="flex-1">
          <label className="text-[10px] text-muted-foreground">Provider (optional)</label>
          <Input
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
            placeholder="fly, aws, gcp…"
            className="h-7 text-xs"
          />
        </div>
        <div className="w-24">
          <label className="text-[10px] text-muted-foreground">Alert at %</label>
          <Input
            type="number"
            min="1"
            max="100"
            value={threshold}
            onChange={(e) => setThreshold(e.target.value)}
            className="h-7 text-xs"
          />
        </div>
      </div>
      <div className="flex gap-1">
        <Button type="submit" size="sm" className="h-7 text-xs">
          Create
        </Button>
        <Button type="button" size="sm" variant="ghost" className="h-7 text-xs" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </form>
  );
}

interface BudgetManagerProps {
  className?: string;
}

export function BudgetManager({ className }: BudgetManagerProps) {
  const { data, isLoading } = useBudgets();
  const createBudget = useCreateBudget();
  const updateBudget = useUpdateBudget();
  const deleteBudget = useDeleteBudget();
  const [showForm, setShowForm] = useState(false);

  const budgets = data?.budgets ?? [];

  function handleCreate(input: CreateBudgetInput) {
    createBudget.mutate(input, { onSuccess: () => setShowForm(false) });
  }

  function handleSave(id: string, name: string, amountUsd: number, alertThreshold: number) {
    updateBudget.mutate({ id, input: { name, amountUsd, alertThreshold } });
  }

  function handleDelete(id: string) {
    deleteBudget.mutate(id);
  }

  return (
    <Card className={cn("", className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">Budgets</CardTitle>
          <Button
            size="sm"
            variant="outline"
            className="h-7 text-xs"
            onClick={() => setShowForm(true)}
            disabled={showForm}
          >
            <Plus className="h-3 w-3 mr-1" />
            Add Budget
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="h-16 rounded-lg bg-muted animate-pulse" />
            ))}
          </div>
        ) : (
          <div className="space-y-2">
            {showForm && (
              <CreateBudgetForm onSubmit={handleCreate} onCancel={() => setShowForm(false)} />
            )}
            {budgets.map((budget) => (
              <BudgetRow
                key={budget.id}
                budget={budget}
                onDelete={handleDelete}
                onSave={handleSave}
              />
            ))}
            {budgets.length === 0 && !showForm && (
              <div className="h-16 flex items-center justify-center text-xs text-muted-foreground">
                No budgets configured
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
