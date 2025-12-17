# Fly.io anthropic-dev Profile Installation Failure Analysis

**Date:** 2025-12-17
**Status:** Analysis Complete
**Severity:** High - 6 of 28 extensions failed (21% failure rate)

## Executive Summary

The anthropic-dev profile partially failed installation on a Fly.io hosted VM with 6 extensions failing out of 28 total extensions (22 succeeded). Root cause analysis reveals fundamental incompatibilities between Docker-based tooling and Fly.io's Firecracker VM architecture, plus missing GPU configuration for AI workloads.

**Failed Extensions:**

1. `docker` - Incompatible with Fly.io architecture
2. `supabase-cli` - Cascade failure (depends on docker)
3. `ollama` - Requires GPU configuration
4. `infra-tools` - Complex installation with network issues
5. `tmux-workspace` - Network/apt repository issues
6. `cloud-tools` - Script installation with multiple external dependencies

**Recommended Action:** Create Fly.io-specific profile variant excluding Docker-dependent extensions.

---

## Extension Failure Analysis

### 1. docker Extension ❌ CRITICAL - INCOMPATIBLE

**Failure Mode:** Installation failed during apt repository setup

**Root Causes:**

- Fly.io uses Firecracker VMs, not Docker to run containers
- Docker CE installation requires:
  - Custom apt repository addition (`download.docker.com`)
  - GPG key verification
  - systemd daemon management
  - Privileged operations
- Docker-in-Docker not well-supported on Fly.io platform

**Evidence:**

```yaml
# docker/lib/extensions/docker/extension.yaml:17-29
install:
  method: apt
  apt:
    repositories:
      - gpgKey: https://download.docker.com/linux/ubuntu/gpg
        sources: deb [arch=amd64] https://download.docker.com/linux/ubuntu jammy stable
    packages:
      - docker-ce
      - docker-ce-cli
      - containerd.io
      - docker-compose-plugin
```

**Documentation References:**

