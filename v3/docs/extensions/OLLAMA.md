# Ollama Extension

> Version: 1.0.0 | Category: ai-dev | Last Updated: 2026-01-26

## Overview

Ollama - Run large language models locally. Provides the ollama CLI for managing and running LLMs like Llama, Mistral, CodeLlama, and more.

## What It Provides

| Tool   | Type   | License | Description              |
| ------ | ------ | ------- | ------------------------ |
| ollama | server | MIT     | Local LLM server and CLI |

## Requirements

- **Disk Space**: 1000 MB (base installation)
- **Memory**: 8192 MB (8 GB minimum for models)
- **Install Time**: ~600 seconds (10 minutes)
- **Dependencies**: None

### Network Domains

- ollama.com
- github.com

## Installation

```bash
extension-manager install ollama
```

## Configuration

### Environment Variables

| Variable        | Value                | Description                        |
| --------------- | -------------------- | ---------------------------------- |
| `OLLAMA_HOST`   | 0.0.0.0:11434        | Server bind address and port       |
| `OLLAMA_MODELS` | $HOME/.ollama/models | Model storage directory            |
| `OLLAMA_TMPDIR` | $HOME/.ollama/tmp    | Temporary directory for operations |

### Install Method

Uses a custom installation script with 1800 second timeout.

### Upgrade Strategy

Reinstall - runs the installation script again.

## Usage Examples

### Starting the Server

```bash
# Start ollama server (runs in background)
ollama serve

# The server listens on http://0.0.0.0:11434
```

### Managing Models

```bash
# Pull a model
ollama pull llama3.2
ollama pull codellama
ollama pull mistral

# List installed models
ollama list

# Show model info
ollama show llama3.2

# Remove a model
ollama rm llama3.2
```

### Running Models

```bash
# Interactive chat
ollama run llama3.2

# Run with a prompt
ollama run llama3.2 "Explain quantum computing"

# Run code model
ollama run codellama "Write a Python function to sort a list"
```

### API Usage

```bash
# Generate completion
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.2",
  "prompt": "Why is the sky blue?"
}'

# Chat API
curl http://localhost:11434/api/chat -d '{
  "model": "llama3.2",
  "messages": [
    { "role": "user", "content": "Hello!" }
  ]
}'
```

### Creating Custom Models

```bash
# Create a Modelfile
cat > Modelfile <<EOF
FROM llama3.2
PARAMETER temperature 0.7
SYSTEM You are a helpful coding assistant.
EOF

# Build the model
ollama create my-assistant -f Modelfile
```

### Popular Models

| Model     | Size   | Use Case               |
| --------- | ------ | ---------------------- |
| llama3.2  | 3B/7B  | General purpose        |
| codellama | 7B/13B | Code generation        |
| mistral   | 7B     | Instruction following  |
| phi3      | 3.8B   | Lightweight, efficient |
| gemma2    | 9B     | Google's open model    |

## Validation

The extension validates the following commands:

- `ollama --version` - Must match pattern `(ollama version|\d+\.\d+\.\d+)`

## Removal

```bash
extension-manager remove ollama
```

This removes:

- ~/.ollama directory (including all models)
- Runs the uninstall script

**Requires confirmation.**

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - Complementary AI tools (Fabric, Codex, etc.)
