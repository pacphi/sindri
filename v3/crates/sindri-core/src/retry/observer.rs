//! Retry observation and logging
//!
//! This module provides the `RetryObserver` trait for monitoring retry attempts
//! and a `TracingObserver` implementation that logs using the `tracing` crate.

use std::error::Error;
use std::time::Duration;

/// Observer trait for retry attempt events
///
/// Implement this trait to receive callbacks during retry execution.
/// This is useful for logging, metrics collection, or debugging.
///
/// # Example
///
/// ```rust
/// use sindri_core::retry::RetryObserver;
/// use std::error::Error;
/// use std::time::Duration;
///
/// struct MetricsObserver {
///     // Your metrics client here
/// }
///
/// impl RetryObserver for MetricsObserver {
///     fn on_attempt_start(&self, attempt: u32, max_attempts: u32) {
///         // Record attempt start metric
///     }
///
///     fn on_attempt_failed(&self, attempt: u32, error: &dyn Error, delay: Duration) {
///         // Record failure metric
///     }
///
///     fn on_success(&self, attempt: u32, total_duration: Duration) {
///         // Record success metric with latency
///     }
///
///     fn on_exhausted(&self, attempts: u32, final_error: &dyn Error) {
///         // Record exhaustion metric
///     }
/// }
/// ```
pub trait RetryObserver: Send + Sync {
    /// Called when an attempt is about to start
    ///
    /// # Arguments
    ///
    /// * `attempt` - The attempt number (1-indexed)
    /// * `max_attempts` - The maximum number of attempts configured
    fn on_attempt_start(&self, attempt: u32, max_attempts: u32);

    /// Called when an attempt fails and will be retried
    ///
    /// # Arguments
    ///
    /// * `attempt` - The attempt number that failed (1-indexed)
    /// * `error` - The error that caused the failure
    /// * `delay` - The delay before the next attempt
    fn on_attempt_failed(&self, attempt: u32, error: &dyn Error, delay: Duration);

    /// Called when the operation succeeds
    ///
    /// # Arguments
    ///
    /// * `attempt` - The attempt number that succeeded (1-indexed)
    /// * `total_duration` - Total time spent across all attempts
    fn on_success(&self, attempt: u32, total_duration: Duration);

    /// Called when all retry attempts are exhausted
    ///
    /// # Arguments
    ///
    /// * `attempts` - Total number of attempts made
    /// * `final_error` - The error from the final attempt
    fn on_exhausted(&self, attempts: u32, final_error: &dyn Error);

    /// Called when the retry is cancelled
    ///
    /// This is called when a predicate determines an error is not retryable.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The attempt number when cancelled (1-indexed)
    /// * `error` - The error that caused cancellation, if available
    fn on_cancelled(&self, attempt: u32, error: Option<&dyn Error>) {
        // Default implementation does nothing
        let _ = (attempt, error);
    }
}

/// A no-op observer that does nothing
///
/// Use this when you don't need observation but the API requires an observer.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpObserver;

impl RetryObserver for NoOpObserver {
    fn on_attempt_start(&self, _attempt: u32, _max_attempts: u32) {}

    fn on_attempt_failed(&self, _attempt: u32, _error: &dyn Error, _delay: Duration) {}

    fn on_success(&self, _attempt: u32, _total_duration: Duration) {}

    fn on_exhausted(&self, _attempts: u32, _final_error: &dyn Error) {}
}

/// An observer that logs retry events using the `tracing` crate
///
/// # Log Levels
///
/// - `on_attempt_start`: DEBUG
/// - `on_attempt_failed`: WARN
/// - `on_success`: INFO (if > 1 attempt) or DEBUG (first attempt)
/// - `on_exhausted`: ERROR
/// - `on_cancelled`: WARN
///
/// # Example
///
/// ```rust
/// use sindri_core::retry::TracingObserver;
///
/// // Create with operation name for better log context
/// let observer = TracingObserver::new("download");
/// ```
#[derive(Debug, Clone)]
pub struct TracingObserver {
    /// Name of the operation being retried (for log context)
    operation: String,
}

