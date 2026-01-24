use crate::types::{ImageInfo, ImageManifest, ImageReference, Platform};
use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, trace};

/// Client for interacting with OCI-compatible container registries
pub struct RegistryClient {
    client: reqwest::Client,
    registry_url: String,
    auth_token: Option<String>,
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
        }
    }

    /// Set authentication token (e.g., GitHub token for GHCR)
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// List all tags for a repository
    pub async fn list_tags(&self, repository: &str) -> Result<Vec<String>> {
        let url = format!("https://{}/v2/{}/tags/list", self.registry_url, repository);

        debug!("Listing tags from: {}", url);

        let mut headers = HeaderMap::new();
        if let Some(token) = &self.auth_token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
        }

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("Failed to list tags")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Registry API error ({}): {}", status, body));
        }

        let tags_response: TagsResponse = response
            .json()
            .await
            .context("Failed to parse tags response")?;

        trace!("Found {} tags", tags_response.tags.len());
        Ok(tags_response.tags)
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
        if let Some(token) = &self.auth_token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
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
        if let Some(token) = &self.auth_token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
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

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    tags: Vec<String>,
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
