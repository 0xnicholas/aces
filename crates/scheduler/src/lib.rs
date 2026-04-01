//! Scheduler - Token bucket rate limiting and priority queue for Agent Kernel
//!
//! This crate implements the scheduling policy from ADR-006: Hybrid token bucket
//! + priority queue scheduling for managing access to limited resources.
//!
//! # Architecture
//!
//! ```text
//! Incoming Request → Priority Assignment → Token Bucket Check → Execute/Queue
//!                         ↓                           ↓
//!                  Priority Queue           Rate Limiting
//! ```
//!
//! # Key Features
//!
//! - **Token Bucket**: Smooth rate limiting with burst support
//! - **Priority Queue**: 4-level priority (Critical > High > Normal > Low)
//! - **Multiple Resources**: Separate buckets for LLM, Tools, Compute
//! - **Async/Await**: Non-blocking scheduling with proper backpressure
//!
//! # Usage
//!
//! ```rust,no_run
//! use scheduler::{Scheduler, DefaultScheduler, SchedulerConfig, Priority};
//! use agent_protocol::Action;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = SchedulerConfig::default();
//! let scheduler = DefaultScheduler::new(config);
//!
//! let action = Action::ToolCall {
//!     tool_id: "calculator".to_string(),
//!     params: serde_json::json!({"expr": "1+1"}),
//! };
//!
//! let task = scheduler.schedule(action, Priority::Normal).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Performance
//!
//! - Scheduling decision: < 0.1ms
//! - Token refill: Millisecond precision
//! - Queue operations: O(1) enqueue, O(1) dequeue per priority level
//!
//! # License
//!
//! Apache 2.0 - See LICENSE-APACHE file for details.

pub mod config;
pub mod error;
pub mod priority;
pub mod scheduler;
pub mod token_bucket;

pub use config::{SchedulerConfig, TokenBucketConfig};
pub use error::{SchedulerError, TaskId, TokenBucketError};
pub use priority::Priority;
pub use scheduler::{DefaultScheduler, ScheduledTask, Scheduler, SchedulerStats, TaskStatus};
pub use token_bucket::TokenBucket;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_exports() {
        // Ensure all key types are exported
        let _ = Priority::default();
        let _config = SchedulerConfig::default();
    }
}
