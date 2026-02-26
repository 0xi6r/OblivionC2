pub mod commands;
pub mod config;
pub mod grpc_client;
pub mod state;

use state::AppState;
use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::connect_to_server,
            commands::disconnect,
            commands::get_connection_status,
            commands::list_campaigns,
            commands::create_campaign,
            commands::control_campaign,
            commands::get_campaign_details,
            commands::list_sessions,
            commands::get_session_details,
            commands::terminate_session,
            commands::create_task,
            commands::create_broadcast_task,
            commands::get_task_result,
            commands::cancel_task,
            commands::subscribe_events,
            commands::get_server_stats,
            commands::execute_shell,
            commands::upload_file,
            commands::download_file,
            commands::get_system_info,
        ])
        .setup(|app| {
            // Initialize state
            let state = app.state::<AppState>();
            
            // Load saved configuration
            if let Ok(config) = config::load_config() {
                *state.config.lock() = config;
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}