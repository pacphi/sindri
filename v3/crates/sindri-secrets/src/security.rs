//! Security utilities for secrets management
//!
//! Provides:
//! - SecureString with zeroize
//! - Audit logging (never logs secret values)
//! - Error sanitization

use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A secure string that is automatically zeroed on drop
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecureString {
    inner: String,
}

impl SecureString {
    /// Create a new secure string
    pub fn new(value: String) -> Self {
        Self { inner: value }
    }

    /// Get the string value (use with caution)
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Convert to owned String (consumes self)
    pub fn into_string(mut self) -> String {
        // Take ownership and replace with empty string before drop
        std::mem::take(&mut self.inner)
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl From<String> for SecureString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecureString {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

impl fmt::Debug for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureString([REDACTED {} bytes])", self.len())
    }
}

impl fmt::Display for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// Audit log entry for secret operations
#[derive(Debug, Clone)]
pub struct AuditLog {
    pub operation: String,
    pub secret_name: String,
    pub source: String,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: std::time::SystemTime,
}

impl AuditLog {
    pub fn new(operation: String, secret_name: String, source: String) -> Self {
        Self {
            operation,
            secret_name,
            source,
            success: true,
            error: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.success = false;
        self.error = Some(error);
        self
    }

    /// Log the audit entry (never logs secret values)
    pub fn log(&self) {
        if self.success {
            tracing::info!(
                operation = %self.operation,
                secret_name = %self.secret_name,
                source = %self.source,
                timestamp = ?self.timestamp,
                "Secret operation successful"
            );
        } else {
            tracing::warn!(
                operation = %self.operation,
                secret_name = %self.secret_name,
                source = %self.source,
                error = ?self.error,
                timestamp = ?self.timestamp,
                "Secret operation failed"
            );
        }
    }
}

/// Sanitize error messages to remove potential secret values
pub fn sanitize_error(error: &str) -> String {
    // Remove anything that looks like it could be a secret
    // This is a simple heuristic - replace with more sophisticated logic if needed

    let patterns_to_redact = [
        // Environment variable assignments
        (r"=([^\s]+)", "=[REDACTED]"),
        // Tokens and keys
        (r"token[=:]\s*([^\s]+)", "token=[REDACTED]"),
        (r"key[=:]\s*([^\s]+)", "key=[REDACTED]"),
        (r"password[=:]\s*([^\s]+)", "password=[REDACTED]"),
        (r"secret[=:]\s*([^\s]+)", "secret=[REDACTED]"),
        // Base64-looking strings (48+ chars of base64 characters)
        (r"[A-Za-z0-9+/]{48,}={0,2}", "[REDACTED_BASE64]"),
    ];

    let mut sanitized = error.to_string();

    for (pattern, replacement) in patterns_to_redact {
        if let Ok(re) = regex::Regex::new(pattern) {
            sanitized = re.replace_all(&sanitized, replacement).to_string();
        }
    }

    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_string_zeroize() {
        let secure = SecureString::new("sensitive-data".to_string());
        assert_eq!(secure.as_str(), "sensitive-data");
        assert_eq!(secure.len(), 14);

        // Drop should zero the memory
        drop(secure);
        // Can't directly test memory is zeroed, but ZeroizeOnDrop ensures it
    }

    #[test]
    fn test_secure_string_debug() {
        let secure = SecureString::new("secret".to_string());
        let debug_str = format!("{:?}", secure);

        assert!(debug_str.contains("REDACTED"));
        assert!(!debug_str.contains("secret"));
    }

    #[test]
    fn test_secure_string_display() {
        let secure = SecureString::new("secret".to_string());
        let display_str = format!("{}", secure);

        assert_eq!(display_str, "[REDACTED]");
    }

    #[test]
    fn test_audit_log_success() {
        let log = AuditLog::new(
            "resolve".to_string(),
            "TEST_SECRET".to_string(),
            "env".to_string(),
        );

        assert!(log.success);
        assert!(log.error.is_none());
    }

    #[test]
    fn test_audit_log_error() {
        let log = AuditLog::new(
            "resolve".to_string(),
            "TEST_SECRET".to_string(),
            "vault".to_string(),
        )
        .with_error("Connection failed".to_string());

        assert!(!log.success);
        assert_eq!(log.error, Some("Connection failed".to_string()));
    }
}
