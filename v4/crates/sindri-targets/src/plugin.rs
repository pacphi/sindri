//! Subprocess-JSON target plugin protocol (ADR-019).
//!
//! A target plugin is a stand-alone executable installed at
//! `~/.sindri/plugins/<kind>/sindri-target-<kind>`. The host (sindri CLI)
//! drives the plugin over stdin/stdout: one JSON line in, one JSON line
//! out. The plugin prints a one-line handshake on startup which the host
//! validates before sending any request:
//!
//! ```text
//! {"protocol":"sindri-target-plugin","version":1}
//! ```
//!
//! After the handshake the host writes a single `PluginRequest` JSON
//! object terminated by `\n`, then reads a single `PluginResponse` JSON
//! object terminated by `\n`. The plugin exits when its stdin closes.
//!
//! The wire format intentionally avoids streaming or pipelining — Wave 3C
//! is about correctness; throughput optimisation is a future concern.
//!
//! ## ADR-027 §"Plugin protocol extension" (Phase 4)
//!
//! Phase 4 of the auth-aware plan adds a single new RPC verb:
//! [`PluginRequest::AuthCapabilities`] with paired
//! [`PluginResponse::AuthCapabilities`]. Plugins that don't implement the
//! verb return an `Error { kind: "method-not-supported", .. }` payload —
//! the host helper [`fetch_auth_capabilities`] treats that as an empty
//! `Vec` so old plugins continue to work unchanged.
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use serde::{Deserialize, Serialize};
use sindri_core::auth::AuthCapability;
use sindri_core::platform::TargetProfile;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Error kind returned by plugins that don't implement a method (ADR-019).
///
/// The host helper [`fetch_auth_capabilities`] recognises this value and
/// degrades gracefully to an empty capability list so legacy plugins keep
/// working without the new verb.
pub const METHOD_NOT_SUPPORTED: &str = "method-not-supported";

/// The protocol identifier emitted in every plugin handshake.
pub const PROTOCOL_ID: &str = "sindri-target-plugin";
/// The current wire-protocol version. Bump when a change is incompatible.
pub const PROTOCOL_VERSION: u32 = 1;

/// A request from the host to a plugin. Encoded as a single JSON line.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum PluginRequest {
    /// Ask the plugin for its target profile.
    Profile,
    /// Execute a shell command on the target.
    Exec {
        cmd: String,
        env: Vec<(String, String)>,
    },
    /// Upload a local file.
    Upload { local: PathBuf, remote: String },
    /// Download a remote file.
    Download { remote: String, local: PathBuf },
    /// Provision the target.
    Create,
    /// Destroy the target.
    Destroy,
    /// Run prerequisite checks.
    CheckPrerequisites,
    /// Ask the plugin for the [`AuthCapability`] list it advertises
    /// (ADR-027 §"Plugin protocol extension", Phase 4). Plugins that do
    /// not implement this verb must reply with
    /// `Error { kind: "method-not-supported", .. }` — the host treats
    /// that as an empty list.
    AuthCapabilities,
}

/// A response from a plugin. Encoded as a single JSON line.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum PluginResponse {
    /// Reply to `Profile`.
    Profile { profile: TargetProfile },
    /// Reply to `Exec`.
    Exec {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    /// Generic success for void requests.
    Ok,
    /// Structured error.
    Error {
        kind: String,
        message: String,
        suggested_fix: Option<String>,
    },
    /// Reply to `CheckPrerequisites`.
    PrereqList { checks: Vec<WirePrereqCheck> },
    /// Reply to [`PluginRequest::AuthCapabilities`] (ADR-027 §Phase 4).
    AuthCapabilities { capabilities: Vec<AuthCapability> },
}

/// Wire-format mirror of [`PrereqCheck`], with `Serialize`/`Deserialize`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WirePrereqCheck {
    pub name: String,
    pub passed: bool,
    pub fix: Option<String>,
}

impl From<WirePrereqCheck> for PrereqCheck {
    fn from(w: WirePrereqCheck) -> Self {
        PrereqCheck {
            name: w.name,
            passed: w.passed,
            fix: w.fix,
        }
    }
}

/// The handshake line a plugin emits on startup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Handshake {
    pub protocol: String,
    pub version: u32,
}

/// A subprocess-driven target plugin.
#[derive(Debug)]
pub struct PluginTarget {
    /// Local target name in sindri.yaml.
    pub name: String,
    /// Plugin kind (e.g. `modal`, `lambda-labs`). Used to locate the binary.
    pub kind: String,
    /// Path to the plugin executable.
    pub binary_path: PathBuf,
    /// Opaque plugin-specific config from `targets.<name>` in sindri.yaml.
    pub config: serde_json::Value,
}

