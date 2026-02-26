pub mod audit;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging(level: &str, format: crate::config::LogFormat) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    match format {
        crate::config::LogFormat::Pretty => {
            tracing_subscriber::fmt()
                .pretty()
                .with_env_filter(env_filter)
                .init();
        }
        crate::config::LogFormat::Json => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().json())
                .with(env_filter)
                .init();
        }
        crate::config::LogFormat::Compact => {
            tracing_subscriber::fmt()
                .compact()
                .with_env_filter(env_filter)
                .init();
        }
    }
}