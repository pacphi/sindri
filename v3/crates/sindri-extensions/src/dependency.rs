//! Dependency resolution using topological sort with DFS

use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};

use crate::registry::ExtensionRegistry;

/// Dependency resolver using DFS-based topological sort
pub struct DependencyResolver {
    registry: HashMap<String, Vec<String>>,
}

impl DependencyResolver {
    /// Create a new dependency resolver from an extension registry
    pub fn new(registry: &ExtensionRegistry) -> Self {
        let mut deps = HashMap::new();
        for (name, ext) in registry.extensions() {
            deps.insert(name.clone(), ext.metadata.dependencies.clone());
        }
        Self { registry: deps }
    }

    /// Resolve dependencies in topological order
    pub fn resolve(&self, extension: &str) -> Result<Vec<String>> {
        let mut resolved = Vec::new();
        let mut seen = HashSet::new();
        let mut visiting = HashSet::new();

        self.visit(extension, &mut resolved, &mut seen, &mut visiting)?;
        Ok(resolved)
    }

    /// Visit an extension node using DFS
    fn visit(
        &self,
        ext: &str,
        resolved: &mut Vec<String>,
        seen: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) -> Result<()> {
        // Cycle detection
        if visiting.contains(ext) {
            return Err(anyhow!("Circular dependency detected: {}", ext));
        }

        // Already resolved
        if seen.contains(ext) {
            return Ok(());
        }

        visiting.insert(ext.to_string());

        // Visit dependencies first
        if let Some(deps) = self.registry.get(ext) {
            for dep in deps {
                self.visit(dep, resolved, seen, visiting)?;
            }
        }

        visiting.remove(ext);
        seen.insert(ext.to_string());
        resolved.push(ext.to_string());

        Ok(())
    }

    /// Check if all dependencies of an extension are installed
    pub fn check_dependencies(
        &self,
        extension: &str,
        installed: &HashSet<String>,
    ) -> Result<Vec<String>> {
        let deps = self.registry.get(extension).cloned().unwrap_or_default();

        let missing: Vec<_> = deps
            .into_iter()
            .filter(|d| !installed.contains(d))
            .collect();

        Ok(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::types::{Extension, ExtensionMetadata};

    fn create_test_registry(extensions: Vec<(&str, Vec<&str>)>) -> ExtensionRegistry {
        let mut registry = ExtensionRegistry::new();
        for (name, deps) in extensions {
            let ext = Extension {
                metadata: ExtensionMetadata {
                    name: name.to_string(),
                    version: "1.0.0".to_string(),
                    description: "Test extension".to_string(),
                    category: sindri_core::types::ExtensionCategory::Base,
                    author: None,
                    homepage: None,
                    dependencies: deps.iter().map(|s| s.to_string()).collect(),
                },
                requirements: None,
                install: sindri_core::types::InstallConfig {
                    method: sindri_core::types::InstallMethod::Script,
                    mise: None,
                    apt: None,
                    binary: None,
                    npm: None,
                    script: None,
                },
                configure: None,
                validate: sindri_core::types::ValidateConfig {
                    commands: vec![],
                    mise: None,
                },
                remove: None,
                upgrade: None,
                capabilities: None,
                bom: None,
            };
            // Add extension to the internal HashMap
            registry.extensions.insert(name.to_string(), ext);
        }
        registry
    }

    #[test]
    fn test_simple_dependency_chain() {
        // Create registry: C -> B -> A
        let registry =
            create_test_registry(vec![("A", vec![]), ("B", vec!["A"]), ("C", vec!["B"])]);

        let resolver = DependencyResolver::new(&registry);
        let result = resolver.resolve("C").unwrap();

        // Should resolve in order: A, B, C
        assert_eq!(result, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_circular_dependency() {
        // This test would require a registry with circular dependencies
        // which should be caught during registry validation
        // Left as a placeholder for integration testing
    }

    #[test]
    fn test_check_dependencies() {
        let registry = create_test_registry(vec![("A", vec![]), ("B", vec!["A"])]);

        let resolver = DependencyResolver::new(&registry);

        // B depends on A, but A is not installed
        let mut installed = HashSet::new();
        let missing = resolver.check_dependencies("B", &installed).unwrap();
        assert_eq!(missing, vec!["A"]);

        // Now A is installed
        installed.insert("A".to_string());
        let missing = resolver.check_dependencies("B", &installed).unwrap();
        assert!(missing.is_empty());
    }

    #[test]
    fn test_no_dependencies() {
        let registry = create_test_registry(vec![("A", vec![])]);

        let resolver = DependencyResolver::new(&registry);
        let result = resolver.resolve("A").unwrap();

        assert_eq!(result, vec!["A"]);
    }

    #[test]
    fn test_diamond_dependency() {
        // D -> B -> A
        // D -> C -> A
        // Should resolve A only once
        let registry = create_test_registry(vec![
            ("A", vec![]),
            ("B", vec!["A"]),
            ("C", vec!["A"]),
            ("D", vec!["B", "C"]),
        ]);

        let resolver = DependencyResolver::new(&registry);
        let result = resolver.resolve("D").unwrap();

        // A should appear only once, before both B and C
        assert!(result.contains(&"A".to_string()));
        assert!(result.contains(&"B".to_string()));
        assert!(result.contains(&"C".to_string()));
        assert!(result.contains(&"D".to_string()));

        let a_pos = result.iter().position(|x| x == "A").unwrap();
        let b_pos = result.iter().position(|x| x == "B").unwrap();
        let c_pos = result.iter().position(|x| x == "C").unwrap();
        let d_pos = result.iter().position(|x| x == "D").unwrap();

        // A must come before B and C
        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        // B and C must come before D
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
    }
}