impl TracingObserver {
    /// Create a new tracing observer
    ///
    /// # Arguments
    ///
    /// * `operation` - A descriptive name for the operation being retried
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
        }
    }

    /// Get the operation name
    pub fn operation(&self) -> &str {
        &self.operation
    }
}

impl Default for TracingObserver {
    fn default() -> Self {
        Self::new("retry")
    }
}

impl RetryObserver for TracingObserver {
    fn on_attempt_start(&self, attempt: u32, max_attempts: u32) {
        tracing::debug!(
            operation = %self.operation,
            attempt = attempt,
            max_attempts = max_attempts,
            "starting attempt"
        );
    }

    fn on_attempt_failed(&self, attempt: u32, error: &dyn Error, delay: Duration) {
        tracing::warn!(
            operation = %self.operation,
            attempt = attempt,
            error = %error,
            delay_ms = delay.as_millis() as u64,
            "attempt failed, will retry"
        );
    }

    fn on_success(&self, attempt: u32, total_duration: Duration) {
        if attempt > 1 {
            tracing::info!(
                operation = %self.operation,
                attempt = attempt,
                total_duration_ms = total_duration.as_millis() as u64,
                "succeeded after retry"
            );
        } else {
            tracing::debug!(
                operation = %self.operation,
                duration_ms = total_duration.as_millis() as u64,
                "succeeded on first attempt"
            );
        }
    }

    fn on_exhausted(&self, attempts: u32, final_error: &dyn Error) {
        tracing::error!(
            operation = %self.operation,
            attempts = attempts,
            error = %final_error,
            "all retry attempts exhausted"
        );
    }

    fn on_cancelled(&self, attempt: u32, error: Option<&dyn Error>) {
        if let Some(err) = error {
            tracing::warn!(
                operation = %self.operation,
                attempt = attempt,
                error = %err,
                "retry cancelled due to non-retryable error"
            );
        } else {
            tracing::warn!(
                operation = %self.operation,
                attempt = attempt,
                "retry cancelled"
            );
        }
    }
}

/// An observer that collects statistics about retry attempts
///
/// Useful for testing and metrics collection.
#[derive(Debug, Default)]
pub struct StatsObserver {
    /// Attempt start events
    pub attempt_starts: std::sync::atomic::AtomicU32,
    /// Failed attempt events
    pub failures: std::sync::atomic::AtomicU32,
    /// Success events
    pub successes: std::sync::atomic::AtomicU32,
    /// Exhaustion events
    pub exhaustions: std::sync::atomic::AtomicU32,
    /// Cancellation events
    pub cancellations: std::sync::atomic::AtomicU32,
}

