# ADR 009: Dependency Resolution with DAG-based Topological Sort

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-008: Extension Type System](008-extension-type-system-yaml-deserialization.md), [Extension Guide](../../EXTENSIONS.md)

## Context

Sindri extensions can depend on other extensions, forming a dependency graph. For example:

- `claude-flow-v3` depends on `nodejs` (for npm-based MCP servers)
- `agentic-qe` depends on `python` (for Python-based tools)
- `spec-kit` depends on `git` (for auto-commit hooks)
- `advanced-dev-env` depends on `nodejs`, `python`, `go` (polyglot setup)

The extension system must:

1. **Resolve dependencies recursively**: If A depends on B and B depends on C, install C → B → A
2. **Detect cycles**: A depends on B, B depends on A (invalid)
3. **Handle diamonds**: A depends on B and C, both B and C depend on D (install D once)
4. **Topological ordering**: Ensure dependencies install before dependents
5. **Fail fast**: Report missing/circular dependencies before installation begins
6. **Support partial graphs**: Install subset of extensions respecting dependencies

The bash implementation used a custom DFS with shell arrays, but it lacked cycle detection and proper error handling. The Rust migration required a robust graph algorithm that could be proven correct.

Example dependency scenarios:

**Linear chain**:
```
spec-kit → git → (no deps)
```
Order: git, spec-kit

**Diamond**:
```
advanced-dev-env → nodejs, python
nodejs → (no deps)
python → (no deps)
```
Order: nodejs, python, advanced-dev-env (or python, nodejs, advanced-dev-env)

**Invalid cycle**:
```
ext-a → ext-b
ext-b → ext-c
ext-c → ext-a
```
Must be rejected with clear error.

## Decision

### DFS-Based Topological Sort

We implement a **Depth-First Search (DFS) topological sort** with explicit cycle detection using a visiting/visited state machine:

```rust
use std::collections::{HashMap, HashSet};
use anyhow::{bail, Result};

pub struct DependencyResolver {
    /// Extension name → list of dependency names
    graph: HashMap<String, Vec<String>>,
    /// Available extensions (from registry)
    registry: HashMap<String, Extension>,
}

impl DependencyResolver {
    pub fn resolve(&self, extensions: &[String]) -> Result<Vec<String>> {
        let mut visiting = HashSet::new();  // Currently in DFS stack
        let mut visited = HashSet::new();   // Completely processed
        let mut order = Vec::new();         // Topological order

        // Start DFS from each requested extension
        for ext in extensions {
            if !visited.contains(ext) {
                self.dfs(ext, &mut visiting, &mut visited, &mut order)?;
            }
        }

        Ok(order)
    }

    fn dfs(
        &self,
        node: &str,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<()> {
        // Check if extension exists
        if !self.registry.contains_key(node) {
            bail!("Extension '{}' not found in registry", node);
        }

        // Cycle detection: node already in current path
        if visiting.contains(node) {
            bail!("Circular dependency detected: {} is part of a cycle", node);
        }

        // Already processed
        if visited.contains(node) {
            return Ok(());
        }

        // Mark as currently visiting
        visiting.insert(node.to_string());

        // Recursively visit dependencies
        if let Some(deps) = self.graph.get(node) {
            for dep in deps {
                self.dfs(dep, visiting, visited, order)?;
            }
        }

        // Mark as fully visited
        visiting.remove(node);
        visited.insert(node.to_string());

        // Add to topological order (post-order traversal)
        order.push(node.to_string());

        Ok(())
    }
}
```

### Cycle Detection with Visiting/Visited Sets

The algorithm uses **three states** for each node:
- **Unvisited**: Not yet explored (not in visiting or visited)
- **Visiting**: Currently in DFS recursion stack (in visiting set)
- **Visited**: Completely processed (in visited set)

Cycle detection: If we encounter a node in the **visiting** set, we have a cycle.

Example cycle detection:
```
Graph: A → B → C → A

DFS from A:
  visiting: {A}
  Visit B:
    visiting: {A, B}
    Visit C:
      visiting: {A, B, C}
      Visit A:
        A in visiting → CYCLE DETECTED
```

### Diamond Dependency Handling

Diamond dependencies are automatically handled by the visited set:

```
Graph:
  A → B, C
  B → D
  C → D

DFS from A:
  Visit A:
    Visit B:
      Visit D:
        visited: {D}, order: [D]
    visited: {D, B}, order: [D, B]
    Visit C:
      Visit D:
        D already in visited → skip
    visited: {D, B, C}, order: [D, B, C]
  visited: {D, B, C, A}, order: [D, B, C, A]
```

D is visited only once, even though both B and C depend on it.

### Integration with petgraph (Optional Enhancement)

For future enhancements (visualization, analysis), we can integrate with `petgraph`:

```rust
use petgraph::graph::DiGraph;
use petgraph::algo::toposort;

pub fn resolve_with_petgraph(&self, extensions: &[String]) -> Result<Vec<String>> {
    let mut graph = DiGraph::new();
    let mut indices = HashMap::new();

    // Build petgraph structure
    for ext in &self.registry.keys() {
        let idx = graph.add_node(ext.clone());
        indices.insert(ext.clone(), idx);
    }

    for (ext, deps) in &self.graph {
        for dep in deps {
            let from = indices[dep];
            let to = indices[ext];
            graph.add_edge(from, to, ());
        }
    }

    // Topological sort (returns Err on cycle)
    let sorted = toposort(&graph, None)
        .map_err(|_| anyhow!("Circular dependency detected"))?;

    Ok(sorted.into_iter()
        .map(|idx| graph[idx].clone())
        .collect())
}
```

