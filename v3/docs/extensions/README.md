# V3 Extensions Documentation

> Last Updated: 2026-01-29 | Total Extensions: 48

This directory contains documentation for all V3 extensions available in Sindri.

## Quick Reference

| Category       | Count | Description                                  |
| -------------- | ----- | -------------------------------------------- |
| Languages      | 10    | Programming language runtimes and toolchains |
| Claude         | 7     | Claude Code integration and tools            |
| AI Agents      | 5     | AI agent frameworks and automation           |
| DevOps         | 6     | Infrastructure, cloud, and deployment tools  |
| MCP            | 5     | Model Context Protocol servers               |
| AI Development | 3     | AI development tools and LLMs                |
| Productivity   | 6     | Development productivity and package tools   |
| Desktop        | 2     | Desktop environments and remote access       |
| Research       | 1     | Research and analysis tools                  |

---

## Language Runtimes

Core programming language support.

| Extension                             | Version | Description                         |
| ------------------------------------- | ------- | ----------------------------------- |
| [NODEJS](NODEJS.md)                   | 1.1.0   | Node.js LTS via mise with pnpm      |
| [PYTHON](PYTHON.md)                   | 1.1.1   | Python 3.13 with uv package manager |
| [GOLANG](GOLANG.md)                   | 1.0.1   | Go 1.25 via mise                    |
| [RUST](RUST.md)                       | 1.0.2   | Rust stable via rustup              |
| [DOTNET](DOTNET.md)                   | 2.1.0   | .NET SDK 10.0 and 8.0               |
| [JVM](JVM.md)                         | 2.1.0   | Java, Kotlin, Scala, Clojure        |
| [RUBY](RUBY.md)                       | 2.0.0   | Ruby 3.4.7 with Rails and Bundler   |
| [PHP](PHP.md)                         | 2.1.0   | PHP 8.4 with Composer and Symfony   |
| [HASKELL](HASKELL.md)                 | 1.0.1   | Haskell with GHC, Cabal, Stack, HLS |
| [NODEJS-DEVTOOLS](NODEJS-DEVTOOLS.md) | 2.2.0   | TypeScript, ESLint, Prettier        |

---

## Claude Integration

Extensions for Claude Code workflows and enhancements.

| Extension                                   | Version | Description                                   |
| ------------------------------------------- | ------- | --------------------------------------------- |
| [CLAUDE-FLOW-V3](CLAUDE-FLOW-V3.md)         | 3.0.0   | Next-gen multi-agent orchestration (v3 alpha) |
| [CLAUDE-FLOW-V2](CLAUDE-FLOW-V2.md)         | 2.7.47  | Stable multi-agent orchestration (v2)         |
| [CLAUDE-CODEPRO](CLAUDE-CODEPRO.md)         | 4.5.29  | TDD-enforced development environment          |
| [CLAUDEUP](CLAUDEUP.md)                     | 1.0.0   | TUI for plugins, MCPs, and settings           |
| [CLAUDISH](CLAUDISH.md)                     | 1.0.0   | OpenRouter proxy for Claude Code              |
| [CLAUDE-CODE-MUX](CLAUDE-CODE-MUX.md)       | 1.0.0   | Multi-provider AI routing proxy               |
| [CLAUDE-MARKETPLACE](CLAUDE-MARKETPLACE.md) | 2.0.0   | Plugin marketplace integration                |

---

## AI Agents

Frameworks for AI agent automation and orchestration.

| Extension                         | Version | Description                        |
| --------------------------------- | ------- | ---------------------------------- |
| [AGENTIC-FLOW](AGENTIC-FLOW.md)   | 1.0.0   | Multi-model AI agent framework     |
| [AGENTIC-QE](AGENTIC-QE.md)       | 1.1.0   | AI-powered quality engineering     |
| [AGENT-BROWSER](AGENT-BROWSER.md) | 0.6.0   | Headless browser automation for AI |
| [AGENT-MANAGER](AGENT-MANAGER.md) | 2.0.0   | AI agent management CLI            |
| [RALPH](RALPH.md)                 | 1.0.0   | Autonomous development system      |

---

## DevOps & Infrastructure

Tools for deployment, infrastructure, and cloud operations.

| Extension                       | Version | Description                          |
| ------------------------------- | ------- | ------------------------------------ |
| [DOCKER](DOCKER.md)             | 1.1.0   | Docker Engine and Compose            |
| [GITHUB-CLI](GITHUB-CLI.md)     | 2.0.0   | GitHub CLI and workflow tools        |
| [CLOUD-TOOLS](CLOUD-TOOLS.md)   | 2.0.0   | AWS, Azure, GCP, Fly.io, and more    |
| [INFRA-TOOLS](INFRA-TOOLS.md)   | 2.0.0   | Terraform, Kubernetes, Ansible       |
| [MONITORING](MONITORING.md)     | 2.0.0   | Claude monitoring and usage tracking |
| [SUPABASE-CLI](SUPABASE-CLI.md) | 2.0.0   | Supabase local development           |

---

## MCP Servers

Model Context Protocol servers for Claude Code integration.

