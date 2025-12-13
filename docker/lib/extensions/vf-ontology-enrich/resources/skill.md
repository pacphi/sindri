---
name: ontology-enrich
description: In-place validation, enrichment, and maintenance of existing ontology files with OWL2 compliance and Perplexity API integration
version: 1.0.0
category: ontology
layer: 1
dependencies:
  - ontology-core
tags:
  - ontology
  - validation
  - enrichment
  - owl2
  - perplexity
---

# Ontology Enrich Skill

## Purpose

Safe, validated enrichment and maintenance of existing ontology files with:

- OWL2 validation before any modification
- Perplexity API content enrichment with UK English focus
- Link validation and broken reference detection
- Automatic rollback on validation failure
- Full field preservation through ontology-core integration

## Architecture

**Layer 1 Skill**: Imports ontology-core library for all parsing/modification operations.

**Key Principle**: Never duplicates parsing logic - delegates to ontology-core for immutable, validated modifications.

## Usage

### 1. Validate Ontology File

```bash
# Check OWL2 compliance without modifications
python -m ontology_enrich.validate \
  --file mainKnowledgeGraph/pages/concept.md
```

**Output**:

- Validation status (PASS/FAIL)
- List of OWL2 violations if any
- Field completeness report

### 2. Enrich Content with Perplexity

```bash
# Enrich definition field with curated research
python -m ontology_enrich.enrich \
  --file mainKnowledgeGraph/pages/concept.md \
  --field definition \
  --api-key $PERPLEXITY_API_KEY
```

**Process**:

1. Parse ontology with full field preservation
2. Validate OWL2 compliance
3. Query Perplexity API with UK English context
4. Extract citations and structured content
5. Immutably modify field
6. Validate modification
7. Write back or rollback on failure

### 3. Fix Broken Links

```bash
# Detect and report broken internal references
python -m ontology_enrich.fix_links \
  --file mainKnowledgeGraph/pages/concept.md \
  --auto-fix
```

**Actions**:

- Validates all `[[wiki-link]]` references
- Checks file existence
- Reports broken links
- Optionally removes or suggests replacements

### 4. Batch Enrichment

```bash
# Enrich multiple files with rate limiting
python -m ontology_enrich.batch \
  --pattern "mainKnowledgeGraph/pages/*.md" \
  --field definition \
  --api-key $PERPLEXITY_API_KEY \
  --rate-limit 10
```

## Integration with Ontology-Core

All parsing and modification operations delegate to ontology-core:

```python
from ontology_core.src.ontology_parser import parse_ontology_block, write_ontology_block
from ontology_core.src.ontology_modifier import modify_field, validate_modification
from ontology_core.src.owl2_validator import validate_ontology

# Parse preserving ALL fields
ontology = parse_ontology_block(file_path)

# Validate before modification
validation = validate_ontology(ontology)
if not validation.is_valid:
    raise ValidationError(validation.errors)

# Immutable modification
modified = modify_field(ontology, 'definition', new_content)

# Validate modification
if validate_modification(modified):
    write_ontology_block(file_path, modified)
else:
    rollback(file_path)
```

## Perplexity API Integration

### UK English Focus

All queries include UK English context:

```python
query = f"""
Context: UK-based technical documentation for AI systems ontology.
Preferred: British English spelling, terminology, and conventions.

Task: Enrich the following ontology definition with:
- Clear, technical explanation
- Real-world examples from UK tech sector
- Citations from authoritative sources
- Relationships to other concepts

Definition: {current_definition}
"""
```

### Citation Extraction

Responses include structured citations:

```python
enriched_content = {
    'definition': 'Enhanced definition text...',
    'citations': [
        {'source': 'Source Name', 'url': 'https://...', 'relevance': 0.95},
        # ...
    ],
    'related_concepts': ['Concept1', 'Concept2']
}
```

## Error Handling

### Validation Failures

```python
try:
    enriched = enrich_with_perplexity(file_path, api_key)
except ValidationError as e:
    logger.error(f"OWL2 validation failed: {e.errors}")
    rollback(file_path)
except PerplexityAPIError as e:
    logger.error(f"API error: {e.message}")
    # Original file unchanged
```

### Automatic Rollback

- Git-based rollback on validation failure
- Preserves original file until validation passes
- Logs all modification attempts

## Examples

### Example 1: Validate Existing File

