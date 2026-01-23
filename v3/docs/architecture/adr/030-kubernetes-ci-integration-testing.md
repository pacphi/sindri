# ADR 030: Kubernetes CI Integration Testing Strategy

**Status**: Accepted
**Date**: 2026-01-23
**Deciders**: Core Team
**Related**: [ADR-029: Local K8s Management](029-local-kubernetes-cluster-management.md), [ADR-021: CI/CD](021-bifurcated-ci-cd-v2-v3.md)

## Context

The new `sindri-clusters` crate requires integration testing that creates real Kubernetes clusters. This introduces:

- Docker dependency in CI
- Time overhead (~30s per cluster creation)
- Resource consumption
- Potential flakiness from external tools

We need a testing strategy that provides confidence without excessive CI time/cost.

## Decision

### GitHub Actions Selection

**Decision**: Use established, maintained actions

| Provider | Action             | Version | Rationale                       |
| -------- | ------------------ | ------- | ------------------------------- |
| Kind     | helm/kind-action   | v1      | Helm-maintained, 0.31.0 default |
| K3d      | AbsaOSS/k3d-action | v2      | Registry support, config files  |

**Alternative considered**: Raw CLI installation
**Rejected**: More maintenance, less stable across runners

### Test Matrix Strategy

**Decision**: Run kind and k3d tests in parallel via matrix

```yaml
strategy:
  fail-fast: false
  matrix:
    provider: [kind, k3d]
```

**Rationale**:

- Parallel reduces wall-clock time
- `fail-fast: false` ensures both run even if one fails
- Matrix provides clear per-provider results

### Three-Tier Test Structure

**Decision**: Implement three CI jobs

1. **k8s-integration-kind**: Use pre-built kind cluster, test sindri commands
2. **k8s-integration-k3d**: Use pre-built k3d cluster + registry
3. **k8s-cluster-lifecycle**: Test sindri k8s create/destroy directly

**Rationale**:

- Jobs 1-2: Fast feedback using action-provided clusters
- Job 3: End-to-end lifecycle testing of sindri CLI

### Artifact Reuse

**Decision**: Download pre-built binary from rust-build job

```yaml
- uses: actions/download-artifact@v6
  with:
    name: sindri-v3-binaries-${{ github.sha }}
```

**Rationale**: Avoids rebuilding, ensures consistent binary

### Version Pinning

**Decision**: Pin Kubernetes version to 1.35.0

```yaml
node_image: kindest/node:v1.35.0
image: rancher/k3s:v1.35.0-k3s1
```

**Rationale**:

- Longest support window (Feb 2027)
- Stable release, well-tested
- Consistent across kind/k3d

### Optional vs Required

**Decision**: K8s integration tests are optional (non-blocking)

**Rationale**:

- Docker runner variability can cause flakiness
- Core Rust tests are required
- K8s tests provide additional confidence, not gatekeeping

## Implementation

### CI Workflow Additions

```yaml
# ============================================
# Kubernetes Integration Tests
# ============================================

k8s-integration-kind:
  name: K8s Integration (Kind)
  runs-on: ubuntu-latest
  needs: [rust-build]
  steps:
    - uses: actions/checkout@v6

    - name: Download sindri binary
      uses: actions/download-artifact@v6
      with:
        name: sindri-v3-binaries-${{ github.sha }}
        path: ./bin

    - name: Setup binary
      run: |
        chmod +x ./bin/sindri
        echo "$PWD/bin" >> $GITHUB_PATH

    - name: Create Kind cluster
      uses: helm/kind-action@v1
      with:
        cluster_name: sindri-test
        version: v0.31.0
        kubectl_version: v1.35.0
        node_image: kindest/node:v1.35.0

    - name: Verify cluster
      run: |
        kubectl cluster-info
        kubectl get nodes

    - name: Test sindri k8s commands
      run: |
        sindri k8s list
        sindri k8s status --name sindri-test

    - name: Cleanup
      if: always()
      run: kind delete cluster --name sindri-test

k8s-integration-k3d:
  name: K8s Integration (K3d)
  runs-on: ubuntu-latest
  needs: [rust-build]
  steps:
    - uses: actions/checkout@v6

    - name: Download sindri binary
      uses: actions/download-artifact@v6
      with:
        name: sindri-v3-binaries-${{ github.sha }}
        path: ./bin

    - name: Setup binary
      run: |
        chmod +x ./bin/sindri
        echo "$PWD/bin" >> $GITHUB_PATH

    - name: Create K3d cluster with registry
      uses: AbsaOSS/k3d-action@v2
      with:
        cluster-name: sindri-test
        k3d-version: v5.8.0
        args: >-
          --agents 1
          --image rancher/k3s:v1.35.0-k3s1
          --registry-create sindri-registry:5000
          --wait

    - name: Verify cluster
      run: |
        kubectl cluster-info
        kubectl get nodes
        docker ps | grep sindri-registry

    - name: Test sindri k8s commands
      run: |
        sindri k8s list
        sindri k8s status --name sindri-test

    - name: Cleanup
      if: always()
      run: k3d cluster delete sindri-test

k8s-cluster-lifecycle:
  name: K8s Cluster Lifecycle Tests
  runs-on: ubuntu-latest
  needs: [rust-build]
  strategy:
    fail-fast: false
    matrix:
      provider: [kind, k3d]
  steps:
    - uses: actions/checkout@v6

    - name: Download sindri binary
      uses: actions/download-artifact@v6
      with:
        name: sindri-v3-binaries-${{ github.sha }}
        path: ./bin

    - name: Setup binary
      run: |
        chmod +x ./bin/sindri
        echo "$PWD/bin" >> $GITHUB_PATH

    - name: Install ${{ matrix.provider }}
      run: |
        if [[ "${{ matrix.provider }}" == "kind" ]]; then
          curl -Lo ./kind https://kind.sigs.k8s.io/dl/v0.31.0/kind-linux-amd64
          chmod +x ./kind
          sudo mv ./kind /usr/local/bin/kind
        else
          curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash
        fi

    - name: Test full lifecycle with sindri CLI
      run: |
        # Create cluster via sindri
        sindri k8s create --provider ${{ matrix.provider }} --name lifecycle-test --nodes 2

        # Verify creation
        sindri k8s list | grep lifecycle-test
        sindri k8s status --name lifecycle-test

        # Get kubeconfig
        sindri k8s config --name lifecycle-test

        # Verify kubectl works with new cluster
        kubectl get nodes

        # Destroy cluster
        sindri k8s destroy --name lifecycle-test --force

        # Verify deletion
        ! sindri k8s list | grep lifecycle-test

    - name: Cleanup on failure
      if: failure()
      run: |
        if [[ "${{ matrix.provider }}" == "kind" ]]; then
          kind delete cluster --name lifecycle-test || true
        else
          k3d cluster delete lifecycle-test || true
        fi
```

