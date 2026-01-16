# Ralph

AI-driven autonomous development system that builds software projects with minimal human intervention.

## Overview

| Property         | Value  |
| ---------------- | ------ |
| **Category**     | ai     |
| **Version**      | 1.0.0  |
| **Installation** | script |
| **Disk Space**   | 300 MB |
| **Memory**       | 512 MB |
| **Dependencies** | nodejs |

## Description

Ralph Inferno is an AI-driven autonomous development system that leverages Claude to build complete software projects while you sleep. The system operates through a structured workflow of discovery, planning, deployment, and review phases.

**Key Features:**

- **Autonomous Discovery**: Claude explores project requirements from multiple perspectives (analyst, PM, UX, architect, business)
- **Structured Planning**: Breaks down findings into numbered specification files
- **Automated Deployment**: Pushes code to GitHub and executes builds on remote VMs
- **Interactive Review**: Opens SSH tunnels for testing generated applications
- **Iterative Improvement**: Supports change requests for bug fixes and enhancements

**⚠️ CRITICAL**: Always run Ralph on a **disposable VM**, never on your local machine. Ralph executes autonomous code generation and deployment.

## Installed Tools

Ralph is executed via `npx ralph-inferno`, which always uses the latest version from npm.

## Configuration

### Project Configuration

Ralph creates a `.ralph/config.json` file during initialization containing:

- Version information
- Language preference
- Cloud provider selection
- VM naming and SSH configuration
- GitHub credentials
- Claude authentication method
- Optional ntfy.sh notification settings

### Environment Variables

| Variable            | Description                   | Required |
| ------------------- | ----------------------------- | -------- |
| `ANTHROPIC_API_KEY` | Claude API key                | Yes      |
| `GITHUB_TOKEN`      | GitHub personal access token  | Yes      |
| `RALPH_HOME`        | Ralph configuration directory | No       |

## Network Requirements

- `registry.npmjs.org` - npm package registry
- `github.com` - Source code repository and deployment
- `api.anthropic.com` - Claude API for autonomous development

## Installation

```bash
# Install ralph extension
extension-manager install ralph

# Initialize in a project directory (creates .ralph/config.json)
npx ralph-inferno install
```

During initialization, you'll configure:

1. VM provider (Hetzner, GCP, DigitalOcean, AWS, self-hosted)
2. GitHub repository settings
3. Claude authentication (API key or CLI subscription)
4. Optional notification settings

## Validation

```bash
# Check Node.js is available
node --version

# Check npx is available
npx --version

# Verify ralph can be executed
test -f .ralph/config.json && echo "Ralph initialized"
```

## Available Commands

Ralph provides several `/ralph:` commands for Claude Code:

| Command                 | Purpose                                  |
| ----------------------- | ---------------------------------------- |
| `/ralph:discover`       | Autonomous exploration with web research |
| `/ralph:plan`           | Specification generation from PRD        |
| `/ralph:deploy`         | GitHub push and VM execution             |
| `/ralph:review`         | SSH tunneling and app testing            |
| `/ralph:change-request` | Bug documentation and fix specifications |
| `/ralph:status`         | Progress monitoring                      |
| `/ralph:abort`          | Execution termination                    |

## Deployment Modes

Ralph supports three deployment modes:

### Quick Mode

- Specification execution
- Build verification
- Fast iteration cycle

### Standard Mode (Default)

- All Quick Mode features
- E2E testing
- Automated change request generation

### Inferno Mode

- All Standard Mode features
- Design review
- Parallel worktree processing
- Maximum automation

## Usage Examples

### Initialize Ralph in a Project

```bash
# Navigate to project directory
cd my-new-project

# Initialize Ralph
npx ralph-inferno install

# Follow interactive prompts to configure VM, GitHub, and auth
```

### Autonomous Development Workflow

```bash
# 1. Start with discovery phase
# In Claude Code:
/ralph:discover

# 2. Generate specifications
/ralph:plan

# 3. Deploy to VM and build
/ralph:deploy

# 4. Review and test
/ralph:review

# 5. Request changes if needed
/ralph:change-request
```

### Update Configuration

```bash
# Re-run installer to update settings (preserves existing config)
npx ralph-inferno install
```

### Check Status

```bash
# In Claude Code:
/ralph:status
```

## Remote VM Requirements

Your remote VM must have:

- SSH access configured
- Git installed
- Claude Code CLI installed
- Either:
  - ANTHROPIC_API_KEY environment variable set, OR
  - Claude Code CLI authenticated via subscription (Max/Pro plan)

## Best Practices

1. **Always use a disposable VM** - Never run on your local machine or production servers
2. **Monitor builds** - Use `/ralph:status` to track progress
3. **Review before merging** - Use `/ralph:review` to test generated code before merging to main
4. **Keep credentials secure** - Store API keys and tokens securely, never commit to git
5. **Start small** - Test with small projects before scaling to larger applications
6. **Use version control** - Ralph integrates with GitHub; keep your repository clean

## Removal

```bash
extension-manager remove ralph
```

Requires confirmation. Removes:

- Ralph home directory (`~/.ralph`)
- Project `.ralph` directory (if present)

**Note**: Does not remove npm package (executed via npx on-demand)

## Troubleshooting

### Authentication Issues

- Ensure `ANTHROPIC_API_KEY` is set or Claude Code CLI is authenticated
- Verify GitHub token has required permissions (repo, workflow)

### VM Connection Issues

- Verify SSH key is added to VM
- Check VM is running and accessible
- Ensure VM has required tools installed

### Build Failures

- Check `/ralph:status` for detailed error logs
- Use `/ralph:change-request` to document issues for AI-powered fixes
- Verify VM has sufficient resources (disk space, memory)

## Related Extensions

- [nodejs](NODEJS.md) - Required Node.js runtime
- [ai-toolkit](AI-TOOLKIT.md) - Additional AI development tools
- [claude-flow-v2](CLAUDE-FLOW-V2.md) - Multi-agent orchestration
- [agentic-qe](AGENTIC-QE.md) - AI-powered testing framework

## References

- **Homepage**: https://github.com/sandstream/ralph-inferno
- **License**: MIT
- **Documentation**: See repository README for detailed workflow guides
