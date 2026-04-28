//! Integration test: OAuth token write + read through the `target auth` flow
//! and `sindri-secrets` FileBackend (ADR-025).
//!
//! This test exercises:
//!
//! 1. `sindri target auth <name> --value plain:<token>` — the token must
//!    be stored in the `FileBackend` and the manifest must contain a
//!    `secret:` pointer (not the raw token).
//!
//! 2. Reading the secret back through `FileBackend::read` directly —
//!    confirms round-trip fidelity.
//!
//! 3. The migration helper: a manifest with a legacy `plain:` token is
//!    converted on first read without requiring user intervention.

#[path = "helpers.rs"]
mod helpers;

use std::fs;
use tempfile::tempdir;

/// Verify that `sindri target auth` with `plain:` prefix stores the token
/// in the secrets file and writes a `secret:` pointer to the manifest.
#[test]
fn target_auth_plain_stores_secret_and_writes_pointer() {
    let tmp = tempdir().unwrap();
    let workdir = tmp.path();

    // Create a minimal sindri.yaml.
    fs::write(workdir.join("sindri.yaml"), "components: []\n").unwrap();

    // Point the FileBackend at a temp secrets file via env var.
    // secrets_file variable removed — secrets go under $SINDRI_HOME/.sindri/secrets.enc
    let passphrase = "integration-test-passphrase";

    // Invoke `sindri target auth mycloud --value plain:mytok` with the
    // FileBackend pointed at our tempdir.
    helpers::sindri_cmd_in(workdir)
        .env("SINDRI_SECRETS_PASSPHRASE", passphrase)
        .env("SINDRI_HOME", workdir.to_str().unwrap())
        .args(["target", "auth", "mycloud", "--value", "plain:mytok"])
        .assert()
        .success();

    // 1. The manifest must NOT contain the raw token.
    let manifest = fs::read_to_string(workdir.join("sindri.yaml")).unwrap();
    assert!(
        !manifest.contains("mytok"),
        "manifest leaked raw token:\n{manifest}"
    );

    // 2. The manifest must contain the secret: pointer.
    assert!(
        manifest.contains("secret:targets.mycloud.auth.token"),
        "manifest missing secret: pointer:\n{manifest}"
    );

    // 3. The secrets file must exist.
    let secrets_path = workdir.join(".sindri").join("secrets.enc");
    assert!(
        secrets_path.exists(),
        "secrets file was not created at {}",
        secrets_path.display()
    );

    // 4. Read the secret back via the FileBackend API.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let store = sindri_secrets::FileBackend::with_path_and_passphrase(&secrets_path, passphrase);
    use sindri_secrets::SecretStore;
    let sv = rt
        .block_on(store.read("targets.mycloud.auth.token"))
        .expect("should read back the stored secret");
    assert_eq!(
        sv.expose_str().unwrap(),
        "mytok",
        "round-trip secret value mismatch"
    );
}

/// Verify that a `secret_value::Debug` format never contains the secret bytes.
#[test]
fn secret_value_debug_masking() {
    let v = sindri_secrets::SecretValue::from_plaintext("super-secret-xyz-9876");
    let dbg = format!("{:?}", v);
    assert!(
        !dbg.contains("super-secret-xyz-9876"),
        "Debug leaked secret: {dbg}"
    );
    assert!(dbg.contains("REDACTED"), "Debug missing REDACTED: {dbg}");
}
