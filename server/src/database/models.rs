use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub operator_id: String,
    pub status: CampaignStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum CampaignStatus {
    Planning,
    Active,
    Paused,
    Closing,
    Archived,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub campaign_id: String,
    pub implant_id: String,
    pub hostname: String,
    pub username: Option<String>,
    pub os_version: Option<String>,
    pub process_id: Option<i32>,
    pub public_key: Option<Vec<u8>>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub status: SessionStatus,
    pub metadata: Option<String>,
    pub encryption_key: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Idle,
    Stale,
    Terminated,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub session_id: String,
    pub campaign_id: String,
    pub task_type: String,
    pub payload: Option<Vec<u8>>,
    pub issued_at: DateTime<Utc>,
    pub issued_by: String,
    pub status: TaskStatus,
    pub timeout_seconds: Option<i32>,
    pub result: Option<Vec<u8>>,
    pub error_message: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OperatorLog {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub operator_id: String,
    pub action: String,
    pub target_session_id: Option<String>,
    pub target_campaign_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub success: bool,
}