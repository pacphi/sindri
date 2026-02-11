# FAQ Data Schema Validation Guide

**Version**: 3.0.0
**Last Updated**: 2026-02-11

---

## Overview

This document describes the validation requirements and maintenance procedures for the Sindri FAQ data schema (v3-faq-data.json).

## Schema Structure

The FAQ data follows a standardized JSON schema with the following top-level structure:

```json
{
  "schemaVersion": "3.0.0",
  "lastUpdated": "YYYY-MM-DD",
  "meta": { ... },
  "categories": [ ... ],
  "personas": [ ... ],
  "useCases": [ ... ],
  "questions": [ ... ]
}
```

## Validation Requirements

### 1. Metadata Validation

**Required Fields**:
- `schemaVersion`: Must be semantic version string (e.g., "3.0.0")
- `lastUpdated`: Must be ISO date format (YYYY-MM-DD)
- `meta.totalQuestions`: Must match actual `questions` array length
- `meta.versionsSupported`: Must be array of version strings
- `meta.categories`: Must match number of category objects
- `meta.personas`: Must match number of persona objects

**Validation**:
```bash
# Verify question count matches
jq '.meta.totalQuestions' v3-faq-data.json
jq '.questions | length' v3-faq-data.json
# These should be equal
```

### 2. Category Validation

**Required Fields**:
- `id`: Kebab-case unique identifier
- `name`: Human-readable name
- `icon`: Icon identifier
- `description`: Brief description of category

**Current Categories** (13):
- getting-started
- configuration
- deployment
- extensions
- secrets
- troubleshooting
- vm-images
- image-management
- kubernetes
- doctor-diagnostics
- migration
- bom
- testing-security

**Validation**:
```bash
# List all category IDs
jq '.categories | map(.id)' v3-faq-data.json

# Verify all questions reference valid categories
jq -r '.questions[] | .category' v3-faq-data.json | sort -u
```

### 3. Persona Validation

**Required Fields**:
- `id`: Kebab-case unique identifier
- `name`: Human-readable name
- `icon`: Icon identifier
- `description`: Persona characteristics
- `keywords`: Array of search terms

**Current Personas** (6):
- individual-developer
- small-team
- enterprise
- ai-ml-researcher
- platform-engineer
- windows-user

**Validation**:
```bash
# Verify all questions reference valid persona IDs
jq -r '.questions[].personas[]' v3-faq-data.json | sort -u
jq -r '.personas[].id' v3-faq-data.json | sort
```

### 4. Use Case Validation

**Required Fields**:
- `id`: Kebab-case unique identifier
- `name`: Human-readable name
- `description`: Use case description
- `relatedPersonas`: Array of persona IDs
- `relatedCategories`: Array of category IDs

**Current Use Cases** (8):
- local-development
- cloud-deployment
- multi-cloud
- production-kubernetes
- vm-image-building
- ai-agent-development
- migration
- security-compliance

**Validation**:
```bash
# Verify all questions reference valid use case IDs
jq -r '.questions[].useCases[]' v3-faq-data.json | sort -u
jq -r '.useCases[].id' v3-faq-data.json | sort
```

### 5. Question Validation

**Required Core Fields**:
- `id`: Unique kebab-case identifier
- `category`: Must reference valid category ID
- `question`: Clear, specific question text
- `answer`: 2-4 sentence answer with relevant commands
- `tags`: Array of 1-8 lowercase, kebab-case tags
- `docs`: Array of at least 1 documentation path

**Required Version Fields**:
- `versionsApplicable`: Array of ["v2"], ["v3"], or ["v2", "v3"]
- `versionSpecifics`: Object with version-specific details

**Required Discovery Fields**:
- `personas`: Array of persona IDs (at least 1)
- `useCases`: Array of use case IDs (at least 1)
- `difficulty`: "beginner" | "intermediate" | "advanced"
- `relatedQuestions`: Array of question IDs (can be empty)
- `keywords`: Array of search keywords

**Required Metadata Fields**:
- `dateAdded`: ISO date string
- `dateUpdated`: ISO date string
- `popularity`: Number (default: 0)
- `upvotes`: Number (default: 0)

**Validation**:
```bash
# Verify all question IDs are unique
jq -r '.questions[].id' v3-faq-data.json | sort | uniq -d

# Verify all relatedQuestions reference existing IDs
jq -r '.questions[].relatedQuestions[]?' v3-faq-data.json | sort -u > related.txt
jq -r '.questions[].id' v3-faq-data.json | sort > ids.txt
comm -23 related.txt ids.txt  # Should be empty

# Verify version specifics match versionsApplicable
jq '.questions[] | select(.versionsApplicable | contains(["v3"])) | select(.versionSpecifics.v3 | not)' v3-faq-data.json
# Should return empty (all v3 questions should have v3 versionSpecifics)
```

### 6. Tag Validation

**Tag Rules**:
- Lowercase with hyphens (kebab-case)
- Maximum 8 tags per question
- Always include version tag (v2, v3, v2-v3-compatible, migration)
- Use descriptive, searchable terms