```bash
$ python -m ontology_enrich.validate \
    --file mainKnowledgeGraph/pages/AI_Agent.md

✓ OWL2 Validation: PASS
✓ Required fields: title, definition, ontology_type
✓ Optional fields: relationships, properties, examples
⚠ Missing field: examples (recommended for Class types)
```

### Example 2: Enrich Definition

```bash
$ python -m ontology_enrich.enrich \
    --file mainKnowledgeGraph/pages/AI_Agent.md \
    --field definition \
    --api-key $PERPLEXITY_API_KEY

⏳ Parsing ontology block...
✓ OWL2 validation passed
⏳ Querying Perplexity API (UK English context)...
✓ Received enriched content with 3 citations
⏳ Validating modification...
✓ Modification validated
✓ File updated: mainKnowledgeGraph/pages/AI_Agent.md

Citations added:
  1. "Artificial Intelligence: A Modern Approach" - Russell & Norvig
  2. UK AI Council Technical Report (2024)
  3. IEEE AI Standards (British English edition)
```

### Example 3: Fix Broken Links

```bash
$ python -m ontology_enrich.fix_links \
    --file mainKnowledgeGraph/pages/Machine_Learning.md \
    --auto-fix

⏳ Scanning references...
⚠ Broken link: [[Deep_Learning_Algorithm]] (file not found)
⚠ Broken link: [[Neural_Network_Architecture]] (file not found)

Suggestions:
  [[Deep_Learning_Algorithm]] → [[Deep_Learning]]
  [[Neural_Network_Architecture]] → [[Neural_Network]]

Apply fixes? [y/N]: y
✓ Updated 2 references
✓ OWL2 validation passed
✓ File updated
```

## API Reference

### EnrichmentWorkflow

Main orchestration class for enrichment operations.

```python
class EnrichmentWorkflow:
    def __init__(self, api_key: str, uk_english: bool = True):
        """Initialize enrichment workflow with Perplexity API."""

    def enrich_field(self, file_path: str, field_name: str) -> EnrichmentResult:
        """Enrich specific field with Perplexity API content."""

    def validate_file(self, file_path: str) -> ValidationResult:
        """Validate OWL2 compliance without modification."""

    def fix_broken_links(self, file_path: str, auto_fix: bool = False) -> LinkReport:
        """Detect and optionally fix broken wiki-link references."""
```

### PerplexityClient

API client with UK English focus and citation extraction.

```python
class PerplexityClient:
    def __init__(self, api_key: str):
        """Initialize Perplexity API client."""

    def enrich_definition(self, current_def: str, context: str) -> EnrichedContent:
        """Query API for enriched content with citations."""

    def extract_citations(self, response: dict) -> List[Citation]:
        """Extract structured citations from API response."""
```

## Configuration

### Environment Variables

```bash
# Required
export PERPLEXITY_API_KEY="pplx-..."

# Optional
export ONTOLOGY_ENRICH_UK_ENGLISH=true
export ONTOLOGY_ENRICH_RATE_LIMIT=10
export ONTOLOGY_ENRICH_CITATION_MIN_RELEVANCE=0.7
```

### Configuration File

`config/enrichment_config.yaml`:

```yaml
perplexity:
  model: "llama-3.1-sonar-large-128k-online"
  temperature: 0.2
  max_tokens: 2000
  uk_english: true

validation:
  strict_owl2: true
  require_citations: true
  min_definition_length: 50

links:
  auto_fix_threshold: 0.8
  suggest_alternatives: true
```

## Best Practices

1. **Always validate before enrichment**: Run validation check before any modification
2. **Use batch mode for multiple files**: Respect rate limits with batch processing
3. **Review citations**: Verify Perplexity citations for accuracy
4. **Incremental enrichment**: Enrich one field at a time
5. **Git integration**: Commit validated changes before further modifications

## Troubleshooting

### OWL2 Validation Fails

```bash
# Check specific validation errors
python -m ontology_enrich.validate \
  --file path/to/file.md \
  --verbose
```

### API Rate Limiting

```bash
# Use batch mode with rate limiting
python -m ontology_enrich.batch \
  --pattern "pages/*.md" \
  --rate-limit 5 \
  --retry-on-limit
```

### Field Preservation Issues

If fields are lost during enrichment:

1. Check ontology-core version compatibility
2. Verify immutable modification pattern usage
3. Review logs for parsing errors

## Future Enhancements

- [ ] Multi-language support (maintain UK English default)
- [ ] Semantic similarity matching for link suggestions
- [ ] Automated relationship inference
- [ ] Integration with ontology-visualizer
- [ ] Bulk citation management
