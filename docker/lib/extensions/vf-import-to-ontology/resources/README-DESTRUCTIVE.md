# Import to Ontology - Destructive Mode

⚠️ **WARNING: DESTRUCTIVE OPERATIONS - NO BACKUPS** ⚠️

This skill **MOVES** content from source files to ontology files and **DELETES** empty source files. **NO BACKUPS ARE CREATED** - ensure your source files are backed up externally before running.

## What This Does

1. **Moves Content** - Extracts blocks from source files and moves them to target ontology files
2. **Handles Images** - Updates image references to shared `assets/` folder
3. **Deletes Empty Files** - Automatically removes source files when all content is moved
4. **Tracks Progress** - Resume capability for large batches
5. **NO BACKUPS** - All operations are permanent

## Quick Start

### Step 1: Prepare Directories

```bash
cd /home/devuser/workspace/project/Metaverse-Ontology

# Verify source directory structure
ls sourceMarkdown/pages/
# Should contain: *.md files

ls sourceMarkdown/assets/
# Should contain: image files (PNG, JPG, etc.)
```

### Step 2: Check Assets

```bash
# Generate asset report
node ~/.claude/skills/import-to-ontology/asset-handler.js \
  sourceMarkdown/pages/ \
  sourceMarkdown/assets/ \
  --report

# Output shows:
# - Files with images
# - Total images
# - Missing assets (if any)
```

### Step 3: Run Import

```bash
# Process directory in batches of 5
node ~/.claude/skills/import-to-ontology/destructive-import.js \
  sourceMarkdown/pages/ \
  logseq/pages/ \
  --batch-size=5
```

## How It Works

### Batch Processing

Large files are processed one at a time or in small batches (default: 5):

```text
📦 BATCH 1: Processing 5 files
────────────────────────────────

📄 Processing: file1.md
   📦 Found 15 content blocks
   [1/15] Block block-1: Blockchain (85%)
      ✅ Moved to BC-0001-blockchain.md
   [2/15] Block block-2: Smart Contract (92%)
      ✅ Moved to BC-0123-smart-contract.md
   ...
   🗑️  Deleting empty source file: file1.md

   📊 Summary: 12 moved, 3 skipped, DELETED

📄 Processing: file2.md
   ...
```

### Image Asset Handling

Images are automatically detected and updated:

```markdown
# Before (in source file)

# After (in target ontology file)
```

**Supported formats**:

- Markdown: ``
- WikiLink: `![[image.png]]`
- HTML: `<img src="path">`

### Progress Tracking

Progress is saved to `/tmp/import-progress.json`:

```json
{
  "sessionId": "import-1730210000-xyz",
  "sourceDir": "/path/to/sources",
  "totalFiles": 200,
  "filesProcessed": 45,
  "filesDeleted": 42,
  "blocksMoved": 543,
  "assetsHandled": 87,
  "processedFiles": ["file1.md", "file2.md", ...],
  "errors": []
}
```

If interrupted, resume by running the same command again.

### Source File Deletion

Files are deleted when "effectively empty":

- Only metadata/frontmatter remains
- Fewer than 3 substantial content lines
- All meaningful blocks have been moved

**Example empty file**:

```markdown
---
title: Old Document
---

# Document

(All content blocks moved to ontology files)
```

## Configuration

Edit `~/.claude/skills/import-to-ontology/destructive-import.js`:

```javascript
const CONFIG = {
  indexPath: ".cache/ontology-index.json",
  backupDir: ".backups",
  progressFile: "/tmp/import-progress.json",
  assetsDir: "assets/",
  batchSize: 5, // Files per batch
  minConfidence: 0.4, // Targeting threshold
};
```

## Safety Features

### 1. Progress Tracking

Resume interrupted imports:

```bash
# Run once - processes 5 files
node destructive-import.js /sources/ /target/

# Interrupt (Ctrl+C)

# Run again - resumes from file 6
node destructive-import.js /sources/ /target/
```

### 2. Error Handling

Errors don't stop the batch:

```text
❌ Error processing file3.md: Target not found
   ⏭️  Continuing with next file...
```

All errors logged to progress file.

### 3. Asset Validation

Missing assets are detected and warned:

```text
⚠️  Asset not found: diagram.png
   Kept original path as fallback
```

## Example Output