impl StatsObserver {
    /// Create a new stats observer
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of attempt starts
    pub fn attempt_starts(&self) -> u32 {
        self.attempt_starts
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the number of failures
    pub fn failures(&self) -> u32 {
        self.failures.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the number of successes
    pub fn successes(&self) -> u32 {
        self.successes.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the number of exhaustions
    pub fn exhaustions(&self) -> u32 {
        self.exhaustions.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the number of cancellations
    pub fn cancellations(&self) -> u32 {
        self.cancellations.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl RetryObserver for StatsObserver {
    fn on_attempt_start(&self, _attempt: u32, _max_attempts: u32) {
        self.attempt_starts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_attempt_failed(&self, _attempt: u32, _error: &dyn Error, _delay: Duration) {
        self.failures
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_success(&self, _attempt: u32, _total_duration: Duration) {
        self.successes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_exhausted(&self, _attempts: u32, _final_error: &dyn Error) {
        self.exhaustions
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_cancelled(&self, _attempt: u32, _error: Option<&dyn Error>) {
        self.cancellations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Implement RetryObserver for Arc<T> where T: RetryObserver
impl<T: RetryObserver + ?Sized> RetryObserver for std::sync::Arc<T> {
    fn on_attempt_start(&self, attempt: u32, max_attempts: u32) {
        (**self).on_attempt_start(attempt, max_attempts)
    }

    fn on_attempt_failed(&self, attempt: u32, error: &dyn Error, delay: Duration) {
        (**self).on_attempt_failed(attempt, error, delay)
    }

    fn on_success(&self, attempt: u32, total_duration: Duration) {
        (**self).on_success(attempt, total_duration)
    }

    fn on_exhausted(&self, attempts: u32, final_error: &dyn Error) {
        (**self).on_exhausted(attempts, final_error)
    }

    fn on_cancelled(&self, attempt: u32, error: Option<&dyn Error>) {
        (**self).on_cancelled(attempt, error)
    }
}

/// Implement RetryObserver for Box<T> where T: RetryObserver
impl<T: RetryObserver + ?Sized> RetryObserver for Box<T> {
    fn on_attempt_start(&self, attempt: u32, max_attempts: u32) {
        (**self).on_attempt_start(attempt, max_attempts)
    }

    fn on_attempt_failed(&self, attempt: u32, error: &dyn Error, delay: Duration) {
        (**self).on_attempt_failed(attempt, error, delay)
    }

    fn on_success(&self, attempt: u32, total_duration: Duration) {
        (**self).on_success(attempt, total_duration)
    }

    fn on_exhausted(&self, attempts: u32, final_error: &dyn Error) {
        (**self).on_exhausted(attempts, final_error)
    }

    fn on_cancelled(&self, attempt: u32, error: Option<&dyn Error>) {
        (**self).on_cancelled(attempt, error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_noop_observer() {
        let observer = NoOpObserver;
        let error = io::Error::other("test");

        // These should all be no-ops
        observer.on_attempt_start(1, 3);
        observer.on_attempt_failed(1, &error, Duration::from_millis(100));
        observer.on_success(2, Duration::from_millis(500));
        observer.on_exhausted(3, &error);
        observer.on_cancelled(2, Some(&error));
    }

    #[test]
    fn test_stats_observer() {
        let observer = StatsObserver::new();
        let error = io::Error::other("test");

        observer.on_attempt_start(1, 3);
        observer.on_attempt_start(2, 3);
        observer.on_attempt_failed(1, &error, Duration::from_millis(100));
        observer.on_success(2, Duration::from_millis(500));

        assert_eq!(observer.attempt_starts(), 2);
        assert_eq!(observer.failures(), 1);
        assert_eq!(observer.successes(), 1);
        assert_eq!(observer.exhaustions(), 0);
    }

    #[test]
    fn test_stats_observer_exhaustion() {
        let observer = StatsObserver::new();
        let error = io::Error::other("test");

        observer.on_attempt_start(1, 3);
        observer.on_attempt_failed(1, &error, Duration::from_millis(100));
        observer.on_attempt_start(2, 3);
        observer.on_attempt_failed(2, &error, Duration::from_millis(200));
        observer.on_attempt_start(3, 3);
        observer.on_exhausted(3, &error);

        assert_eq!(observer.attempt_starts(), 3);
        assert_eq!(observer.failures(), 2);
        assert_eq!(observer.exhaustions(), 1);
    }

    #[test]
    fn test_tracing_observer_creation() {
        let observer = TracingObserver::new("test_operation");
        assert_eq!(observer.operation, "test_operation");

        let default_observer = TracingObserver::default();
        assert_eq!(default_observer.operation, "retry");
    }

    #[test]
    fn test_arc_observer() {
        let observer = std::sync::Arc::new(StatsObserver::new());
        let error = io::Error::other("test");

        observer.on_attempt_start(1, 3);
        observer.on_attempt_failed(1, &error, Duration::from_millis(100));

        assert_eq!(observer.attempt_starts(), 1);
        assert_eq!(observer.failures(), 1);
    }
}
