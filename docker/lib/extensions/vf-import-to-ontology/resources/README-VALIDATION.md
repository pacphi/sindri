# OWL2 Validation Integration

This document describes the integration between the `ontology-import` skill and the `ontology-core` OWL2 validator.

## Architecture

```
ontology-import (Node.js)
    ↓
validation_bridge.js (Node.js)
    ↓
spawn('python3')
    ↓
owl2_validator.py (Python)
    ↓
ValidationReport (JSON-like output)
    ↓
Parse & Return to Node.js
```

## Components

### 1. validation_bridge.js (Node.js)

**Location**: `/home/devuser/workspace/project/multi-agent-docker/skills/import-to-ontology/src/validation_bridge.js`

**Purpose**: Bridge between Node.js import engine and Python OWL2 validator

**Functions**:

- `validateOntologyFile(filePath)` - Validate single file
- `validateContent(content, tempPath)` - Validate in-memory content
- `validateBatch(filePaths)` - Batch validate multiple files
- `formatValidationResult(result)` - Format results for display

**Usage**:

```javascript
const { validateOntologyFile } = require("./src/validation_bridge");

const result = await validateOntologyFile("/path/to/file.md");

if (result.is_valid) {
  console.log(`✅ Valid: ${result.total_axioms} axioms`);
} else {
  console.log(`❌ Invalid: ${result.errors.length} errors`);
}
```

### 2. owl2_validator.py (Python)

**Location**: `/home/devuser/workspace/logseq/skills/ontology-augmenter/src/owl2_validator.py`

**Purpose**: Core OWL2 compliance validation logic

**Validates**:

- Class declarations: `Declaration(Class(ai:MachineLearning))`
- SubClassOf axioms: `SubClassOf(ai:DeepLearning ai:MachineLearning)`
- Property declarations: `Declaration(ObjectProperty(ai:hasAlgorithm))`
- Restrictions: `ObjectSomeValuesFrom(ai:hasAlgorithm ai:Algorithm)`
- Namespace prefixes: `ai:`, `bc:`, `mv:`, `rb:`, `dt:`
- Annotation format: `rdfs:label "Machine Learning"`
- Parentheses balance
- Naming conventions (PascalCase for classes, camelCase for properties)

**Command Line**:

```bash
python3 owl2_validator.py /path/to/file.md
# Exit code 0 = valid, 1 = invalid
```

### 3. import-engine.js (Enhanced)

**Location**: `/home/devuser/workspace/project/multi-agent-docker/skills/import-to-ontology/import-engine.js`

**Enhanced Functions**:

- `validateTargetFile(targetFile, stage)` - Validate before/after moves
- `rollbackMove(source, target, backup, block)` - Rollback on failure
- `insertContentBlock(targetFile, block, target)` - Insert with validation
- `removeBlockFromSource(sourceFile, block)` - Destructive removal

**Validation Workflow**:

1. Pre-move: Validate target file exists and is valid
2. Insert: Add content block to target file
3. Post-move: Re-validate target file with new content
4. Rollback: If validation fails, restore source file
5. Remove: Only remove from source if validation passed

## Workflow Example

### Successful Import

```javascript
// Source: external-notes.md
// Target: AI-0042-machine-learning.md

1. Pre-move validation
   ✅ Target file valid (15 axioms, 3 classes, 5 properties)

2. Content insertion
   → Inserting block-3 into AI-0042-machine-learning.md

3. Post-move validation
   ✅ Target file valid (18 axioms, 4 classes, 6 properties)

4. Remove from source
   ← Removing block-3 from external-notes.md

Result: ✅ Success
```

### Failed Import (Rollback)

```javascript
// Source: external-notes.md
// Target: AI-0042-machine-learning.md

1. Pre-move validation
   ✅ Target file valid (15 axioms)

2. Content insertion
   → Inserting block-5 into AI-0042-machine-learning.md
   (block-5 contains invalid OWL syntax)

3. Post-move validation
   ❌ Target file invalid
   Errors:
     - Line 42: Unbalanced parentheses
     - Line 43: Invalid namespace prefix: xyz:InvalidClass

4. Rollback
   🔄 Rolling back move...
   ✅ Source file restored from backup

Result: ❌ Failed (post-validation-failed)
```

## Validation Report Format

### Python Output (stdout)

```
================================================================================
OWL2 VALIDATION REPORT
================================================================================
File: AI-0042-machine-learning.md
Total Axioms: 18
Classes: 4
Properties: 6
Individuals: 0

✓ VALID - No errors found
⚠ 2 warning(s)

WARNINGS:
--------------------------------------------------------------------------------
Line 35: Property should use camelCase: ai:HasAlgorithm
  Axiom: Declaration(ObjectProperty(ai:HasAlgorithm))...
  Suggestion: Use camelCase for property names, e.g., ai:hasAlgorithm

================================================================================
```

