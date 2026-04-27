#![allow(dead_code)]

pub mod auth;
pub mod cloud;
pub mod docker;
pub mod error;
pub mod local;
pub mod ssh;
pub mod traits;

pub use auth::AuthValue;
pub use docker::DockerTarget;
pub use error::TargetError;
pub use local::LocalTarget;
pub use ssh::SshTarget;
pub use traits::{PrereqCheck, Target};
