//! Cloud target implementations (ADR-017).
//!
//! Each submodule provides a `Target` impl for one cloud kind. The first
//! three (`e2b`, `fly`, `k8s`) landed in PR #210. Wave 3C adds `runpod`,
//! `northflank`, the seven `devpod-*` variants, and `wsl`.
//!
//! Where appropriate the impls shell out to the upstream CLI (`flyctl`,
//! `kubectl`, `devpod`, `wsl`) — the audit explicitly endorses this for
//! Wave 3C; native API integrations come later.

pub mod devpod;
pub mod e2b;
pub mod fly;
pub mod k8s;
pub mod northflank;
pub mod runpod;
pub mod wsl;

pub use devpod::{DevPodKind, DevPodTarget};
pub use e2b::E2bTarget;
pub use fly::FlyTarget;
pub use k8s::KubernetesTarget;
pub use northflank::NorthflankTarget;
pub use runpod::RunPodTarget;
pub use wsl::WslTarget;
