//! Integration tests for the retry module
//!
//! These tests verify the complete retry execution flow including
//! strategies, observers, and error handling.

use std::io;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::retry::error::RetryError;
use crate::retry::executor::{retry_with_policy, RetryExecutorBuilder, SimpleRetryExecutor};
use crate::retry::observer::{NoOpObserver, RetryObserver, StatsObserver, TracingObserver};
use crate::retry::strategies::{
    calculate_delay, AlwaysRetry, ClosurePredicate, MessagePredicate, NeverRetry, RetryPredicate,
};
use crate::types::{RetryPolicy, RetryStrategy};

/// Create a test policy with short delays
fn quick_policy(max_attempts: u32, strategy: RetryStrategy) -> RetryPolicy {
    RetryPolicy {
        max_attempts,
        strategy,
        backoff_multiplier: 2.0,
        initial_delay_ms: 1, // Very short for tests
        max_delay_ms: 10,
    }
}

// ============================================================================
// Strategy Tests
// ============================================================================

#[test]
fn test_strategy_none_always_zero() {
    let policy = RetryPolicy {
        max_attempts: 5,
        strategy: RetryStrategy::None,
        backoff_multiplier: 2.0,
        initial_delay_ms: 1000,
        max_delay_ms: 30000,
    };

    for attempt in 1..=5 {
        assert_eq!(calculate_delay(&policy, attempt, false), Duration::ZERO);
        assert_eq!(calculate_delay(&policy, attempt, true), Duration::ZERO);
    }
}

#[test]
fn test_strategy_fixed_constant_delay() {
    let policy = RetryPolicy {
        max_attempts: 5,
        strategy: RetryStrategy::FixedDelay,
        backoff_multiplier: 2.0,
        initial_delay_ms: 500,
        max_delay_ms: 30000,
    };

    for attempt in 1..=5 {
        assert_eq!(
            calculate_delay(&policy, attempt, false),
            Duration::from_millis(500)
        );
    }
}

#[test]
fn test_strategy_exponential_doubles() {
    let policy = RetryPolicy {
        max_attempts: 5,
        strategy: RetryStrategy::ExponentialBackoff,
        backoff_multiplier: 2.0,
        initial_delay_ms: 100,
        max_delay_ms: 100000, // High enough to not cap
    };

    assert_eq!(
        calculate_delay(&policy, 1, false),
        Duration::from_millis(100)
    ); // 100 * 2^0
    assert_eq!(
        calculate_delay(&policy, 2, false),
        Duration::from_millis(200)
    ); // 100 * 2^1
    assert_eq!(
        calculate_delay(&policy, 3, false),
        Duration::from_millis(400)
    ); // 100 * 2^2
    assert_eq!(
        calculate_delay(&policy, 4, false),
        Duration::from_millis(800)
    ); // 100 * 2^3
    assert_eq!(
        calculate_delay(&policy, 5, false),
        Duration::from_millis(1600)
    ); // 100 * 2^4
}

#[test]
fn test_strategy_exponential_custom_multiplier() {
    let policy = RetryPolicy {
        max_attempts: 4,
        strategy: RetryStrategy::ExponentialBackoff,
        backoff_multiplier: 3.0,
        initial_delay_ms: 100,
        max_delay_ms: 100000,
    };

    assert_eq!(
        calculate_delay(&policy, 1, false),
        Duration::from_millis(100)
    ); // 100 * 3^0
    assert_eq!(
        calculate_delay(&policy, 2, false),
        Duration::from_millis(300)
    ); // 100 * 3^1
    assert_eq!(
        calculate_delay(&policy, 3, false),
        Duration::from_millis(900)
    ); // 100 * 3^2
    assert_eq!(
        calculate_delay(&policy, 4, false),
        Duration::from_millis(2700)
    ); // 100 * 3^3
}

#[test]
fn test_strategy_linear_increments() {
    let policy = RetryPolicy {
        max_attempts: 5,
        strategy: RetryStrategy::LinearBackoff,
        backoff_multiplier: 2.0, // Ignored for linear
        initial_delay_ms: 100,
        max_delay_ms: 100000,
    };

    assert_eq!(
        calculate_delay(&policy, 1, false),
        Duration::from_millis(100)
    ); // 100 * 1
    assert_eq!(
        calculate_delay(&policy, 2, false),
        Duration::from_millis(200)
    ); // 100 * 2
    assert_eq!(
        calculate_delay(&policy, 3, false),
        Duration::from_millis(300)
    ); // 100 * 3
    assert_eq!(
        calculate_delay(&policy, 4, false),
        Duration::from_millis(400)
    ); // 100 * 4
    assert_eq!(
        calculate_delay(&policy, 5, false),
        Duration::from_millis(500)
    ); // 100 * 5
}

