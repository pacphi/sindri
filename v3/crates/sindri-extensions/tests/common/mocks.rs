//! Mock implementations for testing
//!
//! Provides mock implementations of extension components for testing
//! without side effects (filesystem, network, process execution).

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Mock command execution result
#[derive(Clone, Debug)]
pub struct MockCommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl MockCommandResult {
    pub fn success(stdout: &str) -> Self {
        Self {
            stdout: stdout.to_string(),
            stderr: String::new(),
            exit_code: 0,
        }
    }

    pub fn failure(stderr: &str, exit_code: i32) -> Self {
        Self {
            stdout: String::new(),
            stderr: stderr.to_string(),
            exit_code,
        }
    }
}

/// Mock command executor for testing
pub struct MockExecutor {
    /// Pre-configured command responses
    responses: Arc<Mutex<HashMap<String, MockCommandResult>>>,
    /// Recorded command invocations
    invocations: Arc<Mutex<Vec<MockCommandInvocation>>>,
    /// Default response for unknown commands
    default_response: MockCommandResult,
}

/// Record of a command invocation
#[derive(Clone, Debug)]
pub struct MockCommandInvocation {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

impl Default for MockExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl MockExecutor {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            invocations: Arc::new(Mutex::new(Vec::new())),
            default_response: MockCommandResult::success(""),
        }
    }

    /// Set a mock response for a specific command
    pub fn mock_command(&self, command: &str, result: MockCommandResult) {
        self.responses
            .lock()
            .unwrap()
            .insert(command.to_string(), result);
    }

    /// Mock a successful command
    pub fn mock_success(&self, command: &str, stdout: &str) {
        self.mock_command(command, MockCommandResult::success(stdout));
    }

    /// Mock a failed command
    pub fn mock_failure(&self, command: &str, stderr: &str, exit_code: i32) {
        self.mock_command(command, MockCommandResult::failure(stderr, exit_code));
    }

    /// Set the default response for unknown commands
    pub fn set_default_response(&mut self, result: MockCommandResult) {
        self.default_response = result;
    }

    /// Execute a mock command
    pub fn execute(
        &self,
        command: &str,
        args: &[&str],
        cwd: Option<&Path>,
        env: &HashMap<String, String>,
    ) -> MockCommandResult {
        // Record invocation
        self.invocations
            .lock()
            .unwrap()
            .push(MockCommandInvocation {
                command: command.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                cwd: cwd.map(|p| p.to_path_buf()),
                env: env.clone(),
            });

        // Return mock response or default
        let key = format!("{} {}", command, args.join(" "));
        let responses = self.responses.lock().unwrap();

        // Try full command match first
        if let Some(result) = responses.get(&key) {
            return result.clone();
        }

        // Try command-only match
        if let Some(result) = responses.get(command) {
            return result.clone();
        }

        self.default_response.clone()
    }

    /// Get all recorded invocations
    pub fn get_invocations(&self) -> Vec<MockCommandInvocation> {
        self.invocations.lock().unwrap().clone()
    }

    /// Check if a command was invoked
    pub fn was_invoked(&self, command: &str) -> bool {
        self.invocations
            .lock()
            .unwrap()
            .iter()
            .any(|i| i.command == command)
    }

    /// Get invocation count for a command
    pub fn invocation_count(&self, command: &str) -> usize {
        self.invocations
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.command == command)
            .count()
    }

    /// Clear recorded invocations
    pub fn clear_invocations(&self) {
        self.invocations.lock().unwrap().clear();
    }

    /// Clear all mocks and invocations
    pub fn reset(&self) {
        self.responses.lock().unwrap().clear();
        self.invocations.lock().unwrap().clear();
    }
}

/// Mock filesystem for testing
pub struct MockFilesystem {
    /// Virtual files
    files: Arc<Mutex<HashMap<PathBuf, Vec<u8>>>>,
    /// Virtual directories
    directories: Arc<Mutex<Vec<PathBuf>>>,
    /// File read tracking
    reads: Arc<Mutex<Vec<PathBuf>>>,
    /// File write tracking
    writes: Arc<Mutex<Vec<PathBuf>>>,
}

impl Default for MockFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

impl MockFilesystem {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            directories: Arc::new(Mutex::new(Vec::new())),
            reads: Arc::new(Mutex::new(Vec::new())),
            writes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a virtual file
    pub fn add_file(&self, path: impl AsRef<Path>, content: impl AsRef<[u8]>) {
        self.files
            .lock()
            .unwrap()
            .insert(path.as_ref().to_path_buf(), content.as_ref().to_vec());
    }

    /// Add a virtual directory
    pub fn add_directory(&self, path: impl AsRef<Path>) {
        self.directories
            .lock()
            .unwrap()
            .push(path.as_ref().to_path_buf());
    }

    /// Check if file exists
    pub fn file_exists(&self, path: impl AsRef<Path>) -> bool {
        self.files.lock().unwrap().contains_key(path.as_ref())
    }

    /// Check if directory exists
    pub fn directory_exists(&self, path: impl AsRef<Path>) -> bool {
        self.directories
            .lock()
            .unwrap()
            .iter()
            .any(|d| d == path.as_ref())
    }

    /// Read file content
    pub fn read_file(&self, path: impl AsRef<Path>) -> Option<Vec<u8>> {
        self.reads.lock().unwrap().push(path.as_ref().to_path_buf());
        self.files.lock().unwrap().get(path.as_ref()).cloned()
    }

