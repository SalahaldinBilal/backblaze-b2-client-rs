use std::{num::NonZeroU64, time::Duration};

/// The request retry strategy.
#[derive(Debug)]
pub enum RetryStrategy {
    Constant(ConstantRetryStrategy),
    Dynamic(Box<dyn DynamicRetryStrategy + Send + Sync>),
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::Dynamic(Box::new(DefaultRetryStrategy))
    }
}

impl RetryStrategy {
    pub fn wait(&self, current_retry_count: u64) -> Duration {
        match self {
            Self::Constant(c) => c.wait,
            Self::Dynamic(d) => d.wait_time(current_retry_count),
        }
    }

    pub fn count(&self) -> NonZeroU64 {
        match self {
            Self::Constant(c) => c.count,
            Self::Dynamic(d) => d.retry_count(),
        }
    }
}

/// Dictates requests are retried.
#[derive(Debug, Clone)]
pub struct ConstantRetryStrategy {
    /// Number of times to retry.
    /// <br> Default 3.
    pub count: NonZeroU64,
    /// How much to wait between retries.
    /// <br> Default 1 seconds.
    pub wait: Duration,
}

impl Default for ConstantRetryStrategy {
    fn default() -> Self {
        Self {
            count: NonZeroU64::try_from(3).expect("valid number"),
            wait: Duration::from_secs(1),
        }
    }
}

/// A dynamic retry strategy.
pub trait DynamicRetryStrategy: std::fmt::Debug {
    /// Returns the wait time
    fn wait_time(&self, current_retry_count: u64) -> Duration;
    fn retry_count(&self) -> NonZeroU64;
}

#[derive(Debug)]
pub struct DefaultRetryStrategy;

impl DynamicRetryStrategy for DefaultRetryStrategy {
    fn wait_time(&self, current_retry_count: u64) -> Duration {
        Duration::from_secs_f64((current_retry_count * 2) as f64 / 1.2)
    }

    fn retry_count(&self) -> NonZeroU64 {
        NonZeroU64::try_from(5).expect("valid number")
    }
}