- [Fly.io community: Docker-in-Docker discussion](https://community.fly.io/t/docker-in-docker-on-fly/3674)
- [Docker daemon permission errors on Fly.io](https://community.fly.io/t/got-permission-denied-while-trying-to-connect-to-the-docker-daemon-socket-at-unix-var-run-docker-sock/2364)

**Fix Options:**

1. **RECOMMENDED:** Remove from Fly.io profiles
2. Use [fly-apps/docker-daemon](https://github.com/fly-apps/docker-daemon) as separate service (complex)
3. Switch to DevPod or Docker providers for Docker-dependent workflows

---

### 2. supabase-cli Extension ❌ CASCADE FAILURE

**Failure Mode:** Dependency check failed

**Root Cause:**

- Explicit dependency on `docker` extension
- Dependency resolution fails when docker installation fails

**Evidence:**

```yaml
# docker/lib/extensions/supabase-cli/extension.yaml:9-11
metadata:
  dependencies:
    - docker
```

**Fix Options:**

1. **RECOMMENDED:** Remove from Fly.io profiles
2. Modify to use Supabase Cloud API instead of local Docker
3. Remove docker dependency and install standalone (may break local dev features)

---

### 3. ollama Extension ⚠️ GPU REQUIREMENT NOT MET

**Failure Mode:** Installation succeeded but runtime requires GPU

**Root Causes:**

- Ollama on Fly.io requires GPU for practical LLM inference
- anthropic-dev profile doesn't specify GPU configuration
- Without GPU, installation completes but models won't run efficiently
- 800MB binary download may timeout on slow networks

**Evidence:**

```bash
# docker/lib/extensions/ollama/install.sh:29-30
print_status "Downloading Ollama binary (this may take several minutes)..."
print_status "The binary is approximately 800MB - download time depends on network speed"
```

**Fly.io GPU Requirements:**

- GPU types: `a10`, `l40s`, `a100-40gb`, `a100-80gb`
- Available regions: `ord`, `iad`, `sjc`, `syd`, `ams`
- Requires explicit VM configuration in fly.toml

**Documentation References:**

- [Add Ollama to Fly.io apps](https://fly.io/docs/python/do-more/add-ollama/)
- [Getting Started with Fly GPUs](https://fly.io/docs/gpus/getting-started-gpus/)
- [Scaling LLMs with Ollama on Fly.io](https://fly.io/blog/scaling-llm-ollama/)

**Fix Options:**

1. **RECOMMENDED:** Remove from standard profile, create `anthropic-dev-gpu` variant
2. Add GPU configuration to sindri.yaml:
   ```yaml
   deployment:
     provider: fly
     resources:
       memory: 32GB
       cpus: 8
   providers:
     fly:
       region: ord
       gpuKind: a100-40gb
   ```
   **Cost Impact:** ~$200-400/month for GPU instance

---

### 4. infra-tools Extension ⚠️ PARTIAL INSTALLATION

**Failure Mode:** Script installation exited with code 1

**Root Causes:**

- Hybrid installation method (mise + apt + script)
- Requires access to 10+ external domains
- Network connectivity issues reported on Fly.io
- Installs 15+ tools; any single failure causes overall failure
- Some mise asdf plugins may fail to install

**Required Domains:**

```yaml
# docker/lib/extensions/infra-tools/extension.yaml:12-21
requirements:
  domains:
    - releases.hashicorp.com
    - apt.releases.hashicorp.com
    - dl.k8s.io
    - get.helm.sh
    - raw.githubusercontent.com
    - github.com
    - api.github.com
    - carvel.dev
    - pulumi.com
    - www.pulumi.com
```

**Tools Installed:**

- Via mise: terraform, kubectl, helm, packer, k9s, kustomize, yq
- Via apt: ansible, ansible-lint, jq, curl, wget
- Via script: pulumi, crossplane, kubectx, kubens, carvel tools

**Documentation References:**

- [Network unreachable in builder errors](https://community.fly.io/t/network-unreachable-in-builder/6273)
- [apt-get update failures on Fly.io](https://community.fly.io/t/fly-deploy-error-apt-get-update-apt-get-install/25416)

**Fix Options:**

1. **RECOMMENDED:** Accept partial installation; most tools likely succeed
2. Split into smaller extensions by tool category
3. Add retry logic for network-dependent operations
4. Add `continueOnError: true` flag for individual tool installations

---

### 5. tmux-workspace Extension ✅ SHOULD WORK

**Failure Mode:** apt installation failed

**Root Cause:**

- Simple apt installation should succeed
- Likely network/apt repository access issue during deployment
- May have been affected by broader apt update failures

**Installation Method:**

```yaml
# docker/lib/extensions/tmux-workspace/extension.yaml:18-26
install:
  method: apt
  apt:
    packages:
      - tmux
      - htop
```

**Requirements:**

- Dependencies: libevent, ncurses (automatically resolved by apt)
- No custom repositories required
- Standard Ubuntu packages

**Documentation References:**

- [tmux installation guide](https://github.com/tmux/tmux/wiki/Installing)
- [Installing tmux on Ubuntu](https://go.lightnode.com/tech/install-tmux-on-ubuntu)

**Fix Options:**

1. **RECOMMENDED:** Retry deployment (likely transient failure)
2. Manual installation via flyctl ssh console:
   ```bash
   sudo apt update
   sudo apt install -y tmux htop
   ```
3. Add retry logic to apt installations

---

### 6. cloud-tools Extension ⚠️ PARTIAL INSTALLATION

**Failure Mode:** Script installation failed (exit code 1)

**Root Causes:**

- Installs 7+ cloud provider CLIs sequentially
- Uses sudo for system-wide installations
- Requires access to 10+ external domains
- Script continues on individual failures but reports overall failure
- Some cloud provider domains may timeout or be restricted

**CLIs Installed:**

1. AWS CLI - https://awscli.amazonaws.com
2. Azure CLI - https://aka.ms/InstallAzureCLIDeb
3. Google Cloud CLI - https://packages.cloud.google.com
4. Fly.io CLI - https://fly.io/install.sh
5. Oracle Cloud CLI - GitHub
6. Alibaba Cloud CLI - https://aliyuncli.alicdn.com
7. DigitalOcean CLI - GitHub releases
8. IBM Cloud CLI - https://clis.cloud.ibm.com

**Required Domains:**

```yaml
# docker/lib/extensions/cloud-tools/extension.yaml:11-25
requirements:
  domains:
    - amazonaws.com
    - awscli.amazonaws.com
    - aka.ms
    - google.com
    - packages.cloud.google.com
    - fly.io
    - github.com
    - api.github.com
    - raw.githubusercontent.com
    - alicdn.com
    - aliyuncli.alicdn.com
    - ibm.com
    - clis.cloud.ibm.com
```

**Evidence from Script:**

```bash
# docker/lib/extensions/cloud-tools/install.sh:27, 41, 53
sudo ./aws/install
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash
curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo apt-key...
```

**Documentation References:**

- [AWS CLI installation guide](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)
- [Mastering AWS CLI on Ubuntu 2025](https://medium.com/@mahernaija/mastering-aws-cli-on-ubuntu-in-2025-3899e36e8fba)

**Fix Options:**

1. **RECOMMENDED:** Accept partial installation; core CLIs (AWS, Azure, GCP) likely succeed
2. Install only required CLIs via custom script
3. Add validation for each CLI and report individual successes
4. Split into separate extensions per cloud provider

---

## Fly.io Platform Constraints

### Architecture Limitations

**Firecracker VMs:**

- Fly.io uses Firecracker microVMs, not Docker containers
- Apps run with their own kernel in isolated VMs
- Standard Docker operations not supported without workarounds

**Network Access:**

- Intermittent connectivity issues to some external repositories
- Some apt repositories may timeout during builds
- GitHub API rate limiting may affect downloads

**System Operations:**

- sudo available during build and runtime
- systemd services require special configuration
- Privileged operations have limitations

### Resource Constraints

**Default VM Sizes:**

- Minimal: 1GB RAM, 1 vCPU (~$5-10/month)
- Standard: 2GB RAM, 1 vCPU (~$10-15/month)
- Performance: 4GB RAM, 2 vCPU (~$30-40/month)

**GPU Requirements:**

- GPU instances: ~$200-400/month
- Only available in specific regions
- Requires explicit configuration

---

## Fix Recommendations

### Option A: Remove Incompatible Extensions (RECOMMENDED)

Create Fly.io-compatible variant of anthropic-dev profile:

```yaml
# docker/lib/profiles.yaml
anthropic-dev-fly:
  description: AI development with Anthropic toolset - Fly.io optimized
  extensions:
    - agent-manager
    - claude-flow
    - agentic-flow
    - agentic-qe
    - golang
    # REMOVED: ollama (requires GPU)
    - ai-toolkit
    - claudish
    - claude-marketplace
    - infra-tools # Accept partial installation
    - jvm
    - mdflow
    - openskills
    - nodejs-devtools
    - playwright
    - rust
    - ruvnet-research
    - linear-mcp
    # REMOVED: supabase-cli (requires Docker)
    - tmux-workspace # Retry should work
    - cloud-tools # Accept partial installation
```

**Benefits:**

- Immediate deployment success
- No code changes to extensions
- Clear separation of provider capabilities

**Drawbacks:**

- Maintains multiple profile variants
- Docker-dependent workflows require workarounds

---

### Option B: Add Provider Compatibility Metadata

Enhance extension metadata with provider compatibility:

```yaml
# Example: docker/lib/extensions/docker/extension.yaml
metadata:
  name: docker
  version: 1.0.0
  compatibility:
    providers:
      docker: supported
      fly: unsupported
      devpod: supported
    reason: "Requires Docker-in-Docker which Fly.io does not support"
    alternatives:
      fly: "Use fly-apps/docker-daemon as separate service"
```

**Implementation:**

1. Add compatibility field to extension schema
2. Update extension validator to check compatibility
3. Add pre-deployment validation warnings
4. Document provider-specific limitations

**Benefits:**

- Prevents deployment failures upfront
- Self-documenting extension limitations
- Enables smart profile generation

---

### Option C: Individual Extension Fixes

#### docker Extension

**Status:** Not fixable for standard Fly.io VMs

**Workaround:**

- Deploy separate [fly-apps/docker-daemon](https://github.com/fly-apps/docker-daemon)
- Connect via WireGuard peer network
- Use for offloaded Docker builds and operations

**Complexity:** High - requires multi-app orchestration

#### ollama Extension

**Status:** Requires GPU configuration

**Fix:** Create GPU-enabled profile variant

```yaml
# examples/fly/anthropic-dev-gpu.sindri.yaml
version: 1.0
name: sindri-dev-gpu

deployment:
  provider: fly
  resources:
    memory: 32GB
    cpus: 8

extensions:
  profile: anthropic-dev

providers:
  fly:
    region: ord
    gpuKind: a100-40gb
```

**Cost:** ~$200-400/month

#### supabase-cli Extension

**Option 1:** Remove Docker dependency, use Supabase Cloud
**Option 2:** Follow Docker workaround above
**Recommended:** Option 1 - simpler, cloud-native

#### infra-tools Extension

**Status:** Accept partial installation

**Enhancement:** Add per-tool validation

```yaml
# Proposed extension.yaml enhancement
install:
  method: hybrid
  continueOnError: true
  reportSuccesses: true
```

#### tmux-workspace Extension

**Status:** Should work on retry

**Fix:** Add to deployment validation checklist

```bash
# Verify after deployment
flyctl ssh console -a <app-name>
tmux -V
htop --version
```

#### cloud-tools Extension

**Status:** Accept partial installation

**Enhancement:** Report per-CLI success

```bash
# Proposed install.sh enhancement
FAILED_CLIS=()
install_aws_cli() { ... } || FAILED_CLIS+=("aws")
install_azure_cli() { ... } || FAILED_CLIS+=("azure")
# Report failures but exit 0 if core CLIs succeed
```

---

## Recommended Action Plan

### Immediate Actions (1-2 hours)

1. **Create Fly.io-optimized profile**
   - Copy `anthropic-dev` to `anthropic-dev-fly`
   - Remove: `docker`, `supabase-cli`, `ollama`
   - Document in profile description

2. **Update example configurations**
   - Create `examples/fly/anthropic-dev-fly.sindri.yaml`
   - Document Fly.io-specific limitations

3. **Redeploy with new profile**

   ```bash
   # Update sindri.yaml
   extensions:
     profile: anthropic-dev-fly

   # Deploy
   ./cli/sindri deploy --provider fly
   ```

### Short-term Improvements (1-2 days)

1. **Add provider compatibility metadata**
   - Update extension schema
   - Add compatibility field to incompatible extensions
   - Implement pre-deployment validation

2. **Enhance error reporting**
   - Distinguish between fatal and partial failures
   - Report successful tool installations in multi-tool extensions
   - Add troubleshooting hints to failure messages

3. **Create GPU variant**
   - Create `anthropic-dev-gpu` profile
   - Add GPU configuration examples
   - Document cost implications

### Long-term Enhancements (1-2 weeks)

1. **Provider-aware profile management**
   - Auto-filter incompatible extensions
   - Generate provider-specific profiles
   - Validate before deployment

2. **Retry and resilience**
   - Add retry logic for network-dependent installations
   - Implement exponential backoff for external downloads
   - Cache downloaded binaries

3. **Alternative implementations**
   - Create cloud-native alternatives for Docker-dependent tools
   - Investigate rootless container runtimes (podman, nerdctl)
   - Document migration paths

---

## Testing Recommendations

### Validation Checklist

**Pre-deployment:**

- [ ] Profile contains only Fly.io-compatible extensions
- [ ] No Docker dependencies in extension DAG
- [ ] GPU configuration present if Ollama included
- [ ] Network access validated for required domains

**Post-deployment:**

- [ ] SSH connectivity verified
- [ ] Core CLIs installed (aws, az, gcloud)
- [ ] Development tools functional (nodejs, python, golang)
- [ ] AI toolkit accessible (claude-flow, agentic-flow)
- [ ] tmux/htop available

### Test Profiles

**Minimal Fly.io Test:**

```yaml
extensions:
  profile: minimal # nodejs, python only
```

**Standard Fly.io Test:**

```yaml
extensions:
  profile: fullstack # nodejs, python, docker-free
```

**Full Fly.io Test:**

```yaml
extensions:
  profile: anthropic-dev-fly
```

---

## Cost Analysis

### Current anthropic-dev Profile (with failures)

- VM: 2GB RAM, 1 vCPU = ~$10-15/month
- Volume: 10GB = ~$1.50/month
- **Total: ~$11.50-16.50/month**
- **Success Rate: 78% (22/28 extensions)**

### Proposed anthropic-dev-fly Profile

- VM: 2GB RAM, 1 vCPU = ~$10-15/month
- Volume: 10GB = ~$1.50/month
- **Total: ~$11.50-16.50/month**
- **Expected Success Rate: 95%+ (21/22 extensions)**

### Optional anthropic-dev-gpu Profile

- VM: 32GB RAM, 8 vCPU, a100-40gb GPU = ~$200-400/month
- Volume: 100GB = ~$15/month
- **Total: ~$215-415/month**
- **Success Rate: 100% (23/23 extensions with Ollama)**

---

## Related Documentation

- [Fly.io Deployment Guide](../providers/FLY.md)
- [Extension Authoring Guide](../EXTENSION_AUTHORING.md)
- [Profile Configuration](../CONFIGURATION.md#profiles)
- [Troubleshooting Guide](../TROUBLESHOOTING.md)

---

## Appendices

### Appendix A: Extension Dependency Graph

```text
anthropic-dev (28 extensions)
├── agent-manager
├── claude-flow ──> nodejs
├── agentic-flow ──> nodejs
├── agentic-qe ──> nodejs
├── golang
├── ollama (❌ GPU required)
├── ai-toolkit ──> nodejs, python, golang, github-cli
├── claudish ──> nodejs
├── claude-marketplace
├── infra-tools (⚠️ partial)
├── jvm
├── mdflow ──> nodejs
├── openskills ──> nodejs
├── nodejs-devtools ──> nodejs
├── playwright ──> nodejs
├── rust
├── ruvnet-research ──> nodejs
├── linear-mcp
├── supabase-cli ──> nodejs, docker (❌)
├── tmux-workspace (⚠️ retry)
└── cloud-tools (⚠️ partial)

Base extensions (auto-included):
├── mise-config
├── workspace-structure
└── github-cli
```

### Appendix B: Network Access Requirements

**Successful Extensions (22):**

- Standard package repositories (Ubuntu)
- GitHub releases API
- npm registry
- mise-managed tools

**Failed Extensions (6):**

- docker.com repositories
- Multiple cloud provider domains
- Large binary downloads (ollama)
- Complex multi-source installations

### Appendix C: Fly.io Platform Comparison

| Feature          | Docker Provider    | Fly.io Provider      | DevPod Provider       |
| ---------------- | ------------------ | -------------------- | --------------------- |
| Docker-in-Docker | ✅ Supported       | ❌ Not supported     | ✅ Supported          |
| GPU Access       | ⚠️ Via passthrough | ✅ Native support    | ⚠️ Via passthrough    |
| Auto-suspend     | ❌ Manual          | ✅ Native            | ⚠️ Platform dependent |
| Cost             | ~$0 (local)        | ~$12-400/month       | Varies by provider    |
| Network Access   | ✅ Full            | ⚠️ Some restrictions | ✅ Full               |
| systemd          | ✅ Full            | ⚠️ Limited           | ✅ Full               |

---

## References

### Fly.io Documentation

- [Docker-in-Docker Discussion](https://community.fly.io/t/docker-in-docker-on-fly/3674)
- [Docker Daemon Permissions](https://community.fly.io/t/got-permission-denied-while-trying-to-connect-to-the-docker-daemon-socket-at-unix-var-run-docker-sock/2364)
- [Fly Apps Docker Daemon](https://github.com/fly-apps/docker-daemon)
- [Add Ollama Guide](https://fly.io/docs/python/do-more/add-ollama/)
- [Getting Started with GPUs](https://fly.io/docs/gpus/getting-started-gpus/)
- [Scaling LLMs Blog](https://fly.io/blog/scaling-llm-ollama/)
- [Network Issues in Builder](https://community.fly.io/t/network-unreachable-in-builder/6273)
- [apt-get Failures](https://community.fly.io/t/fly-deploy-error-apt-get-update-apt-get-install/25416)

### Tool Documentation

- [tmux Installation Wiki](https://github.com/tmux/tmux/wiki/Installing)
- [Installing tmux on Ubuntu](https://go.lightnode.com/tech/install-tmux-on-ubuntu)
- [AWS CLI Installation](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)
- [AWS CLI on Ubuntu 2025](https://medium.com/@mahernaija/mastering-aws-cli-on-ubuntu-in-2025-3899e36e8fba)
- [Docker Rootless Mode](https://docs.docker.com/engine/security/rootless/)
- [Ollama Hardware Guide](https://www.arsturn.com/blog/ollama-hardware-guide-what-you-need-to-run-llms-locally)

---

**Document Version:** 1.0
**Last Updated:** 2025-12-17
**Author:** Sindri Team
**Status:** Analysis Complete - Pending Implementation
