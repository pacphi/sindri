#![allow(dead_code)]

pub mod binary;
pub mod brew;
pub mod error;
pub mod mise;
pub mod npm;
pub mod registry;
pub mod script;
pub mod sdkman;
pub mod system_pm;
pub mod traits;
pub mod universal;
pub mod winget;

pub use error::BackendError;
pub use traits::InstallBackend;
pub use registry::{backend_for, install_component};
