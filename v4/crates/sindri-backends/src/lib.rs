#![allow(dead_code)]

pub mod binary;
pub mod brew;
pub mod cargo;
pub mod error;
pub mod go_install;
pub mod mise;
pub mod npm;
pub mod pipx;
pub mod registry;
pub mod script;
pub mod sdkman;
pub mod system_pm;
pub mod traits;
pub mod winget;

pub use error::BackendError;
pub use registry::{backend_for, install_component};
pub use traits::InstallBackend;
