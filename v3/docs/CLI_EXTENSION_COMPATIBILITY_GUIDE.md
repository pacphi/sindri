# CLI Extension Compatibility Guide

> **Authoritative reference** for what software versions ship with each Sindri CLI release series.
>
> Other documentation should link here rather than embedding version numbers inline.
> Only extensions with **pinned / specific versions** are listed. Extensions that install
> `dynamic` or `latest` versions (e.g., Composer, Symfony CLI, .NET SDK) are excluded.

---

## What Changed in 3.1.x

The 3.1.0 release upgraded 11 extensions covering ~25 software components.

| Extension       | Software         | 3.0.x           | 3.1.x    |
| --------------- | ---------------- | --------------- | -------- |
| python          | Python           | 3.13            | 3.14     |
| python          | uv / uvx         | 0.9             | 0.10     |
| php             | PHP              | 8.4             | 8.5      |
| nodejs-devtools | ESLint           | 9               | 10       |
| nodejs-devtools | Prettier         | 3.6             | 3.8      |
| mdflow          | mdflow           | 2.33            | 2.35.5   |
| agent-browser   | agent-browser    | 0.9.3           | 0.15.0   |
| agentic-qe      | agentic-qe       | 3.7.1           | 3.7.7    |
| ai-toolkit      | Gemini CLI       | 0.27.1          | 0.30.0   |
| supabase-cli    | Supabase CLI     | 2.76.4          | 2.76.15  |
| haskell         | ghcup            | 0.1.30          | 0.1.50.2 |
| haskell         | Cabal            | 3.14.1.1        | 3.14.1   |
| haskell         | HLS              | 2.13.0.0        | 2.13.0   |
| cloud-tools     | AWS CLI          | 2.33.21         | 2.33.30  |
| cloud-tools     | Google Cloud SDK | 556.0.0         | 558.0.0  |
| cloud-tools     | Aliyun CLI       | 3.2.9           | 3.2.10   |
| cloud-tools     | doctl            | 1.150.0         | 1.151.0  |
| cloud-tools     | flyctl           | 0.4.11          | 0.4.15   |
| infra-tools     | Packer           | _(not tracked)_ | 1.15     |
| infra-tools     | Pulumi           | 3.220.0         | 3.223.0  |
| infra-tools     | Crossplane       | 2.1.4           | 2.2.0    |
| infra-tools     | kapp             | 0.65.0          | 0.65.1   |
| infra-tools     | ytt              | 0.53.0          | 0.53.2   |
| infra-tools     | vendir           | 0.45.1          | 0.45.2   |
| infra-tools     | imgpkg           | 0.47.1          | 0.47.2   |

---

## CLI 3.1.x (2026-02-25)

> Extension Schema: 1.1

### Languages

| Extension       | Software                         | Version  | Source |
| --------------- | -------------------------------- | -------- | ------ |
| python          | Python                           | 3.14     | mise   |
| python          | pip                              | 26.0.1   | mise   |
| python          | uv                               | 0.10     | mise   |
| python          | uvx                              | 0.10     | mise   |
| php             | PHP                              | 8.5      | script |
| nodejs          | Node.js                          | LTS      | mise   |
| nodejs          | npm                              | 11.x     | mise   |
| nodejs          | npx                              | 11.x     | mise   |
| nodejs          | pnpm                             | 10.x     | mise   |
| nodejs-devtools | TypeScript                       | 5.9      | npm    |
| nodejs-devtools | ts-node                          | 10.9     | npm    |
| nodejs-devtools | ESLint                           | 10       | npm    |
| nodejs-devtools | @typescript-eslint/parser        | 8        | npm    |
| nodejs-devtools | @typescript-eslint/eslint-plugin | 8        | npm    |
| nodejs-devtools | Prettier                         | 3.8      | npm    |
| nodejs-devtools | nodemon                          | 3.1      | npm    |
| golang          | Go                               | 1.26     | mise   |
| haskell         | ghcup                            | 0.1.50.2 | script |
| haskell         | GHC                              | 9.12.2   | script |
| haskell         | Cabal                            | 3.14.1   | script |
| haskell         | Stack                            | 3.3.1    | script |
| haskell         | HLS                              | 2.13.0   | script |
| jvm             | Java (OpenJDK)                   | 25       | script |
| jvm             | Maven                            | 3.9.12   | script |
| jvm             | Gradle                           | 9.3.1    | script |
| jvm             | Kotlin                           | 2.3.10   | script |
| jvm             | Scala                            | 3.8.1    | script |
| jvm             | Clojure                          | 1.12     | mise   |
| jvm             | Leiningen                        | 2.12     | mise   |
| ruby            | Ruby                             | 4.0      | mise   |
| ruby            | RubyGems                         | 4.0.3    | mise   |
| ruby            | Bundler                          | 4.0.3    | mise   |
| swift           | Swift                            | 6.2.3    | mise   |
| rust            | rustc                            | stable   | script |
| rust            | Cargo                            | stable   | script |

