---
name: pal-mcp-server
description: AI orchestration and multi-model collaboration MCP server. Use when you need multi-model AI collaboration, code reviews, debugging assistance, planning, consensus building, or advanced reasoning across multiple AI providers (Gemini, OpenAI, Grok, Azure, Ollama, etc.).
allowed-tools: MCP(pal:*)
---

# PAL MCP Server - Provider Abstraction Layer

## Overview

PAL MCP (Provider Abstraction Layer) is an AI orchestration server that enables your AI assistant to collaborate with multiple AI models within a single workflow. It provides 18 specialized MCP tools for enhanced code analysis, problem-solving, and collaborative development.

## Key Features

- **Multi-Model Orchestration** - Connect to Gemini, OpenAI, Azure, Grok, Ollama, and 50+ models
- **Conversation Continuity** - Context flows seamlessly across tools and models
- **CLI-to-CLI Bridge** - Spawn subagents in isolated contexts (clink tool)
- **Professional Workflows** - Systematic code reviews, debugging, planning, consensus building
- **Vision Capabilities** - Analyze screenshots and diagrams
- **Local Model Support** - Run Llama, Mistral locally for privacy

## Available Tools

### Core Collaboration Tools

**`chat`** - Multi-model conversations with code generation

````text
Chat with gemini pro about implementing OAuth2
Continue the chat with o3 to refine the approach
```text

**`thinkdeep`** - Extended reasoning with configurable thinking modes

```text
Use thinkdeep with gemini pro to analyze the architecture
```text

**`consensus`** - Multi-model debate and decision-making

```text
Use consensus with gpt-5 and gemini pro to decide: dark mode or offline support next
```text

**`planner`** - Strategic planning and project breakdown

```text
Use planner to create a detailed implementation plan for the new feature
```text

### Code Quality Tools

**`codereview`** - Professional code reviews with severity levels

```text
Perform a codereview using gemini pro and o3 for the auth module
```text

**`precommit`** - Pre-commit validation and regression prevention

```text
Run precommit check before committing these changes
```text

**`debug`** - Systematic investigation and root cause analysis

```text
Use debug to investigate why the API returns 500 errors
```text

**`refactor`** - Intelligent code refactoring (disabled by default)

```text
Use refactor to improve the payment processing module
```text

**`testgen`** - Test generation with edge cases (disabled by default)

```text
Use testgen to create comprehensive tests for the user authentication
```text

**`secaudit`** - Security audits with OWASP analysis (disabled by default)

```text
Run secaudit on the entire codebase
```text

### Analysis Tools

**`analyze`** - Codebase architecture and pattern analysis (disabled by default)

```text
Use analyze to understand the data flow in this application
```text

**`tracer`** - Static call-flow mapping (disabled by default)

```text
Use tracer to map the execution path from login to dashboard
```text

**`docgen`** - Documentation generation (disabled by default)

```text
Use docgen to create API documentation for the REST endpoints
```text

### Advanced Tools

**`clink`** - CLI-to-CLI bridge (spawn subagents, connect external CLIs)

```text
# Spawn Codex subagent for isolated code review
clink with codex codereviewer to audit auth module for security issues

# Connect Gemini CLI for implementation after consensus
Continue with clink gemini - implement the recommended feature
```text

**`apilookup`** - Force current-year API documentation lookups

```text
Use apilookup to check the latest Kubernetes API changes
```text

**`challenge`** - Critical analysis to prevent reflexive agreement

```text
Use challenge to critically evaluate this architectural decision
```text

### Utility Tools

**`version`** - Server version information

```text
Check the pal server version
```text

**`listmodels`** - Available model listing

```text
List all available models in pal
```text

## Configuration

### API Keys

Configure API keys in `~/extensions/pal-mcp-server/.env`:

```bash
# At least one API key required
GEMINI_API_KEY=your_gemini_key
OPENAI_API_KEY=your_openai_key
XAI_API_KEY=your_grok_key
OPENROUTER_API_KEY=your_openrouter_key

# Optional: Local models (no API key needed)
CUSTOM_API_URL=http://localhost:11434  # Ollama
```text

### Model Selection

**Automatic:** Models are selected automatically based on the task

**Explicit:** Specify models in your prompt

```text
Use chat with gemini pro to discuss the architecture
Perform codereview using o3 and gemini flash
```text

## Example Workflows

### Multi-Model Code Review

```text
1. Perform a codereview using gemini pro and o3
2. Use planner to generate a detailed implementation plan
3. Implement the fixes
4. Run precommit check before committing
```text

### Debugging with Context Continuity

```text
1. Use debug with gemini pro to investigate the memory leak
2. Continue with o3 to verify the hypothesis
3. Use thinkdeep to explore alternative solutions
```text

### Consensus-Driven Development

```text
1. Use consensus with gpt-5, gemini pro, and o3 to decide the best approach
2. Continue with clink gemini to implement the agreed solution
3. Run codereview to validate the implementation
```text

### CLI Subagent Pattern

```text
# Main context: Planning
Use planner to outline the microservice architecture

# Spawn subagent for isolated work
clink with codex codereviewer to audit the payment service in isolation

# Continue in main context with subagent insights
Implement the recommended security improvements
```text

## Supported Providers

### Cloud Providers

- **Gemini** (Google AI) - Pro, Flash, Exp models
- **OpenAI** - GPT-5, o3, o3-mini
- **Anthropic** - Claude models via OpenRouter
- **X.AI** - Grok models
- **Azure OpenAI** - Azure-hosted models
- **OpenRouter** - Unified access to 50+ models
- **DIAL** - Enterprise AI platform

### Local Providers

- **Ollama** - Llama, Mistral, local models
- **vLLM** - Fast local inference
- **LM Studio** - Local model hosting
- **Custom endpoints** - Any OpenAI-compatible API

## Tips

1. **Context Revival**: When context resets, continue conversations with another model to "remind" your primary assistant
2. **Model Strengths**: Use Gemini Pro for extended thinking, Flash for speed, O3 for reasoning
3. **Disable Unused Tools**: Edit `.env` to disable tools you don't need (reduces token usage)
4. **Local Models**: Use Ollama for privacy and zero API costs
5. **Vision Tasks**: Use vision-enabled models for analyzing screenshots and diagrams

## Troubleshooting

### API Keys Not Working

- Check `.env` file in `~/extensions/pal-mcp-server/.env`
- Ensure API keys are valid and have proper permissions
- Restart Claude Code after modifying `.env`

### MCP Server Not Found

- Verify installation: `extension-manager status pal-mcp-server`
- Check Claude Code settings: `~/.claude/settings.json` should contain "pal" server entry
- Restart Claude Code

### Tools Not Showing Up

- Some tools are disabled by default (analyze, refactor, testgen, secaudit, docgen, tracer)
- Enable in `.env`: Set `PAL_ENABLE_ANALYZE=true`, etc.
- Restart Claude Code after enabling

### Python Environment Issues

- Verify Python 3.10+: `python3 --version`
- Check virtual environment: `~/extensions/pal-mcp-server/.pal_venv/bin/python --version`
- Reinstall: `extension-manager upgrade pal-mcp-server`

## Documentation

- Repository: https://github.com/BeehiveInnovations/pal-mcp-server
- Tool Documentation: https://github.com/BeehiveInnovations/pal-mcp-server/tree/main/docs/tools
- Configuration Guide: https://github.com/BeehiveInnovations/pal-mcp-server/blob/main/docs/configuration.md
````
