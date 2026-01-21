#!/usr/bin/env node

/**
 * Validation Bridge - Node.js to Python OWL2 Validator
 *
 * Bridges the ontology-import skill (Node.js) with the ontology-core
 * OWL2 validator (Python) for comprehensive validation during imports.
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Path to Python validator
const VALIDATOR_PATH = path.join(
  __dirname,
  '../../../logseq/skills/ontology-augmenter/src/owl2_validator.py'
);

/**
 * Validate an ontology file using the Python OWL2 validator
 *
 * @param {string} filePath - Absolute path to the markdown file to validate
 * @returns {Promise<ValidationResult>} - Validation results
 */
async function validateOntologyFile(filePath) {
  return new Promise((resolve, reject) => {
    // Verify file exists
    if (!fs.existsSync(filePath)) {
      reject(new Error(`File not found: ${filePath}`));
      return;
    }

    // Verify validator exists
    if (!fs.existsSync(VALIDATOR_PATH)) {
      reject(new Error(`Validator not found at: ${VALIDATOR_PATH}`));
      return;
    }

    console.log(`   🔍 Validating OWL2 compliance: ${path.basename(filePath)}`);

    const python = spawn('python3', [VALIDATOR_PATH, filePath]);

    let stdout = '';
    let stderr = '';

    python.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    python.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    python.on('close', (code) => {
      try {
        // Parse validation report from stdout
        const result = parseValidationOutput(stdout, filePath, code);

        if (result.is_valid) {
          console.log(`      ✅ Valid OWL2 (${result.total_axioms} axioms)`);
        } else {
          console.log(`      ❌ Invalid OWL2 (${result.errors.length} errors, ${result.warnings.length} warnings)`);
        }

        resolve(result);
      } catch (error) {
        // If parsing fails, return error result
        reject(new Error(`Validation parsing failed: ${error.message}\nStderr: ${stderr}`));
      }
    });

    python.on('error', (error) => {
      reject(new Error(`Failed to spawn validator: ${error.message}`));
    });
  });
}

/**
 * Parse validation output from Python validator
 *
 * @param {string} output - Stdout from validator
 * @param {string} filePath - File path being validated
 * @param {number} exitCode - Process exit code
 * @returns {ValidationResult}
 */
function parseValidationOutput(output, filePath, exitCode) {
  const result = {
    file_path: filePath,
    is_valid: exitCode === 0,
    total_axioms: 0,
    errors: [],
    warnings: [],
    classes: [],
    properties: [],
    individuals: []
  };

  // Parse output line by line
  const lines = output.split('\n');
  let inErrorsSection = false;
  let inWarningsSection = false;
  let currentIssue = null;

  for (const line of lines) {
    // Extract total axioms
    const axiomMatch = line.match(/Total Axioms:\s*(\d+)/);
    if (axiomMatch) {
      result.total_axioms = parseInt(axiomMatch[1]);
    }

    // Extract class count
    const classMatch = line.match(/Classes:\s*(\d+)/);
    if (classMatch) {
      result.class_count = parseInt(classMatch[1]);
    }

    // Extract property count
    const propMatch = line.match(/Properties:\s*(\d+)/);
    if (propMatch) {
      result.property_count = parseInt(propMatch[1]);
    }

    // Section markers
    if (line.startsWith('ERRORS:')) {
      inErrorsSection = true;
      inWarningsSection = false;
      continue;
    }
    if (line.startsWith('WARNINGS:')) {
      inErrorsSection = false;
      inWarningsSection = true;
      continue;
    }
    if (line.startsWith('=====')) {
      inErrorsSection = false;
      inWarningsSection = false;
      continue;
    }

    // Parse error/warning lines
    if (inErrorsSection || inWarningsSection) {
      const lineMatch = line.match(/Line (\d+):\s*(.+)/);
      if (lineMatch) {
        currentIssue = {
          line_number: parseInt(lineMatch[1]),
          message: lineMatch[2],
          axiom: '',
          fix_suggestion: ''
        };
      } else if (currentIssue && line.trim().startsWith('Axiom:')) {
        currentIssue.axiom = line.replace(/^\s*Axiom:\s*/, '').trim();
      } else if (currentIssue && line.trim().startsWith('Fix:')) {
        currentIssue.fix_suggestion = line.replace(/^\s*Fix:\s*/, '').trim();
      } else if (currentIssue && line.trim().startsWith('Suggestion:')) {
        currentIssue.fix_suggestion = line.replace(/^\s*Suggestion:\s*/, '').trim();
      } else if (currentIssue && line.trim() === '') {
        // End of issue
        if (inErrorsSection) {
          result.errors.push(currentIssue);
        } else {
          result.warnings.push(currentIssue);
        }
        currentIssue = null;
      }
    }
  }

  return result;
}