### AI Agents

| Extension     | Software      | Version | Source |
| ------------- | ------------- | ------- | ------ |
| agentic-qe    | agentic-qe    | 3.7.7   | npm    |
| agent-browser | agent-browser | 0.15.0  | npm    |

### AI Dev

| Extension  | Software      | Version   | Source |
| ---------- | ------------- | --------- | ------ |
| ai-toolkit | Codex         | 0.101.0   | npm    |
| ai-toolkit | Gemini CLI    | 0.30.0    | npm    |
| ai-toolkit | Grok CLI      | 0.0.34    | npm    |
| clarity    | clarity       | 1.0.0     | script |
| gitnexus   | gitnexus      | 1.2.7     | npm    |
| kilo       | Kilo Code CLI | 1.0.21    | npm    |
| openclaw   | openclaw      | 2026.2.24 | npm    |
| opencode   | opencode      | 1.2.14    | npm    |

### Claude Ecosystem

| Extension      | Software       | Version     | Source |
| -------------- | -------------- | ----------- | ------ |
| claude-codepro | claude-codepro | 4.5.29      | binary |
| claude-flow-v2 | claude-flow    | 2.7.47      | npm    |
| claude-flow-v3 | claude-flow    | 3.1.0-alpha | npm    |
| claudeup       | claudeup       | 3.3.1       | npm    |
| claudish       | claudish       | 4.6.6       | npm    |
| compahook      | compahook      | 1.1.2       | npm    |

### Cloud & Infrastructure

| Extension    | Software         | Version | Source |
| ------------ | ---------------- | ------- | ------ |
| cloud-tools  | AWS CLI          | 2.33.30 | script |
| cloud-tools  | Azure CLI        | 2.83.0  | script |
| cloud-tools  | Google Cloud SDK | 558.0.0 | script |
| cloud-tools  | flyctl           | 0.4.15  | script |
| cloud-tools  | Aliyun CLI       | 3.2.10  | script |
| cloud-tools  | doctl            | 1.151.0 | script |
| cloud-tools  | IBM Cloud CLI    | 2.41.1  | script |
| supabase-cli | Supabase CLI     | 2.76.15 | binary |
| infra-tools  | Terraform        | 1.14    | mise   |
| infra-tools  | Packer           | 1.15    | mise   |
| infra-tools  | kubectl          | 1.35    | mise   |
| infra-tools  | Helm             | 4.1     | mise   |
| infra-tools  | k9s              | 0.50    | mise   |
| infra-tools  | kustomize        | 5.8     | mise   |
| infra-tools  | yq               | 4.52    | mise   |
| infra-tools  | Ansible          | 13.3.0  | apt    |
| infra-tools  | Pulumi           | 3.223.0 | script |
| infra-tools  | Crossplane       | 2.2.0   | script |
| infra-tools  | kubectx          | 0.9.5   | script |
| infra-tools  | kubens           | 0.9.5   | script |
| infra-tools  | kapp             | 0.65.1  | script |
| infra-tools  | ytt              | 0.53.2  | script |
| infra-tools  | kbld             | 0.47.1  | script |
| infra-tools  | vendir           | 0.45.2  | script |
| infra-tools  | imgpkg           | 0.47.2  | script |

### Documentation & Productivity

| Extension  | Software   | Version | Source |
| ---------- | ---------- | ------- | ------ |
| mdflow     | Bun        | 1       | mise   |
| mdflow     | mdflow     | 2.35.5  | npm    |
| openskills | openskills | 1.5.0   | npm    |

