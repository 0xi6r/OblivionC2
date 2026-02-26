use std::env;

pub struct Config {
    pub c2_address: String,
    pub campaign_id: String,
    pub public_key: [u8; 32],
    pub heartbeat_interval: u64,
    pub jitter: bool,
}

pub fn load_config() -> Config {
    // In production, these would be embedded or obfuscated
    Config {
        c2_address: env::var("OBLIVION_C2")
            .unwrap_or_else(|_| "127.0.0.1:4433".to_string()),
        campaign_id: env::var("OBLIVION_CAMPAIGN")
            .unwrap_or_else(|_| "default".to_string()),
        public_key: hex::decode(env::var("OBLIVION_PUBKEY")
            .unwrap_or_else(|_| "00".repeat(32)))
            .unwrap_or_default()
            .try_into()
            .unwrap_or([0u8; 32]),
        heartbeat_interval: 60,
        jitter: true,
    }
}