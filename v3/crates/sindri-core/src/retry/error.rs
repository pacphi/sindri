//! Error types for the retry execution engine
//!
//! This module defines error types that can occur during retry operations,
//! including exhausted retries, cancellation, and timeout conditions.

use std::error::Error;
use std::fmt;
use std::time::Duration;

/// Errors that can occur during retry execution
///
/// The error type is generic over `E`, the underlying error type from the
/// operation being retried.
#[derive(Debug)]
pub enum RetryError<E> {
    /// All retry attempts have been exhausted
    ///
    /// This variant is returned when the maximum number of attempts has been
    /// reached and the operation still failed.
    Exhausted {
        /// Number of attempts made before giving up
        attempts: u32,
        /// The error from the final attempt
        source: E,
        /// Total duration spent across all attempts
        total_duration: Duration,
    },

    /// The retry operation was cancelled
    ///
    /// This can occur if the retry is cancelled externally or if a predicate
    /// determines that retrying should stop.
    Cancelled {
        /// Number of attempts made before cancellation
        attempts: u32,
        /// The last error that occurred, if any
        last_error: Option<E>,
    },

    /// An individual attempt timed out
    ///
    /// This occurs when a single attempt exceeds the per-attempt timeout,
    /// if one was configured.
    AttemptTimeout {
        /// Which attempt timed out
        attempt: u32,
        /// The timeout duration that was exceeded
        timeout: Duration,
    },

    /// The error is not retryable
    ///
    /// This variant is returned when a `RetryPredicate` determines that
    /// the error should not be retried.
    NonRetryable(E),
}

impl<E: fmt::Display> fmt::Display for RetryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetryError::Exhausted {
                attempts,
                source,
                total_duration,
            } => {
                write!(
                    f,
                    "retry exhausted after {} attempts over {:.2}s: {}",
                    attempts,
                    total_duration.as_secs_f64(),
                    source
                )
            }
            RetryError::Cancelled {
                attempts,
                last_error,
            } => {
                if let Some(err) = last_error {
                    write!(f, "retry cancelled after {} attempts: {}", attempts, err)
                } else {
                    write!(f, "retry cancelled after {} attempts", attempts)
                }
            }
            RetryError::AttemptTimeout { attempt, timeout } => {
                write!(
                    f,
                    "attempt {} timed out after {}ms",
                    attempt,
                    timeout.as_millis()
                )
            }
            RetryError::NonRetryable(source) => {
                write!(f, "non-retryable error: {}", source)
            }
        }
    }
}

impl<E: Error + 'static> Error for RetryError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RetryError::Exhausted { source, .. } => Some(source),
            RetryError::Cancelled {
                last_error: Some(err),
                ..
            } => Some(err),
            RetryError::NonRetryable(source) => Some(source),
            _ => None,
        }
    }
}

impl<E> RetryError<E> {
    /// Create a new exhausted error
    pub fn exhausted(attempts: u32, source: E, total_duration: Duration) -> Self {
        RetryError::Exhausted {
            attempts,
            source,
            total_duration,
        }
    }

    /// Create a new cancelled error
    pub fn cancelled(attempts: u32, last_error: Option<E>) -> Self {
        RetryError::Cancelled {
            attempts,
            last_error,
        }
    }

    /// Create a new attempt timeout error
    pub fn attempt_timeout(attempt: u32, timeout: Duration) -> Self {
        RetryError::AttemptTimeout { attempt, timeout }
    }

    /// Create a new non-retryable error
    pub fn non_retryable(source: E) -> Self {
        RetryError::NonRetryable(source)
    }

    /// Get the number of attempts made
    pub fn attempts(&self) -> u32 {
        match self {
            RetryError::Exhausted { attempts, .. } => *attempts,
            RetryError::Cancelled { attempts, .. } => *attempts,
            RetryError::AttemptTimeout { attempt, .. } => *attempt,
            RetryError::NonRetryable(_) => 1,
        }
    }

    /// Check if this error indicates all retries were exhausted
    pub fn is_exhausted(&self) -> bool {
        matches!(self, RetryError::Exhausted { .. })
    }

