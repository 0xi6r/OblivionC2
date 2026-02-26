pub mod campaign;
pub mod config;
pub mod crypto;
pub mod database;
pub mod logging;
pub mod protocol;
pub mod server;
pub mod tasks;

pub use config::ServerConfig;
pub use server::C2Server;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OblivionError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("Invalid message format")]
    InvalidMessage,
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, OblivionError>;