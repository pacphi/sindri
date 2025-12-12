#!/usr/bin/env node

/**
 * Asset Handler for Import-to-Ontology
 *
 * Handles image asset references during content migration:
 * - Detects image references in markdown
 * - Updates paths to shared assets/ folder
 * - Validates assets exist
 * - Copies missing assets if needed
 */

const fs = require('fs');
const path = require('path');

/**
 * Image reference patterns
 */
const IMAGE_PATTERNS = [
  /!\[([^\]]*)\]\(([^)]+)\)/g,           // ![alt](path)
  /!\[\[([^\]]+\.(png|jpg|jpeg|gif|svg|webp))\]\]/gi,  // ![[image.png]]
  /<img\s+src=["']([^"']+)["']/g,       // <img src="path">
];

/**
 * Detect all image references in content
 */
function detectImageReferences(content) {
  const images = [];

  for (const pattern of IMAGE_PATTERNS) {
    let match;
    while ((match = pattern.exec(content)) !== null) {
      const fullMatch = match[0];
      let imagePath;

      if (pattern === IMAGE_PATTERNS[0]) {
        // ![alt](path)
        imagePath = match[2];
      } else if (pattern === IMAGE_PATTERNS[1]) {
        // ![[image.png]]
        imagePath = match[1];
      } else {
        // <img src="path">
        imagePath = match[1];
      }

      images.push({
        fullMatch,
        path: imagePath,
        alt: match[1] || '',
        type: pattern === IMAGE_PATTERNS[0] ? 'markdown' :
              pattern === IMAGE_PATTERNS[1] ? 'wikilink' : 'html',
        line: getLineNumber(content, match.index),
      });
    }
  }

  return images;
}

/**
 * Normalize image path to assets/ folder
 */
function normalizeAssetPath(imagePath, sourceDir, assetsDir) {
  // Remove any leading path components
  const basename = path.basename(imagePath);

  // Check if it's already in assets/ format
  if (imagePath.startsWith('assets/') || imagePath.startsWith('./assets/')) {
    return imagePath;
  }

  // Check if it's an absolute path
  if (path.isAbsolute(imagePath)) {
    return `assets/${basename}`;
  }

  // Check if it's a relative path from source directory
  const absolutePath = path.resolve(sourceDir, imagePath);
  if (fs.existsSync(absolutePath)) {
    // Copy to assets/ if not already there
    const targetPath = path.join(assetsDir, basename);
    if (!fs.existsSync(targetPath)) {
      console.log(`   📋 Copying asset: ${basename}`);
      fs.copyFileSync(absolutePath, targetPath);
    }
    return `assets/${basename}`;
  }

  // Check if it already exists in assets/
  const assetsPath = path.join(assetsDir, basename);
  if (fs.existsSync(assetsPath)) {
    return `assets/${basename}`;
  }

  // Warn about missing asset
  console.warn(`   ⚠️  Asset not found: ${imagePath}`);
  return imagePath; // Keep original path but warn
}

/**
 * Update image references in content
 */
function updateImageReferences(content, sourceDir, targetDir, assetsDir) {
  const images = detectImageReferences(content);

  if (images.length === 0) {
    return { content, images: [], updated: 0 };
  }

  console.log(`   🖼️  Found ${images.length} image references`);

  let updatedContent = content;
  let updated = 0;

  for (const image of images) {
    const normalizedPath = normalizeAssetPath(image.path, sourceDir, assetsDir);

    if (normalizedPath !== image.path) {
      // Update the reference
      let newMatch;

      if (image.type === 'markdown') {
        newMatch = `![${image.alt}](${normalizedPath})`;
      } else if (image.type === 'wikilink') {
        newMatch = `![[${normalizedPath}]]`;
      } else {
        newMatch = `<img src="${normalizedPath}"`;
      }

      updatedContent = updatedContent.replace(image.fullMatch, newMatch);
      updated++;

      console.log(`      Updated: ${image.path} → ${normalizedPath}`);
    }
  }

  return {
    content: updatedContent,
    images,
    updated,
  };
}

/**
 * Validate all assets exist
 */
