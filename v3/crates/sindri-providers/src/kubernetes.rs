//! Kubernetes provider implementation

use crate::templates::{TemplateContext, TemplateRegistry};
use crate::traits::Provider;
use crate::utils::{command_exists, get_command_version};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, ConnectionInfo, DeployOptions, DeployResult, DeploymentPlan, DeploymentState,
    DeploymentStatus, PlannedAction, PlannedResource, Prerequisite, PrerequisiteStatus,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Kubernetes provider for container orchestration
pub struct KubernetesProvider {
    /// Template registry for generating k8s manifests
    templates: TemplateRegistry,
    /// Output directory for generated files
    output_dir: PathBuf,
}

impl KubernetesProvider {
    /// Create a new Kubernetes provider
    pub fn new() -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to initialize templates"),
            output_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create with a specific output directory
    pub fn with_output_dir(output_dir: PathBuf) -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to initialize templates"),
            output_dir,
        }
    }

    /// Check if kind is available
    fn has_kind(&self) -> bool {
        command_exists("kind")
    }

    /// Check if k3d is available
    fn has_k3d(&self) -> bool {
        command_exists("k3d")
    }

    /// Detect existing kind clusters
    async fn detect_kind_clusters(&self) -> Vec<String> {
        let output = Command::new("kind")
            .args(["get", "clusters"])
            .output()
            .await;

        output
            .map(|o| {
                if o.status.success() {
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Detect existing k3d clusters
    async fn detect_k3d_clusters(&self) -> Vec<String> {
        let output = Command::new("k3d")
            .args(["cluster", "list", "-o", "json"])
            .output()
            .await;

        output
            .map(|o| {
                if o.status.success() {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    let clusters: Vec<K3dCluster> =
                        serde_json::from_str(&stdout).unwrap_or_default();
                    clusters.into_iter().map(|c| c.name).collect()
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Detect cluster type (kind/k3d/remote)
    async fn detect_cluster_type(&self) -> ClusterType {
        // Check current kubectl context
        let output = Command::new("kubectl")
            .args(["config", "current-context"])
            .output()
            .await;

        if let Ok(o) = output {
            if o.status.success() {
                let context = String::from_utf8_lossy(&o.stdout).trim().to_string();

                if context.starts_with("kind-") {
                    return ClusterType::Kind;
                } else if context.starts_with("k3d-") {
                    return ClusterType::K3d;
                }
            }
        }

        // Fallback: check if we have kind or k3d clusters
        if !self.detect_kind_clusters().await.is_empty() {
            return ClusterType::Kind;
        }
        if !self.detect_k3d_clusters().await.is_empty() {
            return ClusterType::K3d;
        }

        ClusterType::Remote
    }

    /// Get K8s configuration from SindriConfig
    fn get_k8s_config<'a>(&self, config: &'a SindriConfig) -> K8sDeployConfig<'a> {
        let file = config.inner();

        let k8s_config = file.providers.kubernetes.as_ref();

        // Namespace
        let namespace = k8s_config
            .map(|k| k.namespace.as_str())
            .unwrap_or("default");

        // Storage class
        let storage_class = k8s_config.and_then(|k| k.storage_class.as_deref());

        // Volume size
        let volume_size = file
            .deployment
            .volumes
            .workspace
            .as_ref()
            .map(|v| v.size.clone())
            .unwrap_or_else(|| "10Gi".to_string());

        // GPU configuration
        let gpu_enabled = file
            .deployment
            .resources
            .gpu
            .as_ref()
            .map(|g| g.enabled)
            .unwrap_or(false);

        K8sDeployConfig {
            name: &file.name,
            namespace,
            storage_class,
            volume_size,
            gpu_enabled,
            image: file.deployment.image.as_deref().unwrap_or("sindri:latest"),
        }
    }

    /// Generate Kubernetes manifests
    fn generate_manifests(&self, config: &SindriConfig, output_dir: &Path) -> Result<PathBuf> {
        let k8s_config = self.get_k8s_config(config);
        let mut context = TemplateContext::from_config(config, "none");

        // Add K8s-specific context
        let mut k8s_ctx = HashMap::new();
        k8s_ctx.insert(
            "namespace".to_string(),
            serde_json::json!(k8s_config.namespace),
        );
        if let Some(sc) = k8s_config.storage_class {
            k8s_ctx.insert("storage_class".to_string(), serde_json::json!(sc));
        }

        // Add node selector for GPU if needed
        if k8s_config.gpu_enabled {
            let mut node_selector = HashMap::new();
            node_selector.insert("gpu".to_string(), "nvidia".to_string());
            k8s_ctx.insert(
                "node_selector".to_string(),
                serde_json::json!(node_selector),
            );
        }

        context
            .env_vars
            .insert("k8s".to_string(), serde_json::to_string(&k8s_ctx)?);

        let manifest_content = self.templates.render("k8s-deployment.yaml", &context)?;
        let manifest_path = output_dir.join("k8s-deployment.yaml");

        std::fs::create_dir_all(output_dir)?;
        std::fs::write(&manifest_path, manifest_content)?;

        info!(
            "Generated k8s-deployment.yaml at {}",
            manifest_path.display()
        );
        Ok(manifest_path)
    }

    /// Load image to local cluster
    async fn load_image_to_cluster(
        &self,
        cluster_type: &ClusterType,
        image: &str,
        cluster_name: &str,
    ) -> Result<()> {
        match cluster_type {
            ClusterType::Kind => {
                info!("Loading image {} to kind cluster {}", image, cluster_name);
                let output = Command::new("kind")
                    .args(["load", "docker-image", image, "--name", cluster_name])
                    .output()
                    .await?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow!("Failed to load image to kind: {}", stderr));
                }
            }
            ClusterType::K3d => {
                info!("Importing image {} to k3d cluster {}", image, cluster_name);
                let output = Command::new("k3d")
                    .args(["image", "import", image, "-c", cluster_name])
                    .output()
                    .await?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow!("Failed to import image to k3d: {}", stderr));
                }
            }
            ClusterType::Remote => {
                warn!("Remote cluster detected - ensure image is pushed to a registry accessible by the cluster");
            }
        }

        Ok(())
    }

    /// Apply Kubernetes manifests
    async fn apply_manifests(&self, manifest_path: &Path, namespace: &str) -> Result<()> {
        info!(
            "Applying Kubernetes manifests from {}",
            manifest_path.display()
        );

        // Create namespace if it doesn't exist
        let _ = Command::new("kubectl")
            .args(["create", "namespace", namespace])
            .output()
            .await;

        // Apply manifests
        let output = Command::new("kubectl")
            .args([
                "apply",
                "-f",
                &manifest_path.to_string_lossy(),
                "-n",
                namespace,
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to apply Kubernetes manifests"));
        }

        Ok(())
    }

    /// Get pod name for deployment
    async fn get_pod_name(&self, name: &str, namespace: &str) -> Option<String> {
        let output = Command::new("kubectl")
            .args([
                "get",
                "pods",
                "-n",
                namespace,
                "-l",
                &format!("instance={}", name),
                "-o",
                "jsonpath={.items[0].metadata.name}",
            ])
            .output()
            .await
            .ok()?;

        if output.status.success() {
            let pod_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !pod_name.is_empty() {
                Some(pod_name)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get pod state
    async fn get_pod_state(&self, name: &str, namespace: &str) -> DeploymentState {
        let output = Command::new("kubectl")
            .args([
                "get",
                "pods",
                "-n",
                namespace,
                "-l",
                &format!("instance={}", name),
                "-o",
                "json",
            ])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let pod_list: Result<K8sPodList, _> = serde_json::from_str(&stdout);

                if let Ok(pods) = pod_list {
                    if let Some(pod) = pods.items.first() {
                        return match pod.status.phase.as_str() {
                            "Running" => DeploymentState::Running,
                            "Pending" => DeploymentState::Creating,
                            "Succeeded" => DeploymentState::Stopped,
                            "Failed" => DeploymentState::Error,
                            "Unknown" => DeploymentState::Unknown,
                            _ => DeploymentState::Unknown,
                        };
                    }
                }
                DeploymentState::NotDeployed
            }
            _ => DeploymentState::NotDeployed,
        }
    }

    /// Check if deployment exists
    async fn deployment_exists(&self, name: &str, namespace: &str) -> bool {
        let output = Command::new("kubectl")
            .args(["get", "deployment", name, "-n", namespace, "-o", "name"])
            .output()
            .await;

        output.map(|o| o.status.success()).unwrap_or(false)
    }

    /// Delete deployment and related resources
    async fn delete_deployment(&self, name: &str, namespace: &str) -> Result<()> {
        info!("Deleting deployment {} in namespace {}", name, namespace);

        // Delete deployment
        let _ = Command::new("kubectl")
            .args(["delete", "deployment", name, "-n", namespace])
            .output()
            .await;

        // Delete service
        let _ = Command::new("kubectl")
            .args(["delete", "service", name, "-n", namespace])
            .output()
            .await;

        // Delete PVC
        let pvc_name = format!("{}-home-pvc", name);
        let _ = Command::new("kubectl")
            .args(["delete", "pvc", &pvc_name, "-n", namespace])
            .output()
            .await;

        Ok(())
    }

    /// Scale deployment
    async fn scale_deployment(&self, name: &str, namespace: &str, replicas: u32) -> Result<()> {
        info!("Scaling deployment {} to {} replicas", name, replicas);

        let output = Command::new("kubectl")
            .args([
                "scale",
                "deployment",
                name,
                "-n",
                namespace,
                "--replicas",
                &replicas.to_string(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to scale deployment: {}", stderr));
        }

        Ok(())
    }

    /// Wait for pod to be ready
    async fn wait_for_pod(&self, name: &str, namespace: &str, timeout: u64) -> Result<()> {
        info!("Waiting for pod to be ready (timeout: {}s)", timeout);

        let output = Command::new("kubectl")
            .args([
                "wait",
                "pod",
                "-n",
                namespace,
                "-l",
                &format!("instance={}", name),
                "--for=condition=Ready",
                &format!("--timeout={}s", timeout),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Pod failed to become ready: {}", stderr));
        }

        Ok(())
    }

    /// Get cluster name from current context
    async fn get_current_cluster_name(&self) -> Option<String> {
        let output = Command::new("kubectl")
            .args(["config", "current-context"])
            .output()
            .await
            .ok()?;

        if output.status.success() {
            let context = String::from_utf8_lossy(&output.stdout).trim().to_string();

            // Extract cluster name from context
            if let Some(name) = context.strip_prefix("kind-") {
                Some(name.to_string())
            } else if let Some(name) = context.strip_prefix("k3d-") {
                Some(name.to_string())
            } else {
                Some(context)
            }
        } else {
            None
        }
    }
}

impl Default for KubernetesProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for KubernetesProvider {
    fn name(&self) -> &'static str {
        "kubernetes"
    }

    fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
        let mut missing = Vec::new();
        let mut available = Vec::new();

        // Check kubectl
        if command_exists("kubectl") {
            let version =
                get_command_version("kubectl", "version").unwrap_or_else(|_| "unknown".to_string());
            available.push(Prerequisite {
                name: "kubectl".to_string(),
                description: "Kubernetes CLI".to_string(),
                install_hint: None,
                version: Some(version),
            });
        } else {
            missing.push(Prerequisite {
                name: "kubectl".to_string(),
                description: "Kubernetes CLI".to_string(),
                install_hint: Some("Install: https://kubernetes.io/docs/tasks/tools/".to_string()),
                version: None,
            });
        }

        // Check for local cluster tools (optional)
        if self.has_kind() {
            let version =
                get_command_version("kind", "version").unwrap_or_else(|_| "unknown".to_string());
            available.push(Prerequisite {
                name: "kind".to_string(),
                description: "Kubernetes IN Docker".to_string(),
                install_hint: None,
                version: Some(version),
            });
        }

        if self.has_k3d() {
            let version =
                get_command_version("k3d", "version").unwrap_or_else(|_| "unknown".to_string());
            available.push(Prerequisite {
                name: "k3d".to_string(),
                description: "K3s in Docker".to_string(),
                install_hint: None,
                version: Some(version),
            });
        }

        Ok(PrerequisiteStatus {
            satisfied: missing.is_empty(),
            missing,
            available,
        })
    }

    async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
        let k8s_config = self.get_k8s_config(config);
        let name = k8s_config.name.to_string();
        info!("Deploying {} to Kubernetes", name);

        // Check prerequisites
        let prereqs = self.check_prerequisites()?;
        if !prereqs.satisfied {
            let missing_names: Vec<_> = prereqs.missing.iter().map(|p| p.name.as_str()).collect();
            return Err(anyhow!(
                "Missing prerequisites: {}",
                missing_names.join(", ")
            ));
        }

        // Detect cluster type
        let cluster_type = self.detect_cluster_type().await;
        debug!("Detected cluster type: {:?}", cluster_type);

        // Generate manifests
        let manifest_path = self.generate_manifests(config, &self.output_dir)?;

        if opts.dry_run {
            return Ok(DeployResult {
                success: true,
                name: name.clone(),
                provider: "kubernetes".to_string(),
                instance_id: None,
                connection: None,
                messages: vec![format!(
                    "Would deploy {} using manifests at {}",
                    name,
                    manifest_path.display()
                )],
                warnings: vec![],
            });
        }

        // Check if deployment already exists
        if self.deployment_exists(&name, k8s_config.namespace).await && !opts.force {
            return Err(anyhow!(
                "Deployment '{}' already exists in namespace '{}'. Use --force to recreate.",
                name,
                k8s_config.namespace
            ));
        }

        // Delete existing deployment if force
        if opts.force && self.deployment_exists(&name, k8s_config.namespace).await {
            info!("Removing existing deployment...");
            self.delete_deployment(&name, k8s_config.namespace).await?;
            // Wait a bit for resources to be cleaned up
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }

        // Load image to local cluster if needed
        if matches!(cluster_type, ClusterType::Kind | ClusterType::K3d) {
            if let Some(cluster_name) = self.get_current_cluster_name().await {
                self.load_image_to_cluster(&cluster_type, k8s_config.image, &cluster_name)
                    .await?;
            }
        }

        // Apply manifests
        self.apply_manifests(&manifest_path, k8s_config.namespace)
            .await?;

        // Wait for pod to be ready if requested
        if opts.wait {
            let timeout = opts.timeout.unwrap_or(300);
            self.wait_for_pod(&name, k8s_config.namespace, timeout)
                .await?;
        }

        // Get pod name for connection info
        let pod_name = self.get_pod_name(&name, k8s_config.namespace).await;

        Ok(DeployResult {
            success: true,
            name: name.clone(),
            provider: "kubernetes".to_string(),
            instance_id: pod_name.clone(),
            connection: Some(ConnectionInfo {
                ssh_command: pod_name.as_ref().map(|p| format!(
                    "kubectl exec -it {} -n {} -- /bin/bash",
                    p, k8s_config.namespace
                )),
                http_url: None,
                https_url: None,
                instructions: Some(format!(
                    "Connect with:\n  sindri connect\n  kubectl exec -it <pod-name> -n {} -- /bin/bash",
                    k8s_config.namespace
                )),
            }),
            messages: vec![format!(
                "Deployment '{}' created successfully in namespace '{}'",
                name, k8s_config.namespace
            )],
            warnings: vec![],
        })
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        let k8s_config = self.get_k8s_config(config);
        let name = k8s_config.name;
        info!("Connecting to {} in Kubernetes", name);

        // Check if deployment exists
        if !self.deployment_exists(name, k8s_config.namespace).await {
            return Err(anyhow!(
                "Deployment '{}' not found in namespace '{}'. Deploy first: sindri deploy",
                name,
                k8s_config.namespace
            ));
        }

        // Get pod name
        let pod_name = self
            .get_pod_name(name, k8s_config.namespace)
            .await
            .ok_or_else(|| anyhow!("No pods found for deployment '{}'", name))?;

        // Check pod state
        let state = self.get_pod_state(name, k8s_config.namespace).await;
        if !matches!(state, DeploymentState::Running) {
            return Err(anyhow!(
                "Pod is not running (state: {:?}). Wait for it to be ready first.",
                state
            ));
        }

        // Connect via kubectl exec
        let status = Command::new("kubectl")
            .args([
                "exec",
                "-it",
                &pod_name,
                "-n",
                k8s_config.namespace,
                "--",
                "/bin/bash",
            ])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Failed to connect to pod"));
        }

        Ok(())
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        let k8s_config = self.get_k8s_config(config);
        let name = k8s_config.name.to_string();
        info!("Getting status for {} in Kubernetes", name);

        let state = self.get_pod_state(&name, k8s_config.namespace).await;
        let pod_name = self.get_pod_name(&name, k8s_config.namespace).await;

        // Get pod details if exists
        let mut details = HashMap::new();
        if let Some(ref pn) = pod_name {
            let output = Command::new("kubectl")
                .args(["get", "pod", pn, "-n", k8s_config.namespace, "-o", "json"])
                .output()
                .await;

            if let Ok(o) = output {
                if o.status.success() {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    if let Ok(pod) = serde_json::from_str::<K8sPod>(&stdout) {
                        if let Some(node) = pod.spec.node_name {
                            details.insert("node".to_string(), node);
                        }
                        details.insert("phase".to_string(), pod.status.phase);
                    }
                }
            }
        }

        details.insert("namespace".to_string(), k8s_config.namespace.to_string());

        Ok(DeploymentStatus {
            name,
            provider: "kubernetes".to_string(),
            state,
            instance_id: pod_name,
            image: config.image().map(|s| s.to_string()),
            addresses: vec![],
            resources: None,
            timestamps: Default::default(),
            details,
        })
    }

    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
        let k8s_config = self.get_k8s_config(config);
        let name = k8s_config.name;
        info!("Destroying {} in Kubernetes (force: {})", name, force);

        if !self.deployment_exists(name, k8s_config.namespace).await {
            warn!(
                "Deployment '{}' not found in namespace '{}'",
                name, k8s_config.namespace
            );
            return Ok(());
        }

        self.delete_deployment(name, k8s_config.namespace).await?;

        // Remove generated manifests
        let manifest_path = self.output_dir.join("k8s-deployment.yaml");
        if manifest_path.exists() {
            std::fs::remove_file(manifest_path)?;
        }

        info!("Deployment '{}' destroyed", name);
        Ok(())
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        let k8s_config = self.get_k8s_config(config);
        let name = k8s_config.name.to_string();
        info!("Planning Kubernetes deployment for {}", name);

        let mut actions = vec![PlannedAction {
            action: ActionType::Create,
            resource: "k8s-deployment.yaml".to_string(),
            description: "Generate Kubernetes manifests".to_string(),
        }];

        if !self.deployment_exists(&name, k8s_config.namespace).await {
            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: format!("namespace:{}", k8s_config.namespace),
                description: format!("Create namespace '{}'", k8s_config.namespace),
            });

            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: format!("pvc:{}-home-pvc", name),
                description: format!(
                    "Create persistent volume claim ({})",
                    k8s_config.volume_size
                ),
            });

            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: format!("deployment:{}", name),
                description: "Create Kubernetes deployment".to_string(),
            });

            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: format!("service:{}", name),
                description: "Create Kubernetes service".to_string(),
            });
        }

        let resources = vec![
            PlannedResource {
                resource_type: "deployment".to_string(),
                name: name.clone(),
                config: {
                    let mut m = HashMap::new();
                    m.insert(
                        "namespace".to_string(),
                        serde_json::json!(k8s_config.namespace),
                    );
                    m.insert("replicas".to_string(), serde_json::json!(1));
                    m.insert("image".to_string(), serde_json::json!(k8s_config.image));
                    m
                },
            },
            PlannedResource {
                resource_type: "pvc".to_string(),
                name: format!("{}-home-pvc", name),
                config: {
                    let mut m = HashMap::new();
                    m.insert(
                        "size".to_string(),
                        serde_json::json!(k8s_config.volume_size),
                    );
                    if let Some(sc) = k8s_config.storage_class {
                        m.insert("storageClass".to_string(), serde_json::json!(sc));
                    }
                    m
                },
            },
        ];

        Ok(DeploymentPlan {
            provider: "kubernetes".to_string(),
            actions,
            resources,
            estimated_cost: None,
        })
    }

    async fn start(&self, config: &SindriConfig) -> Result<()> {
        let k8s_config = self.get_k8s_config(config);
        let name = k8s_config.name;
        info!("Starting {} in Kubernetes", name);

        if !self.deployment_exists(name, k8s_config.namespace).await {
            return Err(anyhow!(
                "Deployment '{}' not found. Deploy first: sindri deploy",
                name
            ));
        }

        // Scale up to 1 replica
        self.scale_deployment(name, k8s_config.namespace, 1).await?;

        Ok(())
    }

    async fn stop(&self, config: &SindriConfig) -> Result<()> {
        let k8s_config = self.get_k8s_config(config);
        let name = k8s_config.name;
        info!("Stopping {} in Kubernetes", name);

        if !self.deployment_exists(name, k8s_config.namespace).await {
            return Err(anyhow!("Deployment '{}' not found", name));
        }

        // Scale down to 0 replicas
        self.scale_deployment(name, k8s_config.namespace, 0).await?;

        Ok(())
    }

    fn supports_gpu(&self) -> bool {
        true // K8s can support GPU via node selectors
    }
}

/// Cluster type detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClusterType {
    Kind,
    K3d,
    Remote,
}

/// K8s deployment configuration
struct K8sDeployConfig<'a> {
    name: &'a str,
    namespace: &'a str,
    storage_class: Option<&'a str>,
    volume_size: String,
    gpu_enabled: bool,
    image: &'a str,
}

/// k3d cluster info from JSON
#[derive(Debug, Deserialize)]
struct K3dCluster {
    name: String,
}

/// Kubernetes pod list
#[derive(Debug, Deserialize)]
struct K8sPodList {
    items: Vec<K8sPod>,
}

/// Kubernetes pod
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct K8sPod {
    metadata: K8sPodMetadata,
    spec: K8sPodSpec,
    status: K8sPodStatus,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct K8sPodMetadata {
    name: String,
}

#[derive(Debug, Deserialize)]
struct K8sPodSpec {
    #[serde(rename = "nodeName")]
    node_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct K8sPodStatus {
    phase: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kubernetes_provider_creation() {
        let provider = KubernetesProvider::new();
        assert_eq!(provider.name(), "kubernetes");
    }

    #[test]
    fn test_kubernetes_supports_gpu() {
        let provider = KubernetesProvider::new();
        assert!(provider.supports_gpu());
    }
}
