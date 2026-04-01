//! Token bucket implementation for rate limiting.
//!
//! Provides smooth rate limiting with burst support and millisecond-precision
//! token refill.

use std::time::{Duration, Instant};
use tokio::time;

/// A token bucket for rate limiting.
///
/// Implements the token bucket algorithm:
/// - Tokens are added at a fixed rate (refill_rate per second)
/// - Bucket has a maximum capacity (burst size)
/// - Each request consumes tokens
/// - If insufficient tokens, request must wait
///
/// # Example
///
/// ```rust
/// use scheduler::TokenBucket;
///
/// # async fn example() {
/// // Create bucket with capacity 10, refill 2 tokens/sec
/// let mut bucket = TokenBucket::new("api", 10, 2);
///
/// // Try to consume 1 token (non-blocking)
/// match bucket.try_consume(1) {
///     Ok(()) => println!("Token consumed"),
///     Err(retry_after) => println!("Retry after: {:?}", retry_after),
/// }
///
/// // Consume 1 token (blocking)
/// bucket.consume(1).await;
/// # }
/// ```
#[derive(Debug)]
pub struct TokenBucket {
    name: String,
    capacity: u64,
    tokens: u64,
    refill_rate: u64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifier for this bucket (for debugging/metrics)
    /// * `capacity` - Maximum number of tokens (burst size)
    /// * `refill_rate` - Tokens added per second
    ///
    /// # Example
    ///
    /// ```rust
    /// use scheduler::TokenBucket;
    ///
    /// let bucket = TokenBucket::new("llm-api", 10, 2);
    /// ```
    pub fn new(name: impl Into<String>, capacity: u64, refill_rate: u64) -> Self {
        Self {
            name: name.into(),
            capacity,
            tokens: capacity, // Start with full bucket
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Get the bucket name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get current token count.
    pub fn tokens(&self) -> u64 {
        self.tokens
    }

    /// Get bucket capacity.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Try to consume tokens without blocking.
    ///
    /// # Arguments
    ///
    /// * `amount` - Number of tokens to consume
    ///
    /// # Returns
    ///
    /// * `Ok(())` if tokens were consumed successfully
    /// * `Err(retry_after)` with the duration to wait before retrying
    ///
    /// # Example
    ///
    /// ```rust
    /// use scheduler::TokenBucket;
    ///
    /// let mut bucket = TokenBucket::new("test", 10, 2);
    ///
    /// // First consume should succeed
    /// assert!(bucket.try_consume(1).is_ok());
    ///
    /// // Consume all remaining
    /// let remaining = bucket.tokens();
    /// assert!(bucket.try_consume(remaining).is_ok());
    ///
    /// // Now should fail
    /// assert!(bucket.try_consume(1).is_err());
    /// ```
    pub fn try_consume(&mut self, amount: u64) -> Result<(), Duration> {
        self.refill();

        if amount > self.capacity {
            // Request larger than bucket capacity will never succeed
            return Err(Duration::MAX);
        }

        if self.tokens >= amount {
            self.tokens -= amount;
            Ok(())
        } else {
            let needed = amount - self.tokens;
            let retry_after = if self.refill_rate > 0 {
                Duration::from_millis((needed * 1000) / self.refill_rate)
            } else {
                Duration::MAX
            };
            Err(retry_after)
        }
    }

    /// Consume tokens, blocking until available.
    ///
    /// # Arguments
    ///
    /// * `amount` - Number of tokens to consume
    ///
    /// # Example
    ///
    /// ```rust
    /// use scheduler::TokenBucket;
    ///
    /// # async fn example() {
    /// let mut bucket = TokenBucket::new("test", 10, 100); // 100 tokens/sec
    ///
    /// // Consume all tokens
    /// bucket.consume(10).await;
    ///
    /// // Next consume will wait ~100ms for 1 token
    /// bucket.consume(1).await;
    /// # }
    /// ```
    pub async fn consume(&mut self, amount: u64) {
        loop {
            match self.try_consume(amount) {
                Ok(()) => return,
                Err(retry_after) if retry_after == Duration::MAX => {
                    // Cannot ever satisfy this request
                    panic!(
                        "TokenBucket '{}' cannot satisfy request for {} tokens (capacity: {})",
                        self.name, amount, self.capacity
                    );
                }
                Err(retry_after) => {
                    time::sleep(retry_after).await;
                }
            }
        }
    }

    /// Add tokens to the bucket (manual refill).
    ///
    /// This is useful for testing or manual rate limit adjustments.
    pub fn add_tokens(&mut self, amount: u64) {
        self.tokens = (self.tokens + amount).min(self.capacity);
    }

    /// Refill tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        if self.refill_rate > 0 {
            // Calculate tokens to add based on elapsed milliseconds
            let tokens_to_add = (elapsed.as_millis() as u64 * self.refill_rate) / 1000;
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
        }

        self.last_refill = now;
    }

