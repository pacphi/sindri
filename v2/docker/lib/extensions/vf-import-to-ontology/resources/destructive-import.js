#!/usr/bin/env node

/**
 * Destructive Import Engine
 *
 * MOVES content from source files to target ontology files.
 * - Processes large files one at a time or in small batches
 * - Handles image asset references
 * - Enriches isolated URLs with web-summary skill
 * - Deletes source files when empty
 * - Tracks progress with resume capability
 * - NO BACKUPS CREATED (source files are destructively modified)
 */

const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');
const { detectImageReferences, updateImageReferences, copyAssets } = require('./asset-handler');
const { findBestMatch } = require('./llm-matcher');

// Configuration
const CONFIG = {
  indexPath: path.join(process.cwd(), '.cache/ontology-index.json'),
  progressFile: '/tmp/import-progress.json',
  sourceDir: path.join(process.cwd(), 'sourceMarkdown/pages'),
  assetsDir: path.join(process.cwd(), 'sourceMarkdown/assets'),
  targetDir: path.join(process.cwd(), 'logseq/pages'),
  batchSize: 5,  // Process 5 files at a time
  minConfidence: 0.15,  // Lowered from 0.4 to work with improved semantic matching
  useLLM: true,  // Enable LLM-based fuzzy matching for ambiguous cases
};

// Global state
let INDEX = null;
let PROGRESS = null;

/**
 * Load ontology index
 */
function loadIndex() {
  if (INDEX) return INDEX;

  if (!fs.existsSync(CONFIG.indexPath)) {
    throw new Error(`Index not found: ${CONFIG.indexPath}\nRun: node scripts/generate-index.js`);
  }

  const data = fs.readFileSync(CONFIG.indexPath, 'utf-8');
  INDEX = JSON.parse(data);
  return INDEX;
}

/**
 * Load or initialize progress tracker
 */
function loadProgress(sourceDir) {
  if (fs.existsSync(CONFIG.progressFile)) {
    const data = JSON.parse(fs.readFileSync(CONFIG.progressFile, 'utf-8'));

    // Resume if same source directory
    if (data.sourceDir === sourceDir) {
      console.log(`📂 Resuming previous session (${data.filesProcessed}/${data.totalFiles} files completed)\n`);
      return data;
    }
  }

  // Initialize new progress
  const files = fs.readdirSync(sourceDir).filter(f => f.endsWith('.md'));

  return {
    sessionId: generateSessionId(),
    sourceDir,
    startTime: new Date().toISOString(),
    totalFiles: files.length,
    filesProcessed: 0,
    filesDeleted: 0,
    blocksMoved: 0,
    assetsHandled: 0,
    errors: [],
    processedFiles: [],
  };
}

/**
 * Save progress
 */
function saveProgress(progress) {
  fs.writeFileSync(CONFIG.progressFile, JSON.stringify(progress, null, 2));
}

/**
 * Generate session ID
 */
function generateSessionId() {
  return `import-${Date.now()}-${Math.random().toString(36).substring(7)}`;
}

/**
 * Parse source file into blocks
 */
function parseSourceFile(filePath) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const blocks = [];
  let currentBlock = null;
  let blockId = 1;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Handle both standard markdown (# Heading) and Logseq format (- # Heading)
    const isHeading = line.startsWith('#') || /^-\s+#{1,6}\s/.test(trimmed);

    if (isHeading) {
      if (currentBlock) {
        currentBlock.endLine = i - 1;
        blocks.push(completeBlock(currentBlock));
      }

      currentBlock = {
        id: `block-${blockId++}`,
        type: 'heading',
        content: line,
        startLine: i,
      };
    } else if (line.startsWith('```')) {
      if (currentBlock) {
        currentBlock.endLine = i - 1;
        blocks.push(completeBlock(currentBlock));
      }

      let endLine = i + 1;
      while (endLine < lines.length && !lines[endLine].startsWith('```')) {
        endLine++;
      }

      blocks.push({
        id: `block-${blockId++}`,
        type: 'code',
        content: lines.slice(i, endLine + 1).join('\n'),
        startLine: i,
        endLine: endLine,
      });

      currentBlock = null;
      i = endLine;
    } else if (currentBlock) {
      currentBlock.content += '\n' + line;
    } else if (line.trim()) {
      currentBlock = {
        id: `block-${blockId++}`,
        type: 'paragraph',
        content: line,
        startLine: i,
      };
    }
  }

  if (currentBlock) {
    currentBlock.endLine = lines.length - 1;
    blocks.push(completeBlock(currentBlock));
  }

  return { blocks };
}

