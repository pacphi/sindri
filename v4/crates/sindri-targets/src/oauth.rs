//! OAuth 2.0 Device Authorization Grant (RFC 8628) for cloud-target auth.
//!
//! Closes audit item D3 (Wave 6B). Replaces the previous "run upstream-CLI
//! login" hint with an actual device-flow implementation for providers
//! that support it. The CLI surface lives in
//! `sindri/src/commands/target.rs::auth_target`.
//!
//! ## Flow (RFC 8628)
//!
//! 1. POST to the device-code endpoint to obtain a `device_code`,
//!    `user_code`, `verification_uri`, and polling parameters.
//! 2. Show the user the `user_code` and `verification_uri` so they can
//!    authorise the app in their browser.
//! 3. Poll the token endpoint with the `device_code` until the user
//!    approves (200 OK with `access_token`), denies (`access_denied`),
//!    or the device-code expires (`expired_token`).
//! 4. Persist the resulting `access_token` (sindri stores it via the
//!    existing `targets.<name>.auth.token` path in sindri.yaml; the CLI
//!    layer wraps the value with `plain:` so the `AuthValue` parser
//!    treats it as inline).
//!
//! ## Per-provider gaps
//!
//! Not every provider implements the device flow at a publicly documented
//! endpoint. Today the following are wired up:
//!
//! * **GitHub** — fully supported (`https://github.com/login/device/code`,
//!   `https://github.com/login/oauth/access_token`).
//! * **fly.io** — fly does **not** publish a stable device-flow endpoint
//!   at the time of writing; PATs are issued via the dashboard or
//!   `flyctl auth login`. The hint path is preserved for fly. See
//!   [`provider_supports_oauth`].
//! * **Northflank** — Northflank's public API documents personal
//!   API tokens issued from the dashboard; no device flow is published.
//!   The hint path is preserved.
//!
//! The state machine itself is provider-agnostic and tested against
//! wiremock so additional providers can be wired in without retesting
//! the polling logic.
use crate::error::TargetError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Provider-specific OAuth endpoints + client metadata.
#[derive(Debug, Clone)]
pub struct OAuthProvider {
    /// Human-readable provider id (`"github"`, `"fly"`, `"northflank"`).
    pub id: &'static str,
    /// `POST` here to obtain a device code.
    pub device_code_url: String,
    /// `POST` here to poll for an access token.
    pub token_url: String,
    /// Public OAuth client id (no secret — device flow is for public
    /// clients per RFC 8628 §1).
    pub client_id: String,
    /// Space-delimited scope string. Empty if the provider has no
    /// concept of scopes for the device flow.
    pub scope: String,
}

impl OAuthProvider {
    /// Sindri's GitHub OAuth app. The client id is intentionally public —
    /// device flow does not use a client secret. Operators who want to
    /// use a different OAuth app can override via env var
    /// `SINDRI_GITHUB_CLIENT_ID` (handled at the call site).
    pub fn github(client_id: &str) -> Self {
        Self {
            id: "github",
            device_code_url: "https://github.com/login/device/code".into(),
            token_url: "https://github.com/login/oauth/access_token".into(),
            client_id: client_id.to_string(),
            scope: "repo read:org".into(),
        }
    }

    /// Construct a provider with custom URLs (used by tests against
    /// wiremock and by callers who want to point at a non-default OAuth
    /// app).
    pub fn custom(
        id: &'static str,
        device_code_url: impl Into<String>,
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        Self {
            id,
            device_code_url: device_code_url.into(),
            token_url: token_url.into(),
            client_id: client_id.into(),
            scope: scope.into(),
        }
    }
}

/// Returns `true` if `provider_id` (the target kind, e.g. `"fly"`) has a
/// publicly documented OAuth device-flow endpoint that sindri wires up.
///
/// Today this is `"github"` only. fly.io and Northflank fall through to
/// the hint path. As providers expose stable device-flow URLs this
/// function is the single switch the CLI consults.
pub fn provider_supports_oauth(provider_id: &str) -> bool {
    matches!(provider_id, "github")
}