```text
🚀 Starting DESTRUCTIVE import...

   Source: /home/user/sources
   Target: /home/user/ontology/logseq/pages
   Assets: /home/user/ontology/assets

📋 Copying assets to shared folder...
   ✅ Copied 15 assets to shared folder

📂 Files: 200 pending (0 already processed)

============================================================
📦 BATCH 1: Processing 5 files
============================================================

📄 Processing: blockchain-notes.md
   📦 Found 23 content blocks
   🖼️  Found 3 image references
      Updated: ./img/consensus.png → assets/consensus.png
      Updated: ./img/merkle.png → assets/merkle.png
      Updated: ./img/pow.png → assets/pow.png
   [1/23] Block block-1: Blockchain (95%)
      ✅ Moved to BC-0001-blockchain.md
   [2/23] Block block-2: Consensus Mechanism (88%)
      ✅ Moved to BC-0050-consensus-mechanism.md
   ...
   [23/23] Block block-23: Byzantine Fault Tolerance (76%)
      ✅ Moved to BC-0075-byzantine-fault-tolerance.md
   🗑️  Deleting empty source file: blockchain-notes.md

   📊 Summary: 20 moved, 3 skipped, DELETED

📊 Progress: 1/200 files (0.5%)
   Blocks moved: 20
   Files deleted: 1
   Assets handled: 3
   Errors: 0

...

============================================================
✅ IMPORT COMPLETE
============================================================
Files processed: 200/200
Files deleted: 185
Blocks moved: 2,847
Assets handled: 234
Errors: 2
Duration: 47 minutes

📁 Progress file: /tmp/import-progress.json
```

## Troubleshooting

### Issue: Index not found

```text
Error: Index not found: .cache/ontology-index.json
Run: node scripts/generate-index.js
```

**Fix**:

```bash
cd /home/devuser/workspace/project/Metaverse-Ontology
node scripts/generate-index.js
```

### Issue: Assets not copying

**Check**:

1. Source has `assets/` folder
2. Target `assets/` folder is writable
3. Asset file names don't conflict

**Debug**:

```bash
node ~/.claude/skills/import-to-ontology/asset-handler.js \
  /path/to/sources/ \
  ./assets/ \
  --report
```

### Issue: Too many files skipped

**Causes**:

- Low confidence targeting (<40%)
- Missing target concepts

**Fix**:

```bash
# Lower confidence threshold
# Edit destructive-import.js: minConfidence: 0.3
```

### Issue: Want to undo

**No undo available - files are permanently modified**:

- Ensure you have external backups before running
- Progress tracking in `/tmp/import-progress.json` shows what was modified
- Consider using version control (git) on source files before import

## Performance

**Typical performance** (200 files):

| Metric           | Value           |
| ---------------- | --------------- |
| Processing time  | 45-60 minutes   |
| Files per minute | 3-4 files/min   |
| Blocks per file  | 10-25 blocks    |
| Assets per file  | 1-3 images      |
| Deletion rate    | 85-95% of files |

**Bottlenecks**:

- Semantic targeting (~2s per file)
- File I/O (backups, reads, writes)
- Large files (>500 blocks)

**Optimization**:

- Increase `batchSize` for faster processing
- Disable web enrichment if not needed
- Use SSD for better I/O performance

## Best Practices

### 1. Always Dry Run First

```bash
# Check what will happen
node ~/.claude/skills/import-to-ontology/import-engine.js \
  /sources/file1.md \
  --dry-run
```

### 2. Start with Small Batch

```bash
# Test with 5 files first
node destructive-import.js /sources/ /target/ --batch-size=5
```

### 3. Monitor Progress

```bash
# Watch progress file in another terminal
watch -n 5 'cat /tmp/import-progress.json | jq ".filesProcessed, .blocksMoved"'
```

### 4. Verify Assets

```bash
# After import, check assets are intact
ls -lh assets/ | wc -l
```

## Files

```text
~/.claude/skills/import-to-ontology/
├── SKILL.md                  # Full documentation
├── README.md                 # Quick start
├── README-DESTRUCTIVE.md     # This file
├── import-engine.js          # Original (non-destructive)
├── destructive-import.js     # DESTRUCTIVE batch processor
└── asset-handler.js          # Image asset management
```

## See Also

- [Main Skill Documentation](./SKILL.md)
