# Quick Start: OWL2-Validated Ontology Import

## Installation

No installation needed. The skill is ready to use.

### Verify Setup

```bash
cd /home/devuser/workspace/project/multi-agent-docker/skills/import-to-ontology

# Check validation bridge exists
ls src/validation_bridge.js

# Check Python validator exists
ls /home/devuser/workspace/logseq/skills/ontology-augmenter/src/owl2_validator.py

# Verify Python 3
python3 --version
```

## Basic Usage

### 1. Validate a Single File

```bash
# Standalone validation
node src/validation_bridge.js /path/to/ontology-file.md

# Example output:
# 🔍 Validating OWL2 compliance: ontology-file.md
#    ✅ Valid OWL2 (15 axioms)
#
# 📋 Validation Result: ontology-file.md
#    Status: ✅ VALID
#    Axioms: 15
```

### 2. Import with Validation

```bash
# Dry run (no actual import)
node import-engine.js source-file.md --dry-run

# Import with validation
node import-engine.js source-file.md --force

# Example output:
# 🚀 Processing blocks with OWL2 validation...
#
#    [1/3] Processing block-1...
#       🔍 Validating OWL2 compliance: AI-0042-machine-learning.md
#          ✅ Valid OWL2 (12 axioms)
#       → Inserting into AI-0042-machine-learning.md
#       🔍 Validating OWL2 compliance: AI-0042-machine-learning.md
#          ✅ Valid OWL2 (15 axioms)
#       ← Removing from source
#
# ✅ Import complete!
#    Processed: 3/3
#    Skipped: 0
#    Failed: 0
#
# 📊 Validation Summary:
#    ✅ Valid: 3/3
#    ❌ Invalid: 0/3
```

### 3. Batch Validation

```bash
# Validate multiple files
node src/validation_bridge.js file1.md file2.md file3.md

# Example output:
# 🔍 Batch validation: 3 files
# 🔍 Validating OWL2 compliance: file1.md
#    ✅ Valid OWL2 (10 axioms)
# 🔍 Validating OWL2 compliance: file2.md
#    ❌ Invalid OWL2 (2 errors, 1 warnings)
# 🔍 Validating OWL2 compliance: file3.md
#    ✅ Valid OWL2 (8 axioms)
#
# 📊 Batch Validation Summary:
#    ✅ Valid: 2/3
#    ❌ Invalid: 1/3
```

## Testing

```bash
# Run integration tests
node test/test-validation-integration.js

# Expected: All 5 tests should pass
```

## Common Workflows

### Workflow 1: Import New Content

```bash
# 1. Dry run to analyze
node import-engine.js new-content.md --dry-run

# 2. Review the plan

# 3. Import with validation
node import-engine.js new-content.md --force
```

### Workflow 2: Validate Existing Ontology

```bash
# Validate all ontology files
cd /path/to/mainKnowledgeGraph/pages

# Validate all AI domain files
node /path/to/validation_bridge.js AI-*.md

# Validate all domain files
node /path/to/validation_bridge.js *.md
```

### Workflow 3: Fix Validation Errors

```bash
# 1. Validate file
node src/validation_bridge.js problematic-file.md

# 2. Note errors and line numbers
# Example:
#    Line 42: Unbalanced parentheses
#    Fix: Ensure every '(' has a matching ')'

# 3. Edit file to fix errors

# 4. Re-validate
node src/validation_bridge.js problematic-file.md

# 5. Confirm valid
#    ✅ Valid OWL2 (18 axioms)
```

## Integration with Import Engine

The validation is automatic when using `import-engine.js`:

```javascript
// Validation happens automatically:
1. Pre-move validation  → Check target file is valid
2. Insert content       → Add block to target
3. Post-move validation → Re-check target file
4. Rollback if failed   → Restore source if invalid
5. Remove from source   → Only if validation passed
```

## Error Handling

### Validation Fails

```
❌ Post-move validation failed
   Errors: Unbalanced parentheses, Invalid namespace prefix: xyz

🔄 Rolling back move due to validation failure...
   ✅ Source file restored from backup
   ✅ Rollback complete

Result: ❌ Failed (post-validation-failed)
```

### Validator Not Found

```
Error: Validator not found at: /path/to/owl2_validator.py

Fix: Check VALIDATOR_PATH in validation_bridge.js
```

### Python Not Available

```
Error: spawn python3 ENOENT

Fix: Install Python 3
  sudo apt-get install python3
```

## Configuration

### Enable/Disable Validation

```javascript
// In import-engine.js options
const options = {
  validation: true, // Enable OWL2 validation
  force: true, // Skip dry-run
};

await executeImport(filePath, options);
```

### Validation Settings

```json
{
  "validation": {
    "enabled": true,
    "owl2Compliance": true,
    "rollbackOnFailure": true
  }
}
```

## Performance

### Expected Times

- **Validation**: ~1-2s per file
- **Import with validation**: ~5-10s per file
- **Batch (100 files)**: ~10-20 minutes

### Optimization Tips

1. **Dry run first**: Use `--dry-run` to check before importing
2. **Batch operations**: Validate multiple files at once
3. **Disable for bulk**: Set `validation: false` for non-critical imports
4. **Pre-validate**: Run standalone validation before import

## Troubleshooting

### Check Dependencies

```bash
# Verify all files exist
ls src/validation_bridge.js
ls /home/devuser/workspace/logseq/skills/ontology-augmenter/src/owl2_validator.py

# Verify Python
python3 --version

# Test import
node -e "const { validateOntologyFile } = require('./src/validation_bridge'); console.log('✅ Import successful');"
```

### Manual Validation

```bash
# Run validator directly
python3 /home/devuser/workspace/logseq/skills/ontology-augmenter/src/owl2_validator.py \
  /path/to/file.md

# Check exit code
echo $?  # 0 = valid, 1 = invalid
```

### Debug Mode

```javascript
// Add console.log to validation_bridge.js
python.stdout.on("data", (data) => {
  console.log("Python stdout:", data.toString());
  stdout += data.toString();
});

python.stderr.on("data", (data) => {
  console.log("Python stderr:", data.toString());
  stderr += data.toString();
});
```

## Documentation

- **Full Guide**: [README-VALIDATION.md](./README-VALIDATION.md)
- **Skill Docs**: [SKILL.md](./SKILL.md)
- **Integration Summary**: [INTEGRATION-SUMMARY.md](./INTEGRATION-SUMMARY.md)

## Support

For issues or questions:

1. Check [README-VALIDATION.md](./README-VALIDATION.md) troubleshooting section
2. Run test suite: `node test/test-validation-integration.js`
3. Verify dependencies: `python3 --version` and file paths
4. Check validator output manually