**Common Tags**:
- **Version**: `v3`, `v3-only`, `v2-v3-compatible`, `migration`
- **Features**: `rust-cli`, `binary`, `doctor`, `image-verification`, `cosign`, `sbom`
- **Providers**: `docker`, `fly`, `flyio`, `devpod`, `kubernetes`, `k8s`
- **Level**: `beginner`, `intermediate`, `advanced`
- **Topics**: `install`, `config`, `deploy`, `extensions`, `secrets`, `troubleshooting`

**Validation**:
```bash
# Find questions with more than 8 tags
jq '.questions[] | select(.tags | length > 8) | {id, tagCount: (.tags | length)}' v3-faq-data.json

# Find questions without version tags
jq '.questions[] | select(.tags | any(test("v2|v3|migration")) | not) | .id' v3-faq-data.json
```

### 7. Documentation Path Validation

**Requirements**:
- Each question must have at least 1 doc reference
- Paths should be relative to repository root
- Paths should exist in the repository

**Validation**:
```bash
# Extract all unique doc paths
jq -r '.questions[].docs[]' v3-faq-data.json | sort -u > doc_paths.txt

# Verify paths exist (run from repo root)
while read path; do
  if [ ! -f "$path" ]; then
    echo "Missing: $path"
  fi
done < doc_paths.txt
```

## Content Quality Checks

### Question Quality

1. **Clarity**: Questions should be user-focused and specific
   - Good: "How do I install the Sindri v3 binary?"
   - Bad: "Installation?"

2. **Answer Length**: 2-4 sentences with actionable information
   - Include relevant commands when applicable
   - Mention version differences for shared questions

3. **Version Awareness**: Always indicate which version(s) apply
   - Use `versionsApplicable` array correctly
   - Add `versionSpecifics` for command differences

4. **Command Examples**: Use actual syntax, not placeholders
   - Good: `sindri deploy --provider fly`
   - Bad: `sindri deploy --provider <provider>`

### Cross-Reference Validation

```bash
# Verify all category references are valid
jq -r '.questions[].category' v3-faq-data.json | sort -u > used_categories.txt
jq -r '.categories[].id' v3-faq-data.json | sort > defined_categories.txt
comm -23 used_categories.txt defined_categories.txt  # Should be empty

# Verify all persona references are valid
jq -r '.questions[].personas[]' v3-faq-data.json | sort -u > used_personas.txt
jq -r '.personas[].id' v3-faq-data.json | sort > defined_personas.txt
comm -23 used_personas.txt defined_personas.txt  # Should be empty

# Verify all use case references are valid
jq -r '.questions[].useCases[]' v3-faq-data.json | sort -u > used_usecases.txt
jq -r '.useCases[].id' v3-faq-data.json | sort > defined_usecases.txt
comm -23 used_usecases.txt defined_usecases.txt  # Should be empty
```

## Maintenance Procedures

### Adding New Questions

1. **Determine Category**: Choose or create appropriate category
2. **Assign Personas**: Identify 1-3 target personas
3. **Select Use Cases**: Identify 1-2 relevant use cases
4. **Set Difficulty**: Assess beginner/intermediate/advanced
5. **Add Version Info**: Set `versionsApplicable` and `versionSpecifics`
6. **Tag Appropriately**: Add 1-8 relevant tags
7. **Link Documentation**: Reference at least 1 relevant doc
8. **Find Related Questions**: Link to 2-4 related questions
9. **Add Keywords**: Include searchable terms
10. **Update Metadata**: Set dates, initialize metrics

### Updating Existing Questions

1. **Update `dateUpdated`**: Set to current date
2. **Review Accuracy**: Verify technical content is current
3. **Check Links**: Ensure doc paths still exist
4. **Update Version Info**: Add new version specifics if needed
5. **Refresh Related Questions**: Update cross-references
6. **Increment Metadata**: Update totalQuestions in meta if adding/removing

### Version Updates

When releasing a new Sindri version:

1. **Review Shared Questions**: Update answers to mention new version
2. **Add New Features**: Create questions for new capabilities
3. **Update Version Specifics**: Add new version details to shared questions
4. **Tag Breaking Changes**: Mark migration-related questions
5. **Update Meta**: Add new version to `versionsSupported`

### Quality Assurance Checklist

Before committing changes:

- [ ] Metadata count matches actual questions
- [ ] All question IDs are unique
- [ ] All categories/personas/useCases referenced are defined
- [ ] All relatedQuestions IDs exist
- [ ] All doc paths exist in repository
- [ ] No questions exceed 8 tags
- [ ] All questions have version tags
- [ ] All questions have at least 1 doc reference
- [ ] All questions have at least 1 persona
- [ ] All questions have at least 1 use case
- [ ] Version specifics match versionsApplicable
- [ ] JSON is valid (run through jq)
- [ ] lastUpdated date is current

## Automated Validation Script

Create `validate-faq.sh`:

