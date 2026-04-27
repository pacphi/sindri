use sindri_core::registry::ComponentEntry;
use std::collections::HashMap;

/// Find the dependency path from a collection to a target component (like `npm why`)
pub fn explain_path(
    target: &str,
    root: &str,
    registry: &HashMap<String, ComponentEntry>,
) -> Option<Vec<String>> {
    let mut path = vec![root.to_string()];
    if find_path(root, target, registry, &mut path) {
        Some(path)
    } else {
        None
    }
}

fn find_path(
    current: &str,
    target: &str,
    registry: &HashMap<String, ComponentEntry>,
    path: &mut Vec<String>,
) -> bool {
    if current == target {
        return true;
    }

    let entry = match registry.get(current) {
        Some(e) => e,
        None => return false,
    };

    for dep in &entry.depends_on {
        path.push(dep.clone());
        if find_path(dep, target, registry, path) {
            return true;
        }
        path.pop();
    }

    false
}

pub fn render_explain(path: &[String]) -> String {
    if path.is_empty() {
        return "No path found".to_string();
    }

    let mut lines = Vec::new();
    for (i, node) in path.iter().enumerate() {
        if i == 0 {
            lines.push(node.to_string());
        } else {
            lines.push(format!("{}└─ depends on: {}", "  ".repeat(i - 1), node));
        }
    }
    lines.join("\n")
}
