# VF-Ontology-Enrich

AI-powered ontology enrichment with OWL2 validation.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | ai                     |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 200 MB                 |
| **Memory**       | 512 MB                 |
| **Dependencies** | [python](../PYTHON.md) |

## Description

AI-powered ontology enrichment with OWL2 validation (from VisionFlow) - provides semantic enrichment pipeline with OWL2 compliance, batch processing, and automatic git rollback on failure.

## Installed Tools

| Tool     | Type    | Description         |
| -------- | ------- | ------------------- |
| `rdflib` | library | RDF/OWL2 processing |
| `owlrl`  | library | OWL reasoning       |

## Configuration

### Templates

| Template   | Destination                                | Description         |
| ---------- | ------------------------------------------ | ------------------- |
| `SKILL.md` | `~/extensions/vf-ontology-enrich/SKILL.md` | Skill documentation |

## Secrets (Required)

| Secret               | Description        |
| -------------------- | ------------------ |
| `perplexity_api_key` | Perplexity API key |

## Network Requirements

- `pypi.org` - Python packages
- `api.perplexity.ai` - Perplexity API

## Installation

```bash
extension-manager install vf-ontology-enrich
```

## Validation

```bash
python3 -c "import rdflib; print('ok')"
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-ontology-enrich
```

## Removal

```bash
extension-manager remove vf-ontology-enrich
```

Removes:

- `~/extensions/vf-ontology-enrich`

## Related Extensions

- [vf-import-to-ontology](VF-IMPORT-TO-ONTOLOGY.md) - Document import
- [vf-perplexity](VF-PERPLEXITY.md) - Research API

## Additional Notes

- Strict OWL2 validation with field preservation
- UK English context optimization
- Git-based automatic rollback on enrichment failure
- Batch processing with rate limiting
