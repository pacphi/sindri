# Import to Ontology Skill

Intelligently moves content from source markdown files to appropriate ontology locations with semantic targeting, validation, and web content enrichment.

## Features

✅ **Semantic Targeting** - Uses in-memory ontology index for intelligent concept matching
✅ **Content Block Parsing** - Extracts headings, paragraphs, code blocks intelligently
✅ **WikiLink Detection** - Identifies broken links and suggests fixes
✅ **URL Enrichment** - Integrates with web-summary skill for stub expansion (async)
✅ **Assertion Validation** - Detects and validates claims, statistics, definitions
✅ **Safe Migration** - Creates backups before any modifications
✅ **Dry Run Mode** - Analyze before importing
✅ **Progress Tracking** - Real-time progress for batch imports

## Quick Start

```bash
# Dry run analysis
node ~/.claude/skills/import-to-ontology/import-engine.js source-file.md --dry-run

# Import with confirmation
node ~/.claude/skills/import-to-ontology/import-engine.js source-file.md

# Force import without dry run
node ~/.claude/skills/import-to-ontology/import-engine.js source-file.md --force
```

## Usage with Claude Code

```bash
# Single file import
claude-code "Use import-to-ontology skill to process research-notes.md"

# Directory import
claude-code "Use import-to-ontology skill to import all files from /sources/"

# Dry run first
claude-code "Use import-to-ontology skill with dry-run on blockchain-notes.md"
```

## How It Works

### 1. Content Block Parsing

Intelligently splits source files into semantic blocks:

- **Headings** - Sections starting with `#`
- **Paragraphs** - Continuous text blocks
- **Code Blocks** - Fenced code sections
- **Lists** - Bullet and numbered lists

Each block is analyzed for:

- Keywords (semantic matching)
- WikiLinks (relationship detection)
- URLs (enrichment candidates)
- Assertions (validation targets)

### 2. Semantic Targeting

Uses the ontology index to find optimal placement:

```typescript
// Score concepts by keyword and WikiLink overlap
const target = findTargetConcept(block);
// Returns: {
//   targetFile: "BC-0001-blockchain.md",
//   confidence: 0.85,
//   reasoning: "Matched 5 keywords, 2 links"
// }
```

Confidence levels:

- **High (>70%)**: Auto-import with logging
- **Medium (40-70%)**: Import with review flag
- **Low (<40%)**: Skip and flag for manual review

### 3. Stub Detection & Enrichment

**WikiLink Stubs**: Broken links without target concepts

```markdown
[[New Concept]] ← No file exists
```

→ Creates suggestion to generate concept file

**URL Stubs**: Isolated URLs without descriptions

```markdown
https://example.com/article ← No context
```

→ Calls web-summary skill to fetch title + summary

### 4. Web Summary Integration

For URL stubs, asynchronously calls the web-summary skill:

```typescript
// Async web content fetching (3-10s per URL)
const summary = await webSummarySkill(url);

// Returns enriched content:
{
  title: "Article Title",
  summary: "Key points from the article...",
  semanticLinks: ["[[Concept1]]", "[[Concept2]]"],
  citations: ["Author, Year"]
}
```

Processes URLs in batches of 5 for efficiency.

### 5. Assertion Validation

Detects claims that might be outdated:

- **Definitions** - "X is defined as..."
- **Statistics** - "42% of users..."
- **Citations** - "According to Smith (2020)..."
- **Claims** - "This enables..." / "This provides..."

Flags for manual review or auto-updates based on confidence.

### 6. Safe Content Migration

Before any changes:

1. Creates timestamped backup in `.backups/`
2. Logs all operations to `/tmp/import-ontology-<session>.log`
3. Validates target files exist
4. Inserts content at appropriate section

After successful import:

- Archives source file (or marks as processed)
- Updates ontology index (if applicable)
- Generates migration report

## Configuration

Create `.import-ontology.config.json` in project root:

```json
{
  "sourceDirectory": "/path/to/source/files",
  "ontologyDirectory": "/home/devuser/workspace/project/Metaverse-Ontology/logseq/pages",
  "backupDirectory": ".backups",
  "indexPath": ".cache/ontology-index.json",

  "webSummary": {
    "enabled": true,
    "concurrency": 5,
    "timeout": 10000
  },

  "targeting": {
    "minConfidence": 0.4
  },

  "safety": {
    "createBackups": true,
    "dryRunFirst": true
  }
}
```

## Output Example

```text
📋 Analyzing research-notes.md...

📊 DRY RUN REPORT

Source File: research-notes.md
Total Blocks: 12
Estimated Time: 3 minutes

🎯 Targeting Summary:
   High Confidence (>70%): 8
   Medium Confidence (40-70%): 3
   Low Confidence (<40%): 1

🔗 Enrichment Summary:
   URLs to enrich: 5
   WikiLinks to create: 2

📝 Sample Targets:

   Block: "# Blockchain Consensus Mechanisms..."
   → Blockchain (95% confidence)
     File: BC-0001-blockchain.md
     Reason: Matched 5 keywords, 2 links

   Block: "Smart contracts enable decentralized applications..."
   → Smart Contract (87% confidence)
     File: BC-0123-smart-contract.md
     Reason: Matched 4 keywords, 3 links

⚠️  WARNING: 5 URLs to enrich - this will be slow (~25 seconds)

ℹ️  Add --force flag to proceed with import
```

## Integration with Web Summary Skill

The skill integrates with the `web-summary` skill for URL enrichment:

```javascript
// Detect isolated URLs
const urls = ["https://example.com/blockchain-article"];

// Call web-summary skill (async)
const enriched = await webSummarySkill({
  url: urls[0],
  options: {
    maxLength: 300,
    includeSemanticLinks: true,
    format: "logseq",
  },
});

// Insert enriched content
const formatted = `
- **Source**:
  - ${enriched.summary}
  - **Key Points**: ${enriched.keyPoints.join(", ")}
  - **Related**: ${enriched.semanticLinks.join(", ")}
`;
```

## Performance

**Typical import (50 blocks, 10 URLs)**:

- Parsing: <1s
- Semantic targeting: ~2s
- URL enrichment: ~50s (10 URLs × 5s avg)
- Content insertion: ~5s
- **Total**: ~60s

**Optimization tips**:

- Process files in batches
- Disable web enrichment for faster imports (`webSummary.enabled: false`)
- Increase concurrency for more URLs (`webSummary.concurrency: 10`)

## Files

```text
~/.claude/skills/import-to-ontology/
├── SKILL.md                 # Full skill documentation
├── README.md                # This file
├── import-engine.js         # Core implementation
└── .import-ontology.config.json  # Configuration (optional)
```

## Dependencies

- **Ontology Index**: Requires `.cache/ontology-index.json` (generate with `node scripts/generate-index.js`)
- **Web Summary Skill**: Optional but recommended for URL enrichment
- **Node.js**: v14+ required

## Troubleshooting

**Index not found**:

```bash
# Generate index first
cd /home/devuser/workspace/project/Metaverse-Ontology
node scripts/generate-index.js
```

**Low confidence targeting**:

- Check if source content has WikiLinks to existing concepts
- Add more domain-specific keywords
- Manually specify target file

**Web summary timeout**:

```json
{
  "webSummary": {
    "timeout": 20000 // Increase to 20s
  }
}
```