#[test]
fn test_max_delay_caps_all_strategies() {
    let strategies = [
        RetryStrategy::FixedDelay,
        RetryStrategy::ExponentialBackoff,
        RetryStrategy::LinearBackoff,
    ];

    for strategy in strategies {
        let policy = RetryPolicy {
            max_attempts: 10,
            strategy,
            backoff_multiplier: 10.0,
            initial_delay_ms: 10000,
            max_delay_ms: 5000, // Cap lower than initial
        };

        for attempt in 1..=10 {
            let delay = calculate_delay(&policy, attempt, false);
            assert!(
                delay <= Duration::from_millis(5000),
                "Strategy {:?} at attempt {} exceeded max_delay",
                strategy,
                attempt
            );
        }
    }
}

#[test]
fn test_jitter_stays_within_bounds() {
    let policy = RetryPolicy {
        max_attempts: 3,
        strategy: RetryStrategy::FixedDelay,
        backoff_multiplier: 2.0,
        initial_delay_ms: 1000,
        max_delay_ms: 30000,
    };

    // Run many iterations to test randomness bounds
    for _ in 0..1000 {
        let delay = calculate_delay(&policy, 1, true);
        let base = 1000u64;
        let max_jitter = base / 4; // 25%

        assert!(delay.as_millis() as u64 >= base);
        assert!(delay.as_millis() as u64 <= base + max_jitter);
    }
}

#[test]
fn test_jitter_with_max_delay_cap() {
    let policy = RetryPolicy {
        max_attempts: 3,
        strategy: RetryStrategy::FixedDelay,
        backoff_multiplier: 2.0,
        initial_delay_ms: 1000,
        max_delay_ms: 1000, // Same as initial
    };

    for _ in 0..100 {
        let delay = calculate_delay(&policy, 1, true);
        // After cap, jitter is applied to capped value
        assert!(delay.as_millis() as u64 >= 1000);
        assert!(delay.as_millis() as u64 <= 1250); // 1000 + 25%
    }
}

// ============================================================================
// Predicate Tests
// ============================================================================

#[test]
fn test_always_retry_predicate() {
    let predicate = AlwaysRetry;

    let errors = [
        io::Error::new(io::ErrorKind::NotFound, "not found"),
        io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        io::Error::new(io::ErrorKind::TimedOut, "timeout"),
        io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused"),
    ];

    for error in &errors {
        assert!(
            predicate.should_retry(error),
            "AlwaysRetry should retry all errors"
        );
    }
}

#[test]
fn test_never_retry_predicate() {
    let predicate = NeverRetry;

    let errors = [
        io::Error::new(io::ErrorKind::NotFound, "not found"),
        io::Error::new(io::ErrorKind::TimedOut, "timeout"),
    ];

    for error in &errors {
        assert!(
            !predicate.should_retry(error),
            "NeverRetry should never retry"
        );
    }
}

#[test]
fn test_closure_predicate_selective() {
    let predicate = ClosurePredicate::new(|err: &io::Error| {
        matches!(
            err.kind(),
            io::ErrorKind::TimedOut
                | io::ErrorKind::ConnectionRefused
                | io::ErrorKind::ConnectionReset
        )
    });

    // Should retry
    assert!(predicate.should_retry(&io::Error::new(io::ErrorKind::TimedOut, "timeout")));
    assert!(predicate.should_retry(&io::Error::new(
        io::ErrorKind::ConnectionRefused,
        "refused"
    )));
    assert!(predicate.should_retry(&io::Error::new(io::ErrorKind::ConnectionReset, "reset")));

    // Should not retry
    assert!(!predicate.should_retry(&io::Error::new(io::ErrorKind::NotFound, "not found")));
    assert!(!predicate.should_retry(&io::Error::new(
        io::ErrorKind::PermissionDenied,
        "denied"
    )));
}

#[test]
fn test_message_predicate_network_errors() {
    let predicate = MessagePredicate::network_errors();

    // Should retry (contains network-related patterns)
    assert!(predicate.should_retry(&io::Error::new(io::ErrorKind::Other, "connection timeout")));
    assert!(predicate.should_retry(&io::Error::new(
        io::ErrorKind::Other,
        "Connection Reset by peer"
    )));
    assert!(predicate.should_retry(&io::Error::new(
        io::ErrorKind::Other,
        "network unreachable"
    )));

    // Should not retry (no network patterns)
    assert!(!predicate.should_retry(&io::Error::new(io::ErrorKind::Other, "file not found")));
    assert!(!predicate.should_retry(&io::Error::new(io::ErrorKind::Other, "invalid input")));
}

// ============================================================================
// Observer Tests
// ============================================================================

