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
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use serde::{Deserialize, Serialize};
use sindri_core::platform::TargetProfile;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