/**
 * Complete block with metadata
 */
function completeBlock(block) {
  block.metadata = {
    keywords: extractKeywords(block.content),
    wikiLinks: extractWikiLinks(block.content),
    urls: extractUrls(block.content),
    images: detectImageReferences(block.content),
  };
  return block;
}

/**
 * Extract keywords from text
 */
function extractKeywords(text) {
  const words = text.toLowerCase().replace(/[^a-z0-9\s]/g, ' ').split(/\s+/).filter(w => w.length > 3);
  const stopwords = new Set(['this', 'that', 'with', 'from', 'have', 'been', 'were', 'they', 'what', 'when', 'where']);
  return [...new Set(words.filter(w => !stopwords.has(w)))];
}

/**
 * Extract WikiLinks from text
 */
function extractWikiLinks(text) {
  const regex = /\[\[([^\]]+)\]\]/g;
  const links = [];
  let match;
  while ((match = regex.exec(text)) !== null) {
    links.push(match[1]);
  }
  return [...new Set(links)];
}

/**
 * Extract URLs from text
 */
function extractUrls(text) {
  const regex = /(https?:\/\/[^\s\)]+)/g;
  const matches = text.match(regex);
  return matches ? [...new Set(matches)] : [];
}

/**
 * Find target concept for block using semantic matching
 */
async function findTargetConcept(block) {
  const index = loadIndex();

  // Use new LLM-based semantic matcher
  const match = await findBestMatch(block.content, index, {
    useLLM: CONFIG.useLLM,
    minConfidence: CONFIG.minConfidence,
    topK: 5
  });

  if (!match || match.score < CONFIG.minConfidence) {
    return {
      blockId: block.id,
      targetFile: null,
      confidence: match?.score || 0,
      reasoning: match ? `Low confidence (${(match.score * 100).toFixed(1)}%)` : 'No matches'
    };
  }

  return {
    blockId: block.id,
    targetFile: match.concept.file,
    targetConcept: match.concept.preferredTerm,
    confidence: Math.min(match.score, 0.95),
    reasoning: `${match.method} match: ${match.reasoning}`,
  };
}

/**
 * Detect isolated URLs in content that need enrichment
 */
function detectIsolatedUrls(content) {
  const urlPattern = /https?:\/\/[^\s\[\]()]+/g;
  const matches = content.match(urlPattern) || [];
  return matches.filter((url, index, arr) => arr.indexOf(url) === index); // Unique URLs
}

/**
 * Enrich content with web-summary for isolated URLs
 */
async function enrichContentWithWebSummaries(content) {
  const urls = detectIsolatedUrls(content);
  if (urls.length === 0) return content;

  let enrichedContent = content;

  for (const url of urls) {
    try {
      const summary = await getWebSummary(url);
      if (summary) {
        // Add summary as a blockquote after the URL
        enrichedContent = enrichedContent.replace(
          url,
          `${url}\n> **Summary:** ${summary.substring(0, 200)}${summary.length > 200 ? '...' : ''}`
        );
      }
    } catch (error) {
      // Silently fail - keep original content
    }
  }

  return enrichedContent;
}

/**
 * Call web-summary skill via Z.AI or direct tool
 */