#[test]
fn test_noop_observer_compiles() {
    let observer = NoOpObserver;
    let error = io::Error::new(io::ErrorKind::Other, "test");

    // Just verify these don't panic
    observer.on_attempt_start(1, 3);
    observer.on_attempt_failed(1, &error, Duration::from_millis(100));
    observer.on_success(2, Duration::from_millis(500));
    observer.on_exhausted(3, &error);
    observer.on_cancelled(2, Some(&error));
    observer.on_cancelled(2, None);
}

#[test]
fn test_stats_observer_counts() {
    let observer = StatsObserver::new();
    let error = io::Error::new(io::ErrorKind::Other, "test");

    assert_eq!(observer.attempt_starts(), 0);
    assert_eq!(observer.failures(), 0);
    assert_eq!(observer.successes(), 0);
    assert_eq!(observer.exhaustions(), 0);
    assert_eq!(observer.cancellations(), 0);

    observer.on_attempt_start(1, 3);
    observer.on_attempt_start(2, 3);
    observer.on_attempt_failed(1, &error, Duration::from_millis(100));
    observer.on_success(2, Duration::from_millis(500));

    assert_eq!(observer.attempt_starts(), 2);
    assert_eq!(observer.failures(), 1);
    assert_eq!(observer.successes(), 1);
    assert_eq!(observer.exhaustions(), 0);
    assert_eq!(observer.cancellations(), 0);

    observer.on_exhausted(3, &error);
    observer.on_cancelled(3, None);

    assert_eq!(observer.exhaustions(), 1);
    assert_eq!(observer.cancellations(), 1);
}

#[test]
fn test_tracing_observer_construction() {
    let observer = TracingObserver::new("test-operation");
    assert_eq!(observer.operation(), "test-operation");

    let default_observer = TracingObserver::default();
    assert_eq!(default_observer.operation(), "retry");
}

// ============================================================================
// Executor Integration Tests
// ============================================================================

#[tokio::test]
async fn test_executor_immediate_success() {
    let policy = quick_policy(3, RetryStrategy::FixedDelay);
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
    assert_eq!(observer.exhaustions(), 0);
}

#[tokio::test]
async fn test_executor_success_on_second_attempt() {
    let policy = quick_policy(3, RetryStrategy::FixedDelay);
    let observer = Arc::new(StatsObserver::new());
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();

    let result: Result<&str, RetryError<io::Error>> = RetryExecutorBuilder::new()
        .with_policy(policy)
        .with_observer(observer.clone())
        .with_jitter(false)
        .build()
        .execute(|| {
            let attempts = attempts_clone.clone();
            async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt == 1 {
                    Err(io::Error::new(io::ErrorKind::TimedOut, "first failure"))
                } else {
                    Ok("success on retry")
                }
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success on retry");
    assert_eq!(observer.attempt_starts(), 2);
    assert_eq!(observer.failures(), 1);
    assert_eq!(observer.successes(), 1);
}

#[tokio::test]
async fn test_executor_success_on_last_attempt() {
    let policy = quick_policy(3, RetryStrategy::FixedDelay);
    let observer = Arc::new(StatsObserver::new());
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();

    let result: Result<&str, RetryError<io::Error>> = RetryExecutorBuilder::new()
        .with_policy(policy)
        .with_observer(observer.clone())
        .with_jitter(false)
        .build()
        .execute(|| {
            let attempts = attempts_clone.clone();
            async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt < 3 {
                    Err(io::Error::new(io::ErrorKind::TimedOut, "not yet"))
                } else {
                    Ok("finally!")
                }
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "finally!");
    assert_eq!(observer.attempt_starts(), 3);
    assert_eq!(observer.failures(), 2);
    assert_eq!(observer.successes(), 1);
}

#[tokio::test]
async fn test_executor_all_attempts_exhausted() {
    let policy = quick_policy(3, RetryStrategy::FixedDelay);
    let observer = Arc::new(StatsObserver::new());

    let result: Result<&str, RetryError<io::Error>> = RetryExecutorBuilder::new()
        .with_policy(policy)
        .with_observer(observer.clone())
        .with_jitter(false)
        .build()
        .execute(|| async { Err(io::Error::new(io::ErrorKind::Other, "always fails")) })
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.is_exhausted());
    assert_eq!(err.attempts(), 3);
    assert_eq!(observer.attempt_starts(), 3);
    assert_eq!(observer.failures(), 2); // Last failure triggers exhausted, not failed
    assert_eq!(observer.exhaustions(), 1);
}

#[tokio::test]
async fn test_simple_executor_with_predicate() {
    let policy = quick_policy(5, RetryStrategy::FixedDelay);
    let observer = Arc::new(StatsObserver::new());
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();

    // Only retry timeout errors
    let predicate = ClosurePredicate::new(|err: &io::Error| err.kind() == io::ErrorKind::TimedOut);

    let result: Result<&str, RetryError<io::Error>> =
        SimpleRetryExecutor::<io::Error, _, _>::new(policy)
            .with_predicate(predicate)
            .with_observer(observer.clone())
            .with_jitter(false)
            .execute(|| {
                let attempts = attempts_clone.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt == 1 {
                        Err(io::Error::new(io::ErrorKind::TimedOut, "timeout - retryable"))
                    } else {
                        // Second attempt returns non-retryable error
                        Err(io::Error::new(io::ErrorKind::NotFound, "not found - not retryable"))
                    }
                }
            })
            .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.is_non_retryable());
    assert_eq!(observer.attempt_starts(), 2);
    assert_eq!(observer.failures(), 1);
    assert_eq!(observer.cancellations(), 1);
}

