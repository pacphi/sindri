#!/usr/bin/env python3
"""
decompose_extensions.py

Generates v4/registry-core/components/ atomic components from the v3
extension catalog and v3/profiles.yaml, following ADR-002 (atomic
components) and ADR-006 (collections as meta-components).

Mapping strategy
----------------
- Each v3 `extension.yaml` with a single `install.method` maps 1:1 to a
  v4 component directory `{backend}-{name}/component.yaml`.
- Bundle extensions (ai-toolkit, cloud-tools, infra-tools) are decomposed
  into one atomic component per BOM entry plus a `collection-<bundle>`
  meta-component that preserves the bundle UX.
- Hybrid installs (docker, infra-tools, excalidraw-mcp, xfce-ubuntu) are
  split into base + post-install atoms connected by `depends_on`.
- v3 profiles become `collection-<profile>/component.yaml` meta-components
  whose `depends_on` references the atomic components that replaced the
  listed extensions.

Run
---
    python3 v4/tools/decompose_extensions.py

Outputs
-------
- v4/registry-core/components/<component>/component.yaml   (many)
- v4/registry-core/index.yaml                              (rewritten)
"""
from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "v4" / "registry-core" / "components"
COLLECTIONS_OUT = ROOT / "v4" / "registry-core" / "collections"
INDEX = ROOT / "v4" / "registry-core" / "index.yaml"

ALL_PLATFORMS = [
    ("linux", "x86_64"),
    ("linux", "aarch64"),
    ("macos", "x86_64"),
    ("macos", "aarch64"),
]
ALL_PLUS_WINDOWS = ALL_PLATFORMS + [("windows", "x86_64")]


@dataclass
class Comp:
    backend: str                   # mise|binary|npm|pipx|cargo|apt|script|collection
    name: str                      # unqualified tool name
    version: str
    description: str
    license: str = "MIT"
    homepage: str = ""
    tags: list[str] = field(default_factory=list)
    platforms: list[tuple[str, str]] = field(default_factory=lambda: list(ALL_PLATFORMS))
    install: dict | None = None    # raw yaml-ish dict under install.<backend>
    depends_on: list[str] = field(default_factory=list)
    type_meta: bool = False        # collections

    @property
    def dirname(self) -> str:
        return self.name

    @property
    def id(self) -> str:
        return f"{self.backend}:{self.name}"


def mise(name, version, tool=None, desc=None, **kw):
    tool = tool or name
    return Comp(
        backend="mise", name=name, version=version,
        description=desc or f"{name} via mise",
        install={"mise": {"tools": {tool: version}}}, **kw,
    )


def npm(name, version, package, desc=None, **kw):
    return Comp(
        backend="npm", name=name, version=version,
        description=desc or f"{name} (npm, global)",
        install={"npm": {"package": package, "global": True}}, **kw,
    )


def binary(name, version, url_template, install_path=None, desc=None, **kw):
    install_path = install_path or f"~/.local/bin/{name}"
    checksums = {f"{os_}-{arch}": f"sha256:placeholder-{os_}-{arch}" for os_, arch in kw.get("platforms", ALL_PLATFORMS)}
    return Comp(
        backend="binary", name=name, version=version,
        description=desc or f"{name} (direct binary download)",
        install={"binary": {
            "url_template": url_template,
            "install_path": install_path,
            "checksums": checksums,
        }}, **kw,
    )


def script(name, version, script_path, desc=None, **kw):
    return Comp(
        backend="script", name=name, version=version,
        description=desc or f"{name} via install script",
        install={"script": {"install": script_path, "timeout": 600}}, **kw,
    )


def pipx(name, version, package, desc=None, **kw):
    return Comp(
        backend="pipx", name=name, version=version,
        description=desc or f"{name} via pipx",
        install={"pipx": {"package": package}}, **kw,
    )


def apt(name, version, packages, desc=None, **kw):
    return Comp(
        backend="apt", name=name, version=version,
        description=desc or f"{name} via apt",
        install={"apt": {"packages": packages, "update_first": True}},
        platforms=[("linux", "x86_64"), ("linux", "aarch64")], **kw,
    )


def cargo(name, version, crate, desc=None, **kw):
    return Comp(
        backend="cargo", name=name, version=version,
        description=desc or f"{name} via cargo install",
        install={"cargo": {"crate": crate, "version": version}}, **kw,
    )


def sdkman(name, version, candidate=None, desc=None, **kw):
    """SDKMAN-managed JVM ecosystem tool."""
    candidate = candidate or name
    return Comp(
        backend="sdkman", name=name, version=version,
        description=desc or f"{name} via SDKMAN",
        install={"sdkman": {"candidate": candidate, "version": version}},
        # SDKMAN is Unix-only (Linux + macOS)
        platforms=[p for p in kw.pop("platforms", ALL_PLATFORMS) if p[0] != "windows"],
        **kw,
    )


