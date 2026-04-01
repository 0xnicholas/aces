//! Scheduler trait and default implementation.
//!
//! Provides the core scheduling logic combining priority queues and token buckets.

use crate::{
    config::SchedulerConfig,
    error::{SchedulerError, TaskId},
    priority::Priority,
    token_bucket::TokenBucket,
};
use agent_protocol::{Action, ResourceKind};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

/// Statistics about scheduler state.
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Number of tasks currently executing
    pub executing: usize,
    /// Number of tasks queued at each priority level
    pub queued: [usize; 4],
    /// Total number of tasks completed
    pub completed: u64,
    /// Total number of tasks rejected (queue full)
    pub rejected: u64,
}

impl SchedulerStats {
    /// Total number of queued tasks across all priorities.
    pub fn total_queued(&self) -> usize {
        self.queued.iter().sum()
    }

    /// Total number of tasks (executing + queued).
    pub fn total_active(&self) -> usize {
        self.executing + self.total_queued()
    }
}

/// Status of a scheduled task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// Task is currently executing
    Executing,
    /// Task is queued waiting for resources
    Queued(Duration),
    /// Task completed successfully
    Completed,
    /// Task was cancelled
    Cancelled,
}

/// Information about a scheduled task.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Unique task identifier
    pub id: TaskId,
    /// Current status
    pub status: TaskStatus,
}

/// Core scheduler trait.
///
/// Defines the interface for scheduling actions with priority and resource constraints.
///
/// # Example
///
/// ```rust
/// use scheduler::{Scheduler, DefaultScheduler, SchedulerConfig, Priority};
/// use agent_protocol::Action;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let scheduler = DefaultScheduler::new(SchedulerConfig::default());
///
/// let action = Action::ToolCall {
///     tool_id: "calc".to_string(),
///     params: serde_json::json!({}),
/// };
///
/// let task = scheduler.schedule(action, Priority::Normal).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Scheduler: Send + Sync {
    /// Schedule an action for execution.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to schedule
    /// * `priority` - Priority level for queue ordering
    ///
    /// # Returns
    ///
    /// Returns `ScheduledTask` on success, or `SchedulerError` if:
    /// - Queue is full at the specified priority
    /// - Scheduler is closed
    async fn schedule(
        &self,
        action: Action,
        priority: Priority,
    ) -> Result<ScheduledTask, SchedulerError>;

    /// Cancel a scheduled or executing task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to cancel
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the task was found and cancelled.
    /// Returns `Err(SchedulerError::TaskNotFound)` if the task doesn't exist.
    async fn cancel(&self, task_id: TaskId) -> Result<(), SchedulerError>;

    /// Get current scheduler statistics.
    async fn stats(&self) -> SchedulerStats;

    /// Wait for all queued tasks to complete.
    async fn shutdown(&self);
}

/// A pending task in the queue.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used when processing queue
struct PendingTask {
    id: TaskId,
    action: Action,
    priority: Priority,
    queued_at: Instant,
}

/// Priority queues for each priority level.
#[derive(Debug, Default)]
struct PriorityQueues {
    critical: VecDeque<PendingTask>,
    high: VecDeque<PendingTask>,
    normal: VecDeque<PendingTask>,
    low: VecDeque<PendingTask>,
}

impl PriorityQueues {
    fn get_queue_mut(&mut self, priority: Priority) -> &mut VecDeque<PendingTask> {
        match priority {
            Priority::Critical => &mut self.critical,
            Priority::High => &mut self.high,
            Priority::Normal => &mut self.normal,
            Priority::Low => &mut self.low,
        }
    }

    fn len_at(&self, priority: Priority) -> usize {
        match priority {
            Priority::Critical => self.critical.len(),
            Priority::High => self.high.len(),
            Priority::Normal => self.normal.len(),
            Priority::Low => self.low.len(),
        }
    }

    fn total_len(&self) -> usize {
        self.critical.len() + self.high.len() + self.normal.len() + self.low.len()
    }

    fn find_and_remove(&mut self, task_id: TaskId) -> Option<PendingTask> {
        for queue in [
            &mut self.critical,
            &mut self.high,
            &mut self.normal,
            &mut self.low,
        ] {
            if let Some(pos) = queue.iter().position(|t| t.id == task_id) {
                return queue.remove(pos);
            }
        }
        None
    }
}

/// Token buckets for different resource types.
#[derive(Debug)]
struct TokenBuckets {
    llm: TokenBucket,
    tool: TokenBucket,
    compute: TokenBucket,
}

impl TokenBuckets {
    fn get_mut(&mut self, resource: ResourceKind) -> Option<&mut TokenBucket> {
        match resource {
            ResourceKind::LlmConcurrency => Some(&mut self.llm),
            ResourceKind::ToolCallRate => Some(&mut self.tool),
            ResourceKind::ComputeQuota => Some(&mut self.compute),
            _ => None,
        }
    }
}

