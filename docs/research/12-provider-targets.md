# Targets — The v4 Provider Abstraction

**Where this fits.** Earlier docs (`07-cross-platform.md` §2.6, `11-command-comparison.md` §2.11) gestured at an "execution
target" concept but never fleshed it out. This doc is the dedicated treatment. It
maps v3's provider abstraction onto v4 and shows how a user picks Docker / DevPod /
e2b / Kubernetes / Northflank / Fly / RunPod / anything-future and then installs
the BOM there.

---

## 1. What v3 has today

v3 has a real, well-designed provider abstraction in `sindri-providers/src/traits.rs`:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    fn check_prerequisites(&self) -> Result<PrerequisiteStatus>;
    async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult>;
    async fn connect(&self, config: &SindriConfig) -> Result<()>;
    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus>;
    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()>;
    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan>;
    async fn start(&self, config: &SindriConfig) -> Result<()>;
    async fn stop(&self, config: &SindriConfig) -> Result<()>;
    fn supports_gpu(&self) -> bool { false }
    fn supports_auto_suspend(&self) -> bool { false }
}
```

Seven implementations: **Docker**, **Fly**, **DevPod** (which itself has 7
sub-backends: aws/gcp/azure/digitalocean/k8s/ssh/docker), **E2B**, **Kubernetes**,
**RunPod**, **Northflank**. Each is ~500–2000 lines. Provider is declared once in
`sindri.yaml` (`deployment.provider: e2b`) and locked at init — no deploy-time
override.

**What v3 got right:**

- Unified lifecycle (`deploy`/`connect`/`status`/`destroy`/`plan`/`start`/`stop`).
- `check_prerequisites()` as a first-class pre-flight.
- `supports_gpu()` / `supports_auto_suspend()` capability flags.
- Provider-specific config nested under `providers.{name}`.

**What v3 got wrong / leaky (flagged during audit):**

- Compile-time only — adding RunPod/Northflank meant editing 4–5 files and re-releasing the CLI.
- No deploy-time provider override. Locked at init.
- Auth scattered: E2B uses `E2B_API_KEY`, Fly uses `~/.fly/config.yml`, K8s uses `~/.kube/config`. No unified `sindri auth` flow.
- `DeploymentStatus.details` is a `HashMap<String, String>` — provider-specific fields untyped.
- Extensions are _provider-agnostic_ but behavior diverges silently: `apt`-using extensions fail in Docker's `socket` DinD mode but work in `sysbox`/`privileged`, with no clear error.
- GPU config scattered — trait method is a bool, but actual GPU requirements live in per-provider config, and runtime checks reject configs late (at deploy, not validate).

v4 keeps the good parts, fixes the leaks, renames, and makes the set **extensible** so
future providers don't require a CLI release.

---

## 2. What v4 calls it — **Targets**

Rename: `provider` → `target`. Reason: "provider" is overloaded in our own ecosystem
(backend providers, auth providers). "Target" is the same mental model as
cargo/rustc targets, mise targets, Dagger runners, buildkit targets — "where does
this run?".

**A target is:**

> An addressable, lifecycle-managed execution surface that can accept a `sindri.lock`
> and produce an installed, reachable runtime.

Examples: your laptop, a local Docker container, a Kubernetes namespace, an e2b
sandbox, a Fly machine, a RunPod GPU pod.

**Key insight v3 did not emphasize.** A target exposes a **platform profile** —
`os`, `arch`, and capability flags (`gpu`, `privileged`, `network`, `persistent-fs`,
`system-package-manager-available`). The BOM resolver intersects the target's
profile with each component's `platforms:` list and available backends (`07-cross-platform.md` §4).

This is what unifies the BOM story and the provider story:

```
sindri.yaml (BOM)  ×  target  →  per-target sindri.lock
                                 (same BOM, different backend choices per target profile)