```bash
#!/bin/bash
set -e

FAQ_FILE="docs/faq/src/v3-faq-data.json"

echo "Validating FAQ data schema..."

# Validate JSON syntax
if ! jq empty "$FAQ_FILE" 2>/dev/null; then
  echo "❌ Invalid JSON syntax"
  exit 1
fi

# Check metadata count
META_COUNT=$(jq '.meta.totalQuestions' "$FAQ_FILE")
ACTUAL_COUNT=$(jq '.questions | length' "$FAQ_FILE")

if [ "$META_COUNT" != "$ACTUAL_COUNT" ]; then
  echo "❌ Metadata mismatch: meta says $META_COUNT, actual count is $ACTUAL_COUNT"
  exit 1
else
  echo "✅ Question count: $ACTUAL_COUNT"
fi

# Check for duplicate question IDs
DUPLICATES=$(jq -r '.questions[].id' "$FAQ_FILE" | sort | uniq -d)
if [ -n "$DUPLICATES" ]; then
  echo "❌ Duplicate question IDs found:"
  echo "$DUPLICATES"
  exit 1
else
  echo "✅ All question IDs unique"
fi

# Validate category references
INVALID_CATS=$(comm -23 \
  <(jq -r '.questions[].category' "$FAQ_FILE" | sort -u) \
  <(jq -r '.categories[].id' "$FAQ_FILE" | sort))

if [ -n "$INVALID_CATS" ]; then
  echo "❌ Invalid category references:"
  echo "$INVALID_CATS"
  exit 1
else
  echo "✅ All category references valid"
fi

# Validate persona references
INVALID_PERSONAS=$(comm -23 \
  <(jq -r '.questions[].personas[]' "$FAQ_FILE" | sort -u) \
  <(jq -r '.personas[].id' "$FAQ_FILE" | sort))

if [ -n "$INVALID_PERSONAS" ]; then
  echo "❌ Invalid persona references:"
  echo "$INVALID_PERSONAS"
  exit 1
else
  echo "✅ All persona references valid"
fi

# Validate use case references
INVALID_USECASES=$(comm -23 \
  <(jq -r '.questions[].useCases[]' "$FAQ_FILE" | sort -u) \
  <(jq -r '.useCases[].id' "$FAQ_FILE" | sort))

if [ -n "$INVALID_USECASES" ]; then
  echo "❌ Invalid use case references:"
  echo "$INVALID_USECASES"
  exit 1
else
  echo "✅ All use case references valid"
fi

# Check for questions without version tags
NO_VERSION_TAGS=$(jq -r '.questions[] | select(.tags | any(test("v2|v3|migration")) | not) | .id' "$FAQ_FILE")

if [ -n "$NO_VERSION_TAGS" ]; then
  echo "⚠️  Questions without version tags:"
  echo "$NO_VERSION_TAGS"
else
  echo "✅ All questions have version tags"
fi

# Check for questions exceeding tag limit
EXCESSIVE_TAGS=$(jq '.questions[] | select(.tags | length > 8) | {id, count: (.tags | length)}' "$FAQ_FILE")

if [ -n "$EXCESSIVE_TAGS" ]; then
  echo "⚠️  Questions exceeding 8 tag limit:"
  echo "$EXCESSIVE_TAGS"
else
  echo "✅ All questions within tag limit"
fi

echo ""
echo "✅ Validation complete!"
echo ""
echo "Statistics:"
echo "  Questions: $ACTUAL_COUNT"
echo "  Categories: $(jq '.categories | length' "$FAQ_FILE")"
echo "  Personas: $(jq '.personas | length' "$FAQ_FILE")"
echo "  Use Cases: $(jq '.useCases | length' "$FAQ_FILE")"
echo "  v2-only: $(jq '[.questions[] | select(.versionsApplicable == ["v2"])] | length' "$FAQ_FILE")"
echo "  v3-only: $(jq '[.questions[] | select(.versionsApplicable == ["v3"])] | length' "$FAQ_FILE")"
echo "  Shared: $(jq '[.questions[] | select(.versionsApplicable == ["v2", "v3"] or .versionsApplicable == ["v3", "v2"])] | length' "$FAQ_FILE")"
```

Make executable:
```bash
chmod +x validate-faq.sh
```

## Current Statistics

As of 2026-02-11:

- **Total Questions**: 169
- **Categories**: 13
- **Personas**: 6
- **Use Cases**: 8
- **Version Breakdown**:
  - v2-only: 9 questions
  - v3-only: 78 questions
  - Shared v2/v3: 82 questions

## Future Enhancements

1. Create JSON Schema (.schema.json) for automated validation
2. Add automated tests for FAQ data integrity
3. Set up CI/CD checks for FAQ validation
4. Implement automated link checking for doc references
5. Add analytics tracking for persona/use-case usage
6. Consider A/B testing persona-based discovery
7. Multi-language support for international users

---

## References

- Planning Document: `v3/docs/planning/complete/v3-faq-data-schema-implementation.md`
- FAQ Data: `docs/faq/src/v3-faq-data.json`
- FAQ UI: `docs/faq/src/faq.js`
- Source Documentation: `v3/docs/`
