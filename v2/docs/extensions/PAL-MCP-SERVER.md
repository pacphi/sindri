# PAL MCP Server

AI orchestration and multi-model collaboration MCP server with 18 specialized tools for enhanced code analysis, problem-solving, and collaborative development.

## Overview

| Property         | Value                                              |
| ---------------- | -------------------------------------------------- |
| **Category**     | ai                                                 |
| **Version**      | 9.8.2                                              |
| **Installation** | script                                             |
| **Disk Space**   | 150 MB                                             |
| **Dependencies** | [python](PYTHON.md), [mise-config](MISE-CONFIG.md) |

## Description

PAL MCP (Provider Abstraction Layer) is an AI orchestration server that enables your AI assistant to collaborate with multiple AI models within a single workflow. It provides a comprehensive set of MCP tools for enhanced code analysis, professional code reviews, systematic debugging, strategic planning, consensus building, and multi-model collaboration.

**Key Features:**

- **Multi-Model Orchestration** - Connect to 50+ models across Gemini, OpenAI, Azure, Grok, Ollama, and more
- **Conversation Continuity** - Context flows seamlessly across tools and models
- **CLI-to-CLI Bridge** - Spawn subagents in isolated contexts for parallel work
- **Professional Workflows** - Systematic code reviews, debugging, planning, consensus
- **Vision Capabilities** - Analyze screenshots, diagrams, and visual content
- **Local Model Support** - Run Llama, Mistral locally for privacy and zero API costs

## Installed Tools

| Tool             | Type       | Description                                        |
| ---------------- | ---------- | -------------------------------------------------- |
| `pal-mcp-server` | mcp-server | AI orchestration server (stdio transport)          |
| `chat`           | mcp-tool   | Multi-model conversations with code generation     |
| `thinkdeep`      | mcp-tool   | Extended reasoning with configurable thinking      |
| `planner`        | mcp-tool   | Strategic planning and project breakdown           |
| `consensus`      | mcp-tool   | Multi-model debate and decision-making             |
| `codereview`     | mcp-tool   | Professional code reviews with severity levels     |
| `precommit`      | mcp-tool   | Pre-commit validation and regression prevention    |
| `debug`          | mcp-tool   | Systematic investigation and root cause analysis   |
| `clink`          | mcp-tool   | CLI-to-CLI bridge (spawn subagents)                |
| `analyze`        | mcp-tool   | Codebase architecture analysis (disabled default)  |
| `refactor`       | mcp-tool   | Intelligent code refactoring (disabled default)    |
| `testgen`        | mcp-tool   | Test generation with edge cases (disabled default) |
| `secaudit`       | mcp-tool   | Security audits with OWASP (disabled default)      |
| `docgen`         | mcp-tool   | Documentation generation (disabled default)        |
| `apilookup`      | mcp-tool   | Force current-year API documentation lookups       |
| `challenge`      | mcp-tool   | Critical analysis to prevent reflexive agreement   |
| `tracer`         | mcp-tool   | Static call-flow mapping (disabled default)        |
| `version`        | mcp-tool   | Server version information                         |
| `listmodels`     | mcp-tool   | Available model listing                            |

## Configuration

### Installation Directory

````text
~/extensions/pal-mcp-server/
├── server.py              # MCP server entry point
├── .pal_venv/             # Python virtual environment
├── .env                   # Configuration and API keys
├── run-server.sh          # Setup and run script
├── SKILL.md               # Documentation
└── logs/                  # Server logs
    ├── mcp_server.log     # Main server log
    └── mcp_activity.log   # Tool activity log
```text

### API Keys

Configure at least one API key in `~/extensions/pal-mcp-server/.env`:

```bash
# Cloud Providers (choose one or more)
GEMINI_API_KEY=your_gemini_key
OPENAI_API_KEY=your_openai_key
XAI_API_KEY=your_grok_key
AZURE_OPENAI_API_KEY=your_azure_key
OPENROUTER_API_KEY=your_openrouter_key

# Local Models (no API key needed)
CUSTOM_API_URL=http://localhost:11434  # Ollama
```text

### MCP Client Registration

The extension automatically registers with Claude Code:

```json
{
  "mcpServers": {
    "pal": {
      "command": "/path/to/.pal_venv/bin/python",
      "args": ["/path/to/server.py"],
      "env": {}
    }
  }
}
```text

**Note:** API keys are loaded from `.env` file, not passed via MCP client environment.

### Tool Configuration

Some tools are disabled by default to reduce token usage. Enable in `.env`:

```bash
# Enable optional tools
PAL_ENABLE_ANALYZE=true
PAL_ENABLE_REFACTOR=true
PAL_ENABLE_TESTGEN=true
PAL_ENABLE_SECAUDIT=true
PAL_ENABLE_DOCGEN=true
PAL_ENABLE_TRACER=true
```text

## Network Requirements

- `github.com` - Repository cloning
- `raw.githubusercontent.com` - Resource downloads
- `pypi.org`, `files.pythonhosted.org` - Python package installation
- Various AI provider APIs (depends on configured providers)

## Installation

```bash
extension-manager install pal-mcp-server
```text

**Post-Installation:**

1. Configure API keys in `~/extensions/pal-mcp-server/.env`
2. Restart Claude Code to load the MCP server
3. Verify tools are available in Claude Code

## Usage Examples

### Multi-Model Code Review

```text
Perform a codereview using gemini pro and o3 for the authentication module
```text

The codereview tool will:

- Walk through the codebase systematically
- Analyze with multiple AI models
- Provide severity levels for issues
- Generate actionable recommendations

### Debugging with Context Continuity

```text
1. Use debug with gemini pro to investigate the memory leak in the worker process
2. Continue with o3 to verify the hypothesis about goroutine leaks
3. Use thinkdeep to explore alternative solutions
```text

### Consensus-Driven Development

```text
Use consensus with gpt-5, gemini pro, and o3 to decide:
Should we implement dark mode or offline support next?

