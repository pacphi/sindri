# Shannon Extension

**Category:** Testing
**Version:** 1.0.0
**Install Method:** Script

## Overview

Shannon is a fully autonomous AI pentester that finds actual exploits in web applications by combining white-box source code analysis with black-box dynamic exploitation.

### Key Features

- **Autonomous Vulnerability Discovery**: Finds Injection, XSS, SSRF, and Broken Authentication/Authorization vulnerabilities
- **High Success Rate**: 96.15% success rate on the hint-free, source-aware XBOW Benchmark
- **Hybrid Approach**: Combines white-box source code analysis with black-box dynamic testing
- **Comprehensive Reports**: Generates detailed security assessment reports with exploitation details
- **Docker-based**: Containerized execution environment for safety and reproducibility

## Installation

Shannon requires Docker to be installed and running.

```bash
# Install Shannon extension
sindri extension install shannon

# Verify installation
shannon --version
```

### Requirements

- **Docker**: Must be installed and running ([install guide](https://docs.docker.com/get-docker/))
- **Anthropic API Key**: Required for AI-powered testing
- **Disk Space**: ~100 MB
- **Memory**: 512 MB recommended

### Environment Variables

Required:

```bash
export ANTHROPIC_API_KEY="your-api-key-here"
```

Recommended:

```bash
export CLAUDE_CODE_MAX_OUTPUT_TOKENS=64000
```

## Quick Start

### Basic Pentest

```bash
# Set up credentials
export ANTHROPIC_API_KEY="sk-ant-..."
export CLAUDE_CODE_MAX_OUTPUT_TOKENS=64000

# Run a pentest
shannon start \
  URL=https://your-app.com \
  REPO=/path/to/your/app/source

# Monitor progress
shannon logs

# Check specific workflow
shannon query ID=workflow-id
```

### Local Application Testing

For applications running on localhost, use Docker's host gateway:

```bash
shannon start \
  URL=http://host.docker.internal:3000 \
  REPO=/path/to/your/app
```

## Commands Reference

| Command                   | Description                     | Example                                        |
| ------------------------- | ------------------------------- | ---------------------------------------------- |
| `shannon start`           | Launch autonomous pentest       | `shannon start URL=https://app.com REPO=/path` |
| `shannon logs`            | View real-time worker logs      | `shannon logs`                                 |
| `shannon query`           | Check workflow progress         | `shannon query ID=abc123`                      |
| `shannon stop`            | Stop containers (preserve data) | `shannon stop`                                 |
| `shannon stop CLEAN=true` | Full cleanup                    | `shannon stop CLEAN=true`                      |

## Parameters

### Required Parameters

- **URL**: Target application URL
  - Format: `https://your-app.com` or `http://host.docker.internal:PORT`
  - Use `host.docker.internal` for local apps instead of `localhost`

- **REPO**: Absolute path to application source code
  - Required for white-box analysis
  - Must be accessible from the Docker container

### Optional Parameters

- **CONFIG**: Path to custom configuration file
- **OUTPUT**: Custom output directory (default: `./audit-logs`)
- **ROUTER**: Set to `true` for alternative AI providers (experimental, unsupported)

## Vulnerability Coverage

Shannon specializes in detecting:

1. **Injection Vulnerabilities**
   - SQL Injection
   - Command Injection
   - LDAP Injection

2. **Cross-Site Scripting (XSS)**
   - Reflected XSS
   - Stored XSS
   - DOM-based XSS

3. **Server-Side Request Forgery (SSRF)**
   - Internal service access
   - Cloud metadata exposure

4. **Broken Authentication/Authorization**
   - Privilege escalation
   - Authentication bypass
   - Session management flaws

## Output and Reports

### Output Directory Structure

Results are saved to `./audit-logs/{hostname}_{sessionId}/`:

```
audit-logs/app.com_20260208_123456/
├── session.json                                    # Metrics and session data
├── agents/                                         # Execution logs per agent
├── prompts/                                        # Reproducibility snapshots
└── deliverables/
    └── comprehensive_security_assessment_report.md # Final report
```

### Report Contents

The comprehensive security assessment report includes:

- Executive summary
- Vulnerability findings with severity ratings
- Proof-of-concept exploits
- Remediation recommendations
- Technical details and evidence

## Security and Authorization

**IMPORTANT: Shannon is for authorized security testing only.**

### Authorized Use Cases

✅ **Permitted**:

- Testing your own applications
- Authorized penetration testing engagements
- Security research with permission
- CTF challenges and competitions
- Defensive security assessments
- Bug bounty programs (within scope)

❌ **Prohibited**:

- Unauthorized testing of third-party applications
- Testing without explicit permission
- Malicious exploitation
- Attacks against production systems without authorization

### Best Practices

1. **Always obtain written authorization** before testing any application
2. **Use staging/development environments** instead of production
3. **Document the scope** of your testing engagement
4. **Review and approve** the target URL and repository before starting
5. **Preserve audit logs** for compliance and evidence
6. **Follow responsible disclosure** for any findings

## Configuration

### API Key Setup

Create a `.env` file in your project (alternative to exports):

```bash
ANTHROPIC_API_KEY=sk-ant-your-key-here
CLAUDE_CODE_MAX_OUTPUT_TOKENS=64000
```

### Custom Configuration

Shannon supports custom configuration via CONFIG parameter:

```bash
shannon start \
  URL=https://app.com \
  REPO=/path/to/repo \
  CONFIG=/path/to/config.yaml
```

## Troubleshooting

### Docker Issues

**Problem**: "Docker daemon is not running"

```bash
# Solution: Start Docker Desktop or Docker daemon
# macOS: Open Docker Desktop application
# Linux: sudo systemctl start docker
```

**Problem**: "Cannot connect to Docker daemon"

```bash
# Solution: Check Docker socket permissions
sudo chmod 666 /var/run/docker.sock
```

### Authentication Issues

**Problem**: "Missing ANTHROPIC_API_KEY"

```bash
# Solution: Set the environment variable
export ANTHROPIC_API_KEY="sk-ant-your-key"

# Verify it's set
echo $ANTHROPIC_API_KEY
```

**Problem**: "API rate limit exceeded"

```bash
# Solution: Increase output token limit
export CLAUDE_CODE_MAX_OUTPUT_TOKENS=64000
```

### Network Issues

**Problem**: "Cannot access localhost application"

```bash
# Solution: Use Docker host gateway instead
# ❌ Wrong: URL=http://localhost:3000
# ✅ Correct: URL=http://host.docker.internal:3000
```

### Logs and Debugging

View detailed logs:

```bash
# Real-time logs
shannon logs

# Follow logs continuously
shannon logs | tail -f

# Check Docker container status
docker ps | grep shannon
```

## Performance Considerations

### Resource Requirements

- **CPU**: Multi-core recommended for parallel analysis
- **Memory**: Minimum 512 MB, 2 GB+ recommended for large applications
- **Disk**: Variable based on application size and audit logs
- **Network**: Stable internet connection for AI API calls

### Optimization Tips

1. **Limit scope**: Test specific components instead of entire applications
2. **Use staging data**: Smaller datasets reduce processing time
3. **Monitor resources**: Watch Docker resource usage during execution
4. **Clean up**: Use `shannon stop CLEAN=true` to remove old data

## Integration

### CI/CD Integration

Example GitHub Actions workflow:

```yaml
name: Security Testing
on: [push, pull_request]

jobs:
  pentest:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Shannon
        run: |
          sindri extension install shannon

      - name: Run Shannon
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          shannon start \
            URL=http://localhost:3000 \
            REPO=${{ github.workspace }}

      - name: Upload Results
        uses: actions/upload-artifact@v4
        with:
          name: shannon-reports
          path: audit-logs/
```

### Script Automation

```bash
#!/bin/bash
# automated-pentest.sh

set -euo pipefail

APP_URL="https://staging.app.com"
REPO_PATH="/path/to/repo"

# Run pentest
shannon start URL="${APP_URL}" REPO="${REPO_PATH}"

# Wait for completion (implement polling)
sleep 300

# Extract findings
REPORT_PATH=$(find audit-logs -name "comprehensive_security_assessment_report.md" | head -1)

# Send notification
echo "Pentest complete. Report: ${REPORT_PATH}"
```

## Benchmark Performance

Shannon achieves **96.15% success rate** on the XBOW Benchmark:

- Hint-free testing (no manual guidance)
- Source-aware analysis (white-box access)
- Real exploitation validation

## Project Context

When installed in a Sindri project, Shannon adds guidance to `CLAUDE.md` for:

- Quick command reference
- Best practices
- Security considerations
- Integration examples

## Related Extensions

- **docker**: Docker runtime (required dependency)
- **claude**: Enhanced Claude AI integration
- **mcp**: Model Context Protocol servers

## Resources

- **GitHub Repository**: https://github.com/KeygraphHQ/shannon
- **Docker Installation**: https://docs.docker.com/get-docker/
- **Anthropic Console**: https://console.anthropic.com
- **XBOW Benchmark**: Referenced in Shannon documentation

## Support

For issues or questions:

1. Check Shannon GitHub issues: https://github.com/KeygraphHQ/shannon/issues
2. Review Docker logs: `shannon logs`
3. Verify Docker status: `docker info`
4. Check Sindri extension status: `sindri extension status shannon`

## Changelog

### Version 1.0.0 (2026-02-08)

- Initial Sindri V3 extension release
- Docker-based installation
- Anthropic API integration
- Project context integration
- Comprehensive documentation