    /// Reset the bucket to full capacity.
    pub fn reset(&mut self) {
        self.tokens = self.capacity;
        self.last_refill = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new("test", 10, 2);
        assert_eq!(bucket.name(), "test");
        assert_eq!(bucket.capacity(), 10);
        assert_eq!(bucket.tokens(), 10); // Starts full
    }

    #[test]
    fn test_token_bucket_consume_success() {
        let mut bucket = TokenBucket::new("test", 10, 2);

        assert!(bucket.try_consume(1).is_ok());
        assert_eq!(bucket.tokens(), 9);

        assert!(bucket.try_consume(5).is_ok());
        assert_eq!(bucket.tokens(), 4);
    }

    #[test]
    fn test_token_bucket_consume_failure() {
        let mut bucket = TokenBucket::new("test", 5, 1);

        // Consume all tokens
        assert!(bucket.try_consume(5).is_ok());
        assert_eq!(bucket.tokens(), 0);

        // Should fail
        let result = bucket.try_consume(1);
        assert!(result.is_err());

        let retry_after = result.unwrap_err();
        assert!(retry_after > Duration::ZERO);
    }

    #[test]
    fn test_token_bucket_capacity_exceeded() {
        let mut bucket = TokenBucket::new("test", 5, 1);

        // Try to consume more than capacity
        let result = bucket.try_consume(10);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Duration::MAX);
    }

    #[test]
    fn test_token_bucket_add_tokens() {
        let mut bucket = TokenBucket::new("test", 10, 2);

        bucket.try_consume(5).unwrap();
        assert_eq!(bucket.tokens(), 5);

        bucket.add_tokens(3);
        assert_eq!(bucket.tokens(), 8);

        // Cannot exceed capacity
        bucket.add_tokens(10);
        assert_eq!(bucket.tokens(), 10);
    }

    #[test]
    fn test_token_bucket_reset() {
        let mut bucket = TokenBucket::new("test", 10, 2);

        bucket.try_consume(8).unwrap();
        assert_eq!(bucket.tokens(), 2);

        bucket.reset();
        assert_eq!(bucket.tokens(), 10);
    }

    #[tokio::test]
    async fn test_token_bucket_consume_blocking() {
        let mut bucket = TokenBucket::new("test", 5, 100); // 100 tokens/sec

        // Consume all tokens
        bucket.consume(5).await;
        assert_eq!(bucket.tokens(), 0);

        // This should wait ~10ms for 1 token
        let start = Instant::now();
        bucket.consume(1).await;
        let elapsed = start.elapsed();

        // Should have waited at least 5ms (allowing for some timing variance)
        assert!(elapsed >= Duration::from_millis(5));
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new("test", 10, 100); // 100 tokens/sec

        // Consume all tokens
        bucket.try_consume(10).unwrap();
        assert_eq!(bucket.tokens(), 0);

        // Wait for refill
        sleep(Duration::from_millis(50)).await;

        // Try to consume - should have ~5 tokens now
        let result = bucket.try_consume(4);
        assert!(result.is_ok());
    }

    #[test]
    fn test_token_bucket_zero_refill_rate() {
        let mut bucket = TokenBucket::new("test", 10, 0);

        // Consume all tokens
        assert!(bucket.try_consume(10).is_ok());

        // Next consume should fail with MAX duration
        let result = bucket.try_consume(1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Duration::MAX);
    }
}
