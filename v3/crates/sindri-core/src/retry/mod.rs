//! Retry execution engine with policy-based configuration
//!
//! This module provides a reusable, policy-based retry execution engine that
//! replaces scattered hard-coded retry logic throughout the codebase.
//!
//! # Features
//!
//! - Multiple retry strategies: None, Fixed, Exponential, Linear backoff
//! - Configurable jitter for exponential backoff
//! - Observable retry attempts via the `RetryObserver` trait
//! - Built-in `TracingObserver` for logging
//! - Builder pattern for flexible executor configuration
//! - Thread-safe with Send + Sync bounds
//!
//! # Example
//!
//! ```rust,no_run
//! use sindri_core::retry::{retry_with_policy, RetryError};
//! use sindri_core::types::RetryPolicy;
//!
//! async fn example() -> Result<String, RetryError<std::io::Error>> {
//!     let policy = RetryPolicy::default();
//!
//!     retry_with_policy(&policy, || async {
//!         // Your fallible operation here
//!         Ok("success".to_string())
//!     }).await
//! }
//! ```

mod error;
mod executor;
mod observer;
mod strategies;

pub use error::RetryError;
pub use executor::{retry_with_policy, RetryExecutor, RetryExecutorBuilder, SimpleRetryExecutor};
pub use observer::{NoOpObserver, RetryObserver, StatsObserver, TracingObserver};
pub use strategies::{
    calculate_delay, AlwaysRetry, ClosurePredicate, HttpStatusError, HttpStatusPredicate,
    MessagePredicate, NeverRetry, RetryPredicate,
};

#[cfg(test)]
mod tests;
