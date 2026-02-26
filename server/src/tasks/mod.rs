pub mod engine;
pub mod types;

pub use engine::{TaskEngine, TaskEngineStatistics, TaskNotification};
pub use types::TaskType;

use crate::database::models::{Task, TaskStatus};
use crate::database::repository::TaskRepository;
use crate::protocol::messages::proto::{TaskAssignment, TaskResult};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct TaskEngine {
    repository: Arc<TaskRepository<'static>>,
    pending_tasks: RwLock<std::collections::HashMap<String, Vec<Task>>>,
}

impl TaskEngine {
    pub fn new(repository: Arc<TaskRepository<'static>>) -> Self {
        Self {
            repository,
            pending_tasks: RwLock::new(std::collections::HashMap::new()),
        }
    }
    
    pub async fn queue_task(
        &self,
        session_id: &str,
        campaign_id: &str,
        task_type: TaskType,
        payload: Vec<u8>,
        issued_by: &str,
        timeout_seconds: u32,
    ) -> crate::Result<u64> {
        let task = Task {
            id: 0,
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
        
        let task_id = self.repository.create(&task).await?;
        
        // Cache in memory for quick retrieval
        let mut pending = self.pending_tasks.write().await;
        pending.entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(task);
        
        Ok(task_id)
    }
    
    pub async fn get_pending_tasks(&self, session_id: &str) -> crate::Result<Vec<TaskAssignment>> {
        // First check cache
        let pending = self.pending_tasks.read().await;
        if let Some(tasks) = pending.get(session_id) {
            return Ok(tasks.iter().map(|t| TaskAssignment::from(t.clone())).collect());
        }
        drop(pending);
        
        // Fallback to database
        let tasks = self.repository.get_pending_for_session(session_id).await?;
        Ok(tasks.into_iter().map(TaskAssignment::from).collect())
    }
    
    pub async fn process_result(
        &self,
        task_id: u64,
        result: TaskResult,
    ) -> crate::Result<()> {
        let (status, data, error, exec_time) = match result.result {
            Some(proto::task_result::Result::Output(data)) => {
                (TaskStatus::Completed, Some(data), None, None)
            }
            Some(proto::task_result::Result::Error(msg)) => {
                (TaskStatus::Failed, None, Some(msg), None)
            }
            Some(proto::task_result::Result::Timeout(true)) => {
                (TaskStatus::Timeout, None, None, None)
            }
            None => (TaskStatus::Failed, None, Some("Empty result".to_string()), None),
        };
        
        self.repository.update_status(
            task_id,
            status,
            data,
            error,
            exec_time.map(|t| t as i32),
        ).await?;
        
        Ok(())
    }
}