#[tokio::test]
async fn test_retry_with_policy_convenience_function() {
    let policy = quick_policy(3, RetryStrategy::ExponentialBackoff);
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();

    let result = retry_with_policy(&policy, || {
        let attempts = attempts_clone.clone();
        async move {
            let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if attempt < 2 {
                Err(io::Error::new(io::ErrorKind::Other, "fail once"))
            } else {
                Ok("done")
            }
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "done");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_executor_single_attempt_policy() {
    let policy = quick_policy(1, RetryStrategy::FixedDelay);
    let observer = Arc::new(StatsObserver::new());

    let result: Result<&str, RetryError<io::Error>> = RetryExecutorBuilder::new()
        .with_policy(policy)
        .with_observer(observer.clone())
        .build()
        .execute(|| async { Err(io::Error::new(io::ErrorKind::Other, "single try")) })
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.is_exhausted());
    assert_eq!(err.attempts(), 1);
    assert_eq!(observer.attempt_starts(), 1);
    assert_eq!(observer.failures(), 0); // No retries means no failures, only exhaustion
    assert_eq!(observer.exhaustions(), 1);
}

// ============================================================================
// Error Type Tests
// ============================================================================

#[test]
fn test_retry_error_exhausted_details() {
    let err: RetryError<io::Error> = RetryError::exhausted(
        3,
        io::Error::new(io::ErrorKind::TimedOut, "final timeout"),
        Duration::from_secs(5),
    );

    assert!(err.is_exhausted());
    assert!(!err.is_cancelled());
    assert!(!err.is_timeout());
    assert!(!err.is_non_retryable());
    assert_eq!(err.attempts(), 3);

    let source = err.source_ref().unwrap();
    assert_eq!(source.kind(), io::ErrorKind::TimedOut);
}

#[test]
fn test_retry_error_display_formats() {
    let exhausted: RetryError<io::Error> = RetryError::exhausted(
        3,
        io::Error::new(io::ErrorKind::TimedOut, "timeout"),
        Duration::from_millis(5500),
    );
    let display = format!("{}", exhausted);
    assert!(display.contains("retry exhausted"));
    assert!(display.contains("3 attempts"));
    assert!(display.contains("5.5")); // Duration in seconds

    let cancelled: RetryError<io::Error> = RetryError::cancelled(2, None);
    let display = format!("{}", cancelled);
    assert!(display.contains("retry cancelled"));
    assert!(display.contains("2 attempts"));

    let timeout: RetryError<io::Error> =
        RetryError::attempt_timeout(1, Duration::from_millis(500));
    let display = format!("{}", timeout);
    assert!(display.contains("attempt 1"));
    assert!(display.contains("timed out"));
    assert!(display.contains("500ms"));
}

#[test]
fn test_retry_error_map_err() {
    let err: RetryError<i32> = RetryError::exhausted(3, 42, Duration::from_secs(1));

    let mapped: RetryError<String> = err.map_err(|n| format!("error code: {}", n));

    match mapped {
        RetryError::Exhausted { source, .. } => {
            assert_eq!(source, "error code: 42");
        }
        _ => panic!("Expected Exhausted variant"),
    }
}

#[test]
fn test_retry_error_into_source() {
    let err: RetryError<String> =
        RetryError::exhausted(3, "original".to_string(), Duration::from_secs(1));
    assert_eq!(err.into_source(), Some("original".to_string()));

    let err: RetryError<String> = RetryError::cancelled(2, Some("cancelled".to_string()));
    assert_eq!(err.into_source(), Some("cancelled".to_string()));

    let err: RetryError<String> = RetryError::cancelled(2, None);
    assert_eq!(err.into_source(), None);

    let err: RetryError<String> = RetryError::attempt_timeout(1, Duration::from_millis(100));
    assert_eq!(err.into_source(), None);
}