/**
 * Validate file content (without file on disk)
 *
 * @param {string} content - Markdown content to validate
 * @param {string} tempPath - Temporary file path to use
 * @returns {Promise<ValidationResult>}
 */
async function validateContent(content, tempPath = null) {
  // Create temporary file
  if (!tempPath) {
    tempPath = `/tmp/ontology-validation-${Date.now()}.md`;
  }

  fs.writeFileSync(tempPath, content, 'utf-8');

  try {
    const result = await validateOntologyFile(tempPath);
    return result;
  } finally {
    // Cleanup temp file
    if (fs.existsSync(tempPath)) {
      fs.unlinkSync(tempPath);
    }
  }
}

/**
 * Validate multiple files in batch
 *
 * @param {string[]} filePaths - Array of file paths
 * @returns {Promise<Map<string, ValidationResult>>}
 */
async function validateBatch(filePaths) {
  const results = new Map();

  console.log(`\n🔍 Batch validation: ${filePaths.length} files\n`);

  for (const filePath of filePaths) {
    try {
      const result = await validateOntologyFile(filePath);
      results.set(filePath, result);
    } catch (error) {
      results.set(filePath, {
        file_path: filePath,
        is_valid: false,
        total_axioms: 0,
        errors: [{
          line_number: 0,
          message: `Validation failed: ${error.message}`,
          axiom: '',
          fix_suggestion: ''
        }],
        warnings: []
      });
    }
  }

  // Summary
  const validCount = Array.from(results.values()).filter(r => r.is_valid).length;
  console.log(`\n📊 Batch Validation Summary:`);
  console.log(`   ✅ Valid: ${validCount}/${filePaths.length}`);
  console.log(`   ❌ Invalid: ${filePaths.length - validCount}/${filePaths.length}\n`);

  return results;
}

/**
 * Format validation result for display
 *
 * @param {ValidationResult} result
 * @returns {string}
 */
function formatValidationResult(result) {
  const lines = [];

  lines.push(`\n📋 Validation Result: ${path.basename(result.file_path)}`);
  lines.push(`   Status: ${result.is_valid ? '✅ VALID' : '❌ INVALID'}`);
  lines.push(`   Axioms: ${result.total_axioms}`);

  if (result.errors.length > 0) {
    lines.push(`\n   Errors (${result.errors.length}):`);
    result.errors.forEach((error, i) => {
      lines.push(`     ${i + 1}. Line ${error.line_number}: ${error.message}`);
      if (error.fix_suggestion) {
        lines.push(`        Fix: ${error.fix_suggestion}`);
      }
    });
  }

  if (result.warnings.length > 0) {
    lines.push(`\n   Warnings (${result.warnings.length}):`);
    result.warnings.slice(0, 5).forEach((warning, i) => {
      lines.push(`     ${i + 1}. Line ${warning.line_number}: ${warning.message}`);
    });
    if (result.warnings.length > 5) {
      lines.push(`     ... and ${result.warnings.length - 5} more warnings`);
    }
  }

  return lines.join('\n');
}

module.exports = {
  validateOntologyFile,
  validateContent,
  validateBatch,
  formatValidationResult
};

// CLI interface
if (require.main === module) {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.log('Usage: node validation_bridge.js <file-path> [file-path...]');
    console.log('');
    console.log('Validates OWL2 compliance for ontology markdown files.');
    process.exit(1);
  }

  const filePaths = args.map(p => path.resolve(p));

  if (filePaths.length === 1) {
    // Single file validation
    validateOntologyFile(filePaths[0])
      .then(result => {
        console.log(formatValidationResult(result));
        process.exit(result.is_valid ? 0 : 1);
      })
      .catch(error => {
        console.error(`Error: ${error.message}`);
        process.exit(1);
      });
  } else {
    // Batch validation
    validateBatch(filePaths)
      .then(results => {
        const allValid = Array.from(results.values()).every(r => r.is_valid);
        process.exit(allValid ? 0 : 1);
      })
      .catch(error => {
        console.error(`Error: ${error.message}`);
        process.exit(1);
      });
  }
}
