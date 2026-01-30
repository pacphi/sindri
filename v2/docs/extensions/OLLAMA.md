# Ollama

Run large language models locally.

## Overview

| Property         | Value                 |
| ---------------- | --------------------- |
| **Category**     | ai                    |
| **Version**      | 1.0.0                 |
| **Installation** | script                |
| **Disk Space**   | 1000 MB (plus models) |
| **Dependencies** | none                  |

## Description

Ollama provides a local runtime for running large language models (LLMs) including Llama, Mistral, CodeLlama, Phi, and many more. It offers a simple CLI and REST API for model management and inference.

This extension was factored out of `ai-toolkit` to provide better isolation and a generous installation timeout (30 minutes) for the ~800MB binary download.

## Installed Tools

| Tool     | Type   | Description                         |
| -------- | ------ | ----------------------------------- |
| `ollama` | server | Local LLM runtime and model manager |

## Configuration

### Environment Variables

| Variable      | Value           | Scope  | Description                |
| ------------- | --------------- | ------ | -------------------------- |
| `OLLAMA_HOST` | `0.0.0.0:11434` | bashrc | Ollama server bind address |

## Network Requirements

- `ollama.com` - Binary downloads and model registry
- `github.com` - GitHub releases

## Installation

```bash
extension-manager install ollama
```

**Note:** Installation may take 10-20 minutes depending on network speed due to the ~800MB binary download.

## Usage

### Start the Server

```bash
# Start in foreground
ollama serve

# Start in background
nohup ollama serve > ~/ollama.log 2>&1 &
```

### Manage Models

```bash
# Pull a model
ollama pull llama3.2
ollama pull codellama
ollama pull mistral

# List installed models
ollama list

# Remove a model
ollama rm llama3.2
```

### Run Inference

```bash
# Interactive chat
ollama run llama3.2

# Single prompt
echo "Explain recursion in one sentence" | ollama run llama3.2

# Use with API
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.2",
  "prompt": "Hello, world!"
}'
```

## Validation

```bash
ollama --version
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade ollama
# Or manually:
curl -fsSL https://ollama.com/install.sh | sh
```

## Removal

```bash
extension-manager remove ollama
```

Removes the Ollama binary and `~/.ollama` data directory (including downloaded models).

## Disk Space Considerations

- **Binary:** ~800 MB
- **Models:** 2-70 GB each depending on model size
  - `llama3.2:3b` - ~2 GB
  - `llama3.2:70b` - ~40 GB
  - `codellama:7b` - ~4 GB

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - Additional AI CLI tools (Fabric, Codex, Gemini)
- [openskills](OPENSKILLS.md) - Claude Code skills
