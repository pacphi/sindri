---
name: ms
description: Intelligent MetaSaver command that analyzes prompt complexity and routes to optimal execution method
---

# 🧠 MetaSaver Intelligent Command Router

The `/ms` command analyzes your prompt and automatically routes to the most appropriate execution method.

## Automatic Routing Logic

### 🔴 Ultra-Complex Tasks → Hive-Mind

**Triggers:** Multi-package coordination, enterprise architecture, system-wide changes, 10+ files, migrations
**Keywords:** "enterprise", "architecture", "monorepo", "system-wide", "across packages", "standardize", "coordinate", "migration"
**Action:** `/hive-mind:hive-mind spawn "{prompt}" --queen-type adaptive --auto-spawn`

### 🟡 Medium-Complex Tasks → Claude Flow Swarm

**Triggers:** Multi-file implementations, API development, feature builds, research + coding
**Keywords:** "implement", "build", "create service", "API", "feature", "multi-file", "component", "testing"
**Action:** `/claude-flow-swarm "{prompt}" --strategy auto --review --testing`

### 🟢 Simple Tasks → Enhanced Claude

**Triggers:** Single file work, debugging, explanations, quick fixes
**Keywords:** "explain", "fix", "debug", "help with", "single file", "simple", "quick"
**Action:** Direct processing with Claude Code

## Internal Complexity Analysis

The command scores prompts based on:

- **Ultra-complex keywords:** +10-15 points each
- **Medium-complex keywords:** +5-9 points each
- **Multi-package scope:** +8 points
- **Integration complexity:** +3-5 points

**Routing Thresholds:**

- Score ≥25: Hive-Mind
- Score 7-24: Swarm
- Score <7: Claude

## Claude Thinking Levels

### `ultrathink` - Deep Analysis

**Used for:** Architecture decisions, complex problem-solving, performance optimization
**Triggers:** "architecture", "complex", "analyze", "optimization", "security", "design patterns"

### `think-harder` - Enhanced Analysis

**Used for:** Refactoring, algorithm design, system design
**Triggers:** "refactor", "optimize", "design", "algorithm", "patterns", "best practices"

### `think` - Standard Processing

**Used for:** Straightforward implementations, clear requirements

## Additional Tools (Used with any thinking level)

### Context7 - Library Documentation

**Used for:** Research-heavy tasks, learning new libraries, API documentation
**Triggers:** "research", "learn", "documentation", "library", "framework"

### Sequential Thinking - Complex Problem Solving

**Used for:** Multi-step analysis, architectural decisions, complex debugging
**Triggers:** "analyze", "architecture", "complex problem", "step by step", "debug complex"

## Usage Examples

### Ultra-Complex → Hive-Mind

/ms "Standardize error handling across all microservices in monorepo"

→ `/hive-mind:hive-mind spawn` + adaptive queen

### Medium-Complex → Swarm

/ms "Build JWT auth API with refresh tokens and tests"

→ `/claude-flow-swarm` + testing + review

### Simple → Enhanced Claude

/ms "Fix TypeScript error in user.service.ts line 45"

→ Direct Claude processing with think level

## Advanced Usage

### Override automatic routing

/ms "simple task" --force-hive-mind
/ms "complex task" --force-claude

### Explicit thinking levels (Claude commands)

/ms "design architecture" --ultrathink
/ms "refactor code" --think-harder
/ms "simple fix" --think

### Additional tools

/ms "research React patterns" --context7
/ms "analyze complex bug" --sequential-thinking
/ms "build with new framework" --context7 --sequential-thinking

### Utility options

/ms "any task" --dry-run # Show routing decision only
/ms "any task" --explain-routing # Show why route was chosen

The `/ms` command automatically determines the best execution path for maximum efficiency and quality.

## 🚨 CRITICAL: Enforcement Rules for Claude

**YOU MUST FOLLOW THESE STEPS - NO EXCEPTIONS:**

1. **ALWAYS analyze the prompt first** - Don't proceed directly to work
2. **Calculate complexity score** using the keywords and triggers above
3. **Route according to thresholds** - Don't ignore the routing logic
4. **If routing to Swarm/Hive-Mind:** Use those tools, don't do the work yourself
5. **If routing to Claude:** Apply appropriate thinking level

**FORBIDDEN BEHAVIORS:**

- ❌ Ignoring the routing logic and proceeding directly
- ❌ Not calculating complexity scores
- ❌ Doing medium/ultra-complex work yourself instead of routing
- ❌ Skipping the analysis step

**ACCOUNTABILITY:**

- If you ignore these rules, you're being lazy and not following user instructions
- The user specifically created this command for intelligent routing
- Respect the system and follow it properly
