//! Mock cloud API implementations for testing
//!
//! Provides mock implementations of cloud provider APIs for testing
//! Packer template generation and build workflows.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock image information
#[derive(Clone, Debug)]
pub struct MockImage {
    pub id: String,
    pub name: String,
    #[allow(dead_code)]
    pub created_at: String,
    pub regions: Vec<String>,
    pub tags: HashMap<String, String>,
}

impl MockImage {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            created_at: "2026-01-26T00:00:00Z".to_string(),
            regions: vec!["us-west-2".to_string()],
            tags: HashMap::new(),
        }
    }

    pub fn with_tag(mut self, key: &str, value: &str) -> Self {
        self.tags.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_regions(mut self, regions: Vec<&str>) -> Self {
        self.regions = regions.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Mock build result
#[derive(Clone, Debug)]
pub struct MockBuildResult {
    pub success: bool,
    pub image_id: Option<String>,
    pub duration_seconds: u64,
    pub error: Option<String>,
    #[allow(dead_code)]
    pub logs: Vec<String>,
}

impl MockBuildResult {
    pub fn success(image_id: &str, duration: u64) -> Self {
        Self {
            success: true,
            image_id: Some(image_id.to_string()),
            duration_seconds: duration,
            error: None,
            logs: vec!["Build completed successfully".to_string()],
        }
    }

    pub fn failure(error: &str, duration: u64) -> Self {
        Self {
            success: false,
            image_id: None,
            duration_seconds: duration,
            error: Some(error.to_string()),
            logs: vec![format!("Build failed: {}", error)],
        }
    }
}

/// Mock AWS cloud provider
pub struct MockAwsProvider {
    images: Arc<Mutex<Vec<MockImage>>>,
    build_results: Arc<Mutex<HashMap<String, MockBuildResult>>>,
    api_calls: Arc<Mutex<Vec<String>>>,
}

impl Default for MockAwsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAwsProvider {
    pub fn new() -> Self {
        Self {
            images: Arc::new(Mutex::new(Vec::new())),
            build_results: Arc::new(Mutex::new(HashMap::new())),
            api_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a mock image
    pub fn add_image(&self, image: MockImage) {
        self.images.lock().unwrap().push(image);
    }

    /// Set expected build result for a config
    pub fn set_build_result(&self, config_name: &str, result: MockBuildResult) {
        self.build_results
            .lock()
            .unwrap()
            .insert(config_name.to_string(), result);
    }

    /// List images (mock)
    pub fn list_images(&self, filter: Option<&str>) -> Vec<MockImage> {
        self.api_calls
            .lock()
            .unwrap()
            .push(format!("list_images(filter={:?})", filter));

        let images = self.images.lock().unwrap();
        match filter {
            Some(f) => images
                .iter()
                .filter(|i| i.name.contains(f))
                .cloned()
                .collect(),
            None => images.clone(),
        }
    }

    /// Build image (mock)
    pub fn build(&self, config_name: &str) -> MockBuildResult {
        self.api_calls
            .lock()
            .unwrap()
            .push(format!("build({})", config_name));

        let results = self.build_results.lock().unwrap();
        results
            .get(config_name)
            .cloned()
            .unwrap_or_else(|| MockBuildResult::success("ami-mock123", 120))
    }

    /// Delete image (mock)
    pub fn delete_image(&self, image_id: &str) -> bool {
        self.api_calls
            .lock()
            .unwrap()
            .push(format!("delete_image({})", image_id));

        let mut images = self.images.lock().unwrap();
        let initial_len = images.len();
        images.retain(|i| i.id != image_id);
        images.len() < initial_len
    }

    /// Get all API calls made
    pub fn get_api_calls(&self) -> Vec<String> {
        self.api_calls.lock().unwrap().clone()
    }

    /// Check if an API was called
    pub fn was_called(&self, api_name: &str) -> bool {
        self.api_calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.contains(api_name))
    }

    /// Reset state
    pub fn reset(&self) {
        self.images.lock().unwrap().clear();
        self.build_results.lock().unwrap().clear();
        self.api_calls.lock().unwrap().clear();
    }
}

/// Mock GCP cloud provider
pub struct MockGcpProvider {
    images: Arc<Mutex<Vec<MockImage>>>,
    api_calls: Arc<Mutex<Vec<String>>>,
}

impl Default for MockGcpProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockGcpProvider {
    pub fn new() -> Self {
        Self {
            images: Arc::new(Mutex::new(Vec::new())),
            api_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_image(&self, image: MockImage) {
        self.images.lock().unwrap().push(image);
    }

    pub fn list_images(&self) -> Vec<MockImage> {
        self.api_calls
            .lock()
            .unwrap()
            .push("list_images()".to_string());
        self.images.lock().unwrap().clone()
    }

    pub fn get_api_calls(&self) -> Vec<String> {
        self.api_calls.lock().unwrap().clone()
    }
}

/// Mock Azure cloud provider
pub struct MockAzureProvider {
    images: Arc<Mutex<Vec<MockImage>>>,
    api_calls: Arc<Mutex<Vec<String>>>,
}

impl Default for MockAzureProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAzureProvider {
    pub fn new() -> Self {
        Self {
            images: Arc::new(Mutex::new(Vec::new())),
            api_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[allow(dead_code)]
    pub fn add_image(&self, image: MockImage) {
        self.images.lock().unwrap().push(image);
    }

    #[allow(dead_code)]
    pub fn list_images(&self) -> Vec<MockImage> {
        self.api_calls
            .lock()
            .unwrap()
            .push("list_images()".to_string());
        self.images.lock().unwrap().clone()
    }

    #[allow(dead_code)]
    pub fn get_api_calls(&self) -> Vec<String> {
        self.api_calls.lock().unwrap().clone()
    }
}

/// Mock OCI (Oracle Cloud Infrastructure) provider
pub struct MockOciProvider {
    images: Arc<Mutex<Vec<MockImage>>>,
    api_calls: Arc<Mutex<Vec<String>>>,
}

impl Default for MockOciProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockOciProvider {
    pub fn new() -> Self {
        Self {
            images: Arc::new(Mutex::new(Vec::new())),
            api_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_image(&self, image: MockImage) {
        self.images.lock().unwrap().push(image);
    }

    pub fn list_images(&self) -> Vec<MockImage> {
        self.api_calls
            .lock()
            .unwrap()
            .push("list_images()".to_string());
        self.images.lock().unwrap().clone()
    }

    pub fn get_api_calls(&self) -> Vec<String> {
        self.api_calls.lock().unwrap().clone()
    }
}

/// Mock Alibaba Cloud provider
pub struct MockAlibabaProvider {
    images: Arc<Mutex<Vec<MockImage>>>,
    api_calls: Arc<Mutex<Vec<String>>>,
}

impl Default for MockAlibabaProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAlibabaProvider {
    pub fn new() -> Self {
        Self {
            images: Arc::new(Mutex::new(Vec::new())),
            api_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_image(&self, image: MockImage) {
        self.images.lock().unwrap().push(image);
    }

    pub fn list_images(&self) -> Vec<MockImage> {
        self.api_calls
            .lock()
            .unwrap()
            .push("list_images()".to_string());
        self.images.lock().unwrap().clone()
    }

    pub fn get_api_calls(&self) -> Vec<String> {
        self.api_calls.lock().unwrap().clone()
    }
}

/// Template rendering mock for testing HCL2 generation
pub struct MockTemplateRenderer {
    templates: Arc<Mutex<HashMap<String, String>>>,
    render_calls: Arc<Mutex<Vec<String>>>,
}

impl Default for MockTemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTemplateRenderer {
    pub fn new() -> Self {
        Self {
            templates: Arc::new(Mutex::new(HashMap::new())),
            render_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Set expected template output
    pub fn set_template(&self, cloud: &str, content: &str) {
        self.templates
            .lock()
            .unwrap()
            .insert(cloud.to_string(), content.to_string());
    }

    /// Render template (mock)
    pub fn render(&self, cloud: &str, _context: &HashMap<String, String>) -> Option<String> {
        self.render_calls
            .lock()
            .unwrap()
            .push(format!("render({})", cloud));

        self.templates.lock().unwrap().get(cloud).cloned()
    }

    /// Get render calls
    #[allow(dead_code)]
    pub fn get_render_calls(&self) -> Vec<String> {
        self.render_calls.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_aws_provider() {
        let provider = MockAwsProvider::new();

        // Add an image
        provider.add_image(MockImage::new("ami-123", "sindri-test"));

        // List images
        let images = provider.list_images(None);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].id, "ami-123");

        // Check API calls
        assert!(provider.was_called("list_images"));
    }

    #[test]
    fn test_mock_build_result() {
        let provider = MockAwsProvider::new();
        provider.set_build_result("test-config", MockBuildResult::success("ami-456", 60));

        let result = provider.build("test-config");
        assert!(result.success);
        assert_eq!(result.image_id, Some("ami-456".to_string()));
    }

    #[test]
    fn test_mock_image_builder() {
        let image = MockImage::new("ami-test", "test-image")
            .with_tag("Environment", "test")
            .with_regions(vec!["us-east-1", "us-west-2"]);

        assert_eq!(image.id, "ami-test");
        assert_eq!(image.tags.get("Environment"), Some(&"test".to_string()));
        assert_eq!(image.regions.len(), 2);
    }

    #[test]
    fn test_mock_template_renderer() {
        let renderer = MockTemplateRenderer::new();
        renderer.set_template("aws", "packer { ... }");

        let result = renderer.render("aws", &HashMap::new());
        assert!(result.is_some());
        assert!(result.unwrap().contains("packer"));
    }
}
