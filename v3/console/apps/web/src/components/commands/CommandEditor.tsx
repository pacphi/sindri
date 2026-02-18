import { useRef, useEffect } from "react";
import { cn } from "@/lib/utils";

interface CommandEditorProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit?: () => void;
  placeholder?: string;
  disabled?: boolean;
  rows?: number;
  className?: string;
}

// Simple syntax-highlighted command editor using a contenteditable overlay approach.
// We use a <textarea> as the input and a <pre> overlay for highlighting.
// Highlighted tokens: flags (--flag), env vars (KEY=value), pipes (|), redirects (>, >>), paths (/...)
function highlightCommand(cmd: string): string {
  // Escape HTML entities first
  const escaped = cmd.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  // Apply highlights in order (most specific to least)
  return (
    escaped
      // String literals (single-quoted)
      .replace(/('(?:[^'\\]|\\.)*')/g, '<span class="cmd-string">$1</span>')
      // String literals (double-quoted)
      .replace(/("(?:[^"\\]|\\.)*")/g, '<span class="cmd-string">$1</span>')
      // Environment variables KEY=value
      .replace(/\b([A-Z_][A-Z0-9_]*)=/g, '<span class="cmd-env">$1=</span>')
      // Long flags --flag
      .replace(/(--[a-zA-Z][a-zA-Z0-9_-]*)/g, '<span class="cmd-flag">$1</span>')
      // Short flags -f
      .replace(/(?<=\s|^)(-[a-zA-Z][a-zA-Z0-9]*)/g, '<span class="cmd-flag">$1</span>')
      // Pipe and redirects (already escaped >)
      .replace(/(\|)/g, '<span class="cmd-pipe">$1</span>')
      // Paths starting with /
      .replace(/(\/[a-zA-Z0-9_./-]+)/g, '<span class="cmd-path">$1</span>')
  );
}

export function CommandEditor({
  value,
  onChange,
  onSubmit,
  placeholder = "Enter command...",
  disabled = false,
  rows = 3,
  className,
}: CommandEditorProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const preRef = useRef<HTMLPreElement>(null);

  // Sync scroll position between textarea and highlight overlay
  function syncScroll() {
    if (preRef.current && textareaRef.current) {
      preRef.current.scrollTop = textareaRef.current.scrollTop;
      preRef.current.scrollLeft = textareaRef.current.scrollLeft;
    }
  }

  useEffect(() => {
    if (preRef.current) {
      preRef.current.innerHTML = highlightCommand(value) + "\n";
    }
  }, [value]);

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    // Ctrl+Enter or Cmd+Enter to submit
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      onSubmit?.();
    }
    // Tab inserts spaces instead of moving focus
    if (e.key === "Tab") {
      e.preventDefault();
      const ta = textareaRef.current;
      if (!ta) return;
      const start = ta.selectionStart;
      const end = ta.selectionEnd;
      const newValue = value.substring(0, start) + "  " + value.substring(end);
      onChange(newValue);
      // Restore cursor position after React re-render
      requestAnimationFrame(() => {
        ta.selectionStart = ta.selectionEnd = start + 2;
      });
    }
  }

  return (
    <div
      className={cn(
        "relative font-mono text-sm rounded-md border border-input bg-background overflow-hidden",
        disabled && "opacity-50",
        className,
      )}
    >
      <style>{`
        .cmd-flag   { color: #60a5fa; }
        .cmd-env    { color: #f59e0b; }
        .cmd-string { color: #34d399; }
        .cmd-pipe   { color: #f472b6; font-weight: bold; }
        .cmd-path   { color: #a78bfa; }
      `}</style>

      {/* Syntax highlight overlay (visually behind the textarea) */}
      <pre
        ref={preRef}
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 m-0 overflow-auto whitespace-pre-wrap break-words p-3 text-sm font-mono leading-relaxed"
        style={{ wordBreak: "break-all" }}
      />

      {/* Actual textarea (transparent text so highlight shows through) */}
      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        onScroll={syncScroll}
        disabled={disabled}
        placeholder={placeholder}
        rows={rows}
        spellCheck={false}
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        className={cn(
          "relative z-10 w-full resize-none bg-transparent p-3 leading-relaxed",
          "caret-foreground outline-none placeholder:text-muted-foreground",
          // Make text transparent so highlight layer shows, but keep caret visible
          "text-transparent selection:bg-blue-500/30",
          disabled && "cursor-not-allowed",
        )}
      />
    </div>
  );
}
