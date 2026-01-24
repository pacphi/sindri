//! Configuration loading and management

mod hierarchical_loader;
mod loader;

pub use hierarchical_loader::HierarchicalConfigLoader;
pub use loader::{generate_config, generate_default_config, SindriConfig};
