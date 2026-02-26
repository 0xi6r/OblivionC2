use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address for implant connections
    pub implant_listener: SocketAddr,
    
    /// Server bind address for operator GUI connections
    pub operator_api: SocketAddr,
    
    /// Path to SQLite database
    pub database_path: PathAddr,
    
    /// Path to server long-term identity key
    pub identity_key_path: PathBuf,
    
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Session management settings
    pub session: SessionConfig,
    
    /// Campaign defaults
    pub campaign: CampaignConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub file_path: Option<PathBuf>,
    pub audit_log_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Pretty,
    Json,
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Default heartbeat interval in seconds
    pub default_heartbeat_interval: u32,
    /// Session timeout after missing heartbeats
    pub stale_threshold_seconds: u32,
    /// Maximum concurrent sessions per campaign
    pub max_sessions_per_campaign: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignConfig {
    /// Default operator authentication method
    pub auth_method: AuthMethod,
    /// Require MFA for operator login
    pub require_mfa: bool,
    /// Auto-archive campaigns after days of inactivity
    pub auto_archive_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Certificate,
    Token,
    Oidc { issuer: String, client_id: String },
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            implant_listener: "0.0.0.0:4433".parse().unwrap(),
            operator_api: "127.0.0.1:8443".parse().unwrap(),
            database_path: PathBuf::from("oblivion_c2.db"),
            identity_key_path: PathBuf::from("server_identity.key"),
            logging: LoggingConfig {
                level: "info".to_string(),
                format: LogFormat::Pretty,
                file_path: None,
                audit_log_path: PathBuf::from("audit.log"),
            },
            session: SessionConfig {
                default_heartbeat_interval: 60,
                stale_threshold_seconds: 300,
                max_sessions_per_campaign: 100,
            },
            campaign: CampaignConfig {
                auth_method: AuthMethod::Certificate,
                require_mfa: false,
                auto_archive_days: 30,
            },
        }
    }
}

impl ServerConfig {
    pub fn from_file(path: &PathBuf) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ServerConfig = toml::from_str(&content)
            .map_err(|e| crate::OblivionError::Config(e.to_string()))?;
        Ok(config)
    }
    
    pub fn save_to_file(&self, path: &PathBuf) -> crate::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::OblivionError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}