def collection(name, version, members, desc=None, **kw):
    c = Comp(
        backend="collection", name=name, version=version,
        description=desc or f"{name} collection",
        install={}, depends_on=members, type_meta=True, **kw,
    )
    return c


# ---------------------------------------------------------------------------
# Atomic components derived from v3/extensions (single-tool extensions)
# ---------------------------------------------------------------------------
COMPONENTS: list[Comp] = [
    # --- Existing (kept, re-emitted for parity) ---
    mise("nodejs", "22.0.0", tool="node", desc="Node.js JavaScript runtime via mise",
         license="MIT", homepage="https://nodejs.org", tags=["runtime", "javascript"]),
    mise("python", "3.12.0", desc="Python language runtime via mise",
         license="PSF-2.0", homepage="https://python.org", tags=["runtime", "python"]),
    mise("golang", "1.22.0", tool="go", desc="Go programming language via mise",
         license="BSD-3-Clause", homepage="https://go.dev", tags=["language", "go"]),
    binary("gh", "2.45.0",
           "https://github.com/cli/cli/releases/download/v{version}/gh_{version}_{os}_{arch}.tar.gz",
           desc="GitHub CLI", license="MIT", homepage="https://cli.github.com",
           tags=["github", "cli"]),
    npm("claude-code", "1.0.0", "@anthropic-ai/claude-code",
        desc="Claude Code — Anthropic's official CLI",
        license="Apache-2.0", homepage="https://github.com/anthropics/claude-code",
        tags=["ai", "anthropic", "claude"], depends_on=["mise:nodejs"]),

    # --- Languages / runtimes ---
    mise("rust", "1.83.0", tool="rust", desc="Rust toolchain via mise",
         license="Apache-2.0", homepage="https://rust-lang.org", tags=["language", "rust"]),
    mise("ruby", "3.3.0", desc="Ruby language runtime via mise",
         license="Ruby", homepage="https://ruby-lang.org", tags=["language", "ruby"]),
    mise("swift", "5.10", desc="Swift language toolchain via mise",
         license="Apache-2.0", homepage="https://swift.org", tags=["language", "swift"]),
    mise("haskell", "9.8.2", tool="ghc", desc="Haskell (GHC) via mise",
         license="BSD-3-Clause", homepage="https://haskell.org", tags=["language", "haskell"]),
    script("dotnet", "9.0", "install.sh",
           desc=".NET SDK via Microsoft install script",
           license="MIT", homepage="https://dot.net", tags=["language", "dotnet"]),
    script("php", "8.3", "install.sh",
           desc="PHP language and Composer",
           license="PHP-3.01", homepage="https://php.net", tags=["language", "php"]),
    script("sdkman", "5.19", "install.sh",
           desc="SDKMAN! — the software development kit manager",
           license="Apache-2.0", homepage="https://sdkman.io", tags=["tools", "sdk-manager"]),

    # --- SDKMAN-managed JVM ecosystem (decomposed from v3 jvm bundle) ---
    sdkman("java", "21.0.5-tem", desc="OpenJDK (Temurin) LTS via SDKMAN",
           license="GPL-2.0-with-classpath-exception", homepage="https://adoptium.net",
           tags=["language", "java", "jvm"], depends_on=["script:sdkman"]),
    sdkman("maven", "3.9.15", desc="Apache Maven build tool via SDKMAN",
           license="Apache-2.0", homepage="https://maven.apache.org",
           tags=["build-tool", "java"], depends_on=["script:sdkman"]),
    sdkman("gradle", "8.14", desc="Gradle build tool via SDKMAN",
           license="Apache-2.0", homepage="https://gradle.org",
           tags=["build-tool", "java"], depends_on=["script:sdkman"]),
    sdkman("kotlin", "2.1.20", desc="Kotlin language compiler via SDKMAN",
           license="Apache-2.0", homepage="https://kotlinlang.org",
           tags=["language", "kotlin", "jvm"], depends_on=["script:sdkman"]),
    sdkman("scala", "3.7.0", desc="Scala language compiler via SDKMAN",
           license="Apache-2.0", homepage="https://scala-lang.org",
           tags=["language", "scala", "jvm"], depends_on=["script:sdkman"]),
    sdkman("groovy", "4.0.27", desc="Apache Groovy language via SDKMAN",
           license="Apache-2.0", homepage="https://groovy-lang.org",
           tags=["language", "groovy", "jvm"], depends_on=["script:sdkman"]),
    sdkman("springboot", "3.5.0", candidate="springboot",
           desc="Spring Boot CLI via SDKMAN",
           license="Apache-2.0", homepage="https://spring.io/projects/spring-boot",
           tags=["framework", "java", "spring"], depends_on=["script:sdkman"]),

    # --- Container / runtime base ---
    script("docker", "27.3.0", "install.sh",
           desc="Docker Engine (apt/dnf + post-install config)",
           license="Apache-2.0", homepage="https://docker.com", tags=["container", "docker"]),

    # --- ai-toolkit decomposition (ADR-002) ---
    binary("fabric", "1.4.451",
           "https://github.com/danielmiessler/fabric/releases/download/v{version}/fabric_{os}_{arch}.tar.gz",
           desc="Fabric — modular AI prompt framework",
           license="MIT", homepage="https://github.com/danielmiessler/fabric",
           tags=["ai", "cli", "prompting"]),
    npm("codex", "0.122.0", "@openai/codex",
        desc="OpenAI Codex CLI", license="MIT",
        homepage="https://github.com/openai/openai-codex",
        tags=["ai", "cli", "openai"], depends_on=["mise:nodejs"]),
    npm("gemini-cli", "0.38.2", "@google/gemini-cli",
        desc="Google Gemini CLI", license="Apache-2.0",
        homepage="https://ai.google.dev", tags=["ai", "cli", "google"],
        depends_on=["mise:nodejs"]),
    npm("grok", "1.1.5", "grok-dev",
        desc="Open-source AI coding agent powered by Grok",
        license="MIT", homepage="https://github.com/superagent-ai/grok-cli",
        tags=["ai", "cli", "grok"], depends_on=["mise:nodejs"]),
    npm("droid", "0.109.1", "droid",
        desc="Factory Droid CLI — AI-powered software engineering agent",
        license="UNLICENSED", homepage="https://factory.ai",
        tags=["ai", "cli", "factory"], depends_on=["mise:nodejs"]),

    # --- cloud-tools decomposition ---
    binary("aws-cli", "2.34.33",
           "https://awscli.amazonaws.com/awscli-exe-{os}-{arch}-{version}.zip",
           desc="AWS CLI v2", license="Apache-2.0",
           homepage="https://aws.amazon.com/cli", tags=["cloud", "aws"],
           platforms=[("linux", "x86_64"), ("linux", "aarch64")]),
    pipx("azure-cli", "2.85.0", "azure-cli",
         desc="Azure CLI", license="MIT",
         homepage="https://docs.microsoft.com/cli/azure",
         tags=["cloud", "azure"], depends_on=["mise:python"]),
    script("gcloud", "565.0.0", "install.sh",
           desc="Google Cloud SDK", license="Apache-2.0",
           homepage="https://cloud.google.com/sdk", tags=["cloud", "gcp"]),
    binary("flyctl", "0.4.37",
           "https://github.com/superfly/flyctl/releases/download/v{version}/flyctl_{version}_{os}_{arch}.tar.gz",
           desc="Fly.io CLI", license="Apache-2.0",
           homepage="https://fly.io", tags=["cloud", "fly"]),
    binary("aliyun", "3.3.10",
           "https://github.com/aliyun/aliyun-cli/releases/download/v{version}/aliyun-cli-{os}-{version}-{arch}.tgz",
           desc="Alibaba Cloud CLI", license="Apache-2.0",
           homepage="https://alibabacloud.com/help/cli", tags=["cloud", "alibaba"]),
    binary("doctl", "1.155.0",
           "https://github.com/digitalocean/doctl/releases/download/v{version}/doctl-{version}-{os}-{arch}.tar.gz",
           desc="DigitalOcean CLI", license="Apache-2.0",
           homepage="https://docs.digitalocean.com/reference/doctl",
           tags=["cloud", "digitalocean"]),
    binary("ibmcloud", "2.42.0",
           "https://download.clis.cloud.ibm.com/ibm-cloud-cli/{version}/binaries/IBM_Cloud_CLI_{version}_{os}_{arch}.tgz",
           desc="IBM Cloud CLI", license="Apache-2.0",
           homepage="https://cloud.ibm.com/docs/cli", tags=["cloud", "ibm"],
           platforms=[("linux", "x86_64"), ("linux", "aarch64")]),

    # --- infra-tools decomposition ---
    mise("terraform", "1.14.9", desc="HashiCorp Terraform",
         license="BUSL-1.1", homepage="https://terraform.io", tags=["iac", "terraform"]),
    mise("kubectl", "1.35.4", desc="Kubernetes CLI",
         license="Apache-2.0", homepage="https://kubernetes.io", tags=["k8s", "kubectl"]),
    mise("helm", "4.1.3", desc="Kubernetes package manager",
         license="Apache-2.0", homepage="https://helm.sh", tags=["k8s", "helm"]),
    mise("packer", "1.15.1", desc="HashiCorp Packer",
         license="BUSL-1.1", homepage="https://packer.io", tags=["iac", "packer"]),
    mise("k9s", "0.50.18", desc="Kubernetes TUI (via asdf plugin)",
         license="Apache-2.0", homepage="https://k9scli.io", tags=["k8s", "tui"]),
    mise("kustomize", "5.4.3", desc="Kustomize for Kubernetes",
         license="Apache-2.0", homepage="https://kustomize.io", tags=["k8s"]),
    mise("yq", "4.44.3", desc="YAML processor",
         license="MIT", homepage="https://github.com/mikefarah/yq", tags=["yaml", "cli"]),
    apt("ansible", "13.5.0", ["ansible", "ansible-lint"],
        desc="Ansible configuration management",
        license="GPL-3.0", homepage="https://ansible.com", tags=["cfg-mgmt"]),
    script("pulumi", "3.231.0", "install.sh",
           desc="Pulumi IaC", license="Apache-2.0",
           homepage="https://pulumi.com", tags=["iac", "pulumi"]),
    script("crossplane", "2.2.1", "install.sh",
           desc="Crossplane CLI", license="Apache-2.0",
           homepage="https://crossplane.io", tags=["k8s", "crossplane"]),
    binary("kubectx", "0.10.2",
           "https://github.com/ahmetb/kubectx/releases/download/v{version}/kubectx_v{version}_{os}_{arch}.tar.gz",
           desc="Kubernetes context switcher", license="Apache-2.0",
           homepage="https://github.com/ahmetb/kubectx", tags=["k8s"]),
    binary("kubens", "0.10.2",
           "https://github.com/ahmetb/kubectx/releases/download/v{version}/kubens_v{version}_{os}_{arch}.tar.gz",
           desc="Kubernetes namespace switcher", license="Apache-2.0",
           homepage="https://github.com/ahmetb/kubectx", tags=["k8s"]),
    binary("kapp", "0.65.1",
           "https://github.com/carvel-dev/kapp/releases/download/v{version}/kapp-{os}-{arch}",
           desc="Carvel kapp deployment tool", license="Apache-2.0",
           homepage="https://carvel.dev/kapp", tags=["k8s", "carvel"]),
    binary("ytt", "0.53.2",
           "https://github.com/carvel-dev/ytt/releases/download/v{version}/ytt-{os}-{arch}",
           desc="Carvel ytt YAML templating", license="Apache-2.0",
           homepage="https://carvel.dev/ytt", tags=["k8s", "carvel"]),
    binary("kbld", "0.47.3",
           "https://github.com/carvel-dev/kbld/releases/download/v{version}/kbld-{os}-{arch}",
           desc="Carvel kbld image resolver", license="Apache-2.0",
           homepage="https://carvel.dev/kbld", tags=["k8s", "carvel"]),
    binary("vendir", "0.45.3",
           "https://github.com/carvel-dev/vendir/releases/download/v{version}/vendir-{os}-{arch}",
           desc="Carvel vendir sync", license="Apache-2.0",
           homepage="https://carvel.dev/vendir", tags=["k8s", "carvel"]),
    binary("imgpkg", "0.47.2",
           "https://github.com/carvel-dev/imgpkg/releases/download/v{version}/imgpkg-{os}-{arch}",
           desc="Carvel imgpkg bundle tool", license="Apache-2.0",
           homepage="https://carvel.dev/imgpkg", tags=["k8s", "carvel"]),

    # --- Monitoring / observability ---
    script("monitoring", "1.0.0", "install.sh",
           desc="Prometheus/Grafana local observability stack",
           license="Apache-2.0", homepage="https://prometheus.io", tags=["observability"],
           depends_on=["mise:python"]),

    # --- Claude/AI CLI ecosystem (NPM via mise or direct) ---
    npm("claudeup", "1.0.0", "claudeup",
        desc="claudeup — Claude toolchain updater",
        license="MIT", tags=["ai", "claude"], depends_on=["mise:nodejs"],
           homepage="https://github.com/sindri-dev/claudeup"),
    npm("claudish", "1.0.0", "claudish",
        desc="claudish — terminal companion for Claude",
        license="MIT", tags=["ai", "claude"], depends_on=["mise:nodejs"],
           homepage="https://github.com/sindri-dev/claudish"),
    script("claude-marketplace", "1.0.0", "install.sh",
           desc="Claude Marketplace skill/plugin resolver",
           license="MIT", tags=["ai", "marketplace"], depends_on=["npm:claude-code"],
           homepage="https://github.com/sindri-dev/claude-marketplace"),
    script("claude-codepro", "1.0.0", "install.sh",
           desc="Claude CodePro workflow bundle",
           license="MIT", tags=["ai"], depends_on=["mise:nodejs", "mise:python", "binary:gh"],
           homepage="https://github.com/sindri-dev/claude-codepro"),
    script("claude-code-mux", "1.0.0", "install.sh",
           desc="Multiplexer for multiple Claude Code sessions",
           license="MIT", tags=["ai", "terminal"],
           homepage="https://github.com/sindri-dev/claude-code-mux"),
    npm("agent-manager", "1.0.0", "@anthropic-ai/agent-manager",
        desc="Agent lifecycle manager",
        license="MIT", tags=["ai", "agents"], depends_on=["mise:nodejs"],
           homepage="https://github.com/sindri-dev/agent-manager"),
    npm("agent-skills-cli", "1.0.0", "@sindri/agent-skills-cli",
        desc="CLI for managing agent skills",
        license="MIT", tags=["ai", "skills"], depends_on=["mise:nodejs"],
           homepage="https://github.com/sindri-dev/agent-skills-cli"),
    npm("agentic-flow", "1.0.0", "agentic-flow",
        desc="agentic-flow orchestration CLI",
        license="MIT", tags=["ai", "agents"],
        depends_on=["npm:claude-code", "mise:nodejs"],
           homepage="https://github.com/ruvnet/agentic-flow"),
    npm("agentic-qe", "1.0.0", "agentic-qe",
        desc="Agentic quality-engineering harness",
        license="MIT", tags=["ai", "testing"],
        depends_on=["npm:claude-code", "mise:nodejs", "mise:python"],
           homepage="https://github.com/ruvnet/agentic-qe"),
    npm("ruflo", "1.0.0", "@ruvnet/ruflo",
        desc="ruflo hooks/routing integration",
        license="MIT", tags=["ai", "hooks"],
        depends_on=["npm:claude-code", "mise:nodejs"],
           homepage="https://github.com/ruvnet/ruflo"),
    npm("kilo", "1.0.0", "@kilo/cli",
        desc="Kilo AI CLI",
        license="MIT", tags=["ai"], depends_on=["mise:nodejs"],
           homepage="https://github.com/Kilo-Org/kilocode"),
    npm("compahook", "1.0.0", "compahook",
        desc="compahook — compaction hooks for Claude",
        license="MIT", tags=["ai", "hooks"], depends_on=["mise:nodejs"],
           homepage="https://github.com/ruvnet/compahook"),
    npm("openskills", "1.0.0", "openskills",
        desc="OpenSkills catalog CLI",
        license="MIT", tags=["ai", "skills"], depends_on=["mise:nodejs"],
           homepage="https://github.com/ruvnet/openskills"),
    npm("openclaw", "1.0.0", "@openclaw/cli",
        desc="OpenClaw agent CLI",
        license="MIT", tags=["ai"], depends_on=["mise:nodejs"],
           homepage="https://github.com/ruvnet/openclaw"),
    npm("opencode", "1.2.14", "opencode-ai",
        desc="OpenCode terminal AI coding assistant",
        license="MIT", homepage="https://opencode.ai",
        tags=["ai", "coding"], depends_on=["mise:nodejs"]),
    npm("gitnexus", "1.0.0", "gitnexus",
        desc="GitNexus code-intelligence CLI",
        license="MIT", tags=["code-intelligence"],
        depends_on=["npm:claude-code", "mise:nodejs"],
           homepage="https://github.com/ruvnet/gitnexus"),
    npm("mdflow", "1.0.0", "@mdflow/cli",
        desc="mdflow markdown workflow CLI",
        license="MIT", tags=["docs"], depends_on=["mise:nodejs"],
           homepage="https://github.com/ruvnet/mdflow"),
    npm("loki-mode", "1.0.0", "loki-mode",
        desc="Loki mode — Claude stateful explorer",
        license="MIT", tags=["ai"], depends_on=["mise:nodejs"],
           homepage="https://github.com/ruvnet/loki-mode"),
    npm("p-replicator", "1.0.0", "p-replicator",
        desc="Pattern replicator CLI",
        license="MIT", tags=["ai"], depends_on=["mise:nodejs"],
           homepage="https://github.com/ruvnet/p-replicator"),
    npm("paperclip", "1.0.0", "@paperclip/cli",
        desc="paperclip — context bundler",
        license="MIT", tags=["ai"], depends_on=["mise:nodejs"],
           homepage="https://github.com/ruvnet/paperclip"),
    npm("agent-browser", "1.0.0", "agent-browser",
        desc="Agent-driven browser automation wrapper",
        license="MIT", tags=["ai", "browser"],
        depends_on=["mise:nodejs", "script:playwright"],
           homepage="https://github.com/sindri-dev/agent-browser"),
    npm("nodejs-devtools", "1.0.0", "nodejs-devtools",
        desc="Curated Node.js dev tooling (tsx, vitest, etc.)",
        license="MIT", tags=["nodejs", "tooling"], depends_on=["mise:nodejs"],
           homepage="https://github.com/sindri-dev/nodejs-devtools"),
    npm("ruvnet-research", "1.0.0", "@ruvnet/research",
        desc="Research assistant CLI (Perplexity-backed)",
        license="MIT", tags=["ai", "research"], depends_on=["mise:nodejs"],
           homepage="https://github.com/ruvnet/ruvnet-research"),
    mise("glab", "1.44.0", desc="GitLab CLI via mise",
         license="MIT", homepage="https://gitlab.com/gitlab-org/cli",
         tags=["gitlab", "cli"]),

    # --- MCP servers ---
    script("context7-mcp", "1.0.0", "install.sh",
           desc="Context7 MCP server registration",
           license="MIT", tags=["mcp"], depends_on=["npm:claude-code"],
           homepage="https://context7.com"),
    script("jira-mcp", "1.0.0", "install.sh",
           desc="Atlassian Jira MCP server",
           license="MIT", tags=["mcp", "jira"], depends_on=["npm:claude-code"],
           homepage="https://www.atlassian.com/software/jira"),
    script("linear-mcp", "1.0.0", "install.sh",
           desc="Linear MCP server",
           license="MIT", tags=["mcp", "linear"], depends_on=["npm:claude-code"],
           homepage="https://linear.app"),
    script("excalidraw-mcp", "1.0.0", "install.sh",
           desc="Excalidraw MCP server",
           license="MIT", tags=["mcp", "diagrams"],
           homepage="https://excalidraw.com"),
    script("notebooklm-mcp-cli", "1.0.0", "install.sh",
           desc="NotebookLM MCP bridge",
           license="MIT", tags=["mcp", "notebooklm"], depends_on=["mise:python"],
           homepage="https://notebooklm.google.com"),
    script("pal-mcp-server", "1.0.0", "install.sh",
           desc="PAL MCP server",
           license="MIT", tags=["mcp", "pal"], depends_on=["mise:python"],
           homepage="https://github.com/BeehiveInnovations/pal-mcp-server"),

    # --- Scripts / misc tools ---
    script("goose", "1.0.0", "install.sh",
           desc="Block's Goose agent",
           license="Apache-2.0", homepage="https://block.github.io/goose",
           tags=["ai", "agents"]),
    script("ollama", "0.5.0", "install.sh",
           desc="Ollama local LLM runtime",
           license="MIT", homepage="https://ollama.com", tags=["ai", "llm"]),
    script("spec-kit", "1.0.0", "install.sh",
           desc="GitHub Spec Kit — spec-driven development",
           license="MIT", tags=["spec", "tools"], depends_on=["mise:python"],
           homepage="https://github.com/sindri-dev/spec-kit"),
    script("clarity", "1.0.0", "install.sh",
           desc="Clarity spec generator CLI",
           license="MIT", tags=["spec"], depends_on=["mise:python"],
           homepage="https://github.com/sindri-dev/clarity"),
    script("rtk", "1.0.0", "install.sh",
           desc="Reasoning Toolkit (rtk)",
           license="MIT", tags=["ai", "reasoning"],
           homepage="https://github.com/sindri-dev/rtk"),
    script("ralph", "1.0.0", "install.sh",
           desc="Ralph — personal coding agent",
           license="MIT", tags=["ai", "agents"],
           depends_on=["npm:claude-code", "mise:nodejs"],
           homepage="https://github.com/sindri-dev/ralph"),
    script("github-cli", "2.45.0", "install.sh",
           desc="GitHub CLI install via distro script (alias of binary:gh)",
           license="MIT", tags=["github"], depends_on=["binary:gh"],
           homepage="https://cli.github.com"),
    script("guacamole", "1.5.5", "install.sh",
           desc="Apache Guacamole local stack",
           license="Apache-2.0", homepage="https://guacamole.apache.org",
           tags=["remote-access"]),
    script("xfce-ubuntu", "1.0.0", "install.sh",
           desc="XFCE desktop for Ubuntu containers",
           license="GPL-2.0", tags=["desktop"],
           platforms=[("linux", "x86_64"), ("linux", "aarch64")],
           homepage="https://www.xfce.org"),
    script("draupnir", "1.0.0", "install.sh",
           desc="draupnir — infra forge",
           license="MIT", tags=["tools"],
           homepage="https://github.com/sindri-dev/draupnir"),
    script("openfang", "1.0.0", "install.sh",
           desc="openfang — offensive-sec learning harness",
           license="MIT", tags=["security", "training"],
           homepage="https://github.com/sindri-dev/openfang"),
    script("shannon", "1.0.0", "install.sh",
           desc="Shannon — signal processing playground",
           license="MIT", tags=["ml"],
           homepage="https://github.com/sindri-dev/shannon"),
    script("tmux-workspace", "1.0.0", "install.sh",
           desc="tmux workspace layouts + key bindings",
           license="MIT", tags=["terminal", "tmux"],
           homepage="https://github.com/sindri-dev/tmux-workspace"),
    cargo("ruvector-cli", "0.1.0", "ruvector-cli",
          desc="RuVector CLI (vector DB)",
          license="MIT", tags=["vector-db"], depends_on=["mise:rust"],
           homepage="https://github.com/ruvnet/ruvector"),
    cargo("rvf-cli", "0.1.0", "rvf-cli",
          desc="RVF CLI", license="MIT", tags=["tools"],
          depends_on=["mise:rust"],
           homepage="https://github.com/ruvnet/rvf-cli"),
    script("supabase-cli", "1.200.0", "install.sh",
           desc="Supabase CLI (docker-backed local dev)",
           license="Apache-2.0", homepage="https://supabase.com",
           tags=["database", "supabase"], depends_on=["script:docker"]),
    script("playwright", "1.48.0", "install.sh",
           desc="Playwright browsers + system deps",
           license="Apache-2.0", homepage="https://playwright.dev",
           tags=["browser", "testing"], depends_on=["mise:nodejs"]),
]

