// Path resolution and security validation for configure processing

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Resolves and validates paths for configure operations
pub struct PathResolver {
    extension_dir: PathBuf,
    home_dir: PathBuf,
}

impl PathResolver {
    /// Create a new PathResolver
    pub fn new(extension_dir: PathBuf, home_dir: PathBuf) -> Self {
        Self {
            extension_dir,
            home_dir,
        }
    }

    /// Resolve and validate a source path (must be within extension directory)
    pub async fn resolve_source(&self, extension_name: &str, source: &str) -> Result<PathBuf> {
        // Check for path traversal in the input before any processing
        if source.contains("..") {
            bail!(
                "Source path contains parent directory (..) components: {}",
                source
            );
        }

        // Expand variables in the source path
        let expanded = self.expand_path(source, extension_name)?;

        // Check again after expansion
        if expanded.contains("..") {
            bail!(
                "Source path contains parent directory (..) components: {}",
                expanded
            );
        }

        // Resolve relative to extension directory
        let source_path = self.extension_dir.join(&expanded);

        // Validate the source path
        self.validate_source_path(&source_path, &self.extension_dir)?;

        Ok(source_path)
    }

    /// Resolve and validate a destination path (typically in user's home directory)
    pub async fn resolve_destination(&self, destination: &str) -> Result<PathBuf> {
        // Handle tilde expansion first (before shellexpand)
        let path_to_expand = if destination.starts_with('~') {
            let without_tilde = destination
                .strip_prefix("~/")
                .unwrap_or(destination.strip_prefix('~').unwrap_or(destination));
            format!("{}/{}", self.home_dir.display(), without_tilde)
        } else {
            destination.to_string()
        };

        // Expand environment variables (but not tilde, which we already handled)
        let expanded = self.expand_path(&path_to_expand, "")?;

        // Convert to absolute path
        let dest_path = if Path::new(&expanded).is_absolute() {
            PathBuf::from(expanded)
        } else {
            // Relative paths are resolved from home directory
            self.home_dir.join(expanded)
        };

        // Validate the destination path
        self.validate_destination_path(&dest_path)?;

        Ok(dest_path)
    }

    /// Expand variables in a path string
    fn expand_path(&self, path: &str, extension_name: &str) -> Result<String> {
        // Replace extension name placeholder
        let mut path = path.replace("${EXTENSION_NAME}", extension_name);

        // Only expand environment variables, not tilde (tilde is handled separately)
        // Use shellexpand::env instead of shellexpand::full to avoid tilde expansion
        if path.contains('$') {
            path = shellexpand::env(&path)
                .context("Failed to expand environment variables")?
                .to_string();
        }

        Ok(path)
    }

    /// Validate that a source path is safe and within the extension directory
    fn validate_source_path(&self, path: &Path, extension_dir: &Path) -> Result<()> {
        // Check for path traversal attempts (..) in the path string
        // Do this before attempting canonicalization since the file might not exist yet
        let path_str = path.to_string_lossy();
        if path_str.contains("..") {
            bail!(
                "Source path contains parent directory (..) components: {:?}",
                path
            );
        }

        // Check if path components contain parent directory references
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            bail!(
                "Source path contains parent directory (..) components: {:?}",
                path
            );
        }

        // Only canonicalize if the path exists
        if path.exists() {
            let canonical = path
                .canonicalize()
                .with_context(|| format!("Failed to canonicalize source path: {:?}", path))?;

            // Canonicalize extension directory for comparison
            let canonical_ext_dir = extension_dir.canonicalize().with_context(|| {
                format!(
                    "Failed to canonicalize extension directory: {:?}",
                    extension_dir
                )
            })?;

            // Ensure the path is within the extension directory
            if !canonical.starts_with(&canonical_ext_dir) {
                bail!(
                    "Source path {:?} is outside extension directory {:?}",
                    canonical,
                    canonical_ext_dir
                );
            }
        } else {
            // For non-existent paths, do a simple prefix check
            if !path.starts_with(extension_dir) {
                bail!(
                    "Source path {:?} is outside extension directory {:?}",
                    path,
                    extension_dir
                );
            }
        }

