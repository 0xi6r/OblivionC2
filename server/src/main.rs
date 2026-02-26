use clap::Parser;
use oblivion_c2::{ServerConfig, C2Server};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "oblivion_c2")]
#[command(about = "OblivionC2 Server - Authorized Security Testing Only")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Generate default configuration file
    #[arg(long)]
    generate_config: Option<PathBuf>,
    
    /// Database path (overrides config)
    #[arg(short, long)]
    database: Option<PathBuf>,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Generate config if requested
    if let Some(path) = cli.generate_config {
        let config = ServerConfig::default();
        config.save_to_file(&path)?;
        println!("Default configuration written to: {}", path.display());
        return Ok(());
    }
    
    // Load configuration
    let mut config = if let Some(path) = cli.config {
        ServerConfig::from_file(&path)?
    } else {
        ServerConfig::default()
    };
    
    // Apply CLI overrides
    if let Some(db_path) = cli.database {
        config.database_path = db_path;
    }
    
    // Initialize logging
    oblivion_c2::logging::init_logging(&cli.log_level, config.logging.format.clone());
    
    tracing::info!("OblivionC2 Server starting");
    tracing::info!("Database: {}", config.database_path.display());
    tracing::info!("Implant listener: {}", config.implant_listener);
    tracing::info!("Operator API: {}", config.operator_api);
    
    // Create and run server
    let server = C2Server::new(config).await?;
    server.run().await?;
    
    tracing::info!("OblivionC2 Server shutdown complete");
    
    Ok(())
}