    /// Write file content
    pub fn write_file(&self, path: impl AsRef<Path>, content: impl AsRef<[u8]>) {
        self.writes
            .lock()
            .unwrap()
            .push(path.as_ref().to_path_buf());
        self.files
            .lock()
            .unwrap()
            .insert(path.as_ref().to_path_buf(), content.as_ref().to_vec());
    }

    /// Get all read operations
    pub fn get_reads(&self) -> Vec<PathBuf> {
        self.reads.lock().unwrap().clone()
    }

    /// Get all write operations
    pub fn get_writes(&self) -> Vec<PathBuf> {
        self.writes.lock().unwrap().clone()
    }

    /// Check if a file was read
    pub fn was_read(&self, path: impl AsRef<Path>) -> bool {
        self.reads
            .lock()
            .unwrap()
            .iter()
            .any(|p| p == path.as_ref())
    }

    /// Check if a file was written
    pub fn was_written(&self, path: impl AsRef<Path>) -> bool {
        self.writes
            .lock()
            .unwrap()
            .iter()
            .any(|p| p == path.as_ref())
    }

    /// Reset all state
    pub fn reset(&self) {
        self.files.lock().unwrap().clear();
        self.directories.lock().unwrap().clear();
        self.reads.lock().unwrap().clear();
        self.writes.lock().unwrap().clear();
    }
}

/// Mock network client for testing downloads
pub struct MockNetworkClient {
    /// Pre-configured URL responses
    responses: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// Recorded requests
    requests: Arc<Mutex<Vec<String>>>,
}

impl Default for MockNetworkClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockNetworkClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Mock a URL response
    pub fn mock_url(&self, url: &str, content: impl AsRef<[u8]>) {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), content.as_ref().to_vec());
    }

    /// Simulate a download
    pub fn download(&self, url: &str) -> Option<Vec<u8>> {
        self.requests.lock().unwrap().push(url.to_string());
        self.responses.lock().unwrap().get(url).cloned()
    }

    /// Get all recorded requests
    pub fn get_requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    /// Check if a URL was requested
    pub fn was_requested(&self, url: &str) -> bool {
        self.requests.lock().unwrap().iter().any(|u| u == url)
    }

    /// Reset state
    pub fn reset(&self) {
        self.responses.lock().unwrap().clear();
        self.requests.lock().unwrap().clear();
    }
}

/// Mock hook execution tracker
pub struct MockHookTracker {
    /// Executed hooks (extension_name, hook_type)
    executed: Arc<Mutex<Vec<(String, String)>>>,
}

impl Default for MockHookTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl MockHookTracker {
    pub fn new() -> Self {
        Self {
            executed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record hook execution
    pub fn record(&self, extension: &str, hook_type: &str) {
        self.executed
            .lock()
            .unwrap()
            .push((extension.to_string(), hook_type.to_string()));
    }

    /// Check if a hook was executed
    pub fn was_executed(&self, extension: &str, hook_type: &str) -> bool {
        self.executed
            .lock()
            .unwrap()
            .iter()
            .any(|(e, h)| e == extension && h == hook_type)
    }

    /// Get all executed hooks
    pub fn get_executed(&self) -> Vec<(String, String)> {
        self.executed.lock().unwrap().clone()
    }

    /// Get hook execution order
    pub fn execution_order(&self) -> Vec<String> {
        self.executed
            .lock()
            .unwrap()
            .iter()
            .map(|(e, h)| format!("{}:{}", e, h))
            .collect()
    }

    /// Reset state
    pub fn reset(&self) {
        self.executed.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_executor() {
        let executor = MockExecutor::new();
        executor.mock_success("echo", "hello world");

        let result = executor.execute("echo", &["test"], None, &HashMap::new());
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "hello world");
        assert!(executor.was_invoked("echo"));
    }

    #[test]
    fn test_mock_executor_failure() {
        let executor = MockExecutor::new();
        executor.mock_failure("fail-cmd", "error message", 1);

        let result = executor.execute("fail-cmd", &[], None, &HashMap::new());
        assert_eq!(result.exit_code, 1);
        assert_eq!(result.stderr, "error message");
    }

    #[test]
    fn test_mock_filesystem() {
        let fs = MockFilesystem::new();
        fs.add_file("/test/file.txt", "content");
        fs.add_directory("/test");

        assert!(fs.file_exists("/test/file.txt"));
        assert!(fs.directory_exists("/test"));
        assert!(!fs.file_exists("/nonexistent"));

        let content = fs.read_file("/test/file.txt").unwrap();
        assert_eq!(content, b"content");
        assert!(fs.was_read("/test/file.txt"));
    }

    #[test]
    fn test_mock_network() {
        let client = MockNetworkClient::new();
        client.mock_url("https://example.com/file", "content");

        let content = client.download("https://example.com/file").unwrap();
        assert_eq!(content, b"content");
        assert!(client.was_requested("https://example.com/file"));
    }

    #[test]
    fn test_mock_hook_tracker() {
        let tracker = MockHookTracker::new();
        tracker.record("ext1", "pre-install");
        tracker.record("ext1", "post-install");

        assert!(tracker.was_executed("ext1", "pre-install"));
        assert!(tracker.was_executed("ext1", "post-install"));
        assert!(!tracker.was_executed("ext1", "pre-remove"));

        let order = tracker.execution_order();
        assert_eq!(order, vec!["ext1:pre-install", "ext1:post-install"]);
    }
}