async function getWebSummary(url, timeout = 5000) {
  return new Promise((resolve) => {
    const child = spawn('node', [
      '-e',
      `
        const { spawn } = require('child_process');
        const curl = spawn('curl', [
          '-X', 'POST',
          'http://localhost:9600/chat',
          '-H', 'Content-Type: application/json',
          '-d', JSON.stringify({
            prompt: 'Summarize this URL in 1-2 sentences: ${url}',
            timeout: 3000,
            max_tokens: 100
          }),
          '--max-time', '4'
        ]);

        let output = '';
        curl.stdout.on('data', (data) => { output += data; });
        curl.on('close', () => {
          try {
            const result = JSON.parse(output);
            console.log(result.summary || result.response || '');
          } catch (e) {
            console.log('');
          }
        });
      `
    ]);

    let output = '';
    child.stdout.on('data', (data) => {
      output += data.toString();
    });

    const timer = setTimeout(() => {
      child.kill();
      resolve(null);
    }, timeout);

    child.on('close', () => {
      clearTimeout(timer);
      resolve(output.trim() || null);
    });
  });
}

/**
 * Remove block content from source file (DESTRUCTIVE)
 */
function removeBlockFromSource(sourceFile, block) {
  const content = fs.readFileSync(sourceFile, 'utf-8');
  const lines = content.split('\n');

  // Remove lines from startLine to endLine (inclusive)
  const beforeBlock = lines.slice(0, block.startLine);
  const afterBlock = lines.slice(block.endLine + 1);

  // Write back without the block
  const newContent = beforeBlock.concat(afterBlock).join('\n');
  fs.writeFileSync(sourceFile, newContent, 'utf-8');
}

/**
 * Move block content from source to target (DESTRUCTIVE)
 */
async function moveBlockToTarget(block, target, sourceFile, targetDir) {
  if (!target.targetFile || target.confidence < CONFIG.minConfidence) {
    return { moved: false, reason: 'low-confidence' };
  }

  const targetPath = path.join(targetDir, target.targetFile);

  if (!fs.existsSync(targetPath)) {
    console.warn(`      ⚠️  Target file not found: ${target.targetFile}`);
    return { moved: false, reason: 'target-not-found' };
  }

  // Update image references in block content
  const sourceDir = path.dirname(sourceFile);
  const { content: updatedContent, updated: assetsUpdated } = updateImageReferences(
    block.content,
    sourceDir,
    targetDir,
    CONFIG.assetsDir
  );

  // Enrich with web-summary for isolated URLs
  const enrichedContent = await enrichContentWithWebSummaries(updatedContent);

  // Read target file
  let targetContent = fs.readFileSync(targetPath, 'utf-8');

  // Find insertion point (end of About section or end of file)
  const insertionPoint = findInsertionPoint(targetContent);

  // Insert content (using enriched version with web summaries)
  const newContent =
    targetContent.substring(0, insertionPoint) +
    '\n' + enrichedContent + '\n' +
    targetContent.substring(insertionPoint);

  // Write target file
  fs.writeFileSync(targetPath, newContent, 'utf-8');

  // DESTRUCTIVE: Remove block content from source file
  removeBlockFromSource(sourceFile, block);

  return {
    moved: true,
    targetFile: target.targetFile,
    assetsUpdated,
  };
}

/**
 * Find insertion point in target file
 */
function findInsertionPoint(content) {
  // Try to find end of About section
  const aboutMatch = /## About .+?\n([\s\S]*?)(?=\n##|\n- ##|$)/i.exec(content);

  if (aboutMatch) {
    return aboutMatch.index + aboutMatch[0].length;
  }

  // Default: end of file
  return content.length;
}

/**
 * Check if file is effectively empty (only metadata, no content)
 */
