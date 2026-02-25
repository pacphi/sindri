# Clarity Spec Generation

This project uses Clarity for autonomous specification generation.

## Usage

Invoke `/clarity` followed by references to generate implementable specifications:

```
/clarity https://github.com/someone/repo
/clarity /path/to/codebase "feature description"
/clarity quick /path/to/codebase
/clarity resume
/clarity evaluate
```

## Holdout Scenarios

Do NOT read or reference the `scenarios/` directory. It contains holdout test scenarios for independent evaluation.

## Artifacts

- `.clarity/context.md` — Extracted context from references
- `.clarity/spec.md` — Structured specification
- `scenarios/SC-NNN-*.md` — Holdout behavioral scenarios
- `.clarity/handoff.md` — Implementation prompt
- `.clarity/evaluations/` — Evaluation reports