impl PluginTarget {
    /// Construct a new plugin target.
    pub fn new(name: &str, kind: &str, binary_path: PathBuf, config: serde_json::Value) -> Self {
        PluginTarget {
            name: name.to_string(),
            kind: kind.to_string(),
            binary_path,
            config,
        }
    }

    /// Spawn the plugin, validate the handshake, dispatch one request,
    /// and return the parsed response. Stdin is closed after the request
    /// is sent so the plugin terminates cleanly.
    pub fn dispatch(&self, req: &PluginRequest) -> Result<PluginResponse, TargetError> {
        let mut child = Command::new(&self.binary_path)
            .env("SINDRI_TARGET_NAME", &self.name)
            .env("SINDRI_TARGET_KIND", &self.kind)
            .env("SINDRI_TARGET_CONFIG", self.config.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!(
                    "failed to spawn plugin {}: {}",
                    self.binary_path.display(),
                    e
                ),
            })?;

        let stdout = child.stdout.take().ok_or_else(|| TargetError::ExecFailed {
            target: self.name.clone(),
            detail: "plugin did not expose stdout".into(),
        })?;
        let mut reader = BufReader::new(stdout);

        // Handshake.
        let mut handshake_line = String::new();
        reader
            .read_line(&mut handshake_line)
            .map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!("failed to read plugin handshake: {}", e),
            })?;
        if handshake_line.trim().is_empty() {
            return Err(TargetError::ExecFailed {
                target: self.name.clone(),
                detail: "plugin closed stdout before handshake".into(),
            });
        }
        let hs: Handshake =
            serde_json::from_str(handshake_line.trim()).map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!(
                    "invalid plugin handshake: {} (line: {:?})",
                    e, handshake_line
                ),
            })?;
        if hs.protocol != PROTOCOL_ID {
            return Err(TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!("unexpected handshake protocol: {}", hs.protocol),
            });
        }
        if hs.version != PROTOCOL_VERSION {
            return Err(TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!(
                    "plugin protocol version {} does not match host version {}",
                    hs.version, PROTOCOL_VERSION
                ),
            });
        }

        // Send the request.
        {
            let mut stdin = child.stdin.take().ok_or_else(|| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: "plugin did not expose stdin".into(),
            })?;
            let line = serde_json::to_string(req).map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!("failed to encode plugin request: {}", e),
            })?;
            stdin
                .write_all(line.as_bytes())
                .and_then(|_| stdin.write_all(b"\n"))
                .map_err(|e| TargetError::ExecFailed {
                    target: self.name.clone(),
                    detail: format!("failed to write plugin request: {}", e),
                })?;
            // Drop stdin so the plugin can finish reading.
        }

        // Read the single response line.
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!("failed to read plugin response: {}", e),
            })?;
        let _ = child.wait();
        if response_line.trim().is_empty() {
            return Err(TargetError::ExecFailed {
                target: self.name.clone(),
                detail: "plugin produced no response".into(),
            });
        }
        let resp: PluginResponse =
            serde_json::from_str(response_line.trim()).map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!(
                    "malformed plugin response: {} (line: {:?})",
                    e, response_line
                ),
            })?;
        Ok(resp)
    }

    fn map_error(&self, resp: PluginResponse) -> TargetError {
        match resp {
            PluginResponse::Error {
                kind,
                message,
                suggested_fix,
            } => {
                let detail = match suggested_fix {
                    Some(fix) => format!("{}: {} (fix: {})", kind, message, fix),
                    None => format!("{}: {}", kind, message),
                };
                TargetError::ExecFailed {
                    target: self.name.clone(),
                    detail,
                }
            }
            other => TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!("unexpected plugin response: {:?}", other),
            },
        }
    }
}

