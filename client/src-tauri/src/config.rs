use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    pub server_address: String,
    pub client_cert_path: Option<PathBuf>,
    pub client_key_path: Option<PathBuf>,
    pub ca_cert_path: Option<PathBuf>,
    pub operator_id: String,
    pub theme: Theme,
    pub auto_reconnect: bool,
    pub last_campaign_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            server_address: "https://127.0.0.1:8443".to_string(),
            client_cert_path: None,
            client_key_path: None,
            ca_cert_path: None,
            operator_id: whoami::username(),
            theme: Theme::Dark,
            auto_reconnect: true,
            last_campaign_id: None,
        }
    }
}

pub fn load_config() -> anyhow::Result<GuiConfig> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("No config directory"))?
        .join("oblivion_c2");
    
    std::fs::create_dir_all(&config_dir)?;
    
    let config_path = config_dir.join("gui_config.json");
    
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: GuiConfig = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        let config = GuiConfig::default();
        save_config(&config)?;
        Ok(config)
    }
}

pub fn save_config(config: &GuiConfig) -> anyhow::Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("No config directory"))?
        .join("oblivion_c2");
    
    let config_path = config_dir.join("gui_config.json");
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, content)?;
    Ok(())
}