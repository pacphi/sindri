//! Plugin-protocol RPC client (ADR-019) — auth capability extension
//! (ADR-027 §"Plugin protocol extension", Phase 4 of the auth-aware plan).
//!
//! ADR-019 defines a thin JSON-RPC-shaped protocol for out-of-process target
//! plugins; this module ships the *Phase 4* extension to that protocol — a
//! single new method:
//!
//! ```jsonc
//! // CLI → plugin
//! {"method": "auth_capabilities", "params": {}}
//! // plugin → CLI
//! {"result": {"capabilities": [ /* AuthCapability JSON */ ]}}
//! ```
//!
//! Plugins that do **not** implement the verb return
//! `{"error": {"code": "method-not-supported"}}`. The client treats this
//! exactly the same as the [`Target::auth_capabilities`] trait default —
//! an empty `Vec` — so old plugins keep working.
//!
//! The full plugin transport (process spawn, stdio framing, version
//! negotiation) lives in the `sindri-extensions` crate today. This module
//! only models the request/response shape and the dispatcher contract; tests
//! exercise the contract via a `PluginTransport` test double.

use sindri_core::auth::AuthCapability;

/// Error code returned by plugins that don't implement a method (ADR-019).
pub const METHOD_NOT_SUPPORTED: &str = "method-not-supported";

/// JSON-RPC-shaped error a plugin can return for any method (ADR-019 §3).
///
/// Kept as a flat struct rather than an enum so unknown error codes round-
/// trip without information loss — plugins author error codes freely and the
/// CLI surfaces them verbatim to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRpcError {
    /// Machine-readable error code (e.g. `method-not-supported`,
    /// `transport-error`).
    pub code: String,
    /// Human-readable detail.
    pub message: String,
}

impl PluginRpcError {
    /// True if this error indicates the plugin simply doesn't implement the
    /// requested verb. The `auth_capabilities` client treats this as an
    /// empty result, not a failure.
    pub fn is_method_not_supported(&self) -> bool {
        self.code == METHOD_NOT_SUPPORTED
    }
}

/// Transport abstraction over the ADR-019 RPC channel.
///
/// Real implementations live in `sindri-extensions` and speak JSON over the
/// plugin's stdio. This trait exists so the auth-capability dispatch logic
/// can be unit-tested without spawning subprocesses.
pub trait PluginTransport {
    /// Invoke `method` with the given JSON `params`. Returns the plugin's
    /// JSON `result` on success, or a [`PluginRpcError`] on failure.
    fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, PluginRpcError>;
}

/// Fetch the [`AuthCapability`] list from a plugin via [`PluginTransport`].
///
/// Behaviour, per ADR-027 §"Plugin protocol extension":
/// - On success: the JSON array under `result.capabilities` is decoded into
///   `Vec<AuthCapability>` and returned. Decode errors surface as
///   `Err(_)` so the operator sees a precise diagnostic — they do not get
///   silently demoted to "no capabilities".
/// - On `method-not-supported` (the plugin simply doesn't implement the
///   verb): returns `Ok(Vec::new())`. This is the soft-fallback case
///   that lets old plugins keep working unchanged.
/// - On any other transport error: returns `Err(_)`.
pub fn fetch_auth_capabilities<T: PluginTransport + ?Sized>(
    transport: &T,
) -> Result<Vec<AuthCapability>, PluginRpcError> {
    match transport.call("auth_capabilities", serde_json::json!({})) {
        Ok(result) => {
            let caps = result
                .get("capabilities")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
            serde_json::from_value::<Vec<AuthCapability>>(caps).map_err(|e| PluginRpcError {
                code: "decode-error".to_string(),
                message: format!("invalid auth_capabilities response: {}", e),
            })
        }
        Err(e) if e.is_method_not_supported() => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::auth::AuthSource;
    use std::cell::RefCell;

    /// Test double: records the last method invoked and returns a
    /// pre-canned response.
    struct MockTransport {
        response: RefCell<Result<serde_json::Value, PluginRpcError>>,
        last_method: RefCell<Option<String>>,
    }

    impl MockTransport {
        fn ok(value: serde_json::Value) -> Self {
            Self {
                response: RefCell::new(Ok(value)),
                last_method: RefCell::new(None),
            }
        }

        fn err(e: PluginRpcError) -> Self {
            Self {
                response: RefCell::new(Err(e)),
                last_method: RefCell::new(None),
            }
        }
    }

    impl PluginTransport for MockTransport {
        fn call(
            &self,
            method: &str,
            _params: serde_json::Value,
        ) -> Result<serde_json::Value, PluginRpcError> {
            *self.last_method.borrow_mut() = Some(method.to_string());
            self.response.borrow().clone()
        }
    }

    #[test]
    fn method_not_supported_yields_empty_vec() {
        let t = MockTransport::err(PluginRpcError {
            code: METHOD_NOT_SUPPORTED.to_string(),
            message: "unimplemented".to_string(),
        });
        let caps = fetch_auth_capabilities(&t).unwrap();
        assert!(caps.is_empty());
        assert_eq!(t.last_method.borrow().as_deref(), Some("auth_capabilities"));
    }

    #[test]
    fn implemented_returns_decoded_caps() {
        let t = MockTransport::ok(serde_json::json!({
            "capabilities": [
                {
                    "id": "github_token",
                    "audience": "https://api.github.com",
                    "source": { "kind": "from-env", "var": "GITHUB_TOKEN" },
                    "priority": 25
                }
            ]
        }));
        let caps = fetch_auth_capabilities(&t).unwrap();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].id, "github_token");
        assert_eq!(caps[0].priority, 25);
        match &caps[0].source {
            AuthSource::FromEnv { var } => assert_eq!(var, "GITHUB_TOKEN"),
            other => panic!("expected FromEnv, got {:?}", other),
        }
    }

    #[test]
    fn missing_capabilities_field_decodes_as_empty() {
        // Plugin returned an empty object — treat as no capabilities.
        let t = MockTransport::ok(serde_json::json!({}));
        let caps = fetch_auth_capabilities(&t).unwrap();
        assert!(caps.is_empty());
    }

    #[test]
    fn malformed_capability_surfaces_decode_error() {
        // Invalid AuthSource discriminant — must error, not silently empty.
        let t = MockTransport::ok(serde_json::json!({
            "capabilities": [
                {
                    "id": "bad",
                    "audience": "x",
                    "source": { "kind": "this-kind-does-not-exist" },
                    "priority": 0
                }
            ]
        }));
        let err = fetch_auth_capabilities(&t).unwrap_err();
        assert_eq!(err.code, "decode-error");
        assert!(err.message.contains("auth_capabilities"));
    }

    #[test]
    fn arbitrary_transport_error_propagates() {
        let t = MockTransport::err(PluginRpcError {
            code: "transport-broken".to_string(),
            message: "pipe closed".to_string(),
        });
        let err = fetch_auth_capabilities(&t).unwrap_err();
        assert_eq!(err.code, "transport-broken");
    }
}
