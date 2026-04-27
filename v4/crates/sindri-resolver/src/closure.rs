use crate::error::ResolverError;
use sindri_core::component::ComponentId;
use sindri_core::registry::ComponentEntry;
use std::collections::{HashMap, HashSet, VecDeque};

/// A node in the expanded dependency closure
#[derive(Debug, Clone)]
pub struct ClosureNode {
    pub id: ComponentId,
    pub entry: ComponentEntry,
    pub depth: usize,
}

/// Expand a set of root component addresses into a full dependency closure.
/// Uses BFS with cycle detection (ADR-003, Sprint 3).
pub fn expand_closure(
    roots: &[String],
    registry: &HashMap<String, ComponentEntry>,
) -> Result<Vec<ClosureNode>, ResolverError> {
    let mut visited: HashMap<String, usize> = HashMap::new(); // address -> depth
    let mut in_stack: HashSet<String> = HashSet::new();
    let mut result: Vec<ClosureNode> = Vec::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();

    for root in roots {
        queue.push_back((root.clone(), 0));
    }

    while let Some((address, depth)) = queue.pop_front() {
        if visited.contains_key(&address) {
            continue; // already expanded
        }

        if in_stack.contains(&address) {
            return Err(ResolverError::CycleDetected(format!(
                "Circular dependency detected: {}",
                address
            )));
        }

        let (backend, name) = parse_address(&address)?;
        let entry = registry.get(&address).ok_or_else(|| {
            ResolverError::NotFound(format!("Component '{}' not found in registry", address))
        })?;

        let id = ComponentId {
            backend: backend.clone(),
            name,
        };

        visited.insert(address.clone(), depth);
        in_stack.insert(address.clone());

        // Queue dependencies
        for dep_addr in &entry.depends_on {
            if !visited.contains_key(dep_addr) {
                queue.push_back((dep_addr.clone(), depth + 1));
            }
        }

        result.push(ClosureNode {
            id,
            entry: entry.clone(),
            depth,
        });
        in_stack.remove(&address);
    }

    // Check for version conflicts: same backend:name appearing with different versions
    let mut seen: HashMap<String, String> = HashMap::new();
    for node in &result {
        let key = node.id.to_address();
        let ver = &node.entry.latest;
        if let Some(existing) = seen.get(&key) {
            if existing != ver {
                return Err(ResolverError::VersionConflict(format!(
                    "Conflicting versions for {}: {} vs {}",
                    key, existing, ver
                )));
            }
        } else {
            seen.insert(key, ver.clone());
        }
    }

    Ok(result)
}

fn parse_address(
    address: &str,
) -> Result<(sindri_core::component::Backend, String), ResolverError> {
    let id = ComponentId::parse(address).ok_or_else(|| {
        ResolverError::NotFound(format!("Invalid component address: {}", address))
    })?;
    Ok((id.backend, id.name))
}
