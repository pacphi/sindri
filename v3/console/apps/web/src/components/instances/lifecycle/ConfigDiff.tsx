import { useMemo } from "react";

interface ConfigDiffProps {
  original: string;
  modified: string;
  label?: string;
}

interface DiffLine {
  type: "added" | "removed" | "unchanged";
  content: string;
  lineNum: number;
}

function computeDiff(original: string, modified: string): DiffLine[] {
  const origLines = original.split("\n");
  const modLines = modified.split("\n");
  const result: DiffLine[] = [];

  // Simple line-by-line diff using LCS approach
  const m = origLines.length;
  const n = modLines.length;

  // Build LCS table
  const lcs: number[][] = Array.from({ length: m + 1 }, () => new Array<number>(n + 1).fill(0));
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (origLines[i - 1] === modLines[j - 1]) {
        lcs[i][j] = lcs[i - 1][j - 1] + 1;
      } else {
        lcs[i][j] = Math.max(lcs[i - 1][j], lcs[i][j - 1]);
      }
    }
  }

  // Backtrack to build diff
  const diffEntries: Array<{ type: "added" | "removed" | "unchanged"; content: string }> = [];
  let i = m;
  let j = n;
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && origLines[i - 1] === modLines[j - 1]) {
      diffEntries.unshift({ type: "unchanged", content: origLines[i - 1] ?? "" });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || (lcs[i][j - 1] ?? 0) >= (lcs[i - 1]?.[j] ?? 0))) {
      diffEntries.unshift({ type: "added", content: modLines[j - 1] ?? "" });
      j--;
    } else {
      diffEntries.unshift({ type: "removed", content: origLines[i - 1] ?? "" });
      i--;
    }
  }

  let lineNum = 1;
  for (const entry of diffEntries) {
    result.push({ ...entry, lineNum: lineNum++ });
  }

  return result;
}

export function ConfigDiff({ original, modified, label }: ConfigDiffProps) {
  const diff = useMemo(() => computeDiff(original, modified), [original, modified]);

  const hasChanges = diff.some((l) => l.type !== "unchanged");
  const addedCount = diff.filter((l) => l.type === "added").length;
  const removedCount = diff.filter((l) => l.type === "removed").length;

  return (
    <div className="space-y-2">
      {label && <p className="text-sm font-medium text-muted-foreground">{label}</p>}
      <div className="flex items-center gap-3 text-xs text-muted-foreground">
        {hasChanges ? (
          <>
            <span className="text-green-600 font-mono">+{addedCount}</span>
            <span className="text-red-600 font-mono">-{removedCount}</span>
          </>
        ) : (
          <span className="text-muted-foreground">No changes</span>
        )}
      </div>
      <div className="rounded-md border border-border overflow-auto max-h-72 text-xs font-mono bg-muted/30">
        {diff.map((line) => (
          <div
            key={line.lineNum}
            className={[
              "flex items-start px-3 py-0.5 leading-5",
              line.type === "added" && "bg-green-500/10 text-green-700 dark:text-green-400",
              line.type === "removed" && "bg-red-500/10 text-red-700 dark:text-red-400",
              line.type === "unchanged" && "text-foreground/70",
            ]
              .filter(Boolean)
              .join(" ")}
          >
            <span className="select-none w-5 shrink-0 text-muted-foreground/50 mr-3">
              {line.type === "added" ? "+" : line.type === "removed" ? "-" : " "}
            </span>
            <span className="whitespace-pre">{line.content}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