# ---------------------------------------------------------------------------
# Collections: bundle-replacements + v3 profiles
# ---------------------------------------------------------------------------
COLLECTIONS: list[Comp] = [
    # Bundle-extension replacements (ADR-002 §3 "install-the-whole-set UX preserved")
    collection("ai-toolkit", "2026.04", [
        "binary:fabric", "npm:codex", "npm:gemini-cli",
        "npm:grok", "npm:droid",
    ], desc="AI CLI toolkit (Fabric, Codex, Gemini, Grok, Droid)",
           homepage="https://github.com/sindri-dev/registry-core"),
    collection("cloud-tools", "2026.04", [
        "binary:aws-cli", "pipx:azure-cli", "script:gcloud",
        "binary:flyctl", "binary:aliyun", "binary:doctl", "binary:ibmcloud",
    ], desc="Cloud provider CLIs (AWS, Azure, GCP, Fly.io, Alibaba, DO, IBM)",
           homepage="https://github.com/sindri-dev/registry-core"),
    collection("infra-tools", "2026.04", [
        "mise:terraform", "mise:kubectl", "mise:helm", "mise:packer",
        "mise:k9s", "mise:kustomize", "mise:yq",
        "apt:ansible", "script:pulumi", "script:crossplane",
        "binary:kubectx", "binary:kubens",
        "binary:kapp", "binary:ytt", "binary:kbld",
        "binary:vendir", "binary:imgpkg",
    ], desc="Infrastructure & K8s tooling (Terraform, K8s, Carvel, Pulumi, Ansible)",
           homepage="https://github.com/sindri-dev/registry-core"),

    # jvm collection: replaces the v3 jvm bundle extension
    collection("jvm", "2026.04", [
        "script:sdkman",
        "sdkman:java", "sdkman:maven", "sdkman:gradle",
        "sdkman:kotlin", "sdkman:scala", "sdkman:groovy",
    ], desc="JVM toolchain (Java, Kotlin, Scala, Gradle, Maven, Groovy) via SDKMAN",
           homepage="https://github.com/sindri-dev/registry-core"),

    # v3 profiles → meta-components (ADR-006)
    collection("minimal", "2026.04", [
        "mise:nodejs", "mise:python",
    ], desc="Minimal development setup",
           homepage="https://github.com/sindri-dev/registry-core"),
    collection("fullstack", "2026.04", [
        "mise:nodejs", "mise:python", "script:docker", "npm:nodejs-devtools",
    ], desc="Full-stack web development",
           homepage="https://github.com/sindri-dev/registry-core"),
    # NOTE: collection-anthropic-dev is re-emitted with the full v3 profile
    # membership (supersedes the sprint-1 bootstrap version).
    collection("anthropic-dev", "2026.04", [
        "npm:claude-code", "npm:agent-manager", "npm:ruflo", "npm:agentic-qe",
        "npm:kilo", "script:ralph", "mise:golang", "script:ollama",
        "collection:ai-toolkit", "npm:claudish", "script:claude-marketplace",
        "npm:agent-skills-cli", "npm:compahook", "collection:infra-tools",
        "collection:jvm", "npm:mdflow", "npm:openskills", "script:pal-mcp-server",
        "npm:nodejs-devtools", "script:playwright", "npm:agent-browser",
        "mise:rust", "npm:ruvnet-research", "script:linear-mcp",
        "script:supabase-cli", "script:tmux-workspace",
        "collection:cloud-tools", "script:notebooklm-mcp-cli",
        "script:rtk", "cargo:ruvector-cli", "cargo:rvf-cli",
    ], desc="Anthropic developer stack (v3 default)",
           homepage="https://github.com/sindri-dev/registry-core"),
    collection("systems", "2026.04", [
        "mise:rust", "mise:golang", "mise:haskell",
        "script:docker", "collection:infra-tools",
    ], desc="Systems programming",
           homepage="https://github.com/sindri-dev/registry-core"),
    collection("enterprise", "2026.04", [
        "npm:claude-code", "npm:kilo", "mise:nodejs", "mise:python",
        "mise:golang", "mise:rust", "mise:ruby", "collection:jvm",
        "script:dotnet", "script:docker", "script:jira-mcp",
        "collection:cloud-tools",
    ], desc="Enterprise development (all major languages)",
           homepage="https://github.com/sindri-dev/registry-core"),
    collection("devops", "2026.04", [
        "script:docker", "collection:infra-tools",
        "script:monitoring", "collection:cloud-tools",
    ], desc="DevOps and infrastructure",
           homepage="https://github.com/sindri-dev/registry-core"),
    collection("mobile", "2026.04", [
        "mise:nodejs", "mise:swift", "script:linear-mcp",
        "script:supabase-cli",
    ], desc="Mobile development",
           homepage="https://github.com/sindri-dev/registry-core"),
]