### JavaScript Result Object

```javascript
{
  file_path: '/path/to/AI-0042-machine-learning.md',
  is_valid: true,
  total_axioms: 18,
  class_count: 4,
  property_count: 6,
  errors: [],
  warnings: [
    {
      line_number: 35,
      message: 'Property should use camelCase: ai:HasAlgorithm',
      axiom: 'Declaration(ObjectProperty(ai:HasAlgorithm))',
      fix_suggestion: 'Use camelCase for property names, e.g., ai:hasAlgorithm'
    }
  ]
}
```

## Error Handling

### Common Errors

#### 1. Validator Not Found

```
Error: Validator not found at: /path/to/owl2_validator.py
```

**Fix**: Ensure ontology-augmenter skill is installed:

```bash
ls /home/devuser/workspace/logseq/skills/ontology-augmenter/src/owl2_validator.py
```

#### 2. Python Not Available

```
Error: Failed to spawn validator: spawn python3 ENOENT
```

**Fix**: Install Python 3:

```bash
which python3
# If not found: apt-get install python3
```

#### 3. Validation Parsing Failed

```
Error: Validation parsing failed: Cannot read property 'length' of undefined
```

**Fix**: Check Python validator output format. Run manually:

```bash
python3 owl2_validator.py /path/to/file.md
```

## Configuration

### Enable/Disable Validation

```javascript
// In import-engine.js
const options = {
  validation: true, // Enable OWL2 validation
  rollbackOnFailure: true, // Auto-rollback on validation failure
  force: true, // Skip dry-run
};

await executeImport(filePath, options);
```

### Validation Config

```json
{
  "validation": {
    "enabled": true,
    "owl2Compliance": true,
    "rollbackOnFailure": true,
    "continueOnError": false,
    "logValidationResults": true
  }
}
```

## Testing

### Test Validation Bridge

```bash
# Test single file validation
cd /home/devuser/workspace/project/multi-agent-docker/skills/import-to-ontology
node src/validation_bridge.js /path/to/ontology-file.md

# Expected output:
# 🔍 Validating OWL2 compliance: ontology-file.md
#    ✅ Valid OWL2 (15 axioms)
#
# 📋 Validation Result: ontology-file.md
#    Status: ✅ VALID
#    Axioms: 15
```

### Test Batch Validation

```bash
node src/validation_bridge.js file1.md file2.md file3.md

# Expected output:
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

### Test Import with Validation

```bash
# Dry run first
node import-engine.js test-file.md --dry-run

# Import with validation
node import-engine.js test-file.md --force

# Expected output includes:
# 🚀 Processing blocks with OWL2 validation...
#    [1/5] Processing block-1...
#       🔍 Validating OWL2 compliance: AI-0001-target.md
#          ✅ Valid OWL2 (12 axioms)
#       → Inserting into AI-0001-target.md
#       🔍 Validating OWL2 compliance: AI-0001-target.md
#          ✅ Valid OWL2 (15 axioms)
#       ← Removing from source
```

## Performance

### Validation Times

- **Small file** (5-10 axioms): ~0.5-1s
- **Medium file** (10-50 axioms): ~1-2s
- **Large file** (50+ axioms): ~2-5s

### Batch Import (100 files)

- **Without validation**: ~2-5 minutes
- **With validation**: ~10-20 minutes (adds ~5-10s per file)

**Recommendation**: Use validation for critical imports. For bulk imports, validate sample first.

## Debugging

### Enable Verbose Output

```javascript
// In validation_bridge.js
console.log("Python stdout:", stdout);
console.log("Python stderr:", stderr);
console.log("Exit code:", code);
```

### Manual Validation

```bash
# Run validator directly
python3 /home/devuser/workspace/logseq/skills/ontology-augmenter/src/owl2_validator.py \
  /path/to/file.md

# Check exit code
echo $?  # 0 = valid, 1 = invalid
```

### Check Dependencies

```bash
# Verify file paths
ls /home/devuser/workspace/project/multi-agent-docker/skills/import-to-ontology/src/validation_bridge.js
ls /home/devuser/workspace/logseq/skills/ontology-augmenter/src/owl2_validator.py

# Verify Python 3
python3 --version

# Test import
cd /home/devuser/workspace/project/multi-agent-docker/skills/import-to-ontology
node -e "const { validateOntologyFile } = require('./src/validation_bridge'); console.log('Import successful');"
```

## See Also

- [SKILL.md](./SKILL.md) - Full skill documentation
- - Core validator
- - OWL2 syntax reference
