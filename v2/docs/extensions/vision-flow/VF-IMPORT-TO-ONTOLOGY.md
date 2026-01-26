# VF-Import-To-Ontology

Document to ontology import and semantic indexing.

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

Document to ontology import and semantic indexing (from VisionFlow) - provides document ingestion and semantic indexing to knowledge graphs.

## Installed Tools

| Tool     | Type    | Description    |
| -------- | ------- | -------------- |
| `rdflib` | library | RDF processing |

## Configuration

### Templates

| Template   | Destination                                   | Description         |
| ---------- | --------------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-import-to-ontology/SKILL.md` | Skill documentation |

## Network Requirements

- `pypi.org` - Python packages

## Installation

```bash
extension-manager install vf-import-to-ontology
```

## Validation

```bash
test -d ~/extensions/vf-import-to-ontology
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-import-to-ontology
```

## Removal

```bash
extension-manager remove vf-import-to-ontology
```

Removes:

- `~/extensions/vf-import-to-ontology`

## Related Extensions

- [vf-ontology-enrich](VF-ONTOLOGY-ENRICH.md) - Ontology enrichment
- [vf-pdf](VF-PDF.md) - PDF extraction
- [vf-docx](VF-DOCX.md) - Word extraction