ALL = COMPONENTS + COLLECTIONS


# ---------------------------------------------------------------------------
# YAML emitter (hand-rolled to keep stable ordering + no external deps)
# ---------------------------------------------------------------------------
def indent(text: str, n: int) -> str:
    pad = " " * n
    return "\n".join(pad + ln if ln else ln for ln in text.splitlines())


def emit_install(install: dict | None) -> str:
    if install is None:
        return "install: {}\n"
    if not install:
        return "install: {}\n"
    (backend, block), = install.items()
    out = ["install:", f"  {backend}:"]
    for k, v in block.items():
        if isinstance(v, dict):
            out.append(f"    {k}:")
            for kk, vv in v.items():
                out.append(f'      {kk}: "{vv}"')
        elif isinstance(v, list):
            out.append(f"    {k}:")
            for item in v:
                out.append(f'      - "{item}"')
        elif isinstance(v, bool):
            out.append(f"    {k}: {str(v).lower()}")
        else:
            out.append(f'    {k}: "{v}"')
    return "\n".join(out) + "\n"


def emit_component(c: Comp) -> str:
    platforms = "\n".join(
        f"  - os: {o}\n    arch: {a}" for o, a in c.platforms
    )
    tags = (
        "\n".join(f"    - {t}" for t in c.tags) if c.tags else ""
    )
    meta_lines = [
        "metadata:",
        f"  name: {c.name}",
        f'  version: "{c.version}"',
        f'  description: "{c.description}"',
    ]
    if c.type_meta:
        meta_lines.append("  type: meta")
    meta_lines.append(f"  license: {c.license}")
    if c.homepage:
        meta_lines.append(f'  homepage: "{c.homepage}"')
    if c.tags:
        meta_lines.append("  tags:")
        meta_lines.append(tags)
    meta = "\n".join(meta_lines) + "\n"

    deps = "depends_on: []\n" if not c.depends_on else (
        "depends_on:\n" + "\n".join(f'  - "{d}"' for d in c.depends_on) + "\n"
    )

    body = (
        meta
        + "\nplatforms:\n" + platforms + "\n"
        + "\n" + emit_install(c.install)
        + "\n" + deps
    )
    return body


