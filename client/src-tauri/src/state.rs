use crate::config::GuiConfig;
use crate::grpc_client::GrpcClient;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub config: Mutex<GuiConfig>,
    pub client: RwLock<Option<GrpcClient>>,
    pub event_stream: RwLock<Option<tokio::sync::mpsc::Receiver<Event>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Event {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Mutex::new(GuiConfig::default()),
            client: RwLock::new(None),
            event_stream: RwLock::new(None),
        }
    }
}

impl AppState {
    pub async fn get_client(&self) -> Option<GrpcClient> {
        self.client.read().await.clone()
    }
    
    pub async fn set_client(&self, client: Option<GrpcClient>) {
        *self.client.write().await = client;
    }
    
    pub fn get_config(&self) -> GuiConfig {
        self.config.lock().clone()
    }
    
    pub fn set_config(&self, config: GuiConfig) {
        *self.config.lock() = config;
    }
}