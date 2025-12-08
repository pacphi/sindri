# Frequently Asked Questions

Sindri includes an interactive FAQ page with 40+ searchable questions covering setup, configuration, deployment, extensions, secrets, troubleshooting, architecture, and CI/CD.

## Viewing the FAQ

Open the pre-built FAQ in your browser:

```bash
open docs/faq/index.html
# Or on Linux: xdg-open docs/faq/index.html
```

The FAQ is a self-contained HTML file that works without a server. Features include:

- Full-text search across all questions and answers
- Category filtering (Getting Started, Configuration, Deployment, etc.)
- Keyboard navigation (arrow keys, Enter to expand)
- Links to relevant documentation

## Rebuilding the FAQ

If you modify the FAQ content, rebuild the page:

```bash
pnpm build:faq
```

This combines the source files in `docs/faq/src/` into a single `docs/faq/index.html`.

### Source Files

| File | Purpose |
|------|---------|
| `src/faq-data.json` | Questions, answers, categories, and tags |
| `src/index.html` | HTML template with styles |
| `src/faq.js` | Search, filtering, and UI logic |

### Adding Questions

Edit `docs/faq/src/faq-data.json`:

```json
{
  "id": "unique-id",
  "category": "getting-started",
  "question": "How do I...?",
  "answer": "You can...",
  "tags": ["keyword1", "keyword2"],
  "docs": ["docs/RELEVANT.md"]
}
```

Categories: `getting-started`, `configuration`, `deployment`, `extensions`, `secrets`, `troubleshooting`, `architecture`, `cicd`

After editing, run `pnpm build:faq` to regenerate the page.
