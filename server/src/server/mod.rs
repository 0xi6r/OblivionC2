pub mod listener;
pub mod session_manager;

use crate::config::ServerConfig;
use crate::crypto::keys::ServerIdentity;
use crate::database::{init_database, DbPool};
use crate::database::repository::{AuditRepository, CampaignRepository, SessionRepository, TaskRepository};
use crate::logging::AuditLogger;
use crate::tasks::TaskEngine;
use session_manager::SessionManager;
use listener::ImplantListener;
use std::path::Path;
use std::sync::Arc;
use tokio::signal;

pub struct C2Server {
    config: ServerConfig,
    db_pool: DbPool,
    identity: Arc<ServerIdentity>,
    session_manager: Arc<SessionManager>,
    task_engine: Arc<TaskEngine>,
    audit_logger: Arc<AuditLogger>,
}

impl C2Server {
    pub async fn new(config: ServerConfig) -> crate::Result<Self> {
        // Initialize database
        let db_pool = init_database(&config.database_path).await?;
        
        // Load or generate identity
        let identity = if Path::new(&config.identity_key_path).exists() {
            ServerIdentity::from_file(&config.identity_key_path)?
        } else {
            let new_identity = ServerIdentity::generate()?;
            // Note: In production, you'd want to securely store the seed
            tracing::info!("Generated new server identity");
            new_identity
        };
        
        let identity = Arc::new(identity);
        
        // Create repositories
        let session_repo = Arc::new(SessionRepository::new(&db_pool));
        let task_repo = Arc::new(TaskRepository::new(&db_pool));
        let audit_repo = Arc::new(AuditRepository::new(&db_pool));
        
        // Create managers
        let session_manager = Arc::new(SessionManager::new(
            session_repo,
            config.session.stale_threshold_seconds,
        ));
        
        let task_engine = Arc::new(TaskEngine::new(task_repo));
        
        let audit_logger = Arc::new(AuditLogger::new(audit_repo));
        
        Ok(Self {
            config,
            db_pool,
            identity,
            session_manager,
            task_engine,
            audit_logger,
        })
    }
    
    pub async fn run(self) -> crate::Result<()> {
        tracing::info!("Starting OblivionC2 server");
        
        // Start implant listener
        let implant_listener = ImplantListener::new(
            self.config.clone(),
            self.identity.clone(),
            self.session_manager.clone(),
            self.task_engine.clone(),
        ).await?;
        
        // Start stale session cleanup task
        let session_manager = self.session_manager.clone();
        let cleanup_interval = std::time::Duration::from_secs(60);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                match session_manager.cleanup_stale_sessions().await {
                    Ok(count) if count > 0 => {
                        tracing::info!("Cleaned up {} stale sessions", count);
                    }
                    Ok(_) => {}
                    Err(e) => tracing::error!("Session cleanup error: {}", e),
                }
            }
        });
        
        // Start operator API (placeholder for now)
        let operator_api = run_operator_api(self.config.operator_api, self.db_pool.clone());
        
        // Wait for shutdown signal
        tokio::select! {
            result = implant_listener.run() => {
                tracing::error!("Implant listener exited: {:?}", result);
                result
            }
            result = operator_api => {
                tracing::error!("Operator API exited: {:?}", result);
                result
            }
            _ = signal::ctrl_c() => {
                tracing::info!("Shutdown signal received");
                Ok(())
            }
        }
    }
}

async fn run_operator_api(addr: std::net::SocketAddr, _db_pool: DbPool) -> crate::Result<()> {
    // Placeholder for operator API
    // This would be a gRPC or HTTP API for the GUI client
    tracing::info!("Operator API would start on {}", addr);
    
    // For now, just sleep to keep the task alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}