### MCP Servers

| Extension      | Software       | Version | Source |
| -------------- | -------------- | ------- | ------ |
| pal-mcp-server | pal-mcp-server | 9.8.2   | binary |

### Research

| Extension       | Software       | Version | Source |
| --------------- | -------------- | ------- | ------ |
| ruvnet-research | Goalie         | 1.3     | npm    |
| ruvnet-research | research-swarm | 1.2     | npm    |

### Testing

| Extension  | Software   | Version | Source |
| ---------- | ---------- | ------- | ------ |
| playwright | Playwright | 1.58.2  | npm    |

---

## CLI 3.0.x (2026-02-24)

> Extension Schema: 1.1

### Languages

| Extension       | Software                         | Version  | Source |
| --------------- | -------------------------------- | -------- | ------ |
| python          | Python                           | 3.13     | mise   |
| python          | pip                              | 26.0.1   | mise   |
| python          | uv                               | 0.9      | mise   |
| python          | uvx                              | 0.9      | mise   |
| php             | PHP                              | 8.4      | script |
| nodejs          | Node.js                          | LTS      | mise   |
| nodejs          | npm                              | 11.x     | mise   |
| nodejs          | npx                              | 11.x     | mise   |
| nodejs          | pnpm                             | 10.x     | mise   |
| nodejs-devtools | TypeScript                       | 5.9      | npm    |
| nodejs-devtools | ts-node                          | 10.9     | npm    |
| nodejs-devtools | ESLint                           | 9        | npm    |
| nodejs-devtools | @typescript-eslint/parser        | 8        | npm    |
| nodejs-devtools | @typescript-eslint/eslint-plugin | 8        | npm    |
| nodejs-devtools | Prettier                         | 3.6      | npm    |
| nodejs-devtools | nodemon                          | 3.1      | npm    |
| golang          | Go                               | 1.26     | mise   |
| haskell         | ghcup                            | 0.1.30   | script |
| haskell         | GHC                              | 9.12.2   | script |
| haskell         | Cabal                            | 3.14.1.1 | script |
| haskell         | Stack                            | 3.3.1    | script |
| haskell         | HLS                              | 2.13.0.0 | script |
| jvm             | Java (OpenJDK)                   | 25       | script |
| jvm             | Maven                            | 3.9.12   | script |
| jvm             | Gradle                           | 9.3.1    | script |
| jvm             | Kotlin                           | 2.3.10   | script |
| jvm             | Scala                            | 3.8.1    | script |
| jvm             | Clojure                          | 1.12     | mise   |
| jvm             | Leiningen                        | 2.12     | mise   |
| ruby            | Ruby                             | 4.0      | mise   |
| ruby            | RubyGems                         | 4.0.3    | mise   |
| ruby            | Bundler                          | 4.0.3    | mise   |
| swift           | Swift                            | 6.2.3    | mise   |
| rust            | rustc                            | stable   | script |
| rust            | Cargo                            | stable   | script |

### AI Agents

| Extension     | Software      | Version | Source |
| ------------- | ------------- | ------- | ------ |
| agentic-qe    | agentic-qe    | 3.7.1   | npm    |
| agent-browser | agent-browser | 0.9.3   | npm    |

### AI Dev

| Extension  | Software      | Version   | Source |
| ---------- | ------------- | --------- | ------ |
| ai-toolkit | Codex         | 0.101.0   | npm    |
| ai-toolkit | Gemini CLI    | 0.27.1    | npm    |
| ai-toolkit | Grok CLI      | 0.0.34    | npm    |
| clarity    | clarity       | 1.0.0     | script |
| gitnexus   | gitnexus      | 1.2.7     | npm    |
| kilo       | Kilo Code CLI | 1.0.21    | npm    |
| openclaw   | openclaw      | 2026.2.24 | npm    |
| opencode   | opencode      | 1.2.14    | npm    |

### Claude Ecosystem

| Extension      | Software       | Version     | Source |
| -------------- | -------------- | ----------- | ------ |
| claude-codepro | claude-codepro | 4.5.29      | binary |
| claude-flow-v2 | claude-flow    | 2.7.47      | npm    |
| claude-flow-v3 | claude-flow    | 3.1.0-alpha | npm    |
| claudeup       | claudeup       | 3.3.1       | npm    |
| claudish       | claudish       | 4.6.6       | npm    |
| compahook      | compahook      | 1.1.2       | npm    |