**Note**: petgraph adds ~200KB to binary, so we use manual DFS by default and offer petgraph as optional feature flag.

## Consequences

### Positive

1. **Correctness**: DFS topological sort is well-proven algorithm
2. **Cycle Detection**: Explicit visiting/visited state catches all cycles
3. **Diamond Handling**: Visited set prevents duplicate installations
4. **Performance**: O(V + E) time complexity (linear in graph size)
5. **Memory**: O(V) space for visiting/visited sets
6. **Error Messages**: Clear reporting of missing/circular dependencies
7. **Testing**: Easy to construct test graphs and verify ordering
8. **Debugging**: Can print DFS traversal for troubleshooting
9. **Extensibility**: Can add features like visualization with petgraph
10. **Deterministic**: Given same graph, always produces valid topological order

### Negative

1. **Non-deterministic Order**: Multiple valid topological orders exist (e.g., [B, C, A] vs [C, B, A] for diamond)
2. **Recursion Depth**: Deep dependency chains could hit stack limits (mitigated: typical depth < 10)
3. **No Parallelism**: Topological order is sequential; can't install independent extensions concurrently
4. **Graph Construction**: Must load all extension metadata before resolution
5. **No Conflict Resolution**: Can't choose between multiple versions of same dependency
6. **Error Propagation**: Single missing dependency fails entire resolution

### Neutral

1. **Algorithm Choice**: DFS vs BFS vs Kahn's algorithm (all equivalent for correctness)
2. **petgraph Dependency**: Optional, but useful for advanced features
3. **Order Stability**: Could sort dependencies alphabetically for deterministic order across runs

## Alternatives Considered

### 1. Kahn's Algorithm (BFS-based)

**Description**: Use Kahn's topological sort algorithm (BFS with in-degree counting).

```rust
pub fn resolve_kahns(&self, extensions: &[String]) -> Result<Vec<String>> {
    let mut in_degree = HashMap::new();
    let mut queue = VecDeque::new();
    let mut order = Vec::new();

    // Compute in-degrees
    for (ext, deps) in &self.graph {
        in_degree.entry(ext.clone()).or_insert(0);
        for dep in deps {
            *in_degree.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    // Add nodes with in-degree 0 to queue
    for (ext, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(ext.clone());
        }
    }

    // Process queue
    while let Some(node) = queue.pop_front() {
        order.push(node.clone());

        if let Some(deps) = self.graph.get(&node) {
            for dep in deps {
                let degree = in_degree.get_mut(dep).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(dep.clone());
                }
            }
        }
    }

    // Check for cycles
    if order.len() != self.graph.len() {
        bail!("Circular dependency detected");
    }

    Ok(order)
}
```

**Pros**:
- No recursion (avoids stack overflow)
- Easy to understand (BFS pattern)
- Efficient cycle detection

**Cons**:
- More code (in-degree tracking, queue management)
- Less precise cycle detection (can't identify which node)
- Requires full graph traversal even for subset of extensions

**Rejected**: DFS is more intuitive and provides better error messages.

### 2. petgraph as Primary Implementation

**Description**: Use `petgraph::algo::toposort` as the primary algorithm instead of manual DFS.

**Pros**:
- Fewer lines of code
- Battle-tested library
- Access to graph algorithms (visualization, analysis)

**Cons**:
- Adds ~200KB to binary size
- Less control over error messages
- Dependency on external crate
- Harder to customize behavior

**Rejected**: Manual DFS provides better control and smaller binary. petgraph available as optional feature.

### 3. No Cycle Detection (Fail at Runtime)

**Description**: Skip cycle detection, let installation fail when hitting circular dependency.

**Pros**:
- Simpler implementation
- No visiting set needed
- Faster resolution

**Cons**:
- Poor user experience (fails during installation)
- Could cause infinite loops
- No clear error messages
- Doesn't follow fail-fast principle

**Rejected**: Cycle detection is critical for robust dependency management.

### 4. Parallel Installation of Independent Extensions

**Description**: Identify independent extensions in topological sort, install them in parallel.

**Pros**:
- Faster installation (e.g., install nodejs and python simultaneously)
- Better utilization of resources

**Cons**:
- Much more complex implementation
- Requires async runtime and synchronization
- Error handling becomes complex (partial failures)
- Marginal benefit (most extensions install quickly)

**Rejected**: Added complexity not worth marginal performance gain. Can be added in future if needed.

## Compliance

- ✅ O(V + E) time complexity (linear in graph size)
- ✅ O(V) space complexity
- ✅ Detects all cycles with clear error messages
- ✅ Handles diamond dependencies correctly
- ✅ Supports partial graph resolution
- ✅ 100% test coverage for dependency scenarios
- ✅ petgraph integration as optional feature

## Notes

The DFS algorithm is classical computer science, but the implementation details matter:
- Using `visiting` set (not just a boolean) enables clear cycle detection
- Post-order traversal (adding to order after visiting children) ensures correct topological order
- Checking registry membership catches missing dependencies early

The algorithm is deterministic given a fixed iteration order over dependencies, but multiple valid topological orders exist. We could add alphabetical sorting of dependencies for reproducibility across runs.

Future enhancement: Add `--dry-run` flag to print resolved dependency order without installation.

## Related Decisions

- [ADR-008: Extension Type System](008-extension-type-system-yaml-deserialization.md) - Extension structure
- [ADR-010: GitHub-based Distribution](010-github-extension-distribution.md) - Registry source
- [ADR-011: Multi-Method Installation](011-multi-method-extension-installation.md) - Uses resolved order
- [ADR-012: Registry and Manifest Architecture](012-registry-manifest-dual-state-architecture.md) - Dependency graph source
