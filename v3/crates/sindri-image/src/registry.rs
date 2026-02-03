use crate::types::{ImageInfo, ImageManifest, ImageReference, Platform};
use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{debug, trace};

/// Client for interacting with OCI-compatible container registries
pub struct RegistryClient {
    client: reqwest::Client,
    registry_url: String,
    /// Raw token (e.g., GitHub PAT) for authentication
    auth_token: Option<String>,
    /// Cached bearer token obtained from registry auth endpoint
    bearer_token: RwLock<Option<String>>,
}

impl RegistryClient {
    /// Create a new registry client
    pub fn new(registry_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("sindri-cli/3.0.0")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            registry_url: registry_url.into(),
            auth_token: None,
            bearer_token: RwLock::new(None),
        }
    }

    /// Set authentication token (e.g., GitHub token for GHCR)
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Get a bearer token for GHCR
    /// For public packages, an anonymous token can be obtained.
    /// For private packages, a GitHub PAT with read:packages scope is required.
    async fn get_ghcr_token(&self, repository: &str) -> Result<String> {
        // Check cache first
        if let Some(token) = self.bearer_token.read().unwrap().as_ref() {
            return Ok(token.clone());
        }

        // GHCR uses Docker Registry v2 token authentication
        // Format: https://ghcr.io/token?service=ghcr.io&scope=repository:OWNER/REPO:pull
        let token_url = format!(
            "https://{}/token?service={}&scope=repository:{}:pull",
            self.registry_url, self.registry_url, repository
        );

        debug!("Requesting GHCR token from: {}", token_url);

        // Try with authentication first if we have a token
        let response = if let Some(github_token) = &self.auth_token {
            debug!("Using authenticated request for GHCR token");
            self.client
                .get(&token_url)
                .basic_auth("token", Some(github_token))
                .send()
                .await
                .with_context(|| format!("Failed to request token from {}", token_url))?
        } else {
            // Try anonymous access for public packages
            debug!("Using anonymous request for GHCR token (public package)");
            self.client
                .get(&token_url)
                .send()
                .await
                .with_context(|| format!("Failed to request token from {}", token_url))?
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // If we tried with auth and failed, provide helpful message
            if self.auth_token.is_some() {
                return Err(anyhow!(
                    "GHCR token request failed ({}): {}.\n\
                    Ensure GITHUB_TOKEN has 'read:packages' scope and the package exists at ghcr.io/{}",
                    status,
                    body,
                    repository
                ));
            } else {
                return Err(anyhow!(
                    "GHCR token request failed ({}): {}.\n\
                    This package may be private. Set GITHUB_TOKEN with 'read:packages' scope.",
                    status,
                    body
                ));
            }
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        // Cache the token
        *self.bearer_token.write().unwrap() = Some(token_response.token.clone());

        Ok(token_response.token)
    }

    /// Get the appropriate authorization header for a request
    async fn get_auth_header(&self, repository: &str) -> Result<Option<HeaderValue>> {
        // For GHCR, we need to exchange the GitHub token for a bearer token
        if self.registry_url.contains("ghcr.io") {
            let bearer = self.get_ghcr_token(repository).await?;
            Ok(Some(HeaderValue::from_str(&format!("Bearer {}", bearer))?))
        } else if let Some(token) = &self.auth_token {
            // For other registries, try using the token directly
            Ok(Some(HeaderValue::from_str(&format!("Bearer {}", token))?))
        } else {
            Ok(None)
        }
    }

    /// List all tags for a repository (handles pagination)
    pub async fn list_tags(&self, repository: &str) -> Result<Vec<String>> {
        let mut all_tags = Vec::new();
        let mut url = format!(
            "https://{}/v2/{}/tags/list?n=1000",
            self.registry_url, repository
        );

        loop {
            debug!("Listing tags from: {}", url);

            let mut headers = HeaderMap::new();
            if let Some(auth_header) = self.get_auth_header(repository).await? {
                headers.insert(AUTHORIZATION, auth_header);
            }

            let response = self
                .client
                .get(&url)
                .headers(headers)
                .send()
                .await
                .with_context(|| format!("Failed to connect to registry at {}", url))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow!(
                    "Registry returned {} for {}: {}",
                    status,
                    url,
                    if body.is_empty() {
                        "(no response body)".to_string()
                    } else {
                        body
                    }
                ));
            }

            // Check for Link header for pagination
            let next_url = response
                .headers()
                .get("link")
                .and_then(|h| h.to_str().ok())
                .and_then(|link| parse_link_header(link, &self.registry_url));

            let tags_response: TagsResponse = response
                .json()
                .await
                .context("Failed to parse tags response")?;

            all_tags.extend(tags_response.tags);

            // Continue to next page if available
            match next_url {
                Some(next) => url = next,
                None => break,
            }
        }

        trace!("Found {} tags total", all_tags.len());
        Ok(all_tags)
    }

    /// Get manifest for a specific image tag or digest
    pub async fn get_manifest(&self, repository: &str, reference: &str) -> Result<ImageManifest> {
        let url = format!(
            "https://{}/v2/{}/manifests/{}",
            self.registry_url, repository, reference
        );

        debug!("Fetching manifest from: {}", url);

        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.v2+json",
            ),
        );
        if let Some(auth_header) = self.get_auth_header(repository).await? {
            headers.insert(AUTHORIZATION, auth_header);
        }

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("Failed to get manifest")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get manifest ({}): {}", status, body));
        }

        let manifest: ImageManifest = response.json().await.context("Failed to parse manifest")?;

        Ok(manifest)
    }

    /// Get detailed information about an image
    pub async fn get_image_info(&self, image: &ImageReference) -> Result<ImageInfo> {
        let reference = image
            .tag
            .as_deref()
            .or(image.digest.as_deref())
            .unwrap_or("latest");

        // Get manifest
        let manifest = self.get_manifest(&image.repository, reference).await?;

        // Get config blob for labels
        let config_url = format!(
            "https://{}/v2/{}/blobs/{}",
            self.registry_url, image.repository, manifest.config.digest
        );

        let mut headers = HeaderMap::new();
        if let Some(auth_header) = self.get_auth_header(&image.repository).await? {
            headers.insert(AUTHORIZATION, auth_header);
        }

        let config_response = self
            .client
            .get(&config_url)
            .headers(headers)
            .send()
            .await
            .context("Failed to get config blob")?;

        let config: ImageConfig = config_response
            .json()
            .await
            .context("Failed to parse config blob")?;

        // Extract information
        let size = manifest.layers.iter().map(|l| l.size).sum();
        let labels = config.config.labels.unwrap_or_default();
        let created = config.created;

        // Parse platform from config
        let platforms = vec![Platform {
            os: config.os.unwrap_or_else(|| "linux".to_string()),
            architecture: config.architecture.unwrap_or_else(|| "amd64".to_string()),
            variant: config.variant,
        }];

        // Get all tags for this repository
        let tags = self.list_tags(&image.repository).await.unwrap_or_default();

        Ok(ImageInfo {
            digest: manifest.config.digest.clone(),
            tags,
            size: Some(size),
            created: Some(created),
            labels,
            platforms,
        })
    }

    /// Check if an image exists in the registry
    pub async fn image_exists(&self, image: &ImageReference) -> Result<bool> {
        let reference = image
            .tag
            .as_deref()
            .or(image.digest.as_deref())
            .unwrap_or("latest");

        match self.get_manifest(&image.repository, reference).await {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") || e.to_string().contains("not found") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }
}

// Internal types for registry API responses

/// Parse Link header for pagination
/// Format: <https://ghcr.io/v2/repo/tags/list?n=100&last=tag>; rel="next"
fn parse_link_header(link: &str, registry_url: &str) -> Option<String> {
    for part in link.split(',') {
        let part = part.trim();
        if part.contains("rel=\"next\"") {
            // Extract URL from <...>
            if let Some(start) = part.find('<') {
                if let Some(end) = part.find('>') {
                    let url = &part[start + 1..end];
                    // URL might be relative, make it absolute
                    if url.starts_with('/') {
                        return Some(format!("https://{}{}", registry_url, url));
                    }
                    return Some(url.to_string());
                }
            }
        }
    }
    None
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct ImageConfig {
    created: String,
    #[serde(default)]
    os: Option<String>,
    #[serde(default)]
    architecture: Option<String>,
    #[serde(default)]
    variant: Option<String>,
    config: ConfigDetail,
}

#[derive(Debug, Deserialize)]
struct ConfigDetail {
    #[serde(default)]
    #[serde(rename = "Labels")]
    labels: Option<HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_client_creation() {
        let client = RegistryClient::new("ghcr.io");
        assert_eq!(client.registry_url, "ghcr.io");
        assert!(client.auth_token.is_none());

        let client_with_token = client.with_token("test-token");
        assert_eq!(client_with_token.auth_token, Some("test-token".to_string()));
    }

    #[test]
    fn test_image_reference_repository() {
        let img = ImageReference {
            registry: "ghcr.io".to_string(),
            repository: "pacphi/sindri".to_string(),
            tag: Some("v3.0.0".to_string()),
            digest: None,
        };
        assert_eq!(img.repository, "pacphi/sindri");
    }
}