function isFileEmpty(filePath) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n').filter(line => line.trim());

  // Count non-metadata lines
  const contentLines = lines.filter(line => {
    return !line.startsWith('#') &&
           !line.startsWith('---') &&
           !line.match(/^\w+:/) &&
           line.length > 10;
  });

  return contentLines.length < 3;
}

/**
 * Delete source file if empty
 */
function deleteIfEmpty(filePath) {
  if (isFileEmpty(filePath)) {
    console.log(`   🗑️  Deleting empty source file: ${path.basename(filePath)}`);
    fs.unlinkSync(filePath);
    return true;
  }
  return false;
}

/**
 * Process single file (DESTRUCTIVE - NO BACKUP)
 */
async function processFile(filePath, targetDir, progress) {
  console.log(`\n📄 Processing: ${path.basename(filePath)}`);

  // Parse file
  const { blocks } = parseSourceFile(filePath);
  console.log(`   📦 Found ${blocks.length} content blocks`);

  // Process each block
  let movedCount = 0;
  let skippedCount = 0;
  let assetsHandled = 0;

  for (let i = 0; i < blocks.length; i++) {
    const block = blocks[i];
    const target = await findTargetConcept(block);

    console.log(`   [${i + 1}/${blocks.length}] Block ${block.id}: ${target.targetConcept || 'SKIP'} (${(target.confidence * 100).toFixed(0)}%)`);

    if (target.confidence >= CONFIG.minConfidence) {
      const result = await moveBlockToTarget(block, target, filePath, targetDir);

      if (result.moved) {
        movedCount++;
        assetsHandled += result.assetsUpdated || 0;
        console.log(`      ✅ Moved to ${target.targetFile}`);
      } else {
        skippedCount++;
        console.log(`      ⏭️  Skipped (${result.reason})`);
      }
    } else {
      skippedCount++;
      console.log(`      ⏭️  Skipped (low confidence)`);
    }
  }

  // Check if source file is now empty
  const deleted = deleteIfEmpty(filePath);

  // Update progress
  progress.filesProcessed++;
  progress.blocksMoved += movedCount;
  progress.assetsHandled += assetsHandled;
  if (deleted) progress.filesDeleted++;
  progress.processedFiles.push(path.basename(filePath));
  saveProgress(progress);

  console.log(`\n   📊 Summary: ${movedCount} moved, ${skippedCount} skipped, ${deleted ? 'DELETED' : 'kept'}`);

  return { movedCount, skippedCount, deleted, assetsHandled };
}

/**
 * Process directory in batches (DESTRUCTIVE)
 */
