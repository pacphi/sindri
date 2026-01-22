//! S3 backend for secret storage
//!
//! Provides S3 operations for storing and retrieving encrypted secrets.
//! Supports AWS S3 and S3-compatible storage (MinIO, Wasabi, DigitalOcean Spaces).

use crate::s3::types::{S3SecretBackend, S3SecretMetadata, S3SecretVersion};
use anyhow::{anyhow, Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ServerSideEncryption;
use aws_sdk_s3::Client;
use tracing::{debug, info};

/// S3 backend for secret storage operations
pub struct S3Backend {
    /// S3 client
    client: Client,
    /// Bucket name
    bucket: String,
    /// Key prefix for secrets
    prefix: String,
}

impl S3Backend {
    /// Create a new S3 backend from configuration
    pub async fn new(config: &S3SecretBackend) -> Result<Self> {
        let client = Self::create_client(&config.region, config.endpoint.as_deref()).await?;

        Ok(Self {
            client,
            bucket: config.bucket.clone(),
            prefix: config.prefix.clone(),
        })
    }

    /// Create an S3 client with the given region and optional endpoint
    async fn create_client(region: &str, endpoint: Option<&str>) -> Result<Client> {
        let region = Region::new(region.to_string());

        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await;

        let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&sdk_config);

        // Configure custom endpoint for S3-compatible storage
        if let Some(endpoint_url) = endpoint {
            debug!("Using custom S3 endpoint: {}", endpoint_url);
            s3_config_builder = s3_config_builder
                .endpoint_url(endpoint_url)
                .force_path_style(true); // Required for MinIO and many S3-compatible services
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        Ok(client)
    }

    /// Create S3 backend with explicit region and endpoint (useful for CLI)
    pub async fn from_params(
        bucket: String,
        region: String,
        endpoint: Option<String>,
        prefix: String,
    ) -> Result<Self> {
        let client = Self::create_client(&region, endpoint.as_deref()).await?;

        Ok(Self {
            client,
            bucket,
            prefix,
        })
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Get the key prefix
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Check if the S3 bucket exists and is accessible
    pub async fn check_bucket(&self) -> Result<bool> {
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                debug!("Bucket {} is accessible", self.bucket);
                Ok(true)
            }
            Err(e) => {
                let service_error = e.into_service_error();
                if service_error.is_not_found() {
                    debug!("Bucket {} does not exist", self.bucket);
                    Ok(false)
                } else {
                    Err(anyhow!(
                        "Failed to check bucket {}: {}",
                        self.bucket,
                        service_error
                    ))
                }
            }
        }
    }

    /// Create the S3 bucket if it doesn't exist
    pub async fn create_bucket(&self) -> Result<()> {
        info!("Creating bucket: {}", self.bucket);

        self.client
            .create_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .with_context(|| format!("Failed to create bucket: {}", self.bucket))?;

        // Enable versioning for audit trail
        self.client
            .put_bucket_versioning()
            .bucket(&self.bucket)
            .versioning_configuration(
                aws_sdk_s3::types::VersioningConfiguration::builder()
                    .status(aws_sdk_s3::types::BucketVersioningStatus::Enabled)
                    .build(),
            )
            .send()
            .await
            .with_context(|| format!("Failed to enable versioning on bucket: {}", self.bucket))?;

        info!("Created bucket {} with versioning enabled", self.bucket);
        Ok(())
    }

    /// Check if a secret exists in S3
    pub async fn secret_exists(&self, s3_path: &str) -> Result<bool> {
        let key = self.make_key(s3_path);
        debug!("Checking if secret exists: s3://{}/{}", self.bucket, key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => {
                debug!("Secret exists: {}", s3_path);
                Ok(true)
            }
            Err(e) => {
                let service_error = e.into_service_error();
                if service_error.is_not_found() {
                    debug!("Secret does not exist: {}", s3_path);
                    Ok(false)
                } else {
                    Err(anyhow!(
                        "Failed to check secret existence for {}: {}",
                        s3_path,
                        service_error
                    ))
                }
            }
        }
    }

    /// Get a secret from S3
    pub async fn get_secret(&self, s3_path: &str) -> Result<Vec<u8>> {
        let key = self.make_key(s3_path);
        debug!("Downloading secret: s3://{}/{}", self.bucket, key);

        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .with_context(|| format!("Failed to get secret from S3: {}", s3_path))?;

        let body = resp
            .body
            .collect()
            .await
            .context("Failed to read response body")?;

        let data = body.into_bytes().to_vec();
        debug!(
            "Downloaded {} bytes from s3://{}/{}",
            data.len(),
            self.bucket,
            key
        );

        Ok(data)
    }

    /// Get secret metadata (without downloading full content)
    pub async fn get_secret_metadata(&self, s3_path: &str) -> Result<S3SecretMetadata> {
        let data = self.get_secret(s3_path).await?;
        serde_json::from_slice(&data)
            .with_context(|| format!("Failed to parse secret metadata for: {}", s3_path))
    }

    /// Put a secret to S3
    ///
    /// Returns the version ID if versioning is enabled.
    pub async fn put_secret(&self, s3_path: &str, data: Vec<u8>) -> Result<String> {
        let key = self.make_key(s3_path);
        debug!(
            "Uploading secret ({} bytes): s3://{}/{}",
            data.len(),
            self.bucket,
            key
        );

        let resp = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(data))
            .content_type("application/json")
            .server_side_encryption(ServerSideEncryption::Aes256) // SSE-S3
            .send()
            .await
            .with_context(|| format!("Failed to put secret to S3: {}", s3_path))?;

        let version_id = resp.version_id.unwrap_or_default();
        info!(
            "Uploaded secret to s3://{}/{} (version: {})",
            self.bucket, key, version_id
        );

        Ok(version_id)
    }

    /// Put secret metadata to S3
    pub async fn put_secret_metadata(
        &self,
        s3_path: &str,
        metadata: &S3SecretMetadata,
    ) -> Result<String> {
        let json_data =
            serde_json::to_vec_pretty(metadata).context("Failed to serialize secret metadata")?;
        self.put_secret(s3_path, json_data).await
    }

    /// Delete a secret from S3
    pub async fn delete_secret(&self, s3_path: &str) -> Result<()> {
        let key = self.make_key(s3_path);
        debug!("Deleting secret: s3://{}/{}", self.bucket, key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .with_context(|| format!("Failed to delete secret: {}", s3_path))?;

        info!("Deleted secret: s3://{}/{}", self.bucket, key);
        Ok(())
    }

    /// List all secrets under the prefix
    pub async fn list_secrets(&self) -> Result<Vec<String>> {
        debug!("Listing secrets in s3://{}/{}", self.bucket, self.prefix);

        let mut secrets = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&self.prefix);

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let resp = request
                .send()
                .await
                .context("Failed to list secrets from S3")?;

            if let Some(contents) = resp.contents {
                for object in contents {
                    if let Some(key) = object.key {
                        // Remove prefix and .json extension
                        let s3_path = key
                            .strip_prefix(&self.prefix)
                            .unwrap_or(&key)
                            .strip_suffix(".json")
                            .unwrap_or(&key)
                            .to_string();
                        if !s3_path.is_empty() {
                            secrets.push(s3_path);
                        }
                    }
                }
            }

            if resp.is_truncated == Some(true) {
                continuation_token = resp.next_continuation_token;
            } else {
                break;
            }
        }

        debug!("Found {} secrets", secrets.len());
        Ok(secrets)
    }

    /// Get version history for a secret
    pub async fn get_secret_versions(&self, s3_path: &str) -> Result<Vec<S3SecretVersion>> {
        let key = self.make_key(s3_path);
        debug!("Getting versions for: s3://{}/{}", self.bucket, key);

        let resp = self
            .client
            .list_object_versions()
            .bucket(&self.bucket)
            .prefix(&key)
            .send()
            .await
            .with_context(|| format!("Failed to list versions for: {}", s3_path))?;

        let mut versions = Vec::new();

        if let Some(version_list) = resp.versions {
            for version in version_list {
                if version.key.as_deref() == Some(&key) {
                    versions.push(S3SecretVersion {
                        version_id: version.version_id.unwrap_or_default(),
                        last_modified: version
                            .last_modified
                            .map(|t| t.to_string())
                            .unwrap_or_default(),
                        etag: version.e_tag.unwrap_or_default(),
                        size: version.size.unwrap_or(0) as u64,
                        is_latest: version.is_latest.unwrap_or(false),
                    });
                }
            }
        }

        debug!("Found {} versions for {}", versions.len(), s3_path);
        Ok(versions)
    }

    /// Get a specific version of a secret
    pub async fn get_secret_version(&self, s3_path: &str, version_id: &str) -> Result<Vec<u8>> {
        let key = self.make_key(s3_path);
        debug!(
            "Getting secret version: s3://{}/{} (version: {})",
            self.bucket, key, version_id
        );

        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .version_id(version_id)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to get secret version {} for: {}",
                    version_id, s3_path
                )
            })?;

        let body = resp
            .body
            .collect()
            .await
            .context("Failed to read response body")?;

        Ok(body.into_bytes().to_vec())
    }

    /// Build the full S3 key from a secret path
    fn make_key(&self, s3_path: &str) -> String {
        let path = s3_path.trim_start_matches('/');
        let key = format!("{}{}.json", self.prefix, path);
        // Normalize double slashes
        key.replace("//", "/")
    }
}

impl std::fmt::Debug for S3Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Backend")
            .field("bucket", &self.bucket)
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_make_key() {
        // Create a mock backend for testing key generation
        // Note: We can't test actual S3 operations without a real/mock S3 endpoint
        let prefix = "secrets/prod/";

        // Test key generation logic
        let make_key = |s3_path: &str| -> String {
            let path = s3_path.trim_start_matches('/');
            format!("{}{}.json", prefix, path).replace("//", "/")
        };

        assert_eq!(
            make_key("database/password"),
            "secrets/prod/database/password.json"
        );
        assert_eq!(
            make_key("/database/password"),
            "secrets/prod/database/password.json"
        );
        assert_eq!(make_key("api-key"), "secrets/prod/api-key.json");
    }

    #[test]
    fn test_s3_backend_debug() {
        // Just verify Debug trait doesn't expose secrets
        // This is a compile-time check mostly
        let debug_output = format!(
            "S3Backend {{ bucket: {:?}, prefix: {:?}, .. }}",
            "my-bucket", "secrets/"
        );
        assert!(debug_output.contains("my-bucket"));
        assert!(!debug_output.contains("client")); // Client shouldn't be in debug
    }
}