        Ok(())
    }

    /// Validate that a destination path is safe
    fn validate_destination_path(&self, path: &Path) -> Result<()> {
        // Check for path traversal attempts (..) in the path string
        let path_str = path.to_string_lossy();
        if path_str.contains("..") {
            bail!(
                "Destination path contains parent directory (..) components: {:?}",
                path
            );
        }

        // Check if path components contain parent directory references
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            bail!(
                "Destination path contains parent directory (..) components: {:?}",
                path
            );
        }

        // List of protected system paths that should not be modified
        let protected_paths = [
            "/etc/passwd",
            "/etc/shadow",
            "/etc/group",
            "/etc/sudoers",
            "/bin",
            "/sbin",
            "/usr/bin",
            "/usr/sbin",
            "/boot",
            "/sys",
            "/proc",
        ];

        // Normalize path for comparison (resolve to absolute if possible)
        let check_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            // For relative paths, we already resolved them to home_dir in resolve_destination
            path.to_path_buf()
        };

        // Check if the path or any parent is a protected path
        for protected in &protected_paths {
            let protected_path = Path::new(protected);
            if check_path == protected_path || check_path.starts_with(protected_path) {
                bail!("Destination path {:?} is a protected system path", path);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_resolve_source_basic() {
        let temp = TempDir::new().unwrap();
        let ext_dir = temp.path().join("extensions/test");
        fs::create_dir_all(&ext_dir).unwrap();

        // Create a test file
        let test_file = ext_dir.join("template.txt");
        fs::write(&test_file, "content").unwrap();

        let resolver = PathResolver::new(ext_dir.clone(), temp.path().to_path_buf());
        let result = resolver.resolve_source("test", "template.txt").await;
        let resolved = result.expect("resolve_source should succeed for existing file");
        assert_eq!(resolved, test_file);
    }

    #[tokio::test]
    async fn test_resolve_source_path_traversal() {
        let temp = TempDir::new().unwrap();
        let ext_dir = temp.path().join("extensions/test");
        fs::create_dir_all(&ext_dir).unwrap();

        let resolver = PathResolver::new(ext_dir, temp.path().to_path_buf());
        let result = resolver.resolve_source("test", "../../../etc/passwd").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parent directory"));
    }

    #[tokio::test]
    async fn test_resolve_destination_tilde_expansion() {
        let temp = TempDir::new().unwrap();
        let home_dir = temp.path();

        let resolver = PathResolver::new(temp.path().to_path_buf(), home_dir.to_path_buf());
        let result = resolver.resolve_destination("~/.bashrc").await;
        let resolved = result.expect("resolve_destination with tilde should succeed");
        assert_eq!(resolved, home_dir.join(".bashrc"));
    }

    #[tokio::test]
    async fn test_resolve_destination_protected_path() {
        let temp = TempDir::new().unwrap();

        let resolver = PathResolver::new(temp.path().to_path_buf(), temp.path().to_path_buf());
        let result = resolver.resolve_destination("/etc/passwd").await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("protected system path"));
    }

    #[tokio::test]
    async fn test_expand_path_extension_name() {
        let temp = TempDir::new().unwrap();
        let resolver = PathResolver::new(temp.path().to_path_buf(), temp.path().to_path_buf());

        let result = resolver.expand_path("config-${EXTENSION_NAME}.yaml", "myext");
        let expanded = result.expect("expand_path with EXTENSION_NAME should succeed");
        assert_eq!(expanded, "config-myext.yaml");
    }

    #[test]
    fn test_validate_destination_path_traversal() {
        let temp = TempDir::new().unwrap();
        let resolver = PathResolver::new(temp.path().to_path_buf(), temp.path().to_path_buf());

        let path = PathBuf::from("some/path/../../etc/passwd");
        let result = resolver.validate_destination_path(&path);

        assert!(result.is_err());
    }
}