### Cloud & Infrastructure

| Extension    | Software         | Version | Source |
| ------------ | ---------------- | ------- | ------ |
| cloud-tools  | AWS CLI          | 2.33.21 | script |
| cloud-tools  | Azure CLI        | 2.83.0  | script |
| cloud-tools  | Google Cloud SDK | 556.0.0 | script |
| cloud-tools  | flyctl           | 0.4.11  | script |
| cloud-tools  | Aliyun CLI       | 3.2.9   | script |
| cloud-tools  | doctl            | 1.150.0 | script |
| cloud-tools  | IBM Cloud CLI    | 2.41.1  | script |
| supabase-cli | Supabase CLI     | 2.76.4  | binary |
| infra-tools  | Terraform        | 1.14    | mise   |
| infra-tools  | kubectl          | 1.35    | mise   |
| infra-tools  | Helm             | 4.1     | mise   |
| infra-tools  | k9s              | 0.50    | mise   |
| infra-tools  | kustomize        | 5.8     | mise   |
| infra-tools  | yq               | 4.52    | mise   |
| infra-tools  | Ansible          | 13.3.0  | apt    |
| infra-tools  | Pulumi           | 3.220.0 | script |
| infra-tools  | Crossplane       | 2.1.4   | script |
| infra-tools  | kubectx          | 0.9.5   | script |
| infra-tools  | kubens           | 0.9.5   | script |
| infra-tools  | kapp             | 0.65.0  | script |
| infra-tools  | ytt              | 0.53.0  | script |
| infra-tools  | kbld             | 0.47.1  | script |
| infra-tools  | vendir           | 0.45.1  | script |
| infra-tools  | imgpkg           | 0.47.1  | script |

### Documentation & Productivity

| Extension  | Software   | Version | Source |
| ---------- | ---------- | ------- | ------ |
| mdflow     | Bun        | 1       | mise   |
| mdflow     | mdflow     | 2.33    | npm    |
| openskills | openskills | 1.5.0   | npm    |

### MCP Servers

| Extension      | Software       | Version | Source |
| -------------- | -------------- | ------- | ------ |
| pal-mcp-server | pal-mcp-server | 9.8.2   | binary |

### Research

| Extension       | Software       | Version | Source |
| --------------- | -------------- | ------- | ------ |
| ruvnet-research | Goalie         | 1.3     | npm    |
| ruvnet-research | research-swarm | 1.2     | npm    |

### Testing

| Extension  | Software   | Version | Source |
| ---------- | ---------- | ------- | ------ |
| playwright | Playwright | 1.58.2  | npm    |

---

## Notes

### Scope

This guide covers only extensions with **pinned versions**. Extensions where the
version is `dynamic`, `latest`, or `stable` (e.g., .NET SDK, Rust stable channel,
Composer, Symfony CLI) are excluded because their installed version depends on
what upstream provides at install time.

### Source Column Legend

| Source     | Meaning                                                        |
| ---------- | -------------------------------------------------------------- |
| **mise**   | Installed and version-managed by [mise](https://mise.jdx.dev/) |
| **npm**    | Installed via mise's npm backend                               |
| **script** | Custom install script (`install.sh`) with pinned version       |
| **binary** | Direct binary download from GitHub releases                    |
| **apt**    | System package manager (Ubuntu/Debian)                         |

### Adding a New CLI Version Section

When releasing a new CLI version (e.g., 3.2.x):

1. Add a new **"What Changed in 3.2.x"** section at the top (above the 3.1.x section)
2. Add a new **"CLI 3.2.x"** full matrix section (above the 3.1.x full matrix)
3. Update version numbers from current `extension.yaml` BOM sections
4. Commit with the release

### Cross-References

- [CHANGELOG.md](../CHANGELOG.md) — Release notes with change summaries
- [compatibility-matrix.yaml](../compatibility-matrix.yaml) — Machine-readable CLI ↔ extension version constraints
- [EXTENSIONS.md](EXTENSIONS.md) — Extension catalog, installation, and management guide
- [CLI.md](CLI.md) — CLI command reference
