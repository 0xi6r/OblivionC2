use crate::database::repository::AuditRepository;
use std::sync::Arc;

pub struct AuditLogger {
    repository: Arc<AuditRepository<'static>>,
}

impl AuditLogger {
    pub fn new(repository: Arc<AuditRepository<'static>>) -> Self {
        Self { repository }
    }
    
    pub async fn log(
        &self,
        operator_id: &str,
        action: &str,
        session_id: Option<&str>,
        campaign_id: Option<&str>,
        details: Option<&str>,
        ip_address: Option<&str>,
        success: bool,
    ) -> crate::Result<()> {
        // Log to database
        self.repository.log_action(
            operator_id,
            action,
            session_id,
            campaign_id,
            details,
            ip_address,
            success,
        ).await?;
        
        // Also log to structured logging
        tracing::info!(
            operator_id = %operator_id,
            action = %action,
            session_id = ?session_id,
            campaign_id = ?campaign_id,
            success = success,
            "Audit log"
        );
        
        Ok(())
    }
}