//! Retry delay strategies and predicates
//!
//! This module implements different retry backoff strategies and provides
//! a trait for determining whether an error should be retried.

use crate::types::{RetryPolicy, RetryStrategy};
use rand::RngExt;
use std::error::Error;
use std::time::Duration;

/// Calculate the delay before the next retry attempt
///
/// # Arguments
///
/// * `policy` - The retry policy containing strategy and timing parameters
/// * `attempt` - The current attempt number (1-indexed)
/// * `jitter` - Whether to apply random jitter to the delay
///
/// # Returns
///
/// The duration to wait before the next retry attempt
///
/// # Example
///
/// ```rust
/// use sindri_core::retry::calculate_delay;
/// use sindri_core::types::{RetryPolicy, RetryStrategy};
///
/// let policy = RetryPolicy {
///     max_attempts: 3,
///     strategy: RetryStrategy::ExponentialBackoff,
///     backoff_multiplier: 2.0,
///     initial_delay_ms: 1000,
///     max_delay_ms: 30000,
/// };
///
/// let delay = calculate_delay(&policy, 1, false);
/// assert_eq!(delay.as_millis(), 1000);
///
/// let delay = calculate_delay(&policy, 2, false);
/// assert_eq!(delay.as_millis(), 2000);
/// ```
pub fn calculate_delay(policy: &RetryPolicy, attempt: u32, jitter: bool) -> Duration {
    // Attempt is 1-indexed, but we want 0-indexed for calculations
    let attempt_index = attempt.saturating_sub(1);

    let base_delay_ms = match policy.strategy {
        RetryStrategy::None => 0,

        RetryStrategy::FixedDelay => policy.initial_delay_ms,

        RetryStrategy::ExponentialBackoff => {
            let multiplier = policy.backoff_multiplier.powf(attempt_index as f64);
            (policy.initial_delay_ms as f64 * multiplier) as u64
        }

        RetryStrategy::LinearBackoff => policy.initial_delay_ms * (attempt_index as u64 + 1),
    };

    // Apply max delay cap
    let capped_delay_ms = base_delay_ms.min(policy.max_delay_ms);

    // Apply jitter if requested (adds up to 25% random variation)
    let final_delay_ms = if jitter && capped_delay_ms > 0 {
        let jitter_range = capped_delay_ms / 4;
        let jitter_value = rand::rng().random_range(0..=jitter_range);
        capped_delay_ms + jitter_value
    } else {
        capped_delay_ms
    };

    Duration::from_millis(final_delay_ms)
}

/// A predicate that determines whether an error should be retried
///
/// Implement this trait to customize which errors are retryable. By default,
/// all errors are considered retryable. Use this to short-circuit retries
/// for known non-recoverable errors.
///
/// # Example
///
/// ```rust
/// use sindri_core::retry::RetryPredicate;
/// use std::io::{Error, ErrorKind};
///
/// struct IoRetryPredicate;
///
/// impl RetryPredicate<Error> for IoRetryPredicate {
///     fn should_retry(&self, error: &Error) -> bool {
///         // Don't retry permanent errors
///         !matches!(
///             error.kind(),
///             ErrorKind::NotFound | ErrorKind::PermissionDenied | ErrorKind::InvalidInput
///         )
///     }
/// }
/// ```
pub trait RetryPredicate<E: ?Sized>: Send + Sync {
    /// Determine whether the given error should be retried
    fn should_retry(&self, error: &E) -> bool;
}

/// A predicate that always returns true (all errors are retryable)
#[derive(Debug, Clone, Copy, Default)]
pub struct AlwaysRetry;

impl<E: ?Sized> RetryPredicate<E> for AlwaysRetry {
    fn should_retry(&self, _error: &E) -> bool {
        true
    }
}

/// A predicate that never retries (no errors are retryable)
#[derive(Debug, Clone, Copy)]
pub struct NeverRetry;

impl<E: ?Sized> RetryPredicate<E> for NeverRetry {
    fn should_retry(&self, _error: &E) -> bool {
        false
    }
}

/// A predicate that uses a closure to determine retryability
pub struct ClosurePredicate<F> {
    predicate: F,
}

impl<F> ClosurePredicate<F> {
    /// Create a new closure-based predicate
    pub fn new(predicate: F) -> Self {
        Self { predicate }
    }
}

