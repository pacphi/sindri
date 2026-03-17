//! Configuration loading and management

mod hierarchical_loader;
pub mod loader;

pub use hierarchical_loader::HierarchicalConfigLoader;
pub use loader::{
    default_image_registry, generate_config, generate_default_config, ImageVersionResolver,
    SindriConfig, DEFAULT_IMAGE_NAME, DEFAULT_REGISTRY_HOST, DEFAULT_REGISTRY_OWNER,
};
