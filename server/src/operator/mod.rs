pub mod grpc;
pub mod handlers;

use crate::config::ServerConfig;
use crate::database::DbPool;
use crate::campaign::CampaignManager;
use crate::server::session_manager::SessionManager;
use crate::tasks::TaskEngine;
use crate::logging::AuditLogger;
use std::sync::Arc;

pub struct OperatorApi {
    config: ServerConfig,
    db_pool: DbPool,
    campaign_manager: Arc<CampaignManager>,
    session_manager: Arc<SessionManager>,
    task_engine: Arc<TaskEngine>,
    audit_logger: Arc<AuditLogger>,
}

impl OperatorApi {
    pub fn new(
        config: ServerConfig,
        db_pool: DbPool,
        campaign_manager: Arc<CampaignManager>,
        session_manager: Arc<SessionManager>,
        task_engine: Arc<TaskEngine>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        Self {
            config,
            db_pool,
            campaign_manager,
            session_manager,
            task_engine,
            audit_logger,
        }
    }
    
    pub async fn run(self) -> crate::Result<()> {
        let addr = self.config.operator_api;
        tracing::info!("Operator API starting on {}", addr);
        
        // Build gRPC server
        let service = grpc::operator_server::OperatorServer::new(self);
        
        tonic::transport::Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .map_err(|e| crate::OblivionError::Io(e.into()))?;
        
        Ok(())
    }
}