async function processDirectory(sourceDir, targetDir, options = {}) {
  console.log('🚀 Starting DESTRUCTIVE import...\n');
  console.log(`   Source: ${sourceDir}`);
  console.log(`   Target: ${targetDir}`);
  console.log(`   Assets: ${CONFIG.assetsDir}\n`);
  console.log('   ⚠️  WARNING: NO BACKUPS - Files will be permanently modified/deleted\n');

  // Load index
  loadIndex();

  // Copy assets from source to shared folder
  const sourceAssetsDir = CONFIG.assetsDir;
  if (fs.existsSync(sourceAssetsDir)) {
    console.log('📋 Assets available in source folder');
  }

  // Load progress
  const progress = loadProgress(sourceDir);
  PROGRESS = progress;

  // Get list of files
  const allFiles = fs.readdirSync(sourceDir)
    .filter(f => f.endsWith('.md'))
    .map(f => path.join(sourceDir, f));

  // Filter out already processed files
  const pendingFiles = allFiles.filter(f =>
    !progress.processedFiles.includes(path.basename(f))
  );

  console.log(`📂 Files: ${pendingFiles.length} pending (${progress.filesProcessed} already processed)\n`);

  if (pendingFiles.length === 0) {
    console.log('✅ All files already processed!');
    return progress;
  }

  // Process in batches
  const batchSize = options.batchSize || CONFIG.batchSize;

  for (let i = 0; i < pendingFiles.length; i += batchSize) {
    const batch = pendingFiles.slice(i, Math.min(i + batchSize, pendingFiles.length));

    console.log(`\n${'='.repeat(60)}`);
    console.log(`📦 BATCH ${Math.floor(i / batchSize) + 1}: Processing ${batch.length} files`);
    console.log('='.repeat(60));

    for (const filePath of batch) {
      try {
        await processFile(filePath, targetDir, progress);
      } catch (error) {
        console.error(`\n❌ Error processing ${path.basename(filePath)}: ${error.message}`);
        progress.errors.push({
          file: path.basename(filePath),
          error: error.message,
          timestamp: new Date().toISOString(),
        });
        saveProgress(progress);
      }
    }

    // Progress summary
    const pct = (progress.filesProcessed / progress.totalFiles * 100).toFixed(1);
    console.log(`\n📊 Progress: ${progress.filesProcessed}/${progress.totalFiles} files (${pct}%)`);
    console.log(`   Blocks moved: ${progress.blocksMoved}`);
    console.log(`   Files deleted: ${progress.filesDeleted}`);
    console.log(`   Assets handled: ${progress.assetsHandled}`);
    console.log(`   Errors: ${progress.errors.length}`);
  }

  // Final report
  console.log(`\n${'='.repeat(60)}`);
  console.log('✅ IMPORT COMPLETE');
  console.log('='.repeat(60));
  console.log(`Files processed: ${progress.filesProcessed}/${progress.totalFiles}`);
  console.log(`Files deleted: ${progress.filesDeleted}`);
  console.log(`Blocks moved: ${progress.blocksMoved}`);
  console.log(`Assets handled: ${progress.assetsHandled}`);
  console.log(`Errors: ${progress.errors.length}`);
  console.log(`Duration: ${Math.ceil((Date.now() - new Date(progress.startTime).getTime()) / 60000)} minutes`);

  if (progress.errors.length > 0) {
    console.log(`\n⚠️  Errors occurred during import:`);
    progress.errors.forEach(e => console.log(`   - ${e.file}: ${e.error}`));
  }

  console.log(`\n📁 Progress file: ${CONFIG.progressFile}`);

  return progress;
}

// CLI Interface
if (require.main === module) {
  const args = process.argv.slice(2);

  if (args.length < 2) {
    console.log('Usage: node destructive-import.js <source-dir> <target-dir> [--batch-size=5]');
    console.log('\n⚠️  WARNING: This is a DESTRUCTIVE operation!');
    console.log('   - Content is MOVED from source files');
    console.log('   - Source files are DELETED when empty');
    console.log('   - NO BACKUPS ARE CREATED');
    console.log('\nDefault paths (can be overridden):');
    console.log(`   Source: ${CONFIG.sourceDir}`);
    console.log(`   Target: ${CONFIG.targetDir}`);
    console.log(`   Assets: ${CONFIG.assetsDir}`);
    process.exit(1);
  }

  const sourceDir = path.resolve(args[0]);
  const targetDir = path.resolve(args[1]);

  const batchSizeArg = args.find(a => a.startsWith('--batch-size='));
  const batchSize = batchSizeArg ? parseInt(batchSizeArg.split('=')[1]) : CONFIG.batchSize;

  if (!fs.existsSync(sourceDir)) {
    console.error(`Error: Source directory not found: ${sourceDir}`);
    process.exit(1);
  }

  if (!fs.existsSync(targetDir)) {
    console.error(`Error: Target directory not found: ${targetDir}`);
    process.exit(1);
  }

  processDirectory(sourceDir, targetDir, { batchSize })
    .then(progress => {
      console.log(`\n✅ Import completed successfully`);
    })
    .catch(error => {
      console.error('\n❌ Fatal error:', error);
      process.exit(1);
    });
}

module.exports = {
  processFile,
  processDirectory,
  loadProgress,
};