| Extension                           | Version | Description                             |
| ----------------------------------- | ------- | --------------------------------------- |
| [CONTEXT7-MCP](CONTEXT7-MCP.md)     | 1.0.0   | Version-specific library documentation  |
| [JIRA-MCP](JIRA-MCP.md)             | 2.0.0   | Atlassian Jira/Confluence integration   |
| [LINEAR-MCP](LINEAR-MCP.md)         | 2.1.0   | Linear issue tracking integration       |
| [PAL-MCP-SERVER](PAL-MCP-SERVER.md) | 9.8.2   | Multi-model AI orchestration (18 tools) |
| [SPEC-KIT](SPEC-KIT.md)             | 1.0.0   | GitHub spec-kit for AI workflows        |

---

## AI Development

Tools for developing with and running AI models.

| Extension                   | Version | Description                             |
| --------------------------- | ------- | --------------------------------------- |
| [OLLAMA](OLLAMA.md)         | 1.0.0   | Local LLM server (Llama, Mistral, etc.) |
| [AI-TOOLKIT](AI-TOOLKIT.md) | 2.1.0   | Fabric, Codex, Gemini, Grok CLIs        |
| [GOOSE](GOOSE.md)           | 1.0.0   | Block's AI engineering automation       |

---

## Productivity & Package Managers

Development productivity and workflow tools.

| Extension                           | Version | Description                         |
| ----------------------------------- | ------- | ----------------------------------- |
| [MISE-CONFIG](MISE-CONFIG.md)       | 2.0.0   | Global mise tool version manager    |
| [SDKMAN](SDKMAN.md)                 | 1.0.0   | SDKMAN for JVM tools and SDKs       |
| [TMUX-WORKSPACE](TMUX-WORKSPACE.md) | 2.0.0   | Terminal multiplexer setup          |
| [MDFLOW](MDFLOW.md)                 | 1.0.0   | Markdown to AI agent CLI            |
| [OPENSKILLS](OPENSKILLS.md)         | 2.0.0   | Claude Code skill manager           |
| [PLAYWRIGHT](PLAYWRIGHT.md)         | 2.0.0   | Browser automation framework        |

---

## Desktop Environments

GUI and remote desktop access.

| Extension                     | Version | Description                      |
| ----------------------------- | ------- | -------------------------------- |
| [XFCE-UBUNTU](XFCE-UBUNTU.md) | 2.0.0   | XFCE desktop with xRDP           |
| [GUACAMOLE](GUACAMOLE.md)     | 2.0.0   | Web-based remote desktop gateway |

---

## Research

Research and analysis tools.

| Extension                             | Version | Description                     |
| ------------------------------------- | ------- | ------------------------------- |
| [RUVNET-RESEARCH](RUVNET-RESEARCH.md) | 1.0.0   | Goalie and Research-Swarm tools |

---

## Installation

Install any extension using the extension-manager:

```bash
# Install a single extension
extension-manager install <extension-name>

# Install multiple extensions
extension-manager install nodejs python docker

# List available extensions
extension-manager list

# Get extension info
extension-manager info <extension-name>
```

## Dependency Graph

Some extensions have dependencies on others:

```
mise-config
  |-- nodejs
  |     |-- nodejs-devtools
  |     |-- playwright
  |     |     |-- agent-browser
  |     |-- claude-flow-v2
  |     |-- claude-flow-v3
  |     |-- agentic-flow
  |     |-- agentic-qe
  |     |-- ai-toolkit (also: python, golang, github-cli)
  |     |-- ralph
  |     |-- mdflow
  |     |-- claudeup
  |     |-- claudish
  |     |-- openskills
  |-- python
  |     |-- spec-kit
  |     |-- pal-mcp-server
  |     |-- monitoring
  |-- haskell
  |-- jvm (also: sdkman)

sdkman
  |-- jvm (also: mise-config)

docker
  |-- supabase-cli
```

## Extension Categories

### By Install Method

| Method | Extensions                                                                                                                                       |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| mise   | nodejs, python, golang, haskell, nodejs-devtools, infra-tools, claude-flow-_, agentic-_, mdflow, openskills, claudeup, claudish, ruvnet-research |
| script | rust, dotnet, jvm, ruby, php, docker, github-cli, ollama, ai-toolkit, goose, ralph, agent-manager, cloud-tools, monitoring, guacamole            |
| apt    | tmux-workspace, xfce-ubuntu                                                                                                                      |
| hybrid | docker, infra-tools, xfce-ubuntu                                                                                                                 |

### By Authentication Required

| Requirement   | Extensions                                               |
| ------------- | -------------------------------------------------------- |
| Anthropic API | claude-flow-\*, agentic-qe, ralph                        |
| Optional API  | ai-toolkit, ollama, claudish, context7-mcp, supabase-cli |
| OAuth Flow    | jira-mcp, linear-mcp                                     |
| GitHub Token  | github-cli                                               |

---

## Contributing

To add a new extension:

1. Create `extension.yaml` in `/v3/extensions/<name>/`
2. Follow the YAML schema
3. Add documentation in `/v3/docs/extensions/<NAME>.md`
4. Update this README

See [Extension Authoring Guide](../EXTENSION_AUTHORING.md) for details.
