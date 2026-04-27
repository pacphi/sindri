use sindri_core::registry::ComponentEntry;
use std::collections::HashMap;

/// Render a dependency graph as a text tree (Sprint 8)
pub fn render_tree(
    root: &str,
    registry: &HashMap<String, ComponentEntry>,
    reverse: bool,
) -> String {
    if reverse {
        render_reverse_tree(root, registry)
    } else {
        let mut out = String::new();
        render_node(root, registry, 0, &mut Vec::new(), &mut out);
        out
    }
}

fn render_node(
    addr: &str,
    registry: &HashMap<String, ComponentEntry>,
    depth: usize,
    visited: &mut Vec<String>,
    out: &mut String,
) {
    let prefix = if depth == 0 {
        String::new()
    } else {
        format!("{}├─ ", "│  ".repeat(depth - 1))
    };

    out.push_str(&format!("{}{}\n", prefix, addr));

    if visited.contains(&addr.to_string()) {
        out.push_str(&format!("{}│  (circular)\n", "│  ".repeat(depth)));
        return;
    }

    visited.push(addr.to_string());

    if let Some(entry) = registry.get(addr) {
        let deps = &entry.depends_on;
        for dep in deps.iter() {
            render_node(dep, registry, depth + 1, visited, out);
        }
    }

    visited.pop();
}

fn render_reverse_tree(target: &str, registry: &HashMap<String, ComponentEntry>) -> String {
    let dependents: Vec<&str> = registry
        .iter()
        .filter(|(_, e)| e.depends_on.iter().any(|d| d == target))
        .map(|(addr, _)| addr.as_str())
        .collect();

    let mut out = format!("{} (required by)\n", target);
    for dep in dependents {
        out.push_str(&format!("  └─ {}\n", dep));
    }
    out
}

/// Render a Mermaid graph diagram
pub fn render_mermaid(roots: &[&str], registry: &HashMap<String, ComponentEntry>) -> String {
    let mut lines = vec!["graph TD".to_string()];
    let mut visited = std::collections::HashSet::new();

    for root in roots {
        render_mermaid_node(root, registry, &mut visited, &mut lines);
    }

    lines.join("\n")
}

fn render_mermaid_node(
    addr: &str,
    registry: &HashMap<String, ComponentEntry>,
    visited: &mut std::collections::HashSet<String>,
    lines: &mut Vec<String>,
) {
    if visited.contains(addr) {
        return;
    }
    visited.insert(addr.to_string());

    if let Some(entry) = registry.get(addr) {
        for dep in &entry.depends_on {
            let safe_addr = addr.replace([':', '-'], "_");
            let safe_dep = dep.replace([':', '-'], "_");
            lines.push(format!("    {} --> {}", safe_addr, safe_dep));
            render_mermaid_node(dep, registry, visited, lines);
        }
    }
}