function validateAssets(images, assetsDir) {
  const missing = [];
  const existing = [];

  for (const image of images) {
    const assetPath = path.join(assetsDir, path.basename(image.path));

    if (fs.existsSync(assetPath)) {
      existing.push(image.path);
    } else {
      missing.push(image.path);
    }
  }

  return {
    total: images.length,
    existing: existing.length,
    missing: missing.length,
    missingPaths: missing,
  };
}

/**
 * Copy assets from source to target
 */
function copyAssets(sourceAssetsDir, targetAssetsDir) {
  if (!fs.existsSync(sourceAssetsDir)) {
    console.log('   ℹ️  No source assets directory found');
    return { copied: 0 };
  }

  if (!fs.existsSync(targetAssetsDir)) {
    fs.mkdirSync(targetAssetsDir, { recursive: true });
  }

  const files = fs.readdirSync(sourceAssetsDir);
  let copied = 0;

  for (const file of files) {
    const sourcePath = path.join(sourceAssetsDir, file);
    const targetPath = path.join(targetAssetsDir, file);

    // Skip if already exists
    if (fs.existsSync(targetPath)) {
      continue;
    }

    // Copy file
    fs.copyFileSync(sourcePath, targetPath);
    copied++;
  }

  if (copied > 0) {
    console.log(`   ✅ Copied ${copied} assets to shared folder`);
  }

  return { copied };
}

/**
 * Get line number from index
 */
function getLineNumber(content, index) {
  return content.substring(0, index).split('\n').length;
}

/**
 * Generate asset report
 */
function generateAssetReport(sourceDir, assetsDir) {
  const files = fs.readdirSync(sourceDir)
    .filter(f => f.endsWith('.md'));

  const report = {
    totalFiles: files.length,
    filesWithImages: 0,
    totalImages: 0,
    uniqueImages: new Set(),
    missingAssets: [],
  };

  for (const file of files) {
    const filePath = path.join(sourceDir, file);
    const content = fs.readFileSync(filePath, 'utf-8');

    const images = detectImageReferences(content);

    if (images.length > 0) {
      report.filesWithImages++;
      report.totalImages += images.length;

      images.forEach(img => {
        report.uniqueImages.add(path.basename(img.path));
      });
    }
  }

  // Check which assets are missing
  for (const imageName of report.uniqueImages) {
    const assetPath = path.join(assetsDir, imageName);
    if (!fs.existsSync(assetPath)) {
      report.missingAssets.push(imageName);
    }
  }

  return {
    ...report,
    uniqueImages: Array.from(report.uniqueImages),
  };
}

module.exports = {
  detectImageReferences,
  normalizeAssetPath,
  updateImageReferences,
  validateAssets,
  copyAssets,
  generateAssetReport,
};

// CLI Interface
if (require.main === module) {
  const args = process.argv.slice(2);

  if (args.length < 2) {
    console.log('Usage: node asset-handler.js <source-dir> <assets-dir> [--report]');
    process.exit(1);
  }

  const sourceDir = path.resolve(args[0]);
  const assetsDir = path.resolve(args[1]);
  const reportMode = args.includes('--report');

  if (!fs.existsSync(sourceDir)) {
    console.error(`Error: Source directory not found: ${sourceDir}`);
    process.exit(1);
  }

  if (reportMode) {
    console.log('📊 Asset Report\n');
    const report = generateAssetReport(sourceDir, assetsDir);

    console.log(`Files: ${report.filesWithImages}/${report.totalFiles} with images`);
    console.log(`Images: ${report.totalImages} total, ${report.uniqueImages.length} unique`);

    if (report.missingAssets.length > 0) {
      console.log(`\n⚠️  Missing Assets (${report.missingAssets.length}):`);
      report.missingAssets.forEach(asset => console.log(`   - ${asset}`));
    } else {
      console.log('\n✅ All assets present in shared folder');
    }
  } else {
    // Copy assets
    const sourceAssetsDir = path.join(sourceDir, 'assets');
    copyAssets(sourceAssetsDir, assetsDir);
  }
}