```

Same `sindri.yaml` can produce a `sindri.linux-amd64.lock` (apt + mise), a
`sindri.e2b.lock` (mise-only — no sudo), and a `sindri.macos.lock` (brew + mise).
The manifest is target-agnostic; lockfiles are target-specific.

---

## 3. The target types v4 ships with

Ported from v3, same set, renamed consistently, with a clean trait boundary.

| Target       | Category          | Provisions                                   | Typical user               |
| ------------ | ----------------- | -------------------------------------------- | -------------------------- |
| `local`      | local host        | the calling machine itself                   | everyday dev on laptop     |
| `docker`     | local container   | a single container (optionally DinD)         | isolated local env         |
| `kubernetes` | orchestration     | pod + service in a namespace                 | team shared cluster        |
| `ssh`        | remote host       | existing Linux host                          | bare-metal / VPS           |
| `e2b`        | cloud sandbox     | ephemeral sandbox                            | CI, transient AI workloads |
| `fly`        | cloud VM          | Fly machine with auto-stop                   | global edge dev env        |
| `devpod`     | meta-provider     | delegates to AWS/GCP/Azure/DO/k8s/ssh/docker | cloud dev containers       |
| `runpod`     | GPU cloud         | GPU pod + persistent volume                  | ML/AI training             |
| `northflank` | K8s PaaS          | container service with auto-scaling          | managed hosting            |
| `wsl`        | Windows subsystem | WSL2 distro                                  | Windows devs               |

`local` is new as a first-class target — v3 treated "run on my host" as an
accident of running the CLI. v4 makes it explicit so `apply`/`diff`/`status` all
work the same way regardless of whether you're targeting your laptop or an e2b
sandbox.

## 4. Target provisioning _is_ in scope — the full infra-as-code story

**v2 and v3 Sindri don't just reach an existing compute surface — they provision
the whole thing.** `sindri deploy` on Fly creates the Fly app, allocates machines,
mounts volumes, opens ports, sets auto-stop. On Northflank it creates the project,
service, ports, volume, autoscaling. On Kubernetes it applies deployments,
services, PVCs, ingresses. On RunPod it creates the pod with the right GPU SKU,
volume, exposed ports. **v4 retains and standardizes this capability.** It's one
of Sindri's most valuable properties — users stop juggling `flyctl apps create` +
`flyctl machine run` + `flyctl volumes create` + `flyctl ips allocate` and just
declare their target in YAML.

The split is clean:

| Concern                                                                                                                           | Lives in               | Lifecycle verb                                            |
| --------------------------------------------------------------------------------------------------------------------------------- | ---------------------- | --------------------------------------------------------- |
| **Target infrastructure** (the compute surface itself: app, pod, machine, volume, service, ports, autoscaling, firewall, network) | `targets.<name>.infra` | `sindri target create`, `target update`, `target destroy` |
| **Target software** (what's installed inside: runtimes, CLIs, AI tools)                                                           | `components`           | `sindri apply`                                            |
| **Target runtime** (start / stop / connect / observe)                                                                             | `targets.<name>`       | `sindri target start/stop/shell/status`                   |

`sindri apply` is smart: if the target doesn't exist, it calls `target create`
first, then installs the BOM. `target create --dry-run` prints the full infra
plan (think `terraform plan` scoped to one target).

## 5. Manifest shape — per-target schemas

Each target kind has its own typed schema. Sindri validates every target block
against its JSON Schema at `sindri validate`. Unknown fields are rejected with
a suggestion. All provider-specific config lives in `targets.<name>.*`; no
out-of-band YAML anywhere.

### 5.1 Anatomy of a target entry

```yaml
targets:
  <name>: # user-defined alias (e.g. "laptop", "sandbox", "gpu")
    type:
      <kind> # local | docker | ssh | kubernetes | e2b | fly |
      # devpod-aws | devpod-gcp | devpod-azure |
      # devpod-digitalocean | devpod-kubernetes |
      # devpod-ssh | devpod-docker | runpod | northflank | wsl

    # Auth — CONTROL-PLANE credentials Sindri uses to talk to the provider's API.
    # Never enters the workload. Prefixed-value indirection (§8).
    auth:
      <credential>: env:FOO | secret:vault/path | file:~/.cred | cli:flyctl | keychain:foo

    # Infra-as-code — the provisioned surface. Schema differs by kind (§5.3).
    # May contain its own `env:` block for WORKLOAD-PLANE variables (see §5.1.1).
    infra: { ... }

    # Capability overrides — rare; useful when the default profile needs tuning.
    capabilities:
      gpu: { type: nvidia-h100, count: 1 }
      privileged: true

    # Pre/post hooks (think Terraform provisioners).
    hooks:
      pre_create: ["scripts/setup-firewall.sh"]
      post_create: ["scripts/smoke-ping.sh"]
      pre_destroy: ["scripts/backup.sh"]
```

### 5.1.1 `auth:` vs `infra.env:` — two boundaries, two audiences

These look superficially similar (both accept `env:FOO` / `secret:vault/...` values)
but serve completely different roles and must never be conflated.

```
 ┌─────────────────────────────────────────────────────────────────┐
 │                      YOUR MACHINE (Sindri CLI)                   │
 │                                                                  │
 │   `targets.<name>.auth:`  ──────────┐                            │
 │   ("control-plane creds")           │                            │
 │                                     ▼                            │
 │                      ┌──────────────────────────────┐            │
 │                      │  Sindri resolves auth locally│            │
 │                      │  (env/file/vault/keychain)   │            │
 │                      └──────────────┬───────────────┘            │
 │                                     │ Bearer token /             │
 │                                     │ kubeconfig / SSH key       │
 │                                     ▼                            │
 └─────────────────────────────────────┼────────────────────────────┘
                                       │
                               ┌───────▼────────┐
                               │  PROVIDER API  │  (Northflank / Fly / RunPod / K8s /
                               │ (control plane)│   e2b / Docker daemon / …)
                               └───────┬────────┘
                                       │ create pod, machine,
                                       │ app, namespace, sandbox, …
                                       ▼
 ┌─────────────────────────────────────────────────────────────────┐
 │                    PROVISIONED WORKLOAD (target)                 │
 │                                                                  │
 │   `targets.<name>.infra.env:` ─────┐                             │
 │   ("workload-plane variables")     │                             │
 │                                    ▼                             │
 │                      ┌─────────────────────────────┐             │
 │                      │ Provider injects as process │             │
 │                      │ env vars / container env    │             │
 │                      └─────────────┬───────────────┘             │
 │                                    ▼                             │
 │          your BOM runs here: node, claude-code, codex, …         │
 │          reads ANTHROPIC_API_KEY, OPENAI_API_KEY, …              │
 └─────────────────────────────────────────────────────────────────┘
