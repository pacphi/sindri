//! Retry execution engine
//!
//! This module provides the core retry execution logic with configurable
//! policies, predicates, and observers.

use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;
use std::time::Instant;

use crate::types::RetryPolicy;

use super::error::RetryError;
use super::observer::{NoOpObserver, RetryObserver};
use super::strategies::{calculate_delay, AlwaysRetry, RetryPredicate};

/// Execute an async operation with retry logic based on a policy
///
/// This is a convenience function for simple retry scenarios. For more
/// control, use `RetryExecutorBuilder`.
///
/// # Arguments
///
/// * `policy` - The retry policy to use
/// * `op` - A closure that returns a future representing the operation
///
/// # Returns
///
/// The result of the operation, or a `RetryError` if all attempts fail.
///
/// # Example
///
/// ```rust,no_run
/// use sindri_core::retry::retry_with_policy;
/// use sindri_core::types::RetryPolicy;
///
/// async fn example() {
///     let policy = RetryPolicy::default();
///
///     let result = retry_with_policy(&policy, || async {
///         // Simulated operation that might fail
///         Ok::<_, std::io::Error>("success")
///     }).await;
/// }
/// ```
pub async fn retry_with_policy<F, Fut, T, E>(policy: &RetryPolicy, op: F) -> Result<T, RetryError<E>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Error + Send + 'static,
{
    SimpleRetryExecutor::<E, _, _>::new(policy.clone())
        .execute(op)
        .await
}

/// Builder for configuring a `RetryExecutor`
///
/// # Example
///
/// ```rust
/// use sindri_core::retry::{RetryExecutorBuilder, TracingObserver};
/// use sindri_core::types::RetryPolicy;
///
/// let executor = RetryExecutorBuilder::new()
///     .with_policy(RetryPolicy::default())
///     .with_observer(TracingObserver::new("download"))
///     .with_jitter(true)
///     .build();
/// ```
pub struct RetryExecutorBuilder<P = AlwaysRetry, O = NoOpObserver> {
    policy: RetryPolicy,
    predicate: P,
    observer: O,
    jitter: bool,
}

impl Default for RetryExecutorBuilder<AlwaysRetry, NoOpObserver> {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryExecutorBuilder<AlwaysRetry, NoOpObserver> {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            policy: RetryPolicy::default(),
            predicate: AlwaysRetry,
            observer: NoOpObserver,
            jitter: true, // Jitter enabled by default
        }
    }
}