impl<E, F> RetryPredicate<E> for ClosurePredicate<F>
where
    F: Fn(&E) -> bool + Send + Sync,
{
    fn should_retry(&self, error: &E) -> bool {
        (self.predicate)(error)
    }
}

/// A predicate for HTTP status codes
#[derive(Debug, Clone)]
pub struct HttpStatusPredicate {
    /// Status codes that should be retried
    retryable_codes: Vec<u16>,
}

impl HttpStatusPredicate {
    /// Create a predicate with default retryable status codes
    ///
    /// Default retryable codes: 408, 425, 429, 500, 502, 503, 504
    pub fn default_http() -> Self {
        Self {
            retryable_codes: vec![408, 425, 429, 500, 502, 503, 504],
        }
    }

    /// Create a predicate with custom retryable status codes
    pub fn with_codes(codes: Vec<u16>) -> Self {
        Self {
            retryable_codes: codes,
        }
    }

    /// Check if a status code is retryable
    pub fn is_retryable_code(&self, code: u16) -> bool {
        self.retryable_codes.contains(&code)
    }
}

/// A trait for errors that contain HTTP status information
pub trait HttpStatusError {
    /// Get the HTTP status code if available
    fn status_code(&self) -> Option<u16>;
}

impl<E: HttpStatusError> RetryPredicate<E> for HttpStatusPredicate {
    fn should_retry(&self, error: &E) -> bool {
        error
            .status_code()
            .map(|code| self.is_retryable_code(code))
            .unwrap_or(true) // If no status code, assume retryable
    }
}

/// A predicate that retries only on specific error messages
#[derive(Debug, Clone)]
pub struct MessagePredicate {
    /// Patterns that indicate retryable errors
    retryable_patterns: Vec<String>,
}

impl MessagePredicate {
    /// Create a new message predicate with the given patterns
    pub fn new(patterns: Vec<String>) -> Self {
        Self {
            retryable_patterns: patterns,
        }
    }

    /// Create a predicate for common network errors
    pub fn network_errors() -> Self {
        Self::new(vec![
            "timeout".to_string(),
            "timed out".to_string(),
            "connection reset".to_string(),
            "connection refused".to_string(),
            "network unreachable".to_string(),
            "temporary failure".to_string(),
        ])
    }
}