/// Default scheduler implementation.
///
/// Combines priority queues with token bucket rate limiting.
#[derive(Debug)]
pub struct DefaultScheduler {
    config: SchedulerConfig,
    queues: Arc<RwLock<PriorityQueues>>,
    buckets: Arc<RwLock<TokenBuckets>>,
    stats: Arc<RwLock<SchedulerStats>>,
}

impl DefaultScheduler {
    /// Create a new scheduler with the given configuration.
    pub fn new(config: SchedulerConfig) -> Self {
        let buckets = TokenBuckets {
            llm: TokenBucket::new(
                "llm",
                config.llm_bucket.capacity,
                config.llm_bucket.refill_rate,
            ),
            tool: TokenBucket::new(
                "tool",
                config.tool_bucket.capacity,
                config.tool_bucket.refill_rate,
            ),
            compute: TokenBucket::new(
                "compute",
                config.compute_bucket.capacity,
                config.compute_bucket.refill_rate,
            ),
        };

        Self {
            config,
            queues: Arc::new(RwLock::new(PriorityQueues::default())),
            buckets: Arc::new(RwLock::new(buckets)),
            stats: Arc::new(RwLock::new(SchedulerStats::default())),
        }
    }

    /// Check if the queue is full at the given priority.
    async fn is_queue_full(&self, priority: Priority) -> bool {
        let queues = self.queues.read().await;
        queues.len_at(priority) >= self.config.max_queue_depth
    }

    /// Enqueue a task at the given priority.
    async fn enqueue(&self, task: PendingTask) {
        let priority = task.priority;
        let mut queues = self.queues.write().await;
        let queue = queues.get_queue_mut(priority);
        queue.push_back(task);
        trace!(
            "Task enqueued at priority {:?}, queue depth now {}",
            priority,
            queue.len()
        );
    }

    /// Get the resource kind required for an action.
    fn resource_for_action(action: &Action) -> ResourceKind {
        match action {
            Action::ToolCall { .. } => ResourceKind::ToolCallRate,
            Action::CallAgent { .. } => ResourceKind::ComputeQuota,
            Action::ContextRead { .. } | Action::ContextWrite { .. } => ResourceKind::ContextBudget,
            _ => ResourceKind::ComputeQuota, // Future action variants
        }
    }
}

#[async_trait]
impl Scheduler for DefaultScheduler {
    async fn schedule(
        &self,
        action: Action,
        priority: Priority,
    ) -> Result<ScheduledTask, SchedulerError> {
        trace!("Scheduling action with priority {:?}", priority);

        // Check queue depth
        if self.is_queue_full(priority).await {
            warn!(
                "Queue full at priority {:?}, max depth: {}",
                priority, self.config.max_queue_depth
            );
            let mut stats = self.stats.write().await;
            stats.rejected += 1;
            return Err(SchedulerError::QueueFull {
                priority,
                max_depth: self.config.max_queue_depth,
            });
        }

        // Determine resource type and check token bucket
        let resource = Self::resource_for_action(&action);
        let mut buckets = self.buckets.write().await;

        let bucket = buckets
            .get_mut(resource)
            .ok_or(SchedulerError::UnknownResource(resource))?;

        match bucket.try_consume(1) {
            Ok(()) => {
                // Token available, can execute immediately
                debug!("Token available for {:?}, executing immediately", resource);
                let mut stats = self.stats.write().await;
                stats.executing += 1;

                Ok(ScheduledTask {
                    id: TaskId::new(),
                    status: TaskStatus::Executing,
                })
            }
            Err(retry_after) => {
                // Need to queue the task
                if retry_after > self.config.queue_timeout {
                    warn!(
                        "Retry time {:?} exceeds queue timeout {:?}",
                        retry_after, self.config.queue_timeout
                    );
                    let mut stats = self.stats.write().await;
                    stats.rejected += 1;
                    return Err(SchedulerError::RateLimitExceeded {
                        resource,
                        retry_after,
                    });
                }

                let task = PendingTask {
                    id: TaskId::new(),
                    action,
                    priority,
                    queued_at: Instant::now(),
                };

                let task_id = task.id;
                self.enqueue(task).await;

                let mut stats = self.stats.write().await;
                let queue_idx = priority.as_u8() as usize;
                stats.queued[queue_idx] += 1;

                Ok(ScheduledTask {
                    id: task_id,
                    status: TaskStatus::Queued(retry_after),
                })
            }
        }
    }

