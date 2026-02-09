# Shannon Extension for Sindri V3

Shannon is a fully autonomous AI pentester that finds actual exploits in web applications by combining white-box source code analysis with black-box dynamic exploitation.

## Quick Start

```bash
# Install Shannon extension
sindri extension install shannon

# Set required API key
export ANTHROPIC_API_KEY="your-api-key"
export CLAUDE_CODE_MAX_OUTPUT_TOKENS=64000

# Run a pentest
shannon start URL=https://your-app.com REPO=/path/to/your/repo

# Monitor progress
shannon logs
```

## Features

- **Autonomous Vulnerability Discovery**: Injection, XSS, SSRF, Broken Auth/AuthZ
- **96.15% Success Rate**: On hint-free XBOW Benchmark
- **Hybrid Testing**: White-box + black-box analysis
- **Docker-based**: Containerized for safety and reproducibility

## Requirements

- Docker installed and running
- Anthropic API key
- Source code access for target application

## Files

```
v3/extensions/shannon/
├── extension.yaml        # Extension definition
├── scripts/
│   ├── install.sh        # Installation script
│   └── uninstall.sh      # Uninstallation script
├── templates/
│   └── SKILL.md          # Project context for CLAUDE.md
└── README.md             # This file
```

## Installation Method

Uses **script** installation method:

- Clones Shannon repository to `~/.shannon/`
- Creates wrapper script for easy command access
- Pre-pulls Docker images
- Requires Docker to be installed

## Documentation

- Extension docs: `v3/docs/extensions/SHANNON.md`
- Main catalog: `v3/docs/EXTENSIONS.md`
- GitHub: https://github.com/KeygraphHQ/shannon

## Security Notice

**IMPORTANT**: Shannon is for authorized security testing only.

✅ Permitted:

- Your own applications
- Authorized penetration tests
- Security research with permission
- CTF challenges
- Bug bounty programs (within scope)

❌ Prohibited:

- Unauthorized testing
- Malicious exploitation
- Production attacks without authorization

## Troubleshooting

### Docker not running

```bash
# macOS: Open Docker Desktop
# Linux: sudo systemctl start docker
```

### Missing API key

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key"
echo $ANTHROPIC_API_KEY  # Verify
```

### Local app access

```bash
# Use Docker host gateway instead of localhost
shannon start URL=http://host.docker.internal:3000 REPO=/path
```

## Output

Results are saved to `./audit-logs/{hostname}_{sessionId}/`:

- `session.json` - Metrics and data
- `deliverables/comprehensive_security_assessment_report.md` - Final report
- `agents/` - Execution logs
- `prompts/` - Reproducibility snapshots

## Version

Current version: 1.0.0

## License

Shannon: See https://github.com/KeygraphHQ/shannon for license information
