use super::models::*;
use super::DbPool;
use crate::Result;

pub struct CampaignRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> CampaignRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
    
    pub async fn create(&self, campaign: &Campaign) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO campaigns (id, name, description, operator_id, status, created_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            campaign.id,
            campaign.name,
            campaign.description,
            campaign.operator_id,
            campaign.status,
            campaign.created_at,
            campaign.metadata
        )
        .execute(self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Campaign>> {
        let campaign = sqlx::query_as!(
            Campaign,
            r#"SELECT * FROM campaigns WHERE id = ?"#,
            id
        )
        .fetch_optional(self.pool)
        .await?;
        
        Ok(campaign)
    }
    
    pub async fn list_active(&self) -> Result<Vec<Campaign>> {
        let campaigns = sqlx::query_as!(
            Campaign,
            r#"SELECT * FROM campaigns WHERE status IN ('active', 'paused') ORDER BY created_at DESC"#
        )
        .fetch_all(self.pool)
        .await?;
        
        Ok(campaigns)
    }
    
    pub async fn update_status(&self, id: &str, status: CampaignStatus) -> Result<()> {
        sqlx::query!(
            r#"UPDATE campaigns SET status = ? WHERE id = ?"#,
            status,
            id
        )
        .execute(self.pool)
        .await?;
        
        Ok(())
    }
}

pub struct SessionRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> SessionRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
    
    pub async fn create(&self, session: &Session) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO sessions (id, campaign_id, implant_id, hostname, username, os_version, 
                                  process_id, public_key, status, metadata, encryption_key)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            session.id,
            session.campaign_id,
            session.implant_id,
            session.hostname,
            session.username,
            session.os_version,
            session.process_id,
            session.public_key,
            session.status,
            session.metadata,
            session.encryption_key
        )
        .execute(self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Session>> {
        let session = sqlx::query_as!(
            Session,
            r#"SELECT * FROM sessions WHERE id = ?"#,
            id
        )
        .fetch_optional(self.pool)
        .await?;
        
        Ok(session)
    }
    
    pub async fn list_by_campaign(&self, campaign_id: &str) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as!(
            Session,
            r#"SELECT * FROM sessions WHERE campaign_id = ? ORDER BY last_seen DESC"#,
            campaign_id
        )
        .fetch_all(self.pool)
        .await?;
        
        Ok(sessions)
    }
    
    pub async fn update_status(&self, id: &str, status: SessionStatus) -> Result<()> {
        sqlx::query!(
            r#"UPDATE sessions SET status = ? WHERE id = ?"#,
            status,
            id
        )
        .execute(self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn mark_stale_sessions(&self, threshold_seconds: i64) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE sessions 
            SET status = 'stale' 
            WHERE status = 'active' 
            AND last_seen < datetime('now', '-' || ? || ' seconds')
            "#,
            threshold_seconds
        )
        .execute(self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
}

pub struct TaskRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> TaskRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
    
    pub async fn create(&self, task: &Task) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO tasks (session_id, campaign_id, task_type, payload, issued_by, status, timeout_seconds)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING id
            "#,
            task.session_id,
            task.campaign_id,
            task.task_type,
            task.payload,
            task.issued_by,
            task.status,
            task.timeout_seconds
        )
        .fetch_one(self.pool)
        .await?;
        
        Ok(result.id)
    }
    
    pub async fn get_pending_for_session(&self, session_id: &str) -> Result<Vec<Task>> {
        let tasks = sqlx::query_as!(
            Task,
            r#"
            SELECT * FROM tasks 
            WHERE session_id = ? AND status = 'pending'
            ORDER BY issued_at ASC
            LIMIT 10
            "#,
            session_id
        )
        .fetch_all(self.pool)
        .await?;
        
        Ok(tasks)
    }
    
    pub async fn update_status(
        &self,
        task_id: i64,
        status: TaskStatus,
        result: Option<Vec<u8>>,
        error: Option<String>,
        execution_time_ms: Option<i32>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE tasks 
            SET status = ?, result = ?, error_message = ?, completed_at = CURRENT_TIMESTAMP, execution_time_ms = ?
            WHERE id = ?
            "#,
            status,
            result,
            error,
            execution_time_ms,
            task_id
        )
        .execute(self.pool)
        .await?;
        
        Ok(())
    }
}

pub struct AuditRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> AuditRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
    
    pub async fn log_action(
        &self,
        operator_id: &str,
        action: &str,
        session_id: Option<&str>,
        campaign_id: Option<&str>,
        details: Option<&str>,
        ip_address: Option<&str>,
        success: bool,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO operator_logs (operator_id, action, target_session_id, target_campaign_id, details, ip_address, success)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            operator_id,
            action,
            session_id,
            campaign_id,
            details,
            ip_address,
            success
        )
        .execute(self.pool)
        .await?;
        
        Ok(())
    }
}