/// Response body from the device-code endpoint (RFC 8628 §3.2).
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    /// Some servers (notably GitHub) emit `verification_uri_complete`.
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    /// Lifetime of the device code in seconds.
    pub expires_in: u64,
    /// Minimum polling interval in seconds.
    pub interval: u64,
}

/// Successful token response (RFC 8628 §3.5 + RFC 6749 §5.1).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    /// Some providers (GitHub) include this; others omit it.
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

/// Polling outcome after a single token-endpoint call. The state machine
/// in [`poll_until_done`] dispatches on this.
#[derive(Debug)]
pub enum PollOutcome {
    /// User approved — stop polling and use this token.
    Success(TokenResponse),
    /// User has not yet approved — wait and retry.
    Pending,
    /// Server told us to slow down — bump interval and retry.
    SlowDown,
    /// User explicitly denied — abort.
    AccessDenied,
    /// Device code is no longer valid — abort.
    Expired,
}

/// Step the device flow forward by one HTTP round trip.
///
/// Returns the parsed response. Errors are HTTP-level; OAuth-level
/// errors (`authorization_pending`, etc.) come back as `Ok` variants of
/// [`PollOutcome`] because the state machine treats them as control flow,
/// not failures.
pub async fn poll_once(
    client: &reqwest::Client,
    provider: &OAuthProvider,
    device_code: &str,
) -> Result<PollOutcome, TargetError> {
    let mut form = vec![
        ("client_id", provider.client_id.as_str()),
        ("device_code", device_code),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
    ];
    if !provider.scope.is_empty() {
        form.push(("scope", provider.scope.as_str()));
    }
    let resp = client
        .post(&provider.token_url)
        .header("accept", "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|e| TargetError::Http {
            target: provider.id.into(),
            detail: format!("token poll request failed: {}", e),
        })?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.map_err(|e| TargetError::Http {
        target: provider.id.into(),
        detail: format!("could not parse token response: {}", e),
    })?;

    if status.is_success() && body.get("access_token").is_some() {
        let token: TokenResponse = serde_json::from_value(body).map_err(|e| TargetError::Http {
            target: provider.id.into(),
            detail: format!("invalid token response: {}", e),
        })?;
        return Ok(PollOutcome::Success(token));
    }

    let err = body
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown_error");
    Ok(match err {
        "authorization_pending" => PollOutcome::Pending,
        "slow_down" => PollOutcome::SlowDown,
        "access_denied" => PollOutcome::AccessDenied,
        "expired_token" => PollOutcome::Expired,
        other => {
            return Err(TargetError::AuthFailed {
                target: provider.id.into(),
                detail: format!("OAuth error '{}': {}", other, body),
            })
        }
    })
}

/// Trait abstracting the sleep + clock used by [`poll_until_done`].
///
/// The default impl ([`RealSleeper`]) uses `tokio::time::sleep` and
/// `std::time::Instant`. Tests inject a virtual clock that advances
/// instantly so polling tests run in milliseconds.
pub trait Sleeper {
    /// Sleep for `dur` (typically the poll interval).
    fn sleep(&mut self, dur: Duration) -> impl std::future::Future<Output = ()> + Send;
    /// Total simulated elapsed time. Used to enforce `expires_in`.
    fn elapsed(&self) -> Duration;
}

/// Production sleeper backed by `tokio::time::sleep`.
#[derive(Debug)]
pub struct RealSleeper {
    started: std::time::Instant,
}

impl Default for RealSleeper {
    fn default() -> Self {
        Self {
            started: std::time::Instant::now(),
        }
    }
}

impl Sleeper for RealSleeper {
    async fn sleep(&mut self, dur: Duration) {
        tokio::time::sleep(dur).await;
    }
    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}

/// Virtual sleeper used in tests. Does not actually sleep — just
/// accumulates simulated elapsed time.
#[derive(Debug, Default)]
pub struct FakeSleeper {
    elapsed: Duration,
}