    async fn cancel(&self, task_id: TaskId) -> Result<(), SchedulerError> {
        let mut queues = self.queues.write().await;

        if queues.find_and_remove(task_id).is_some() {
            debug!("Cancelled task {}", task_id);
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id))
        }
    }

    async fn stats(&self) -> SchedulerStats {
        self.stats.read().await.clone()
    }

    async fn shutdown(&self) {
        trace!("Scheduler shutting down, waiting for queue to empty");

        loop {
            let queues = self.queues.read().await;
            if queues.total_len() == 0 {
                break;
            }
            drop(queues);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        debug!("Scheduler shutdown complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TokenBucketConfig;

    fn test_config() -> SchedulerConfig {
        SchedulerConfig {
            llm_bucket: TokenBucketConfig::new(5, 10),
            tool_bucket: TokenBucketConfig::new(3, 5),
            compute_bucket: TokenBucketConfig::new(10, 20),
            max_queue_depth: 10,
            queue_timeout: Duration::from_secs(5),
        }
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = DefaultScheduler::new(test_config());
        let stats = scheduler.stats().await;
        assert_eq!(stats.total_queued(), 0);
        assert_eq!(stats.executing, 0);
    }

    #[tokio::test]
    async fn test_schedule_immediate_execution() {
        let scheduler = DefaultScheduler::new(test_config());

        let action = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };

        let task = scheduler.schedule(action, Priority::Normal).await.unwrap();
        assert!(matches!(task.status, TaskStatus::Executing));
    }

    #[tokio::test]
    async fn test_schedule_rate_limited() {
        let config = SchedulerConfig {
            tool_bucket: TokenBucketConfig::new(2, 1), // Only 2 tokens
            ..test_config()
        };
        let scheduler = DefaultScheduler::new(config);

        // Consume all tokens
        for _ in 0..2 {
            let action = Action::ToolCall {
                tool_id: "test".to_string(),
                params: serde_json::json!({}),
            };
            let task = scheduler.schedule(action, Priority::Normal).await.unwrap();
            assert!(matches!(task.status, TaskStatus::Executing));
        }

        // Next request should be queued
        let action = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };
        let task = scheduler.schedule(action, Priority::Normal).await.unwrap();
        assert!(matches!(task.status, TaskStatus::Queued(..)));
    }

    #[tokio::test]
    async fn test_queue_full() {
        // Use a bucket with capacity=1 and slow refill, then consume the token
        let config = SchedulerConfig {
            tool_bucket: TokenBucketConfig::new(1, 1), // 1 token, slow refill
            max_queue_depth: 2,
            queue_timeout: Duration::from_secs(60), // Long timeout
            ..test_config()
        };
        let scheduler = DefaultScheduler::new(config);

        // First request consumes the only token
        let action1 = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };
        let result = scheduler.schedule(action1, Priority::Normal).await;
        assert!(result.is_ok(), "First task should execute: {:?}", result);

        // Next 2 requests should be queued (waiting for refill)
        for i in 0..2 {
            let action = Action::ToolCall {
                tool_id: "test".to_string(),
                params: serde_json::json!({}),
            };
            let result = scheduler.schedule(action, Priority::Normal).await;
            assert!(result.is_ok(), "Task {} should be queued: {:?}", i, result);
            assert!(
                matches!(result.unwrap().status, TaskStatus::Queued(..)),
                "Task {} should have Queued status",
                i
            );
        }

        // Next request should fail with QueueFull
        let action = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };
        let result = scheduler.schedule(action, Priority::Normal).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SchedulerError::QueueFull { .. }
        ));
    }

    #[tokio::test]
    async fn test_cancel_task() {
        // Use a bucket with capacity=1, consume it, then queue a task
        let config = SchedulerConfig {
            tool_bucket: TokenBucketConfig::new(1, 1), // 1 token
            queue_timeout: Duration::from_secs(60),
            ..test_config()
        };
        let scheduler = DefaultScheduler::new(config);

        // First consume the only token
        let action1 = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };
        let _ = scheduler.schedule(action1, Priority::Normal).await.unwrap();

        let action = Action::ToolCall {
            tool_id: "test".to_string(),
            params: serde_json::json!({}),
        };
        let task = scheduler.schedule(action, Priority::Normal).await.unwrap();

        // Cancel the task
        let result = scheduler.cancel(task.id).await;
        assert!(result.is_ok());

        // Cancel again should fail
        let result = scheduler.cancel(task.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scheduler_stats() {
        let scheduler = DefaultScheduler::new(test_config());

        let initial_stats = scheduler.stats().await;
        assert_eq!(initial_stats.total_queued(), 0);

        // Schedule some tasks
        for _ in 0..3 {
            let action = Action::ToolCall {
                tool_id: "test".to_string(),
                params: serde_json::json!({}),
            };
            scheduler.schedule(action, Priority::Normal).await.unwrap();
        }

        let stats = scheduler.stats().await;
        assert!(stats.total_active() >= 3);
    }
}