Continue with clink gemini to implement the recommended feature
```text

### CLI Subagent Pattern

```text
# Main context: High-level planning
Use planner to outline the microservice architecture for the payment system

# Spawn isolated subagent for detailed review
clink with codex codereviewer to audit the payment service for security issues

# Continue in main context with insights from subagent
Implement the recommended security improvements based on the review
```text

### Strategic Planning

```text
Use planner to create a detailed implementation plan for migrating to Kubernetes
```text

### Pre-Commit Validation

```text
Run precommit check to validate these changes before I commit
```text

## Validation

```bash
extension-manager status pal-mcp-server
```text

**Expected Output:**

- Installation directory exists
- Python virtual environment configured
- MCP server registered in Claude Code

**Manual Validation:**

```bash
cd ~/extensions/pal-mcp-server
.pal_venv/bin/python server.py --version
```text

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade pal-mcp-server
```text

This will:

- Pull latest version from GitHub
- Reinstall Python dependencies
- Preserve your `.env` configuration

## Removal

```bash
extension-manager remove pal-mcp-server
```text

Removes:

- `~/extensions/pal-mcp-server` directory
- MCP server registration from `~/.claude/settings.json`

**Note:** Backup your `.env` file if you want to preserve API key configuration.

## Supported AI Providers

### Cloud Providers

- **Google Gemini** - Pro, Flash, Exp models (1M+ token context)
- **OpenAI** - GPT-5, o3, o3-mini, GPT-4 family
- **Anthropic** - Claude models (via OpenRouter)
- **X.AI** - Grok models
- **Azure OpenAI** - Azure-hosted models
- **OpenRouter** - Unified access to 50+ models
- **DIAL** - Enterprise AI platform

### Local Providers

- **Ollama** - Llama, Mistral, Codellama, and 100+ models
- **vLLM** - Fast local inference server
- **LM Studio** - Desktop local model hosting
- **Custom Endpoints** - Any OpenAI-compatible API

## Troubleshooting

### MCP Server Not Found in Claude Code

**Problem:** Tools don't appear in Claude Code

**Solutions:**

1. Check registration: `cat ~/.claude/settings.json | grep pal`
2. Verify installation: `extension-manager status pal-mcp-server`
3. Restart Claude Code
4. Check logs: `tail -f ~/extensions/pal-mcp-server/logs/mcp_server.log`

### API Keys Not Working

**Problem:** "Authentication failed" errors

**Solutions:**

1. Verify `.env` file: `cat ~/extensions/pal-mcp-server/.env`
2. Check API key validity with provider
3. Ensure at least one API key is configured
4. Restart Claude Code after modifying `.env`

### Python Environment Issues

**Problem:** "Module not found" errors

**Solutions:**

1. Verify Python version: `python3 --version` (3.10+ required)
2. Check virtual environment: `~/extensions/pal-mcp-server/.pal_venv/bin/python --version`
3. Reinstall: `extension-manager upgrade pal-mcp-server`

### Tools Not Showing Up

**Problem:** Only some tools are available

**Solutions:**

1. Some tools are disabled by default (analyze, refactor, testgen, secaudit, docgen, tracer)
2. Enable in `.env`: Set `PAL_ENABLE_<TOOL>=true`
3. Restart Claude Code after enabling

### Context Limit Issues

**Problem:** "Token limit exceeded" errors

**Solutions:**

1. Use models with larger context windows (Gemini Pro: 1M tokens)
2. Enable conversation summarization in `.env`
3. Use `clink` to spawn subagents for isolated work

## Tips & Best Practices

1. **Context Revival**: When your main assistant's context resets, continue conversations with another model to "remind" your assistant
2. **Model Selection**: Use Gemini Pro for extended thinking, Flash for speed, o3 for reasoning
3. **Disable Unused Tools**: Edit `.env` to disable tools you don't need (reduces token usage)
4. **Local Models**: Use Ollama for privacy and zero API costs
5. **Vision Tasks**: Use vision-enabled models for analyzing screenshots and diagrams
6. **Subagent Pattern**: Use `clink` to spawn isolated subagents for heavy analysis without polluting your main context

## Source Project

- **Repository:** [BeehiveInnovations/pal-mcp-server](https://github.com/BeehiveInnovations/pal-mcp-server)
- **License:** MIT
- **PURL:** `pkg:github/BeehiveInnovations/pal-mcp-server@v9.8.2`
- **Documentation:** [Tool Documentation](https://github.com/BeehiveInnovations/pal-mcp-server/tree/main/docs/tools)

## Related Extensions

- [python](PYTHON.md) - Required runtime for the MCP server
- [ollama](OLLAMA.md) - Local LLM runtime for privacy and offline usage
- [context7-mcp](CONTEXT7-MCP.md) - Library documentation MCP server
- [linear-mcp](LINEAR-MCP.md) - Linear project management integration
- [jira-mcp](JIRA-MCP.md) - Atlassian Jira/Confluence integration
````
