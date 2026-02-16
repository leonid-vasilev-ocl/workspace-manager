use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ns::{Notification, get_notification};

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceConfig {
    pub name: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct Workspace {
    pub name: Option<String>,
    pub path: PathBuf,
    pub notification: Option<Notification>,
}

impl From<WorkspaceConfig> for Workspace {
    fn from(config: WorkspaceConfig) -> Self {
        let notification = get_notification(&config);
        Workspace {
            name: config.name,
            path: config.path,
            notification,
        }
    }
}

pub trait WorkspaceName {
    fn get_name_or_last_path(&self) -> Result<&str>;
}

impl WorkspaceName for WorkspaceConfig {
    fn get_name_or_last_path(&self) -> Result<&str> {
        self.name
            .as_deref()
            .or_else(|| self.path.file_name().and_then(|os| os.to_str()))
            .ok_or_else(|| anyhow!("can't get name from path: {}", self.path.to_string_lossy()))
    }
}

impl WorkspaceName for Workspace {
    fn get_name_or_last_path(&self) -> Result<&str> {
        self.name
            .as_deref()
            .or_else(|| self.path.file_name().and_then(|os| os.to_str()))
            .ok_or_else(|| anyhow!("can't get name from path: {}", self.path.to_string_lossy()))
    }
}

impl AsRef<Path> for Workspace {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