impl Target for PluginTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        &self.kind
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        match self.dispatch(&PluginRequest::Profile)? {
            PluginResponse::Profile { profile } => Ok(profile),
            other => Err(self.map_error(other)),
        }
    }

    fn exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let req = PluginRequest::Exec {
            cmd: cmd.to_string(),
            env: env
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        };
        match self.dispatch(&req)? {
            PluginResponse::Exec {
                stdout,
                stderr,
                exit_code,
            } => {
                if exit_code == 0 {
                    Ok((stdout, stderr))
                } else {
                    Err(TargetError::ExecFailed {
                        target: self.name.clone(),
                        detail: format!("plugin exit code {}: {}", exit_code, stderr.trim()),
                    })
                }
            }
            other => Err(self.map_error(other)),
        }
    }

    fn upload(&self, local: &Path, remote: &str) -> Result<(), TargetError> {
        let req = PluginRequest::Upload {
            local: local.to_path_buf(),
            remote: remote.to_string(),
        };
        match self.dispatch(&req)? {
            PluginResponse::Ok => Ok(()),
            other => Err(self.map_error(other)),
        }
    }

    fn download(&self, remote: &str, local: &Path) -> Result<(), TargetError> {
        let req = PluginRequest::Download {
            remote: remote.to_string(),
            local: local.to_path_buf(),
        };
        match self.dispatch(&req)? {
            PluginResponse::Ok => Ok(()),
            other => Err(self.map_error(other)),
        }
    }

    fn create(&self) -> Result<(), TargetError> {
        match self.dispatch(&PluginRequest::Create)? {
            PluginResponse::Ok => Ok(()),
            other => Err(self.map_error(other)),
        }
    }

    fn destroy(&self) -> Result<(), TargetError> {
        match self.dispatch(&PluginRequest::Destroy)? {
            PluginResponse::Ok => Ok(()),
            other => Err(self.map_error(other)),
        }
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        match self.dispatch(&PluginRequest::CheckPrerequisites) {
            Ok(PluginResponse::PrereqList { checks }) => {
                checks.into_iter().map(PrereqCheck::from).collect()
            }
            Ok(other) => vec![PrereqCheck::fail(
                &format!("plugin '{}' protocol", self.kind),
                &format!("unexpected response: {:?}", other),
            )],
            Err(e) => vec![PrereqCheck::fail(
                &format!("plugin '{}' available", self.kind),
                &e.to_string(),
            )],
        }
    }

    /// Forward to the plugin's `AuthCapabilities` RPC. Plugins that don't
    /// implement the verb (returning `kind: "method-not-supported"`) are
    /// degraded to an empty list per ADR-027 §"Plugin protocol extension".
    /// Any other transport / protocol error degrades to empty as well so a
    /// flaky plugin can't break the resolver hot path; the error is logged
    /// at `warn` level for operators.
    fn auth_capabilities(&self) -> Vec<AuthCapability> {
        match self.dispatch(&PluginRequest::AuthCapabilities) {
            Ok(PluginResponse::AuthCapabilities { capabilities }) => capabilities,
            Ok(PluginResponse::Error { kind, .. }) if kind == METHOD_NOT_SUPPORTED => Vec::new(),
            Ok(other) => {
                tracing::warn!(
                    target: "sindri::plugin",
                    plugin = %self.kind,
                    "unexpected response to auth_capabilities: {:?}",
                    other
                );
                Vec::new()
            }
            Err(e) => {
                tracing::warn!(
                    target: "sindri::plugin",
                    plugin = %self.kind,
                    "auth_capabilities transport failed: {}",
                    e
                );
                Vec::new()
            }
        }
    }
}

// ─── Host-side helper (ADR-027 §"Plugin protocol extension", Phase 4) ──────

/// JSON-RPC-shaped error a [`PluginTransport`] can return for any method
/// (ADR-019 §3). Kept as a flat struct rather than an enum so unknown error
/// codes round-trip without information loss — plugins author error codes
/// freely and the CLI surfaces them verbatim to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRpcError {
    /// Machine-readable error code (e.g. `method-not-supported`,
    /// `transport-error`).
    pub code: String,
    /// Human-readable detail.
    pub message: String,
}

impl std::fmt::Display for PluginRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PluginRpcError {}

impl PluginRpcError {
    /// True if this error indicates the plugin simply doesn't implement
    /// the requested verb. The [`fetch_auth_capabilities`] helper treats
    /// this as an empty result, not a failure.
    pub fn is_method_not_supported(&self) -> bool {
        self.code == METHOD_NOT_SUPPORTED
    }
}

/// Transport abstraction over the ADR-019 RPC channel.
///
/// The production transport is the spawn-a-subprocess path implemented by
/// [`PluginTarget::dispatch`] above. This trait exists so the
/// auth-capability dispatch logic can be unit-tested without spawning
/// subprocesses, and so future transports (in-process, IPC) can be plugged
/// in without rewriting helpers.
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
/// - On success: the JSON array under `result.capabilities` is decoded
///   into `Vec<AuthCapability>` and returned. Decode errors surface as
///   `Err(_)` so the operator sees a precise diagnostic — they are NOT
///   silently demoted to "no capabilities".
/// - On `method-not-supported` (the plugin simply doesn't implement the
///   verb): returns `Ok(Vec::new())`. This is the soft-fallback case that
///   lets old plugins keep working unchanged.
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

