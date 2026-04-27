//! Integration tests for the subprocess-JSON plugin protocol (ADR-019).
//!
//! These shell out to small fixture scripts under `tests/fixtures/` that
//! mimic a real target plugin: handshake → one request → one response.
use sindri_targets::{PluginRequest, PluginResponse, PluginTarget, Target};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push(name);
    p
}

#[cfg(unix)]
#[test]
fn plugin_target_dispatches_exec_through_stdio() {
    let bin = fixture("fake-plugin.sh");
    let plugin = PluginTarget::new("fake", "fake", bin, serde_json::json!({"hello": "world"}));
    let resp = plugin.dispatch(&PluginRequest::Exec {
        cmd: "echo hello".into(),
        env: vec![],
    });
    let resp = resp.expect("dispatch should succeed");
    match resp {
        PluginResponse::Exec {
            stdout, exit_code, ..
        } => {
            assert_eq!(exit_code, 0);
            assert!(stdout.contains("hello"));
        }
        other => panic!("unexpected response: {:?}", other),
    }
}

#[cfg(unix)]
#[test]
fn plugin_target_profile_round_trip() {
    let bin = fixture("fake-plugin.sh");
    let plugin = PluginTarget::new("fake", "fake", bin, serde_json::Value::Null);
    let profile = plugin.profile().expect("profile");
    assert_eq!(profile.platform.triple(), "x86_64-unknown-linux-gnu");
}

#[cfg(unix)]
#[test]
fn plugin_target_check_prerequisites_returns_list() {
    let bin = fixture("fake-plugin.sh");
    let plugin = PluginTarget::new("fake", "fake", bin, serde_json::Value::Null);
    let checks = plugin.check_prerequisites();
    assert_eq!(checks.len(), 1);
    assert!(checks[0].passed);
    assert_eq!(checks[0].name, "fake");
}

#[cfg(unix)]
#[test]
fn malformed_plugin_response_propagates_error() {
    let bin = fixture("bad-handshake-plugin.sh");
    let plugin = PluginTarget::new("bad", "bad", bin, serde_json::Value::Null);
    let err = plugin
        .dispatch(&PluginRequest::Profile)
        .expect_err("malformed handshake should error");
    let msg = err.to_string();
    assert!(
        msg.contains("handshake"),
        "expected handshake error, got: {}",
        msg
    );
}
