# compahook

Persistent memory layer for Claude Code's /compact command. Improves post-compact context quality by extracting, scoring, and re-injecting high-signal information that would otherwise be lost during conversation compaction.

## The Problem

When Claude Code compacts a conversation, the built-in summary can lose important details:

- Architectural decisions
- Error resolutions
- File edit purposes
- Explicitly marked context (IMPORTANT:, REMEMBER:, etc.)

After compaction, the model may re-ask questions or forget critical constraints.

## The Solution

compahook provides three hooks that work in sequence around the compact cycle:

```
[Normal work] ─── PostToolUse ──→ logs edits/commands to working-state.jsonl
                                          │
[/compact triggers] ─ PreCompact ──→ reads transcript, scores items,
                                     writes compact-context.json,
                                     outputs compact instructions to stdout
                                          │
[Session resumes] ── SessionStart ─→ reads compact-context.json,
                                     injects structured markdown via
                                     additionalContext
```

The result: after compaction, the model receives a structured summary of decisions, modified files, unresolved issues, and explicitly marked context — ranked by relevance.

## Installation

The extension installs Node.js LTS and compahook via mise.

The post-install hook automatically runs `compahook install` to register three hooks in `~/.claude/settings.json`:

- `compahook-collect` (PostToolUse)
- `compahook-extract` (PreCompact)
- `compahook-restore` (SessionStart)

No manual configuration is required - compahook is ready to use immediately after installation.

## Verification

```bash
# Check installation status
compahook status

# Expected output:
# compahook: installed (PostToolUse, PreCompact, SessionStart)
```

## Usage

### View Project Metrics

```bash
# Current project metrics
compahook stats

# Live monitoring (refreshes every 2s)
compahook watch

# Custom refresh interval (5 seconds)
compahook watch 5000
```

### Global Metrics

Monitor compahook performance across all projects:

```bash
# Global metrics
compahook stats --global

# Live global monitoring
compahook watch --global
```

### Reset Metrics

```bash
# Clear metrics for current project
compahook reset-metrics
```

## How It Works

### 1. Collector (PostToolUse)

Every time Claude Code uses Write, Edit, MultiEdit, or Bash, the collector appends a one-line JSON entry to `<project>/.claude/memory/working-state.jsonl`:

```jsonl
{"ts":1706000000,"type":"file_edit","file":"src/auth.js","tool":"Write"}
{"ts":1706000001,"type":"command","cmd":"npm test"}
```

Automatically prunes to 500 lines (keeps newest 400).

### 2. Extractor (PreCompact)

When `/compact` triggers, the extractor:

- Reads the conversation transcript (JSONL provided by Claude Code)
- Classifies each message: goals, decisions, file edits, commands, errors, markers
- Loads previous context and increments cycle ages for carried items
- Deduplicates items using type-aware keys with NFKC normalization
- Scores items using: `recency(position²) × typeWeight × markerBoost × decay(cycleAge)`
- Filters stale items below threshold, enforces hard cap (500 items)
- Takes the top 30 scored items for output
- Merges with recent working-state entries
- Writes `<project>/.claude/memory/compact-context.json`
- Outputs natural-language compact instructions to stdout

### 3. Restorer (SessionStart)

When the session resumes after compaction, the restorer:

- Reads `compact-context.json` (only if < 5 minutes old)
- Formats a structured markdown summary (max 4000 chars)
- Outputs it as `additionalContext` which Claude Code injects into the new session

## Scoring System

Items are ranked by a composite score:

| Factor           | Formula                                                                         |
| ---------------- | ------------------------------------------------------------------------------- |
| **Recency**      | `(position / total)²` — recent items score higher                               |
| **Type weight**  | decision: 1.0, error: 1.5, goal: 0.85, file_edit: 0.7, command: 0.5, read: 0.3  |
| **Marker boost** | 1.5× multiplier for messages containing IMPORTANT:, REMEMBER:, NOTE:, CRITICAL: |
| **Cycle decay**  | `exp(-cycleAge / halfLife)` — items decay ~63% per 10 compaction cycles         |

## Configuration

Create `<project>/.claude/memory/config.json` to override defaults:

```json
{
  "maxContextSize": 4000,
  "maxItems": 30,
  "maxWorkingStateLines": 500,
  "pruneKeepLines": 400,
  "recencyExponent": 2,
  "markerKeywords": ["IMPORTANT:", "REMEMBER:", "NOTE:", "CRITICAL:", "TODO:", "FIXME:"],
  "markerBoost": 1.5,
  "stalenessMinutes": 5,
  "decayHalfLife": 10,
  "minScoreThreshold": 0.001,
  "maxPreservedItems": 500,
  "enableTelemetry": true,
  "typeWeights": {
    "decision": 1.0,
    "error": 1.5,
    "goal": 0.85,
    "file_edit": 0.7,
    "command": 0.5,
    "marker": 1.0,
    "read": 0.3,
    "generic": 0.2
  }
}
```

All fields are optional — unspecified values use defaults.

## Storage

Per-project memory is stored in `<project>/.claude/memory/`:

| File                   | Purpose                                                |
| ---------------------- | ------------------------------------------------------ |
| `working-state.jsonl`  | Rolling log of file edits and commands                 |
| `compact-context.json` | Structured context from last compaction                |
| `config.json`          | Optional per-project configuration                     |
| `metrics.json`         | Performance metrics and token savings tracking         |
| `telemetry.jsonl`      | Pipeline telemetry logs (when `enableTelemetry: true`) |

Global state is stored in `~/.claude/`:

| File                      | Purpose                                             |
| ------------------------- | --------------------------------------------------- |
| `compahook-projects.json` | Registry of active projects (max 200, self-pruning) |

**Important**: Add `.claude/memory/` to your `.gitignore`.

## Uninstalling

To remove hooks without uninstalling the package:

```bash
compahook uninstall
```

To completely uninstall:

```bash
npm uninstall -g compahook
```

The npm uninstall automatically removes hooks from `~/.claude/settings.json` via the preuninstall lifecycle script.

## Debug Mode

Enable debug logging with:

```bash
export COMPAHOOK_DEBUG=1
```

Debug output goes to stderr to avoid corrupting hook stdout.

## Requirements

- Node.js >= 18
- Claude Code CLI

## Security

compahook is hardened against common attacks:

- Path traversal protection
- Symlink attack prevention
- stdin DoS protection (1MB cap)
- Atomic file writes
- TOCTOU race protection
- Prototype pollution prevention
- Processing caps (5000 item limit, 1MB line limit, 10MB file size cap)

## Zero Dependencies

compahook uses only Node.js built-in modules (fs, path, os, readline). No external dependencies.

## License

MIT

## Resources

- [npm Package](https://www.npmjs.com/package/compahook)