```

|                      | `auth:`                                                                                | `infra.env:`                                                                                  |
| -------------------- | -------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| **Who reads it**     | Sindri CLI                                                                             | The workload running inside the target                                                        |
| **Who writes it**    | The user, once, on their machine                                                       | The user, per target, in YAML                                                                 |
| **Where it ends up** | In-memory during `target create` / `apply`; gone when CLI exits                        | Persisted in the provider as machine env / container env / K8s ConfigMap+Secret / Fly secrets |
| **Network boundary** | Never leaves the CLI host (except as an auth header on an outbound API call)           | Sent to the provider during provisioning; lives in the provider's storage                     |
| **Typical values**   | `NORTHFLANK_API_TOKEN`, kubeconfig, RunPod API key, Fly auth token, Docker socket path | `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `DATABASE_URL`, feature flags                          |
| **Granularity**      | One set per target (authenticates Sindri's API client)                                 | Per-workload (the process in the pod/machine/container)                                       |
| **Trust required**   | The provider (they see whether your API key is valid)                                  | The provider _plus_ anyone with `target shell` access                                         |

**Concrete walkthrough for the Northflank example below:**

```yaml
managed:
  type: northflank
  auth:
    apiToken:
      env:NORTHFLANK_API_TOKEN # ← Sindri reads this on YOUR laptop.
      #   Used once to call api.northflank.com
      #   to create the project/service/volume.
      #   Never sent to the workload.
  infra:
    # … service/volume/ports declarations …
    env:
      ANTHROPIC_API_KEY:
        secret:vault/anthropic/key # ← Sindri resolves this locally,
        #   then ASKS Northflank to inject it
        #   into the container as an env var.
        #   The running workload sees it;
        #   Northflank stores it as a secret.
```

Two different API calls, two different trust decisions. Separating them makes the
policy surface (§08) work: "allow Sindri to talk to Northflank" and "allow the
workload to hold an Anthropic key" are distinct permissions you might want to
audit independently.

**A few guard rails the resolver should enforce:**

1. **`auth:` values are never eligible to appear in `infra.env:` verbatim** without an explicit reference. (Typo-catching: `apiToken: $NORTHFLANK_API_TOKEN` in `env:` is almost certainly a mistake — the workload doesn't need Sindri's API token.)
2. **`infra.env:` values that resolve from `secret:…` are materialized as native provider secrets when the provider supports them** (Fly `flyctl secrets set`, Northflank secret groups, K8s `Secret` + `envFrom`, e2b metadata). Plain literals go as plain env vars. The resolver never embeds a secret value in the infra lockfile.
3. **Control-plane secrets (`auth:`) are never persisted to disk by Sindri** — resolved on-demand, held in memory only for the duration of one API call, zeroed afterward.

### 5.1.2 Other target blocks that carry workload-plane data

The Northflank example is the clearest case but the same pattern appears elsewhere:

- **Fly** — `infra.env:` for env, `infra.secrets:` for values pushed through `flyctl secrets set` (first-class provider distinction Sindri respects).
- **Kubernetes** — `infra.env:` compiles to the pod spec `env:`, `infra.secretRefs:` to `envFrom.secretRef`.
- **Docker** — `infra.env:` becomes `-e` flags on `docker run`.
- **RunPod** — `infra.env:` becomes the pod env, `infra.secretMounts:` becomes files at `/run/secrets/`.
- **e2b** — `infra.sandbox.metadata:` is the supported mechanism (e2b is API-driven, not env-var-driven).

In every case, the `auth:` block authenticates Sindri to the provider; the
`infra.*` sub-blocks describe what the provider should do, including what
workload-plane env/secrets it should inject on Sindri's behalf.

The `infra:` block is where the real per-target detail lives. Below are
representative schemas for each target kind — intentionally concrete so users and
reviewers can judge whether v3's capabilities are preserved.

### 5.2 Top-level file shape

```yaml
apiVersion: sindri.dev/v4
kind: BillOfMaterials
name: my-project

registries:
  - oci://ghcr.io/sindri-dev/registry-core:2026.04

targets: # many allowed
  laptop: { ... }
  sandbox: { ... }
  gpu: { ... }

defaultTarget: laptop

components: # target-agnostic BOM
  mise:nodejs: "22.11.0"
  collection:anthropic-dev: "2026.04"
```

### 5.3 Per-kind schemas

#### `local` — the calling machine

```yaml
laptop:
  type: local
  # No auth. No infra. The only configurable knob is where state lives.
  state:
    root: ~/.sindri/state/my-project
```

#### `docker` — local container with optional DinD

```yaml
box:
  type: docker
  infra:
    image: ghcr.io/sindri-dev/base:ubuntu-24.04 # or buildFromSource: ./Dockerfile
    name: my-project-box # container name
    network: bridge # bridge | host | none | <custom>
    restart: unless-stopped
    ports:
      - "8080:8080"
      - "3000:3000"
    volumes:
      - "./workspace:/workspace"
      - "sindri-state:/home/developer/.sindri" # named volume
    extraHosts:
      - "host.docker.internal:host-gateway"
    runtime: sysbox-runc # runc | sysbox-runc | auto
    dind:
      enabled: true
      mode: sysbox # sysbox | privileged | socket | auto
      storageDriver: overlay2
      storageSize: 50GB
    env:
      ANTHROPIC_API_KEY: secret:vault/anthropic/key
    resources:
      cpus: "4"
      memory: "8g"
      shmSize: "2g"
```

#### `ssh` — existing Linux host

```yaml
colo:
  type: ssh
  auth:
    key: file:~/.ssh/id_ed25519
    # or passphrase: keychain:my-ssh-passphrase
  infra:
    host: build01.internal
    user: deploy
    port: 22
    jumpHost: bastion.internal # optional
    knownHostsPolicy: strict # strict | tofu | accept-new
    workdir: /opt/sindri-projects/my-project
```

Nothing to "create" — the host exists. `target create` is a no-op beyond making
the workdir.

#### `kubernetes` — pod + service in a namespace

```yaml
cluster:
  type: kubernetes
  auth:
    kubeconfig: file:~/.kube/config
    context: dev-cluster
  infra:
    namespace: me-dev
    createNamespace: true
    serviceAccount: sindri-dev
    pod:
      nodeSelector:
        node-type: dev
      tolerations: []
      resources:
        requests: { cpu: "2", memory: "4Gi" }
        limits: { cpu: "4", memory: "8Gi", "nvidia.com/gpu": "1" } # GPU optional
      image: ghcr.io/sindri-dev/base:ubuntu-24.04
    storage:
      persistentVolumeClaim:
        name: sindri-dev-pvc
        storageClass: gp3
        size: 50Gi
        mountPath: /home/developer
    networking:
      service:
        type: ClusterIP # ClusterIP | NodePort | LoadBalancer
        ports:
          - { name: http, port: 8080 }
      ingress:
        enabled: true
        className: nginx
        host: dev-me.platform.acme.com
        tls: { secretName: dev-me-tls }
```

#### `fly` — Fly machine with auto-stop

```yaml
edge:
  type: fly

  # CONTROL-PLANE: token Sindri presents to api.fly.io.
  auth:
    token: cli:flyctl # reuse flyctl-stored creds

  infra:
    app: my-dev-env # Fly app name — created if absent
    organization: personal
    primaryRegion: iad
    regions: [iad, sjc, lhr] # multi-region
    machine:
      cpuKind: performance # shared | performance
      cpus: 2
      memory: 2048 # MB
      autoStopMachines: true
      autoStartMachines: true
      minMachinesRunning: 0 # 0 = full auto-stop
    volumes:
      - name: workspace
        sizeGb: 20
        mountPath: /home/developer
        region: iad
    services:
      - ports: [22] # SSH in
        protocol: tcp
        internalPort: 10022
      - ports: [80, 443]
        handlers: [http, tls]
        internalPort: 8080

    # WORKLOAD-PLANE: plain env baked into the machine config (visible in `fly config`).
    env:
      FLY_HTTP_CHECK_INTERVAL: 30s

    # WORKLOAD-PLANE, encrypted: pushed via `flyctl secrets set`.
    # Fly distinguishes these natively; Sindri honors the split.
    secrets:
      ANTHROPIC_API_KEY: secret:vault/anthropic/key
```

#### `e2b` — ephemeral sandbox

```yaml
sandbox:
  type: e2b
  auth:
    apiKey: env:E2B_API_KEY
  infra:
    template:
      alias: anthropic-dev # template derived from this BOM
      buildOnDeploy: false # true = rebuild template from sindri.yaml
      reuse: true
    sandbox:
      timeout: 600 # seconds before auto-shutdown
      autoPause: true
      autoResume: true
      internetAccess: true
      allowedDomains: ["api.anthropic.com", "*.anthropic.com"]
      blockedDomains: []
      publicAccess: false
      metadata:
        project: my-project
        owner: me@example.com
```

#### `runpod` — GPU pod + persistent volume

```yaml
gpu:
  type: runpod
  auth:
    apiKey: secret:vault/runpod/api_key
  infra:
    image: ghcr.io/sindri-dev/base-cuda:ubuntu-24.04
    gpu:
      typeId: "NVIDIA H100 80GB"
      count: 1
      cloudType: SECURE # SECURE | COMMUNITY
      region: US-EAST # optional filter
      spotBid: null # or a float for spot pricing
    compute:
      containerDiskGb: 40
      minVcpus: 8
      minMemoryGb: 32
    volume:
      sizeGb: 200
      mountPath: /workspace
    networking:
      exposePorts: [22, 8888] # SSH + Jupyter
      startSsh: true
    env:
      HF_TOKEN: secret:vault/hf/token
```

#### `northflank` — K8s PaaS service

```yaml
managed:
  type: northflank

  # CONTROL-PLANE: credentials Sindri uses to call api.northflank.com.
  # Never injected into the running workload.
  auth:
    apiToken: env:NORTHFLANK_API_TOKEN

  # What Sindri asks Northflank to provision.
  infra:
    project:
      name: my-project # created if absent
      region: us-central
    service:
      name: dev-env
      computePlan: nf-compute-50
      instances: 1 # 0 to pause
      gpu:
        type: nvidia-h100
        count: 1
      registryCredentials: ghcr-my-org # optional
    volume:
      sizeGb: 50
      mountPath: /workspace
    ports:
      - { name: http, internalPort: 8080, protocol: HTTP, public: true, domain: dev.acme.com }
      - { name: ssh, internalPort: 22, protocol: TCP, public: false }
    autoScaling:
      enabled: false
      # or: { minInstances: 1, maxInstances: 5, targetCpuPercent: 70 }

    # WORKLOAD-PLANE: env vars the running container reads.
    # Sindri resolves values locally, then asks Northflank to inject.
    # Values from `secret:...` land as Northflank secrets, not plain env.
    env:
      ANTHROPIC_API_KEY: secret:vault/anthropic/key
      OPENAI_API_KEY: secret:vault/openai/key
      FEATURE_FLAGS: "beta,xyz" # plain literals OK for non-sensitive
```

#### `devpod-*` — one entry per sub-backend

DevPod's v3 nesting (`providers.devpod.{aws,gcp,…}`) flattens to explicit types
per §14 open question #35:

```yaml
cloud:
  type: devpod-aws # or devpod-gcp / devpod-azure / ...
  auth:
    profile: file:~/.aws/credentials#sindri-dev # AWS profile spec
  infra:
    buildRepository: ghcr.io/acme/sindri-dev # pushed dev image
    region: us-west-2
    subnetId: subnet-abc123
    securityGroupId: sg-xyz789
    instanceType: m6i.xlarge
    diskSize: 100
    useSpot: true
```

Other `devpod-*` variants follow the same pattern with AWS/GCP/Azure/DO-specific
fields.

#### `wsl` — Windows Subsystem for Linux

```yaml
wsl:
  type: wsl
  infra:
    distribution: Ubuntu-24.04 # or a custom .tar
    name: sindri-dev
    createIfMissing: true
    shutdownOnDetach: false
```

### 5.4 Invoking against a target

```bash
# Target lifecycle — provisions the surface.
sindri target create gpu                           # create the RunPod pod + volume
sindri target create --dry-run edge                # show the Fly plan
sindri target update edge                          # re-converge (e.g., new region added)
sindri target destroy sandbox --force

# Software lifecycle — installs the BOM on the surface.
sindri apply --target gpu                          # create if needed, then install
sindri diff --target cluster
sindri target shell sandbox
sindri target status edge
```

Each target produces `sindri.<target>.lock` (software) and, where applicable,
`sindri.<target>.infra.lock` (resolved infra spec with concrete IDs: machine IDs,
app IDs, PVC names). Both are safe and recommended to commit.

## 6. `target create` in detail — what actually happens

Not v3's `deploy` verb (which conflated provisioning + installation). v4 separates:

```
sindri target create gpu
  → read  targets.gpu.infra
  → plan  provider API calls (e.g., runpod.create_pod, runpod.attach_volume)
  → prompt user (unless --yes)
  → execute API calls in dependency order
  → write sindri.gpu.infra.lock
  → run  hooks.post_create if defined

sindri apply --target gpu
  → if target doesn't exist → target create gpu (idempotent no-op if it does)
  → resolve sindri.yaml × target.profile() → sindri.gpu.lock
  → for each component in topological order:
       target.exec(backend install command)
  → run  configure, validate, hooks, project-init, collision-handling (01-current-state.md §8)
  → emit sindri.gpu.bom.spdx.json
```

Two lockfiles, two distinct idempotent operations, two distinct failure modes.
Users who want the full v2/v3 one-shot experience get it via `sindri apply
--target gpu` which chains both.

## 7. The `Target` trait — what v4 tightens

```rust
#[async_trait]
pub trait Target: Send + Sync {
    fn kind(&self) -> TargetKind;
    fn profile(&self) -> TargetProfile;           // NEW — os, arch, capabilities
    async fn check_prerequisites(&self) -> Result<PrerequisiteStatus>;

    // Lifecycle (unchanged from v3 intent, renamed to match `sindri` verbs):
    async fn plan(&self, lock: &Lockfile) -> Result<Plan>;
    async fn create(&self) -> Result<()>;           // provision the target itself
    async fn apply(&self, lock: &Lockfile) -> Result<ApplyResult>;
    async fn status(&self) -> Result<Status>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn destroy(&self, force: bool) -> Result<()>;

    // Execution primitives backends call into:
    async fn exec(&self, cmd: Command) -> Result<ExecOutput>;     // remote shell
    async fn upload(&self, src: &Path, dst: &RemotePath) -> Result<()>;
    async fn download(&self, src: &RemotePath, dst: &Path) -> Result<()>;
    async fn shell(&self) -> Result<()>;            // interactive session
}

pub struct TargetProfile {
    pub os: Os,                                    // linux | macos | windows
    pub arch: Arch,                                // x86_64 | aarch64
    pub distro: Option<LinuxDistro>,               // when os=linux
    pub capabilities: Capabilities,
}

pub struct Capabilities {
    pub gpu: Option<GpuSpec>,
    pub privileged: bool,
    pub persistent_fs: bool,
    pub outbound_network: bool,
    pub system_package_manager: Option<SystemPkgMgr>,  // apt | dnf | pacman | apk | none
    pub container_runtime: Option<ContainerRuntime>,
    pub auto_suspend: bool,
}
```

### Three tightening wins vs v3

1. **`profile()` drives backend selection.** The BOM resolver asks the target
   "what's your OS/arch/capabilities?" and uses that as the platform axis for
   component admissibility and backend preference. No more "apt works on e2b
   sometimes, silently breaks others."

2. **`exec`/`upload`/`download`/`shell` are first-class trait methods.** Backends
   (mise, apt, binary, etc.) don't care whether they're running locally or across
   SSH or through the e2b API — they call `target.exec(...)`. This was implicit
   in v3, now explicit. It's what makes the same backend reusable across all
   targets.

3. **Typed `Status` replaces `HashMap<String, String>`.** Each target kind
   declares its extras as a proper sum type, serialized via `serde_yaml` tagged
   unions. The console UI, CLI, and API all share the schema.

## 8. Authentication — unified model

v3's scattered auth story becomes a single abstraction:

```yaml
targets:
  gpu:
    type: runpod
    auth:
      apiKey: secret:vault/runpod/api_key
```

Supported auth-value prefixes:

| Prefix                    | Meaning                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------- |
| `env:VAR`                 | Read from environment variable                                                            |
| `file:~/.path`            | Read contents of a file                                                                   |
| `secret:<backend>/<path>` | Pull from Sindri's secret subsystem (existing `secrets` crate)                            |
| `cli:<binary>`            | Delegate to an installed CLI's stored creds (e.g., `cli:flyctl`, `cli:gh`, `cli:kubectl`) |
| `oauth:<provider>`        | Interactive OAuth flow, cached per user                                                   |
| `keychain:<name>`         | OS keychain (macOS Keychain, Windows Credential Manager, libsecret)                       |
| _plain_                   | Inline literal (allowed but `sindri validate` warns)                                      |

`sindri target auth <name>` walks the user through populating missing auth for one
target. `sindri doctor --target <name>` runs `check_prerequisites()` and reports
clearly what's missing: CLI binary, API key, kubeconfig context, etc.

## 9. Prerequisites & doctor integration

Every target's `check_prerequisites()` returns a typed `PrerequisiteStatus`:

```
$ sindri doctor --target gpu
Target gpu (runpod)
  ✔ runpodctl 1.15.0 (or API-only mode)
  ✘ RUNPOD_API_KEY — not set
      → fix: sindri target auth gpu
           or: export RUNPOD_API_KEY=...
  ✔ network: reachable (api.runpod.io)
  ⚠ GPU quota: only H100 available; A100 requested
      → fix: sindri target edit gpu  # change gpuTypeId
```

Prerequisites can be **auto-installable** when the missing item is itself a Sindri
component:

```
$ sindri doctor --target cluster
Target cluster (kubernetes)
  ✘ kubectl not found
      → fix: sindri add binary:kubectl --target laptop --apply
```

This ties targets back into the BOM — bootstrapping a new target is itself
declarative.

## 10. Extensibility — plugin targets (the v3 pain point)

v3 required editing 4–5 files and cutting a CLI release to add a new target. v4
should not. Three options, roughly ordered by effort:

### Option A — compile-time only, but cleaner (lowest effort)

Keep the `Target` trait compile-time only. Accept that "new target = CLI release."
This is simpler and is how devbox, mise, and most of the field actually work.

### Option B — OCI-distributed target plugins

Publish targets as OCI artifacts alongside component registries:

```
oci://ghcr.io/sindri-dev/targets/runpod:2026.04
```

A target plugin contains:

- A `target.yaml` schema describing config, capabilities, auth keys, prereqs.
- A WASM module (or native helper binary) implementing the `Target` trait via a
  defined host-guest ABI.
- A typed JSON Schema for the `targets.<name>` config block.

Sindri fetches the plugin on first use, caches, verifies cosign signature, and
loads. Users add a new target with:

```bash
sindri target add mytarget modal oci://ghcr.io/myorg/targets/modal:1.0
sindri target auth mytarget
```

Trade-offs:

- **+** Adding RunPod, Modal, Azure Container Apps, Replit sandboxes is just
  "publish an OCI artifact" — same ergonomics as components.
- **+** Sindri ships with a small set (local, docker, ssh, kubernetes); everything
  else is optional.
- **−** WASM sandbox means target plugins can't trivially shell out; requires
  host-provided capabilities. More design work.
- **−** Versioning the Target ABI across Sindri releases is nontrivial.

### Option C — subprocess-based plugins (simpler than WASM, recommended for v4.0)

Target plugins are just binaries on `$PATH` named `sindri-target-<name>`, speaking
a stable JSON-over-stdio protocol (like `terraform-provider-*`, `kubectl` plugins,
`gh` extensions). Lower ceiling than WASM, less sandboxed, but dramatically
simpler.

**Leaning:** Option C for v4.0 to de-risk, with a path to Option B in v4.1+ if
users want stronger isolation. The ABI story is the real blocker for WASM at
v4.0 scope.

## 11. Lifecycle walkthrough — one command across all targets

Same user, same `sindri.yaml`, different targets:

```bash
# Target 1: my laptop
sindri apply --target laptop
  → profile: macos-aarch64, { gpu: none, privileged: false, pkg_mgr: brew }
  → picks: brew:gh, mise:nodejs, binary:fabric  (based on 08-install-policy.md preference chain)
  → runs: locally via shell

# Target 2: Docker container
sindri apply --target box
  → profile: linux-aarch64 (matches host), { gpu: none, privileged: yes via sysbox, pkg_mgr: apt }
  → picks: apt:docker-ce (allowed because privileged), mise:nodejs, binary:gh
  → runs: via `docker exec` into the managed container

# Target 3: e2b sandbox
sindri apply --target sandbox
  → profile: linux-x86_64 (e2b runtime), { gpu: none, privileged: no, pkg_mgr: none }
  → picks: mise:nodejs, binary:gh  (apt denied, no pkg_mgr capability)
  → runs: via e2b WebSocket API exec

# Target 4: RunPod GPU box
sindri apply --target gpu
  → profile: linux-x86_64, { gpu: H100, privileged: yes, pkg_mgr: apt }
  → picks: mise:nodejs, mise:python, apt:cuda-toolkit (if GPU-conditional)
  → runs: via SSH proxy into the RunPod pod
```

Same BOM, four different lockfiles, four different reachability pipes. This is
what the provider abstraction was for in v3; v4 makes it cleaner and extensible.

## 12. CLI surface for targets

From `sindri target ...` (already in `11-command-comparison.md` §2.11, expanded here with the infra verbs):

```
# Declaration — writes to sindri.yaml targets block
sindri target add <name> <kind> [--config <file> | <k=v>...]
sindri target edit <name>                      # wrap $EDITOR with validation
sindri target remove <name>

# Infra-as-code lifecycle — creates/updates the compute surface
sindri target create <name> [--dry-run] [--yes]    # provision from targets.<name>.infra
sindri target update <name> [--dry-run]            # re-converge declared vs actual
sindri target destroy <name> [--force]             # delete app/pod/machine + volumes

# Runtime lifecycle
sindri target start <name> / stop <name>       # suspend/resume (where supported)
sindri target status [<name>]                  # live state: running, IPs, resource usage

# Access
sindri target shell <name> [--cmd "…"]         # interactive (or one-shot cmd)
sindri target exec <name> -- <cmd> <args>      # non-interactive execution
sindri target upload <name> <src> <dst>
sindri target download <name> <src> <dst>

# Config & health
sindri target ls                               # all targets with health summary
sindri target use <name>                       # set defaultTarget in sindri.yaml
sindri target info <name>                      # effective config + infra lock
sindri target auth <name>                      # walk user through auth setup
sindri target doctor [<name>]                  # prereqs + auth + connectivity + quota

# Plugins
sindri target plugin ls
sindri target plugin install <oci-ref>
sindri target plugin trust <name> --signer …
```

Every mutation verb validates the target block against the plugin's JSON schema
and the global admission policy (`08-install-policy.md`).

## 13. Relationship to BOM and lockfile

Three distinct artifacts per target, each with one owner:

| Artifact                               | Owner     | Written by                       | Purpose                                                                      |
| -------------------------------------- | --------- | -------------------------------- | ---------------------------------------------------------------------------- |
| `sindri.yaml` → `targets.<name>.infra` | user      | `sindri target add/edit`         | What compute surface the user wants                                          |
| `sindri.<name>.infra.lock`             | resolver  | `sindri target create / update`  | Resolved provider-API state (app IDs, machine IDs, PVC names, allocated IPs) |
| `sindri.yaml` → `components`           | user      | `sindri add/remove/pin`          | What software to install (target-agnostic)                                   |
| `sindri.<name>.lock`                   | resolver  | `sindri resolve --target <name>` | Pinned component digests for this target's profile                           |
| `sindri.<name>.bom.spdx.json`          | installer | `sindri apply --target <name>`   | SBOM of what was actually installed                                          |

The composition story:

1. **`sindri.yaml`** — declarative, target-agnostic BOM + typed `targets:` block.
2. **`sindri target create <name>`** — provisions the surface from `targets.<name>.infra`.
   Writes `sindri.<name>.infra.lock` with concrete provider IDs.
3. **`sindri resolve --target <name>`** — reads the target's `profile()`,
   intersects with each component's `platforms:` list, picks backends per
   `08-install-policy.md` preference chain, writes `sindri.<name>.lock`.
4. **`sindri apply --target <name>`** — chains `target create` (idempotent) +
   `resolve` + per-backend installs via `target.exec()`, verifies state, emits
   the SBOM.

Targets and components **never know about each other directly**. The resolver is
the mediator. This is what makes both extensible: you can add a new target without
changing any component, and add a new component without changing any target —
provided the platform profile matches.

## 14. What stays out of v4.0

- **Multi-target "deploy everywhere" verb** (e.g., `sindri apply --target all`). Useful but scope creep for v4.0; users can script it.
- **Target federation / fleet management** (many-boxes-at-once with central control plane). Belongs in a higher layer.
- **Cross-cloud infra primitives** — v4 manages _one_ compute surface per target (one Fly app, one RunPod pod, one K8s namespace scope). It does not manage, say, VPC peering between a Fly app and an AWS RDS instance. Users who need that keep Terraform/Pulumi for the surrounding graph; Sindri owns the dev-environment surface inside it.
- **Sindri as a general IaC tool.** The infra each target creates is the _compute surface required for the BOM to run on_. That's scoped and useful. Extending `target create` to arbitrary resources (S3 buckets, IAM roles, DNS records) is out of scope — use Terraform/Pulumi/CDK for those and let them reference the target by its lock.
- **WASM target plugin ABI** (Option B in §10). Targets are subprocess-plugged for v4.0; WASM deferred to v4.1+.
- **"Convert a running target back into YAML"** (reverse provisioning). Imports from v3 configs or bare-cloud resources are out of scope.

Explicitly _in scope_ and retained from v2/v3: the full provision-the-compute-surface-from-YAML capability. Fly app + machines + volumes + services; Northflank project + service + ports + volume + autoscaling; K8s namespace + deployment + PVC + ingress; RunPod pod + GPU + volume + exposed ports; e2b template build + sandbox lifecycle. All captured in typed per-target schemas (§5.3).

## 15. Recommendations

1. **Rename `provider` → `target`** across code, docs, config. Pain once, clarity forever.
2. **Ship v4.0 with the seven v3 providers + `local` + `wsl` + `ssh`** as compile-time built-ins. Don't regress capability.
3. **Retain full infra-as-code provisioning** — the per-target `infra:` schemas in §5.3 are the contract; no regression from v2/v3.
4. **Add `profile()` to the trait from day one.** Foundation for clean BOM×target resolution. Retrofitting later is hard.
5. **Standardize auth with the prefixed-value model (§8).** Pay the unification cost in v4.0 so every future target inherits it for free.
6. **Subprocess plugin protocol** over stdio-JSON for v4.0; document it, let early community targets start forming. Graduate to WASM in v4.1+.
7. **Per-target lockfiles × 2.** `sindri.<name>.lock` (software) + `sindri.<name>.infra.lock` (provider state). Explicit and correct. Single-lockfile alternatives hide real differences.
8. **Publish a JSON Schema per target kind** at `schemas.sindri.dev/v4/targets/<kind>.json`, referenced by an auto-emitted pragma — same pattern as the BOM schema (`09-imperative-ux.md` §6). IDE autocomplete on `targets.<name>.infra.*` is the make-or-break UX for non-trivial target configs.
9. **`sindri doctor --target <name>`** must become the first thing a user runs before a new deploy. Make the error messages actionable and auto-fix-capable.

## 16. New open questions (adding to §05)

31. **Per-target lockfiles vs unified lockfile.**
    `sindri.<name>.lock` per target is the proposed default (§13, §15.7). The
    alternative is one lockfile with per-target sections. Affects git hygiene,
    merge conflicts, and user mental model.

32. **Target plugin extensibility — subprocess or WASM?**
    Subprocess (§8 Option C) for v4.0, WASM (Option B) later? Or commit to
    compile-time only for v4.0 and revisit after? Affects how fast external
    targets like Modal, Replit, Lambda can appear.

33. **Default target when `sindri.yaml` declares none.**
    Implicit `local`? Error prompting user to pick? Relates to whether the
    happy-path user who never opens YAML even needs to think about targets.

34. **Auth unification backward compatibility.**
    Does v4 preserve v3's env-var-per-provider pattern (`E2B_API_KEY`, `FLY_API_TOKEN`)
    as shorthand, or force everyone through `auth:` blocks from day one?
    Leaning: support both; env vars are an implicit `env:` prefix.

35. **Can a target be a "virtual" aggregate?**
    Is `target: devpod` a real thing or just a router to its sub-backends (aws/gcp/…)?
    v3 treats it as one provider with a `type:` selector. Consider collapsing in v4:
    `target: { type: devpod-aws, ... }` rather than nested `devpod.aws`.

36. **Target infra boundary — where does Sindri stop and Terraform start?**
    Sindri owns _the dev-environment surface_: one Fly app, one K8s namespace scope,
    one RunPod pod, etc. It does _not_ own the surrounding graph (VPCs, IAM, DNS
    beyond what the target itself creates). This line needs to be spelled out in
    docs so users know whether to reach for Sindri or Terraform for adjacent
    concerns. Concretely: does `targets.cluster.infra.networking.ingress` own the
    ingress DNS record, or just the Ingress resource? Leaning: own only what the
    target's provider API can atomically create/destroy; anything outside that is
    explicitly a reference the user wires in (e.g., `tls.secretName` references
    a secret Sindri doesn't manage).

37. **Updatability of target infra — all fields, or just some?**
    Some target-infra fields are safely updatable in place (instance count,
    scheduled autoscaling); others require destroy+recreate (region change,
    volume shrink). The `target update` command must classify changes the same
    way `terraform plan` does, and either do the right thing or fail loudly.
    Needs a per-target-kind classification table as part of the implementation.