def _preserve_checksums(comp: Comp, generated: str) -> str:
    """Keep any real (non-placeholder) checksums already in the existing file."""
    existing_path = (COLLECTIONS_OUT if comp.type_meta else OUT) / comp.name / "component.yaml"
    if not existing_path.exists():
        return generated
    try:
        import yaml as _yaml
        existing = _yaml.safe_load(existing_path.read_text())
        existing_checksums = (
            (existing.get("install") or {})
            .get("binary", {})
            .get("checksums", {})
        )
        for platform_key, sha_val in existing_checksums.items():
            if not str(sha_val).startswith("sha256:placeholder"):
                placeholder = f"sha256:placeholder-{platform_key}"
                if placeholder in generated:
                    generated = generated.replace(placeholder, str(sha_val))
    except Exception:
        pass
    return generated


def write_components() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    COLLECTIONS_OUT.mkdir(parents=True, exist_ok=True)
    for c in ALL:
        base = COLLECTIONS_OUT if c.type_meta else OUT
        d = base / c.name
        d.mkdir(parents=True, exist_ok=True)
        content = _preserve_checksums(c, emit_component(c))
        (d / "component.yaml").write_text(content)


def write_index() -> None:
    lines = [
        "version: 1",
        "registry: sindri/core",
        'generated_at: "2026-04-24"',
        "components:",
    ]
    for c in ALL:
        lines.append(f"  - name: {c.name}")
        lines.append(f"    backend: {c.backend}")
        lines.append(f'    latest: "{c.version}"')
        lines.append(f'    description: "{c.description}"')
        kind = "collection" if c.type_meta else "component"
        lines.append(f"    kind: {kind}")
        oci_path = f"collections/{c.name}" if c.type_meta else c.name
        lines.append(
            f'    oci_ref: "ghcr.io/sindri-dev/registry-core/{oci_path}:{c.version}"'
        )
        if c.depends_on:
            lines.append("    depends_on:")
            for d in c.depends_on:
                lines.append(f'      - "{d}"')
    INDEX.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    write_components()
    write_index()
    kinds = {}
    for c in ALL:
        kinds[c.backend] = kinds.get(c.backend, 0) + 1
    print(f"Wrote {len(ALL)} components to {OUT}")
    for k in sorted(kinds):
        print(f"  {k}: {kinds[k]}")