/// Fetch an interactive credential prompt response from a remote target via
/// [`PluginTransport`] (Phase 2A — ADR-027 §6, plan §"Open Q2").
///
/// The CLI sends:
/// ```jsonc
/// {"method": "prompt_for_credential",
///  "params": {"prompt": "...", "secret": true, "timeout_secs": 60}}
/// ```
///
/// Plugins return:
/// ```jsonc
/// {"result": {"value": "<entered string>"}}
/// ```
///
/// Behaviour, per ADR-027 §"Plugin protocol extension":
/// - On success: returns `Ok(value)`. The value lives only in this call's
///   stack frame and is dropped by the redeemer caller after one
///   redemption pass — never persisted, never logged.
/// - On `method-not-supported`: returns
///   `Err(PluginRpcError{code: "method-not-supported", ...})` so callers
///   can fall back to local stdin (the [`Target::prompt_for_credential`]
///   trait default) or surface a precise diagnostic.
/// - On decode / transport error: returns `Err(_)`.
///
/// Note: unlike [`fetch_auth_capabilities`], we **don't** soften
/// `method-not-supported` to a default value here — the caller has to make
/// an explicit policy choice about how to behave, because a missing
/// `prompt_for_credential` in a remote target is a different failure mode
/// from an empty capability list.
pub fn prompt_for_credential_via_plugin<T: PluginTransport + ?Sized>(
    transport: &T,
    prompt: &str,
    secret: bool,
    timeout_secs: u64,
) -> Result<String, PluginRpcError> {
    let params = serde_json::json!({
        "prompt": prompt,
        "secret": secret,
        "timeout_secs": timeout_secs,
    });
    let result = transport.call("prompt_for_credential", params)?;
    let value = result
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PluginRpcError {
            code: "decode-error".to_string(),
            message: "missing string field `value` in prompt_for_credential response".to_string(),
        })?;
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::auth::AuthSource;
    use std::cell::RefCell;

    #[test]
    fn request_response_round_trip() {
        let cases: Vec<PluginRequest> = vec![
            PluginRequest::Profile,
            PluginRequest::Exec {
                cmd: "uname -a".into(),
                env: vec![("FOO".into(), "bar".into())],
            },
            PluginRequest::Upload {
                local: PathBuf::from("/tmp/a"),
                remote: "/tmp/b".into(),
            },
            PluginRequest::Download {
                remote: "/tmp/r".into(),
                local: PathBuf::from("/tmp/l"),
            },
            PluginRequest::Create,
            PluginRequest::Destroy,
            PluginRequest::CheckPrerequisites,
            PluginRequest::AuthCapabilities,
        ];
        for req in cases {
            let s = serde_json::to_string(&req).unwrap();
            let back: PluginRequest = serde_json::from_str(&s).unwrap();
            assert_eq!(req, back);
        }

        // Response variants are not Eq (TargetProfile is not Eq) but they
        // must round-trip through JSON without error.
        let resp = PluginResponse::Exec {
            stdout: "ok\n".into(),
            stderr: "".into(),
            exit_code: 0,
        };
        let s = serde_json::to_string(&resp).unwrap();
        let _: PluginResponse = serde_json::from_str(&s).unwrap();

        let err = PluginResponse::Error {
            kind: "auth".into(),
            message: "expired".into(),
            suggested_fix: Some("re-login".into()),
        };
        let s = serde_json::to_string(&err).unwrap();
        let _: PluginResponse = serde_json::from_str(&s).unwrap();
    }

    #[test]
    fn handshake_round_trip() {
        let h = Handshake {
            protocol: PROTOCOL_ID.into(),
            version: PROTOCOL_VERSION,
        };
        let s = serde_json::to_string(&h).unwrap();
        let back: Handshake = serde_json::from_str(&s).unwrap();
        assert_eq!(h, back);
    }

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

    #[test]
    fn prompt_for_credential_round_trips_value() {
        let t = MockTransport::ok(serde_json::json!({
            "value": "user-entered-secret"
        }));
        let v = prompt_for_credential_via_plugin(&t, "API key:", true, 60).unwrap();
        assert_eq!(v, "user-entered-secret");
        assert_eq!(
            t.last_method.borrow().as_deref(),
            Some("prompt_for_credential")
        );
    }

    #[test]
    fn prompt_for_credential_missing_value_field_errors() {
        let t = MockTransport::ok(serde_json::json!({}));
        let err = prompt_for_credential_via_plugin(&t, "x", false, 0).unwrap_err();
        assert_eq!(err.code, "decode-error");
    }

    #[test]
    fn prompt_for_credential_method_not_supported_propagates() {
        // Unlike fetch_auth_capabilities, the prompt RPC must SURFACE the
        // method-not-supported error rather than swallow it — callers
        // need an explicit fallback decision.
        let t = MockTransport::err(PluginRpcError {
            code: METHOD_NOT_SUPPORTED.to_string(),
            message: "no prompt support".to_string(),
        });
        let err = prompt_for_credential_via_plugin(&t, "x", false, 0).unwrap_err();
        assert!(err.is_method_not_supported());
    }
}