impl Sleeper for FakeSleeper {
    async fn sleep(&mut self, dur: Duration) {
        self.elapsed += dur;
    }
    fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// Drive the polling state machine until the user approves, denies, or
/// the device code expires.
///
/// Returns the access token on success. The caller is responsible for
/// presenting the `verification_uri` + `user_code` to the user before
/// calling this function (typically `request_device_code` returns a
/// [`DeviceCodeResponse`] that the CLI prints; then this is invoked with
/// the same response).
pub async fn poll_until_done<S: Sleeper>(
    client: &reqwest::Client,
    provider: &OAuthProvider,
    device: &DeviceCodeResponse,
    sleeper: &mut S,
) -> Result<TokenResponse, TargetError> {
    let mut interval = Duration::from_secs(device.interval.max(1));
    let expires = Duration::from_secs(device.expires_in);
    loop {
        if sleeper.elapsed() >= expires {
            return Err(TargetError::AuthFailed {
                target: provider.id.into(),
                detail: format!(
                    "device code expired after {}s without approval",
                    expires.as_secs()
                ),
            });
        }
        sleeper.sleep(interval).await;
        match poll_once(client, provider, &device.device_code).await? {
            PollOutcome::Success(tok) => return Ok(tok),
            PollOutcome::Pending => continue,
            PollOutcome::SlowDown => {
                interval += Duration::from_secs(5);
                continue;
            }
            PollOutcome::AccessDenied => {
                return Err(TargetError::AuthFailed {
                    target: provider.id.into(),
                    detail: "user denied the authorisation request".into(),
                })
            }
            PollOutcome::Expired => {
                return Err(TargetError::AuthFailed {
                    target: provider.id.into(),
                    detail: "device code expired before approval".into(),
                })
            }
        }
    }
}

/// Request a device code from `provider`.
pub async fn request_device_code(
    client: &reqwest::Client,
    provider: &OAuthProvider,
) -> Result<DeviceCodeResponse, TargetError> {
    let mut form = vec![("client_id", provider.client_id.as_str())];
    if !provider.scope.is_empty() {
        form.push(("scope", provider.scope.as_str()));
    }
    let resp = client
        .post(&provider.device_code_url)
        .header("accept", "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|e| TargetError::Http {
            target: provider.id.into(),
            detail: format!("device-code request failed: {}", e),
        })?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(TargetError::Http {
            target: provider.id.into(),
            detail: format!("device-code endpoint returned HTTP {}: {}", status, body),
        });
    }
    let parsed: DeviceCodeResponse = resp.json().await.map_err(|e| TargetError::Http {
        target: provider.id.into(),
        detail: format!("could not parse device-code response: {}", e),
    })?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_provider(uri: &str) -> OAuthProvider {
        OAuthProvider::custom(
            "github",
            format!("{}/login/device/code", uri),
            format!("{}/login/oauth/access_token", uri),
            "test-client-id",
            "repo",
        )
    }

    #[tokio::test]
    async fn supports_oauth_truthy_only_for_github() {
        assert!(provider_supports_oauth("github"));
        assert!(!provider_supports_oauth("fly"));
        assert!(!provider_supports_oauth("northflank"));
        assert!(!provider_supports_oauth("local"));
    }

    #[tokio::test]
    async fn request_device_code_parses_full_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/device/code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "DC-123",
                "user_code": "ABCD-1234",
                "verification_uri": "https://example.com/device",
                "verification_uri_complete": "https://example.com/device?user_code=ABCD-1234",
                "expires_in": 900,
                "interval": 5,
            })))
            .mount(&server)
            .await;
        let provider = make_provider(&server.uri());
        let client = reqwest::Client::new();
        let resp = request_device_code(&client, &provider).await.unwrap();
        assert_eq!(resp.device_code, "DC-123");
        assert_eq!(resp.user_code, "ABCD-1234");
        assert_eq!(resp.expires_in, 900);
        assert_eq!(resp.interval, 5);
        assert!(resp.verification_uri_complete.is_some());
    }

    #[tokio::test]
    async fn poll_succeeds_after_two_pending_responses() {
        let server = MockServer::start().await;
        // First two polls return authorization_pending, third returns success.
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .and(body_string_contains("device_code=DC-1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"error": "authorization_pending"})),
            )
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "gho_abc",
                "token_type": "bearer",
                "scope": "repo",
            })))
            .mount(&server)
            .await;

        let provider = make_provider(&server.uri());
        let client = reqwest::Client::new();
        let device = DeviceCodeResponse {
            device_code: "DC-1".into(),
            user_code: "AAAA-BBBB".into(),
            verification_uri: "https://example.com".into(),
            verification_uri_complete: None,
            expires_in: 900,
            interval: 1,
        };
        let mut sleeper = FakeSleeper::default();
        let token = poll_until_done(&client, &provider, &device, &mut sleeper)
            .await
            .unwrap();
        assert_eq!(token.access_token, "gho_abc");
        assert_eq!(token.token_type, "bearer");
    }

    #[tokio::test]
    async fn poll_returns_access_denied_immediately_on_denial() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "error": "access_denied"
            })))
            .mount(&server)
            .await;
        let provider = make_provider(&server.uri());
        let client = reqwest::Client::new();
        let device = DeviceCodeResponse {
            device_code: "DC-2".into(),
            user_code: "CCCC-DDDD".into(),
            verification_uri: "https://example.com".into(),
            verification_uri_complete: None,
            expires_in: 900,
            interval: 1,
        };
        let mut sleeper = FakeSleeper::default();
        let err = poll_until_done(&client, &provider, &device, &mut sleeper)
            .await
            .unwrap_err();
        match err {
            TargetError::AuthFailed { detail, .. } => {
                assert!(detail.contains("denied"), "got: {}", detail)
            }
            other => panic!("expected AuthFailed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn poll_times_out_when_device_expires() {
        let server = MockServer::start().await;
        // Always pending.
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "error": "authorization_pending"
            })))
            .mount(&server)
            .await;
        let provider = make_provider(&server.uri());
        let client = reqwest::Client::new();
        let device = DeviceCodeResponse {
            device_code: "DC-3".into(),
            user_code: "EEEE-FFFF".into(),
            verification_uri: "https://example.com".into(),
            verification_uri_complete: None,
            // Tight expiry — fake sleeper increments by `interval` each
            // step, so this expires after one tick.
            expires_in: 5,
            interval: 10,
        };
        let mut sleeper = FakeSleeper::default();
        let err = poll_until_done(&client, &provider, &device, &mut sleeper)
            .await
            .unwrap_err();
        match err {
            TargetError::AuthFailed { detail, .. } => {
                assert!(detail.contains("expired"), "got: {}", detail)
            }
            other => panic!("expected AuthFailed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn slow_down_increases_interval() {
        let server = MockServer::start().await;
        // First poll: slow_down. Second poll: success.
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "error": "slow_down"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tok-slow",
                "token_type": "bearer",
            })))
            .mount(&server)
            .await;

        let provider = make_provider(&server.uri());
        let client = reqwest::Client::new();
        let device = DeviceCodeResponse {
            device_code: "DC-4".into(),
            user_code: "GGGG-HHHH".into(),
            verification_uri: "https://example.com".into(),
            verification_uri_complete: None,
            expires_in: 900,
            interval: 1,
        };
        let mut sleeper = FakeSleeper::default();
        let token = poll_until_done(&client, &provider, &device, &mut sleeper)
            .await
            .unwrap();
        assert_eq!(token.access_token, "tok-slow");
        // Initial interval 1s + slow_down bump 5s = at least 6s simulated.
        assert!(
            sleeper.elapsed() >= Duration::from_secs(6),
            "expected slow-down bump, got elapsed={:?}",
            sleeper.elapsed()
        );
    }

    #[tokio::test]
    async fn unknown_oauth_error_returns_auth_failed() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_grant"
            })))
            .mount(&server)
            .await;
        let provider = make_provider(&server.uri());
        let client = reqwest::Client::new();
        let outcome = poll_once(&client, &provider, "DC-5").await;
        assert!(outcome.is_err(), "expected AuthFailed, got {:?}", outcome);
    }
}
