# ADR 046: LLM Proxy Environment Variable Injection for Draupnir

- **Status:** Accepted
- **Date:** 2026-03-03
- **Phase:** Ecosystem
- **Relates to:** [Draupnir ADR-004](https://github.com/pacphi/draupnir/docs/architecture/adr/004-llm-traffic-interception.md), [Mimir ADR-0011](https://github.com/pacphi/mimir/docs/adr/0011-llm-token-cost-tracking.md)

## Context

Mimir needs visibility into per-instance LLM API costs across the fleet. Draupnir (the per-instance Go agent) implements a local HTTP reverse proxy on port 9090 that intercepts LLM API traffic and reports token usage to Mimir. For this proxy to work, LLM SDK calls must be redirected through it.

All major LLM provider SDKs support a `*_BASE_URL` environment variable that overrides the default API endpoint:

| Provider      | Environment Variable    | SDK Support                     |
| ------------- | ----------------------- | ------------------------------- |
| Anthropic     | `ANTHROPIC_BASE_URL`    | `@anthropic-ai/sdk`, Claude CLI |
| OpenAI        | `OPENAI_BASE_URL`       | `openai` Python, `openai-node`  |
| Google Gemini | `GOOGLE_API_BASE_URL`   | Google AI SDK                   |
| Groq          | `GROQ_BASE_URL`         | Groq SDK                        |
| Mistral       | `MISTRAL_BASE_URL`      | Mistral SDK                     |
| Cohere        | `CO_API_URL`            | Cohere SDK                      |
| Together      | `TOGETHER_API_BASE`     | Together SDK                    |
| Azure OpenAI  | `AZURE_OPENAI_ENDPOINT` | Azure OpenAI SDK                |

The challenge is injecting these env vars into the container environment so that all extensions and SSH sessions use them, without requiring any extension modifications.

## Decision

### 1. Draupnir extension.yaml `configure.environment` section

The Draupnir extension definition in Sindri now includes a `configure.environment` block that sets 8 `*_BASE_URL` variables pointing to `http://localhost:9090/v1/<provider>`:

```yaml
configure:
  environment:
    - name: ANTHROPIC_BASE_URL
      value: "http://localhost:9090/v1/anthropic"
      condition: "{{ env.SINDRI_LLM_ADAPTER != 'none' }}"
    # ... (8 providers total)
```

Each variable is conditionally set only when `SINDRI_LLM_ADAPTER` is not `"none"`. This ensures the proxy is only activated when Draupnir's LLM adapter is enabled.

### 2. Entrypoint env var propagation

The container's `entrypoint.sh` script propagates environment variables to SSH sessions via `/etc/profile.d/sindri-environment.sh`. The propagation pattern matcher now includes `*_BASE_URL`, `*_API_BASE`, `*_API_URL`, and `*_ENDPOINT` suffixes, ensuring the LLM proxy env vars are available in:

- SSH sessions (sourced from `/etc/profile.d/`)
- Extension installation (via `--preserve-env` in sudo)
- Interactive terminals via Mimir's WebSocket PTY

## Consequences

### Positive

- **Zero extension modification** — all SDKs respect their `*_BASE_URL` env var
- **Conditional activation** — disabled when `SINDRI_LLM_ADAPTER=none`
- **Transparent** — extensions call `localhost:9090` which forwards to the real API
- **No TLS MITM** — local HTTP → Draupnir → HTTPS → provider API (no certificate injection)

### Negative

- **~1ms added latency** per LLM call (local proxy hop) — negligible vs API latency
- Extensions that hardcode API URLs bypass the proxy (covered by Draupnir's Tier 2 eBPF on Linux)
- `SINDRI_LLM_ADAPTER` must be set before Draupnir starts — no hot-reload

### Files changed

- `v3/extensions/draupnir/extension.yaml` — added `configure.environment` section
- `v3/docker/scripts/entrypoint.sh` — added `*_BASE_URL` pattern to SSH env propagation