### Test File Structure

```
v3/crates/sindri-clusters/tests/
├── common/
│   └── mod.rs               # Shared test utilities
├── kind_integration.rs      # Kind-specific integration tests
├── k3d_integration.rs       # K3d-specific integration tests
└── lifecycle_tests.rs       # Cross-provider lifecycle tests
```

### Integration Test Example

```rust
// v3/crates/sindri-clusters/tests/kind_integration.rs

use sindri_clusters::{ClusterConfig, ClusterProvider, KindProvider};

#[tokio::test]
#[ignore] // Requires Docker
async fn test_kind_cluster_lifecycle() {
    let provider = KindProvider::new();

    // Skip if kind not installed
    if !provider.check_installed() {
        eprintln!("Skipping: kind not installed");
        return;
    }

    let config = ClusterConfig {
        name: "test-lifecycle".to_string(),
        version: "v1.35.0".to_string(),
        nodes: 1,
        ..Default::default()
    };

    // Create
    let info = provider.create(&config).await.expect("Failed to create cluster");
    assert_eq!(info.name, "test-lifecycle");
    assert_eq!(info.context, "kind-test-lifecycle");

    // Verify exists
    assert!(provider.exists("test-lifecycle").await);

    // List
    let clusters = provider.list().await.expect("Failed to list");
    assert!(clusters.iter().any(|c| c.name == "test-lifecycle"));

    // Status
    let status = provider.status("test-lifecycle").await.expect("Failed to get status");
    assert!(status.ready);

    // Destroy
    provider.destroy("test-lifecycle", true).await.expect("Failed to destroy");

    // Verify gone
    assert!(!provider.exists("test-lifecycle").await);
}
```

## Consequences

### Positive

1. Real cluster testing catches integration issues
2. Matrix provides cross-provider coverage
3. Artifact reuse speeds up workflow
4. Version pinning ensures reproducibility

### Negative

1. ~3-5 min added to CI pipeline
2. Docker runner availability dependency
3. Potential flakiness from external tools

### Trade-offs

- Chose action-based clusters over manual: reliability
- Chose optional over required: pragmatic CI

## CI Best Practices Applied

| Practice           | Implementation                     |
| ------------------ | ---------------------------------- |
| Parallel execution | Matrix strategy for kind/k3d       |
| Artifact reuse     | Download pre-built binary          |
| Fast feedback      | Fail-fast disabled for matrix      |
| Cleanup on failure | `if: failure()` cleanup steps      |
| Version pinning    | Explicit kind v0.31.0, k3s v1.35.0 |
| Registry testing   | K3d with `--registry-create`       |
| Caching            | Cargo cache from rust-build        |

## References

- helm/kind-action: https://github.com/helm/kind-action
- AbsaOSS/k3d-action: https://github.com/AbsaOSS/k3d-action
- K8s testing best practices: https://seifrajhi.github.io/blog/testing-kubernetes-clusters-and-components/
- Testkube for K8s native testing: https://testkube.io/blog/automate-and-enhance-ci-cd-testing-with-github-actions-and-testkube