impl<P, O> RetryExecutorBuilder<P, O> {
    /// Set the retry policy
    pub fn with_policy(mut self, policy: RetryPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Set the retry predicate
    ///
    /// The predicate determines whether an error should be retried.
    pub fn with_predicate<P2>(self, predicate: P2) -> RetryExecutorBuilder<P2, O> {
        RetryExecutorBuilder {
            policy: self.policy,
            predicate,
            observer: self.observer,
            jitter: self.jitter,
        }
    }

    /// Set the observer
    ///
    /// The observer receives callbacks during retry execution.
    pub fn with_observer<O2>(self, observer: O2) -> RetryExecutorBuilder<P, O2> {
        RetryExecutorBuilder {
            policy: self.policy,
            predicate: self.predicate,
            observer,
            jitter: self.jitter,
        }
    }

    /// Enable or disable jitter
    ///
    /// Jitter adds random variation to retry delays to prevent thundering herd.
    /// Enabled by default.
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Build the executor
    pub fn build(self) -> RetryExecutor<P, O> {
        RetryExecutor {
            policy: self.policy,
            predicate: self.predicate,
            observer: self.observer,
            jitter: self.jitter,
        }
    }
}

/// A retry executor with configurable policy, predicate, and observer
///
/// Use `RetryExecutorBuilder` to create an instance.
pub struct RetryExecutor<P, O> {
    policy: RetryPolicy,
    predicate: P,
    observer: O,
    jitter: bool,
}

impl<P, O> RetryExecutor<P, O>
where
    O: RetryObserver,
{
    /// Execute an operation with retry logic
    ///
    /// # Arguments
    ///
    /// * `op` - A closure that returns a future representing the operation
    ///
    /// # Returns
    ///
    /// The result of the operation, or a `RetryError` if all attempts fail.
    pub async fn execute<F, Fut, T, E>(&self, mut op: F) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Error + Send + 'static,
        P: RetryPredicate<E>,
    {
        let start = Instant::now();
        let mut last_error: Option<E> = None;

        for attempt in 1..=self.policy.max_attempts {
            self.observer
                .on_attempt_start(attempt, self.policy.max_attempts);

            match op().await {
                Ok(result) => {
                    self.observer.on_success(attempt, start.elapsed());
                    return Ok(result);
                }
                Err(err) => {
                    // Check if this error should be retried
                    if !self.predicate.should_retry(&err) {
                        self.observer.on_cancelled(attempt, Some(&err));
                        return Err(RetryError::non_retryable(err));
                    }

                    // Check if this was the last attempt
                    if attempt >= self.policy.max_attempts {
                        self.observer.on_exhausted(attempt, &err);
                        return Err(RetryError::exhausted(attempt, err, start.elapsed()));
                    }

                    // Calculate delay for next attempt
                    let delay = calculate_delay(&self.policy, attempt, self.jitter);

                    self.observer.on_attempt_failed(attempt, &err, delay);

                    last_error = Some(err);

                    // Wait before next attempt
                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // This should not be reached, but handle it gracefully
        Err(RetryError::cancelled(
            self.policy.max_attempts,
            last_error,
        ))
    }
}

/// A simple retry executor that works with concrete error types
///
/// This is a simpler alternative to `RetryExecutor` that doesn't require
/// the predicate to work with trait objects.
pub struct SimpleRetryExecutor<E, P, O> {
    policy: RetryPolicy,
    predicate: P,
    observer: O,
    jitter: bool,
    _phantom: PhantomData<E>,
}

impl<E> SimpleRetryExecutor<E, AlwaysRetry, NoOpObserver> {
    /// Create a new simple retry executor with default settings
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            policy,
            predicate: AlwaysRetry,
            observer: NoOpObserver,
            jitter: true,
            _phantom: PhantomData,
        }
    }
}

impl<E, P, O> SimpleRetryExecutor<E, P, O> {
    /// Set the retry predicate
    pub fn with_predicate<P2>(self, predicate: P2) -> SimpleRetryExecutor<E, P2, O> {
        SimpleRetryExecutor {
            policy: self.policy,
            predicate,
            observer: self.observer,
            jitter: self.jitter,
            _phantom: PhantomData,
        }
    }

    /// Set the observer
    pub fn with_observer<O2>(self, observer: O2) -> SimpleRetryExecutor<E, P, O2> {
        SimpleRetryExecutor {
            policy: self.policy,
            predicate: self.predicate,
            observer,
            jitter: self.jitter,
            _phantom: PhantomData,
        }
    }

    /// Enable or disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }
}

impl<E, P, O> SimpleRetryExecutor<E, P, O>
where
    E: std::fmt::Display + Send + 'static,
    P: RetryPredicate<E>,
    O: RetryObserver,
{
    /// Execute an operation with retry logic
    pub async fn execute<F, Fut, T>(&self, mut op: F) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let start = Instant::now();
        let mut last_error: Option<E> = None;

        for attempt in 1..=self.policy.max_attempts {
            self.observer
                .on_attempt_start(attempt, self.policy.max_attempts);

            match op().await {
                Ok(result) => {
                    self.observer.on_success(attempt, start.elapsed());
                    return Ok(result);
                }
                Err(err) => {
                    // Check if this error should be retried
                    if !self.predicate.should_retry(&err) {
                        // Create a displayable error wrapper for observer
                        let display_err = DisplayError(format!("{}", err));
                        self.observer.on_cancelled(attempt, Some(&display_err));
                        return Err(RetryError::non_retryable(err));
                    }

                    // Check if this was the last attempt
                    if attempt >= self.policy.max_attempts {
                        let display_err = DisplayError(format!("{}", err));
                        self.observer.on_exhausted(attempt, &display_err);
                        return Err(RetryError::exhausted(attempt, err, start.elapsed()));
                    }

                    // Calculate delay for next attempt
                    let delay = calculate_delay(&self.policy, attempt, self.jitter);

                    let display_err = DisplayError(format!("{}", err));
                    self.observer.on_attempt_failed(attempt, &display_err, delay);

                    last_error = Some(err);

                    // Wait before next attempt
                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // This should not be reached, but handle it gracefully
        Err(RetryError::cancelled(
            self.policy.max_attempts,
            last_error,
        ))
    }
}

/// A simple wrapper to convert Display types to Error for observer callbacks
#[derive(Debug)]
struct DisplayError(String);

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for DisplayError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retry::observer::StatsObserver;
    use crate::retry::strategies::ClosurePredicate;
    use crate::types::RetryStrategy;
    use std::io;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    fn test_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            strategy: RetryStrategy::FixedDelay,
            backoff_multiplier: 2.0,
            initial_delay_ms: 10, // Short delays for tests
            max_delay_ms: 100,
        }
    }

    #[tokio::test]
    async fn test_immediate_success() {
        let policy = test_policy();
        let observer = Arc::new(StatsObserver::new());

        let result: Result<&str, RetryError<io::Error>> = RetryExecutorBuilder::new()
            .with_policy(policy)
            .with_observer(observer.clone())
            .build()
            .execute(|| async { Ok("success") })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(observer.attempt_starts(), 1);
        assert_eq!(observer.successes(), 1);
        assert_eq!(observer.failures(), 0);
    }

    #[tokio::test]
    async fn test_success_after_retry() {
        let policy = test_policy();
        let observer = Arc::new(StatsObserver::new());
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<&str, RetryError<io::Error>> = RetryExecutorBuilder::new()
            .with_policy(policy)
            .with_observer(observer.clone())
            .with_jitter(false) // Disable jitter for predictable tests
            .build()
            .execute(|| {
                let attempts = attempts_clone.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt < 2 {
                        Err(io::Error::new(io::ErrorKind::TimedOut, "timeout"))
                    } else {
                        Ok("success")
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(observer.attempt_starts(), 2);
        assert_eq!(observer.failures(), 1);
        assert_eq!(observer.successes(), 1);
    }

    #[tokio::test]
    async fn test_all_attempts_exhausted() {
        let policy = test_policy();
        let observer = Arc::new(StatsObserver::new());

        let result: Result<&str, RetryError<io::Error>> = RetryExecutorBuilder::new()
            .with_policy(policy.clone())
            .with_observer(observer.clone())
            .with_jitter(false)
            .build()
            .execute(|| async {
                Err(io::Error::new(io::ErrorKind::TimedOut, "always fails"))
            })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_exhausted());
        assert_eq!(err.attempts(), policy.max_attempts);
        assert_eq!(observer.attempt_starts(), 3);
        assert_eq!(observer.failures(), 2); // Only 2 failures logged (before retry)
        assert_eq!(observer.exhaustions(), 1);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let policy = test_policy();
        let observer = Arc::new(StatsObserver::new());

        let predicate = ClosurePredicate::new(|err: &io::Error| {
            // Don't retry NotFound errors
            err.kind() != io::ErrorKind::NotFound
        });

        let result: Result<&str, RetryError<io::Error>> = SimpleRetryExecutor::new(policy)
            .with_predicate(predicate)
            .with_observer(observer.clone())
            .execute(|| async { Err(io::Error::new(io::ErrorKind::NotFound, "not found")) })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_non_retryable());
        assert_eq!(observer.attempt_starts(), 1);
        assert_eq!(observer.cancellations(), 1);
    }

    #[tokio::test]
    async fn test_retry_with_policy_convenience() {
        let policy = test_policy();
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = retry_with_policy(&policy, || {
            let attempts = attempts_clone.clone();
            async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt < 2 {
                    Err(io::Error::new(io::ErrorKind::TimedOut, "timeout"))
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_zero_max_attempts() {
        let policy = RetryPolicy {
            max_attempts: 0,
            ..test_policy()
        };

        let result: Result<&str, RetryError<io::Error>> = retry_with_policy(&policy, || async {
            Err(io::Error::new(io::ErrorKind::Other, "error"))
        })
        .await;

        // With 0 max attempts, we should get a cancelled error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_single_attempt() {
        let policy = RetryPolicy {
            max_attempts: 1,
            ..test_policy()
        };
        let observer = Arc::new(StatsObserver::new());

        let result: Result<&str, RetryError<io::Error>> = RetryExecutorBuilder::new()
            .with_policy(policy)
            .with_observer(observer.clone())
            .build()
            .execute(|| async { Err(io::Error::new(io::ErrorKind::Other, "error")) })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_exhausted());
        assert_eq!(observer.attempt_starts(), 1);
        assert_eq!(observer.exhaustions(), 1);
        assert_eq!(observer.failures(), 0); // No failures, only exhaustion
    }
}