    /// Check if this error indicates cancellation
    pub fn is_cancelled(&self) -> bool {
        matches!(self, RetryError::Cancelled { .. })
    }

    /// Check if this error indicates a timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, RetryError::AttemptTimeout { .. })
    }

    /// Check if this error is non-retryable
    pub fn is_non_retryable(&self) -> bool {
        matches!(self, RetryError::NonRetryable(_))
    }

    /// Get the underlying error, consuming this error
    pub fn into_source(self) -> Option<E> {
        match self {
            RetryError::Exhausted { source, .. } => Some(source),
            RetryError::Cancelled { last_error, .. } => last_error,
            RetryError::NonRetryable(source) => Some(source),
            RetryError::AttemptTimeout { .. } => None,
        }
    }

    /// Get a reference to the underlying error
    pub fn source_ref(&self) -> Option<&E> {
        match self {
            RetryError::Exhausted { source, .. } => Some(source),
            RetryError::Cancelled { last_error, .. } => last_error.as_ref(),
            RetryError::NonRetryable(source) => Some(source),
            RetryError::AttemptTimeout { .. } => None,
        }
    }

    /// Map the error type using a closure
    pub fn map_err<F, E2>(self, f: F) -> RetryError<E2>
    where
        F: FnOnce(E) -> E2,
    {
        match self {
            RetryError::Exhausted {
                attempts,
                source,
                total_duration,
            } => RetryError::Exhausted {
                attempts,
                source: f(source),
                total_duration,
            },
            RetryError::Cancelled {
                attempts,
                last_error,
            } => RetryError::Cancelled {
                attempts,
                last_error: last_error.map(f),
            },
            RetryError::AttemptTimeout { attempt, timeout } => {
                RetryError::AttemptTimeout { attempt, timeout }
            }
            RetryError::NonRetryable(source) => RetryError::NonRetryable(f(source)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_exhausted_error() {
        let err: RetryError<io::Error> = RetryError::exhausted(
            3,
            io::Error::new(io::ErrorKind::TimedOut, "timeout"),
            Duration::from_secs(5),
        );

        assert!(err.is_exhausted());
        assert!(!err.is_cancelled());
        assert!(!err.is_timeout());
        assert!(!err.is_non_retryable());
        assert_eq!(err.attempts(), 3);
    }

    #[test]
    fn test_cancelled_error() {
        let err: RetryError<io::Error> = RetryError::cancelled(2, None);

        assert!(!err.is_exhausted());
        assert!(err.is_cancelled());
        assert_eq!(err.attempts(), 2);
    }

    #[test]
    fn test_timeout_error() {
        let err: RetryError<io::Error> = RetryError::attempt_timeout(1, Duration::from_millis(500));

        assert!(err.is_timeout());
        assert_eq!(err.attempts(), 1);
    }

    #[test]
    fn test_non_retryable_error() {
        let err: RetryError<io::Error> =
            RetryError::non_retryable(io::Error::new(io::ErrorKind::NotFound, "not found"));

        assert!(err.is_non_retryable());
        assert_eq!(err.attempts(), 1);
    }

    #[test]
    fn test_into_source() {
        let err: RetryError<String> =
            RetryError::exhausted(3, "original error".to_string(), Duration::from_secs(1));

        assert_eq!(err.into_source(), Some("original error".to_string()));
    }

    #[test]
    fn test_map_err() {
        let err: RetryError<i32> = RetryError::exhausted(3, 42, Duration::from_secs(1));

        let mapped = err.map_err(|n| format!("error code: {}", n));
        assert!(
            matches!(mapped, RetryError::Exhausted { source, .. } if source == "error code: 42")
        );
    }

    #[test]
    fn test_display() {
        let err: RetryError<io::Error> = RetryError::exhausted(
            3,
            io::Error::new(io::ErrorKind::TimedOut, "connection timeout"),
            Duration::from_secs(5),
        );

        let display = format!("{}", err);
        assert!(display.contains("retry exhausted"));
        assert!(display.contains("3 attempts"));
        assert!(display.contains("connection timeout"));
    }
}