impl<E: Error> RetryPredicate<E> for MessagePredicate {
    fn should_retry(&self, error: &E) -> bool {
        let error_msg = error.to_string().to_lowercase();
        self.retryable_patterns
            .iter()
            .any(|pattern| error_msg.contains(&pattern.to_lowercase()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_none_strategy() {
        let policy = RetryPolicy {
            max_attempts: 3,
            strategy: RetryStrategy::None,
            backoff_multiplier: 2.0,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        };

        assert_eq!(calculate_delay(&policy, 1, false), Duration::ZERO);
        assert_eq!(calculate_delay(&policy, 2, false), Duration::ZERO);
        assert_eq!(calculate_delay(&policy, 3, false), Duration::ZERO);
    }

    #[test]
    fn test_fixed_strategy() {
        let policy = RetryPolicy {
            max_attempts: 3,
            strategy: RetryStrategy::FixedDelay,
            backoff_multiplier: 2.0,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        };

        assert_eq!(
            calculate_delay(&policy, 1, false),
            Duration::from_millis(1000)
        );
        assert_eq!(
            calculate_delay(&policy, 2, false),
            Duration::from_millis(1000)
        );
        assert_eq!(
            calculate_delay(&policy, 3, false),
            Duration::from_millis(1000)
        );
    }

    #[test]
    fn test_exponential_strategy() {
        let policy = RetryPolicy {
            max_attempts: 5,
            strategy: RetryStrategy::ExponentialBackoff,
            backoff_multiplier: 2.0,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        };

        // attempt 1: 1000 * 2^0 = 1000
        assert_eq!(
            calculate_delay(&policy, 1, false),
            Duration::from_millis(1000)
        );
        // attempt 2: 1000 * 2^1 = 2000
        assert_eq!(
            calculate_delay(&policy, 2, false),
            Duration::from_millis(2000)
        );
        // attempt 3: 1000 * 2^2 = 4000
        assert_eq!(
            calculate_delay(&policy, 3, false),
            Duration::from_millis(4000)
        );
        // attempt 4: 1000 * 2^3 = 8000
        assert_eq!(
            calculate_delay(&policy, 4, false),
            Duration::from_millis(8000)
        );
        // attempt 5: 1000 * 2^4 = 16000
        assert_eq!(
            calculate_delay(&policy, 5, false),
            Duration::from_millis(16000)
        );
    }

    #[test]
    fn test_linear_strategy() {
        let policy = RetryPolicy {
            max_attempts: 3,
            strategy: RetryStrategy::LinearBackoff,
            backoff_multiplier: 2.0,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        };

        // attempt 1: 1000 * 1 = 1000
        assert_eq!(
            calculate_delay(&policy, 1, false),
            Duration::from_millis(1000)
        );
        // attempt 2: 1000 * 2 = 2000
        assert_eq!(
            calculate_delay(&policy, 2, false),
            Duration::from_millis(2000)
        );
        // attempt 3: 1000 * 3 = 3000
        assert_eq!(
            calculate_delay(&policy, 3, false),
            Duration::from_millis(3000)
        );
    }

    #[test]
    fn test_max_delay_cap() {
        let policy = RetryPolicy {
            max_attempts: 10,
            strategy: RetryStrategy::ExponentialBackoff,
            backoff_multiplier: 2.0,
            initial_delay_ms: 1000,
            max_delay_ms: 5000,
        };

        // attempt 5: 1000 * 2^4 = 16000, but capped at 5000
        assert_eq!(
            calculate_delay(&policy, 5, false),
            Duration::from_millis(5000)
        );
    }

    #[test]
    fn test_jitter_bounds() {
        let policy = RetryPolicy {
            max_attempts: 3,
            strategy: RetryStrategy::FixedDelay,
            backoff_multiplier: 2.0,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        };

        // With jitter, delay should be between base and base + 25%
        for _ in 0..100 {
            let delay = calculate_delay(&policy, 1, true);
            assert!(delay >= Duration::from_millis(1000));
            assert!(delay <= Duration::from_millis(1250)); // base + 25%
        }
    }

    #[test]
    fn test_jitter_no_effect_on_zero_delay() {
        let policy = RetryPolicy {
            max_attempts: 3,
            strategy: RetryStrategy::None,
            backoff_multiplier: 2.0,
            initial_delay_ms: 0,
            max_delay_ms: 30000,
        };

        // Jitter should not affect zero delay
        assert_eq!(calculate_delay(&policy, 1, true), Duration::ZERO);
    }

    #[test]
    fn test_always_retry_predicate() {
        let predicate = AlwaysRetry;
        let error = io::Error::new(io::ErrorKind::NotFound, "not found");

        assert!(predicate.should_retry(&error));
    }

    #[test]
    fn test_never_retry_predicate() {
        let predicate = NeverRetry;
        let error = io::Error::new(io::ErrorKind::TimedOut, "timeout");

        assert!(!predicate.should_retry(&error));
    }

    #[test]
    fn test_closure_predicate() {
        let predicate = ClosurePredicate::new(|err: &io::Error| {
            matches!(
                err.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
            )
        });

        let timeout_err = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        let not_found_err = io::Error::new(io::ErrorKind::NotFound, "not found");

        assert!(predicate.should_retry(&timeout_err));
        assert!(!predicate.should_retry(&not_found_err));
    }

    #[test]
    fn test_http_status_predicate() {
        let predicate = HttpStatusPredicate::default_http();

        // Default retryable codes
        assert!(predicate.is_retryable_code(408)); // Request Timeout
        assert!(predicate.is_retryable_code(429)); // Too Many Requests
        assert!(predicate.is_retryable_code(500)); // Internal Server Error
        assert!(predicate.is_retryable_code(502)); // Bad Gateway
        assert!(predicate.is_retryable_code(503)); // Service Unavailable
        assert!(predicate.is_retryable_code(504)); // Gateway Timeout

        // Non-retryable codes
        assert!(!predicate.is_retryable_code(400)); // Bad Request
        assert!(!predicate.is_retryable_code(401)); // Unauthorized
        assert!(!predicate.is_retryable_code(404)); // Not Found
    }

    #[test]
    fn test_message_predicate() {
        let predicate = MessagePredicate::network_errors();

        let timeout_err = io::Error::new(io::ErrorKind::TimedOut, "connection timed out");
        let not_found_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let reset_err = io::Error::new(io::ErrorKind::ConnectionReset, "connection reset by peer");

        assert!(predicate.should_retry(&timeout_err));
        assert!(!predicate.should_retry(&not_found_err));
        assert!(predicate.should_retry(&reset_err));
    }
}
