# Shannon - Autonomous AI Pentester

Shannon is installed and available for autonomous penetration testing of web applications.

## Usage

Shannon combines white-box source code analysis with black-box dynamic exploitation to find actual vulnerabilities.

### Basic Workflow

```bash
# 1. Ensure API key is set
export ANTHROPIC_API_KEY="your-api-key"
export CLAUDE_CODE_MAX_OUTPUT_TOKENS=64000

# 2. Run a pentest (requires URL + source code)
shannon start URL=https://your-app.com REPO=/path/to/your/repo

# 3. Monitor progress
shannon logs              # Real-time logs
shannon query ID=<id>     # Check specific workflow

# 4. Review results
# Output: ./audit-logs/{hostname}_{sessionId}/
# - session.json (metrics)
# - deliverables/comprehensive_security_assessment_report.md
```

### Commands

| Command                          | Purpose                         |
| -------------------------------- | ------------------------------- |
| `shannon start URL=... REPO=...` | Launch autonomous pentest       |
| `shannon logs`                   | View real-time worker logs      |
| `shannon query ID=...`           | Check workflow progress         |
| `shannon stop`                   | Stop containers (preserve data) |
| `shannon stop CLEAN=true`        | Full cleanup (remove all data)  |

### Parameters

- **URL**: Target application URL (required)
  - For local apps: use `http://host.docker.internal:PORT` instead of localhost
- **REPO**: Path to application source code (required)
- **CONFIG**: Optional configuration file path
- **OUTPUT**: Custom output directory
- **ROUTER**: Set `true` for alternative AI providers (experimental)

### Vulnerability Coverage

Shannon detects:

- Injection vulnerabilities (SQL, Command, etc.)
- Cross-Site Scripting (XSS)
- Server-Side Request Forgery (SSRF)
- Broken Authentication/Authorization
- 96.15% success rate on hint-free XBOW Benchmark

### Output Structure

Results are saved to `./audit-logs/{hostname}_{sessionId}/`:

```
audit-logs/{hostname}_{sessionId}/
├── session.json                                    # Metrics and session data
├── agents/                                         # Execution logs
├── prompts/                                        # Reproducibility snapshots
└── deliverables/
    └── comprehensive_security_assessment_report.md # Final report
```

### Requirements

- Docker must be running
- Source code access (white-box testing only)
- ANTHROPIC_API_KEY environment variable
- Recommended: `CLAUDE_CODE_MAX_OUTPUT_TOKENS=64000`

### Best Practices

1. **Always test authorized applications only** - Shannon is for authorized security testing
2. **Use staging environments** - Test against non-production instances
3. **Review source code access** - Ensure repository path is correct
4. **Monitor resource usage** - Docker containers may require significant resources
5. **Preserve audit logs** - Keep reports for compliance and remediation tracking

### Local Testing Setup

For testing applications running locally:

```bash
# Don't use localhost - use Docker's host gateway
shannon start \
  URL=http://host.docker.internal:3000 \
  REPO=/path/to/your/app
```

### Troubleshooting

- **Docker not running**: Ensure Docker Desktop/daemon is started
- **API key issues**: Verify ANTHROPIC_API_KEY is exported in current shell
- **Container failures**: Check `shannon logs` for error details
- **Permission denied**: Ensure repository path is accessible

## Documentation

- GitHub: https://github.com/KeygraphHQ/shannon
- Docker Install: https://docs.docker.com/get-docker/
- Anthropic API: https://console.anthropic.com
