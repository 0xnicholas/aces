//! Configuration for the scheduler.
//!
//! Defines configuration structures for token buckets and scheduler behavior.

use std::time::Duration;

/// Configuration for a token bucket.
///
/// Token buckets implement smooth rate limiting with burst support.
///
/// # Fields
///
/// * `capacity` - Maximum number of tokens the bucket can hold (burst size)
/// * `refill_rate` - Number of tokens added per second
///
/// # Example
///
/// ```rust
/// use scheduler::TokenBucketConfig;
///
/// // Allow burst of 10 requests, refill at 100 requests/minute
/// let config = TokenBucketConfig {
///     capacity: 10,
///     refill_rate: 100 / 60, // ~1.67 tokens/sec
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenBucketConfig {
    /// Maximum number of tokens (burst capacity)
    pub capacity: u64,
    /// Tokens added per second
    pub refill_rate: u64,
}

impl TokenBucketConfig {
    /// Create a new token bucket configuration.
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        Self {
            capacity,
            refill_rate,
        }
    }

    /// Create a configuration for LLM API rate limiting.
    ///
    /// Default: capacity=10, refill_rate=2 (120 requests/minute)
    pub fn llm_default() -> Self {
        Self {
            capacity: 10,
            refill_rate: 2, // 2 tokens/sec = 120/min
        }
    }

    /// Create a configuration for tool call rate limiting.
    ///
    /// Default: capacity=5, refill_rate=1 (60 requests/minute)
    pub fn tool_default() -> Self {
        Self {
            capacity: 5,
            refill_rate: 1, // 1 token/sec = 60/min
        }
    }

    /// Create a configuration for compute resource limiting.
    ///
    /// Default: capacity=20, refill_rate=10 (600 operations/minute)
    pub fn compute_default() -> Self {
        Self {
            capacity: 20,
            refill_rate: 10, // 10 tokens/sec
        }
    }

    /// Calculate the time to refill a given number of tokens.
    pub fn refill_time(&self, tokens: u64) -> Duration {
        if self.refill_rate == 0 {
            Duration::MAX
        } else {
            Duration::from_millis((tokens * 1000) / self.refill_rate)
        }
    }
}

impl Default for TokenBucketConfig {
    fn default() -> Self {
        Self::compute_default()
    }
}

/// Configuration for the scheduler.
///
/// Controls token bucket settings, queue limits, and timeout behavior.
///
/// # Example
///
/// ```rust
/// use scheduler::SchedulerConfig;
/// use std::time::Duration;
///
/// let config = SchedulerConfig {
///     max_queue_depth: 1000,
///     queue_timeout: Duration::from_secs(30),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Token bucket for LLM API calls
    pub llm_bucket: TokenBucketConfig,
    /// Token bucket for tool calls
    pub tool_bucket: TokenBucketConfig,
    /// Token bucket for compute resources
    pub compute_bucket: TokenBucketConfig,
    /// Maximum depth for each priority queue
    pub max_queue_depth: usize,
    /// Maximum time a task can wait in queue
    pub queue_timeout: Duration,
}

impl SchedulerConfig {
    /// Create a new scheduler configuration with the specified parameters.
    pub fn new(
        llm_bucket: TokenBucketConfig,
        tool_bucket: TokenBucketConfig,
        compute_bucket: TokenBucketConfig,
        max_queue_depth: usize,
        queue_timeout: Duration,
    ) -> Self {
        Self {
            llm_bucket,
            tool_bucket,
            compute_bucket,
            max_queue_depth,
            queue_timeout,
        }
    }

    /// Set the maximum queue depth.
    pub fn with_max_queue_depth(mut self, depth: usize) -> Self {
        self.max_queue_depth = depth;
        self
    }

    /// Set the queue timeout.
    pub fn with_queue_timeout(mut self, timeout: Duration) -> Self {
        self.queue_timeout = timeout;
        self
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            llm_bucket: TokenBucketConfig::llm_default(),
            tool_bucket: TokenBucketConfig::tool_default(),
            compute_bucket: TokenBucketConfig::compute_default(),
            max_queue_depth: 1000,
            queue_timeout: Duration::from_secs(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_config_creation() {
        let config = TokenBucketConfig::new(100, 10);
        assert_eq!(config.capacity, 100);
        assert_eq!(config.refill_rate, 10);
    }

    #[test]
    fn test_token_bucket_refill_time() {
        let config = TokenBucketConfig::new(100, 10);
        // 10 tokens at 10/sec = 1 second
        assert_eq!(config.refill_time(10), Duration::from_secs(1));
        // 5 tokens at 10/sec = 500ms
        assert_eq!(config.refill_time(5), Duration::from_millis(500));
    }

    #[test]
    fn test_token_bucket_zero_refill_rate() {
        let config = TokenBucketConfig::new(100, 0);
        assert_eq!(config.refill_time(10), Duration::MAX);
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.max_queue_depth, 1000);
        assert_eq!(config.queue_timeout, Duration::from_secs(30));
        assert_eq!(config.llm_bucket.capacity, 10);
        assert_eq!(config.tool_bucket.capacity, 5);
    }

    #[test]
    fn test_scheduler_config_builder() {
        let config = SchedulerConfig::default()
            .with_max_queue_depth(500)
            .with_queue_timeout(Duration::from_secs(60));

        assert_eq!(config.max_queue_depth, 500);
        assert_eq!(config.queue_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_default_configs() {
        let llm = TokenBucketConfig::llm_default();
        assert_eq!(llm.capacity, 10);
        assert_eq!(llm.refill_rate, 2);

        let tool = TokenBucketConfig::tool_default();
        assert_eq!(tool.capacity, 5);
        assert_eq!(tool.refill_rate, 1);

        let compute = TokenBucketConfig::compute_default();
        assert_eq!(compute.capacity, 20);
        assert_eq!(compute.refill_rate, 10);
    }
}
